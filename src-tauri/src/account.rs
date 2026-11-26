//! The Friendship Reminder account: Sign in with Apple, traded for a Supabase session that is kept in the Keychain.
//! The rules live in friends-account; this file only opens the sign-in page, waits for it to come back,
//! and connects the Keychain and the windows.
//!
//! On the Mac, signing in happens in the browser: the app opens Supabase's Sign in with Apple page,
//! and when it's done the browser comes back to a one-off address on this Mac
//! (`http://127.0.0.1:<port>/auth/callback`) with a code the app trades for a session. (Apple's own
//! sign-in sheet needs the Mac App Store: apps signed with a Developer ID can't use it.) On the
//! iPhone, Apple's own Sign in with Apple sheet hands the app an identity token, which is traded for
//! a session the same way. (A web page in a sheet would make iOS ask first whether the app may use
//! the account server's address, which looks alarming.)
//!
//! Signing in is optional. A build without the Supabase settings (`FRIENDS_SUPABASE_URL` and
//! `FRIENDS_SUPABASE_PUBLISHABLE_KEY` at build time, or in `.env.local` for debug builds) shows the
//! Account section as unavailable, and touches neither the network nor the Keychain.

#[cfg(target_os = "macos")]
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard};
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};

use friends_account::{
    AccountState, AccountView, Attempts, AuthError, Client, Config, HttpTransport, Restore,
    Session, Started,
};
#[cfg(target_os = "macos")]
use friends_account::{Callback, Pkce};
use friends_sync::SyncView;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

/// Sent to every window when the account changes, with the new [`AccountView`].
const ACCOUNT_CHANGED: &str = "account-changed";

/// The Keychain item holding the session, as JSON.
const KEYCHAIN_SERVICE: &str = "io.github.charliespy.friends";
const KEYCHAIN_ACCOUNT: &str = "account-session";

/// How long a sign-in waits for the browser to come back.
#[cfg(target_os = "macos")]
const BROWSER_TIMEOUT: Duration = Duration::from_secs(600);

/// What the browser shows once it's back (the iPhone's sheet just closes).
const SIGNED_IN: &str = "You're signed in to Friendship Reminder. You can close this tab.";
const FAILED: &str =
    "Signing in to Friendship Reminder didn't work. Go back to Friendship Reminder to try again.";
const CANCELLED: &str = "Signing in to Friendship Reminder was cancelled. You can close this tab.";

/// The browser sign-ins, each with its sign-in address (unblocking it wakes the sign-in).
#[cfg(target_os = "macos")]
type Waiting = Attempts<Arc<tiny_http::Server>>;
/// The sign-ins in the sheet. The sheet has its own Cancel, so there's nothing to wake.
#[cfg(target_os = "ios")]
type Waiting = Attempts<()>;

pub struct Account {
    /// Why this build can't sign in, or the project to sign in to.
    config: Result<Config, String>,
    state: Mutex<AccountState>,
    waiting: Mutex<Waiting>,
    /// How syncing is going, for the Account section.
    sync: Mutex<SyncView>,
}

/// What the Account section shows: the account, and how syncing is going while signed in.
#[derive(Debug, Clone, Serialize)]
pub struct Shown {
    #[serde(flatten)]
    account: AccountView,
    sync: Option<SyncView>,
}

impl Account {
    pub fn new() -> Account {
        Account {
            config: available(),
            state: Mutex::new(AccountState::default()),
            waiting: Mutex::new(Waiting::default()),
            sync: Mutex::new(SyncView::default()),
        }
    }

    fn state(&self) -> MutexGuard<'_, AccountState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn waiting(&self) -> MutexGuard<'_, Waiting> {
        self.waiting.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn view(&self) -> Shown {
        self.view_of(&self.state())
    }

    fn view_of(&self, state: &AccountState) -> Shown {
        let account = match &self.config {
            Err(reason) => AccountView::Unavailable {
                reason: reason.clone(),
            },
            Ok(_) => state.view(),
        };
        let sync = matches!(account, AccountView::SignedIn { .. })
            .then(|| self.sync.lock().unwrap_or_else(|e| e.into_inner()).clone());
        Shown { account, sync }
    }

    pub fn config(&self) -> Option<Config> {
        self.config.clone().ok()
    }

    fn client(&self) -> Result<Client<HttpTransport>, String> {
        let config = self.config.clone()?;
        Ok(Client::new(config, HttpTransport::default()))
    }
}

/// The account server's settings: built in, or (debug builds only) from `.env.local` at the repo root.
fn available() -> Result<Config, String> {
    let built_in = Config::new(
        option_env!("FRIENDS_SUPABASE_URL"),
        option_env!("FRIENDS_SUPABASE_PUBLISHABLE_KEY"),
    );
    #[cfg(debug_assertions)]
    let built_in = built_in.or_else(|| {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../.env.local");
        Config::from_env_file(&std::fs::read_to_string(path).ok()?)
    });
    built_in.ok_or_else(|| {
        "This copy of Friendship Reminder was built without an account server.".to_string()
    })
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Records how syncing is going and tells the windows.
pub fn set_sync(app: &AppHandle, change: impl FnOnce(&mut SyncView)) {
    change(
        &mut app
            .state::<Account>()
            .sync
            .lock()
            .unwrap_or_else(|e| e.into_inner()),
    );
    publish(app);
}

/// Who is signed in, by account id.
pub fn signed_in_user(app: &AppHandle) -> Option<String> {
    app.state::<Account>()
        .state()
        .session()
        .map(|s| s.user_id.clone())
}

/// Forgets how syncing went (a new sign-in, or a sign-out).
fn reset_sync(account: &Account) {
    *account.sync.lock().unwrap_or_else(|e| e.into_inner()) = SyncView::default();
}

/// A session to sync with: the signed-in one, refreshed first when its access token has run out (or
/// `force`, when the server just refused it). `Ok(None)` when signed out; `Err` says why a refresh
/// that was needed didn't work. Only the sync thread calls this (and `restore`, before it starts), so
/// two refreshes never race with the same refresh token.
pub fn session_for_sync(app: &AppHandle, force: bool) -> Result<Option<Session>, String> {
    let account = app.state::<Account>();
    let Ok(client) = account.client() else {
        return Ok(None);
    };
    let (generation, session) = {
        let state = account.state();
        match state.session() {
            Some(s) => (state.generation(), s.clone()),
            None => return Ok(None),
        }
    };
    if !force && !session.needs_refresh(now()) {
        return Ok(Some(session));
    }
    let refreshed = client.refresh(&session.refresh_token, now());
    let failure = refreshed.as_ref().err().map(|e| e.to_string());
    let finished = account.state().finish_restore(generation, refreshed, save);
    match finished {
        Ok(Some(unwanted)) => end_on_server(client, unwanted),
        Ok(None) => {}
        Err(e) => eprintln!("friends: couldn't update the Keychain: {e}"),
    }
    publish(app);
    match failure {
        Some(e) => Err(e),
        None => Ok(account.state().session().cloned()),
    }
}

/// Sends the account to the windows. Sent while holding the state, so two changes at once can't send
/// their views out of order.
fn publish(app: &AppHandle) {
    let shown = {
        let account = app.state::<Account>();
        let state = account.state();
        account.view_of(&state)
    };
    let signed_in = matches!(shown.account, AccountView::SignedIn { .. });
    let _ = app.emit(ACCOUNT_CHANGED, shown);
    if signed_in {
        crate::finish_guide_sign_in(app);
    }
}

/// errSecItemNotFound: the Keychain holds no such item.
#[cfg(any(target_os = "macos", target_os = "ios"))]
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

/// Saves the session in the Keychain, or removes it for `None`.
fn save(session: Option<&Session>) -> Result<(), String> {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        use security_framework::passwords::{delete_generic_password, set_generic_password};
        match session {
            Some(s) => {
                let json = serde_json::to_vec(s).map_err(|e| e.to_string())?;
                set_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT, &json)
                    .map_err(|e| e.to_string())
            }
            None => match delete_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT) {
                Err(e) if e.code() != ERR_SEC_ITEM_NOT_FOUND => Err(e.to_string()),
                _ => Ok(()),
            },
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    {
        let _ = session;
        Ok(())
    }
}

/// The saved session: `Ok(None)` when none was saved (or it can't be read back as one), `Err` when
/// the Keychain wouldn't say.
fn load() -> Result<Option<Session>, String> {
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        match security_framework::passwords::get_generic_password(
            KEYCHAIN_SERVICE,
            KEYCHAIN_ACCOUNT,
        ) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes).ok()),
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    Ok(None)
}

/// At launch: picks up a saved session and refreshes it, which also checks it's still good
/// (`AccountState` decides what each outcome means).
pub fn restore(app: &AppHandle) {
    let account = app.state::<Account>();
    let Ok(client) = account.client() else {
        return;
    };
    let before = account.state().generation();
    let saved = load();
    if let Err(e) = &saved {
        eprintln!("friends: couldn't read the saved sign-in: {e}");
    }
    let restore = account.state().start_restore(before, saved);
    publish(app);
    let (generation, saved) = match restore {
        Restore::Refresh {
            generation,
            session,
        } => (generation, session),
        Restore::End(old) => return end_on_server(client, old),
        Restore::Done => return,
    };
    let refreshed = client.refresh(&saved.refresh_token, now());
    if let Err(e) = &refreshed {
        eprintln!("friends: couldn't refresh the saved sign-in: {e:?}");
    }
    let finished = account.state().finish_restore(generation, refreshed, save);
    match finished {
        Ok(Some(unwanted)) => end_on_server(client, unwanted),
        Ok(None) => {}
        Err(e) => eprintln!("friends: couldn't update the Keychain: {e}"),
    }
    publish(app);
}

/// Runs `f` off the main thread: the Keychain can wait on a macOS prompt, and the window must not.
async fn blocking<T: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&AppHandle) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || f(&app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_account(app: AppHandle) -> Result<Shown, String> {
    blocking(app, |app| Ok(app.state::<Account>().view())).await
}

/// Opens Sign in with Apple (in the browser on the Mac, Apple's sheet on iPhone) and waits for it,
/// then trades what comes back for a session. Starting again (or cancelling) while one waits makes the earlier one stop quietly.
#[tauri::command]
pub async fn sign_in_with_apple(app: AppHandle, attempt: String) -> Result<Shown, String> {
    blocking(app, move |app| sign_in(app, &attempt)).await
}

/// Stops sign-in `attempt`, whether it's waiting for the browser yet or not.
#[tauri::command]
pub async fn cancel_sign_in(app: AppHandle, attempt: String) -> Result<Shown, String> {
    blocking(app, move |app| {
        let account = app.state::<Account>();
        #[cfg(target_os = "macos")]
        if let Some(server) = account.waiting().cancel(&attempt) {
            server.unblock();
        }
        #[cfg(target_os = "ios")]
        account.waiting().cancel(&attempt);
        Ok(account.view())
    })
    .await
}

/// Preferences closed: stops a sign-in still waiting for the browser, since its Cancel is gone and it
/// would otherwise finish out of sight.
#[cfg(target_os = "macos")]
pub fn cancel_waiting(app: &AppHandle) {
    if let Some(server) = app.state::<Account>().waiting().cancel_all() {
        server.unblock();
    }
}

#[cfg(target_os = "macos")]
fn sign_in(app: &AppHandle, attempt: &str) -> Result<Shown, String> {
    let account = app.state::<Account>();
    let client = account.client()?;
    // A one-off address on this Mac only, on a port the system picks.
    let server = tiny_http::Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = server
        .server_addr()
        .to_ip()
        .ok_or("The sign-in address has no port.")?
        .port();
    let server = Arc::new(server);
    match account.waiting().start(attempt, server.clone()) {
        Started::Cancelled => return Ok(account.view()),
        Started::Waiting { replaced } => {
            if let Some(old) = replaced {
                old.unblock();
            }
        }
    }
    let waited = (|| {
        let redirect = format!("http://127.0.0.1:{port}{}", friends_account::CALLBACK_PATH);
        let pkce = Pkce::new()?;
        // Cancelled already (Cancel clicked straight away)? Then don't open the browser at all.
        if !account.waiting().is_waiting(attempt) {
            return Ok(None);
        }
        open_in_browser(&client.authorize_url("apple", &redirect, &pkce))?;
        let back = wait_for_browser(&server, || account.waiting().is_waiting(attempt))?;
        Ok(back.map(|(code, request)| (code, request, pkce.verifier)))
    })();
    let (code, request, verifier) = match waited {
        Ok(Some(back)) => back,
        // Cancelled, or replaced by a newer sign-in: nothing to report.
        Ok(None) => return Ok(account.view()),
        Err(e) => {
            account.waiting().finish(attempt);
            return Err(e);
        }
    };
    let shown = finish_sign_in(
        app,
        attempt,
        client,
        |c| c.exchange_code(&code, &verifier, now()),
        |page| answer(request, 200, page),
    );
    if matches!(&shown, Ok(s) if matches!(s.account, AccountView::SignedIn { .. })) {
        // Back from the browser to Friendship Reminder.
        let _ = crate::desktop::show_preferences(app);
    }
    shown
}

#[cfg(target_os = "ios")]
fn sign_in(app: &AppHandle, attempt: &str) -> Result<Shown, String> {
    use friends_account::Nonce;
    use tauri_plugin_phone_sheets::PhoneSheetsExt;
    let account = app.state::<Account>();
    let client = account.client()?;
    {
        let mut waiting = account.waiting();
        // Apple's sheet is up for another attempt: it carries on (taking its place would throw
        // away a sign-in the user may be finishing).
        if waiting.is_busy() {
            return Err("Sign in with Apple is open already.".into());
        }
        if let Started::Cancelled = waiting.start(attempt, ()) {
            return Ok(account.view());
        }
    }
    let waited = (|| {
        let nonce = Nonce::new()?;
        if !account.waiting().is_waiting(attempt) {
            return Ok(None);
        }
        // `None`: closed without signing in.
        let token = app.phone_sheets().sign_in_with_apple(&nonce.hashed)?;
        Ok(token.map(|t| (t, nonce)))
    })();
    let (token, nonce) = match waited {
        Ok(Some(back)) => back,
        Ok(None) => {
            account.waiting().finish(attempt);
            return Ok(account.view());
        }
        Err(e) => {
            account.waiting().finish(attempt);
            return Err(e);
        }
    };
    finish_sign_in(
        app,
        attempt,
        client,
        |c| c.sign_in_with_id_token("apple", &token, &nonce, now()),
        |_| {},
    )
}

/// Gets a session (`trade` trades what Apple handed back) and signs in with it, unless sign-in `attempt` was cancelled or
/// replaced meanwhile. `respond` gets the page to show where signing in happened.
fn finish_sign_in(
    app: &AppHandle,
    attempt: &str,
    client: Client<HttpTransport>,
    trade: impl FnOnce(&Client<HttpTransport>) -> Result<Session, AuthError>,
    respond: impl FnOnce(&str),
) -> Result<Shown, String> {
    let account = app.state::<Account>();
    let session = match trade(&client) {
        Ok(s) => s,
        Err(e) => {
            // Cancelled meanwhile: the failure doesn't matter any more.
            if !account.waiting().finish(attempt) {
                respond(CANCELLED);
                return Ok(account.view());
            }
            respond(FAILED);
            return Err(e.to_string());
        }
    };
    // Still the sign-in the user wants (not cancelled or replaced while the code was traded)?
    if !account.waiting().finish(attempt) {
        respond(CANCELLED);
        end_on_server(client, session);
        return Ok(account.view());
    }
    reset_sync(&account);
    let (replaced, saved) = account.state().signed_in(session, save);
    if let Err(e) = saved {
        eprintln!("friends: couldn't save the sign-in in the Keychain: {e}");
    }
    if let Some(old) = replaced {
        end_on_server(client, old);
    }
    respond(SIGNED_IN);
    publish(app);
    crate::sync::nudge(app, crate::sync::Nudge::Now);
    Ok(account.view())
}

#[cfg(target_os = "macos")]
/// Waits for the browser to come back to the sign-in address with a code, answering anything else it
/// asks for (a site icon). The code, and the browser's request to answer once the code is traded;
/// `None` once `still_waiting` says the sign-in was cancelled or replaced (which unblocks the server).
fn wait_for_browser(
    server: &tiny_http::Server,
    still_waiting: impl Fn() -> bool,
) -> Result<Option<(String, tiny_http::Request)>, String> {
    let deadline = Instant::now() + BROWSER_TIMEOUT;
    loop {
        if !still_waiting() {
            return Ok(None);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err("Signing in took too long. Try again.".into());
        }
        // `None`: the time ran out, or the server was unblocked; the loop's checks tell which.
        let Some(request) = server.recv_timeout(left).map_err(|e| e.to_string())? else {
            continue;
        };
        match friends_account::parse_callback(request.url()) {
            Callback::Code(code) => return Ok(Some((code, request))),
            Callback::Failed(reason) => {
                answer(request, 200, FAILED);
                return Err(format!("Sign in with Apple didn't finish: {reason}"));
            }
            Callback::Other => answer(request, 404, "Not found."),
        }
    }
}

#[cfg(target_os = "macos")]
fn open_in_browser(url: &str) -> Result<(), String> {
    let status = std::process::Command::new("/usr/bin/open")
        .arg(url)
        .status()
        .map_err(|e| format!("Couldn't open the browser: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("Couldn't open the browser.".into())
    }
}

#[cfg(target_os = "macos")]
/// Answers the browser with a short page.
fn answer(request: tiny_http::Request, status: u16, text: &str) {
    let body = format!(
        "<!doctype html><meta charset=utf-8><title>Friendship Reminder</title>\
         <body style=\"font:15px -apple-system,sans-serif;margin:4em auto;max-width:32em;text-align:center\">\
         <p>{text}</p></body>"
    );
    let html =
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
            .expect("a valid header");
    let _ = request.respond(
        tiny_http::Response::from_string(body)
            .with_status_code(status)
            .with_header(html),
    );
}

/// Ends a session on Supabase's side, in the background. Offline, it just stays unused until it
/// runs out.
fn end_on_server(client: Client<HttpTransport>, session: Session) {
    std::thread::spawn(move || {
        if let Err(e) = client.end_session(&session, now()) {
            eprintln!("friends: couldn't end the session on the server: {e:?}");
        }
    });
}

/// Signs out once the Keychain has let go of the session (one left behind would sign back in at the
/// next launch), then ends the session on Supabase's side in the background.
#[tauri::command]
pub async fn sign_out(app: AppHandle) -> Result<Shown, String> {
    blocking(app, |app| {
        let account = app.state::<Account>();
        let old = account
            .state()
            .sign_out(save)
            .map_err(|e| format!("Couldn't remove the sign-in from the Keychain: {e}"))?;
        reset_sync(&account);
        publish(app);
        if let (Some(old), Ok(client)) = (old, account.client()) {
            end_on_server(client, old);
        }
        Ok(account.view())
    })
    .await
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    /// Connects to the sign-in address and, on another thread, asks for `target` and reads the answer.
    fn browse(port: u16, target: &str) -> std::thread::JoinHandle<String> {
        let request =
            format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        std::thread::spawn(move || {
            s.write_all(request.as_bytes()).unwrap();
            let mut answer = String::new();
            s.read_to_string(&mut answer).unwrap();
            answer
        })
    }

    fn server() -> (Arc<tiny_http::Server>, u16) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let port = server.server_addr().to_ip().unwrap().port();
        (Arc::new(server), port)
    }

    #[test]
    fn the_code_comes_back_past_an_idle_connection_and_is_answered_after() {
        let (server, port) = server();
        // A connection that never says anything (a browser preconnect) must not hold anything up.
        let _idle = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let s = server.clone();
        let waiting = std::thread::spawn(move || wait_for_browser(&s, || true));
        assert!(browse(port, "/favicon.ico")
            .join()
            .unwrap()
            .starts_with("HTTP/1.1 404"));
        let page = browse(port, "/auth/callback?code=abc");
        let (code, request) = waiting.join().unwrap().unwrap().unwrap();
        assert_eq!(code, "abc");
        // The browser hears back only once the code is traded.
        answer(request, 200, SIGNED_IN);
        let page = page.join().unwrap();
        assert!(page.starts_with("HTTP/1.1 200"));
        assert!(page.contains("You're signed in to Friendship Reminder"));
    }

    #[test]
    fn a_refused_sign_in_comes_back_with_its_reason() {
        let (server, port) = server();
        let page = browse(
            port,
            "/auth/callback?error=access_denied&error_description=Nope",
        );
        let e = wait_for_browser(&server, || true).unwrap_err();
        assert!(e.contains("Nope"));
        assert!(page.join().unwrap().contains("didn't work"));
    }

    #[test]
    fn a_cancel_wakes_the_sign_in_waiting_for_the_browser() {
        let (server, _) = server();
        let waiting = Arc::new(Mutex::new(Waiting::default()));
        waiting.lock().unwrap().start("a", server.clone());
        let w = waiting.clone();
        let stop = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            if let Some(server) = w.lock().unwrap().cancel("a") {
                server.unblock();
            }
        });
        let waited = wait_for_browser(&server, || waiting.lock().unwrap().is_waiting("a"));
        assert!(waited.unwrap().is_none());
        stop.join().unwrap();
    }

    #[test]
    fn stopping_closes_the_sign_in_address() {
        let (server, port) = server();
        drop(server);
        let gone = (0..50).any(|_| {
            std::thread::sleep(Duration::from_millis(20));
            TcpStream::connect(("127.0.0.1", port)).is_err()
        });
        assert!(gone);
    }
}
