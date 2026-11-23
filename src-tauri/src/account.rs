//! The Friends account: Sign in with Apple, traded for a Supabase session that is kept in the Keychain.
//! The rules live in friends-account; this file only opens the browser, listens for it to come back,
//! and connects the Keychain and the windows.
//!
//! Signing in happens in the browser: the app opens Supabase's Sign in with Apple page, and when it's
//! done the browser comes back to a one-off address on this Mac (`http://127.0.0.1:<port>/auth/callback`)
//! with a code the app trades for a session. (Apple's own sign-in sheet needs the Mac App Store: apps
//! signed with a Developer ID can't use it.)
//!
//! Signing in is optional. A build without the Supabase settings (`FRIENDS_SUPABASE_URL` and
//! `FRIENDS_SUPABASE_PUBLISHABLE_KEY` at build time, or in `.env.local` for debug builds) shows the
//! Account section as unavailable, and touches neither the network nor the Keychain.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use friends_account::{
    parse_callback, AccountState, AccountView, Callback, Client, Config, HttpTransport, Pkce,
    Session,
};
use tauri::{AppHandle, Emitter, Manager};

/// Sent to every window when the account changes, with the new [`AccountView`].
const ACCOUNT_CHANGED: &str = "account-changed";

/// The Keychain item holding the session, as JSON.
const KEYCHAIN_SERVICE: &str = "io.github.charliespy.friends";
const KEYCHAIN_ACCOUNT: &str = "account-session";

/// How long a sign-in waits for the browser to come back.
const BROWSER_TIMEOUT: Duration = Duration::from_secs(600);

/// How long the app waits for a browser connection to say what it wants.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// What ends a sign-in's wait for the browser.
enum Wake {
    Browser(Callback),
    /// Cancel was clicked, or a newer sign-in started.
    Cancelled,
}

/// The sign-in waiting for the browser: its number, and how to wake it.
type Waiting = Option<(u64, Sender<Wake>)>;

pub struct Account {
    /// Why this build can't sign in, or the project to sign in to.
    config: Result<Config, String>,
    state: Mutex<AccountState>,
    waiting: Mutex<Waiting>,
    /// Numbers each sign-in.
    attempts: AtomicU64,
}

impl Account {
    pub fn new() -> Account {
        Account {
            config: available(),
            state: Mutex::new(AccountState::default()),
            waiting: Mutex::new(None),
            attempts: AtomicU64::new(0),
        }
    }

    fn state(&self) -> MutexGuard<'_, AccountState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn waiting(&self) -> MutexGuard<'_, Waiting> {
        self.waiting.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn view(&self) -> AccountView {
        match &self.config {
            Err(reason) => AccountView::Unavailable {
                reason: reason.clone(),
            },
            Ok(_) => self.state().view(),
        }
    }

    fn client(&self) -> Result<Client<HttpTransport>, String> {
        let config = self.config.clone()?;
        Ok(Client::new(config, HttpTransport::default()))
    }

    /// Stops the sign-in waiting for the browser, if any, and makes `next` the one waiting from now on.
    fn wait_with(&self, next: Waiting) {
        if let Some((_, old)) = std::mem::replace(&mut *self.waiting(), next) {
            let _ = old.send(Wake::Cancelled);
        }
    }

    /// Forgets sign-in `id` as the waiting one, if it still is.
    fn done_waiting(&self, id: u64) {
        let mut waiting = self.waiting();
        if waiting.as_ref().is_some_and(|(w, _)| *w == id) {
            *waiting = None;
        }
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
    built_in.ok_or_else(|| "This copy of Friends was built without an account server.".to_string())
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn publish(app: &AppHandle) {
    let _ = app.emit(ACCOUNT_CHANGED, app.state::<Account>().view());
}

/// errSecItemNotFound: the Keychain holds no such item.
#[cfg(target_os = "macos")]
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

/// Saves the session in the Keychain, or removes it for `None`.
fn save(session: Option<&Session>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
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
    #[cfg(not(target_os = "macos"))]
    {
        let _ = session;
        Ok(())
    }
}

/// The saved session: `Ok(None)` when none was saved (or it can't be read back as one), `Err` when
/// the Keychain wouldn't say.
fn load() -> Result<Option<Session>, String> {
    #[cfg(target_os = "macos")]
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
    #[cfg(not(target_os = "macos"))]
    Ok(None)
}

/// At launch: picks up a saved session and refreshes it, which also checks it's still good
/// (`AccountState` decides what each outcome means).
pub fn restore(app: &AppHandle) {
    let account = app.state::<Account>();
    let Ok(client) = account.client() else {
        return;
    };
    let saved = load();
    if let Err(e) = &saved {
        eprintln!("friends: couldn't read the saved sign-in: {e}");
    }
    let to_refresh = saved.as_ref().ok().cloned().flatten();
    let generation = account.state().start_restore(saved);
    publish(app);
    let (Some(generation), Some(saved)) = (generation, to_refresh) else {
        return;
    };
    let refreshed = client.refresh(&saved.refresh_token, now());
    if let Err(e) = &refreshed {
        eprintln!("friends: couldn't refresh the saved sign-in: {e:?}");
    }
    if let Err(e) = account.state().finish_restore(generation, refreshed, save) {
        eprintln!("friends: couldn't update the Keychain: {e}");
    }
    publish(app);
}

#[tauri::command]
pub fn get_account(account: tauri::State<'_, Account>) -> AccountView {
    account.view()
}

/// Opens Sign in with Apple in the browser and waits for it to come back, then trades the code for a
/// session. Starting again (or cancelling) while one waits makes the earlier one stop quietly.
#[tauri::command]
pub async fn sign_in_with_apple(app: AppHandle) -> Result<AccountView, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || sign_in(&handle))
        .await
        .map_err(|e| e.to_string())?
}

/// Stops a sign-in that is waiting for the browser.
#[tauri::command]
pub fn cancel_sign_in(account: tauri::State<'_, Account>) -> AccountView {
    account.wait_with(None);
    account.view()
}

fn sign_in(app: &AppHandle) -> Result<AccountView, String> {
    let account = app.state::<Account>();
    let client = account.client()?;
    let id = account.attempts.fetch_add(1, Ordering::SeqCst) + 1;
    let (tx, rx) = mpsc::channel();
    account.wait_with(Some((id, tx.clone())));

    let code = match wait_for_browser(&client, tx, &rx) {
        Ok(Some(code)) => code,
        // Cancelled, or replaced by a newer sign-in: nothing to report.
        Ok(None) => return Ok(account.view()),
        Err(e) => {
            account.done_waiting(id);
            return Err(e);
        }
    };
    let (code, verifier) = code;
    let session = match client.exchange_code(&code, &verifier, now()) {
        Ok(s) => s,
        Err(e) => {
            account.done_waiting(id);
            return Err(e.to_string());
        }
    };
    // Still the sign-in the user wants (not cancelled or replaced while the code was traded)? Checked
    // and applied in one step, together with the Keychain.
    let (kept, saved) = {
        let mut state = account.state();
        let mut waiting = account.waiting();
        let still_wanted = waiting.as_ref().is_some_and(|(w, _)| *w == id);
        if still_wanted {
            *waiting = None;
        }
        state.signed_in(session.clone(), still_wanted, save)
    };
    if !kept {
        end_on_server(client, session);
        return Ok(account.view());
    }
    if let Err(e) = saved {
        eprintln!("friends: couldn't save the sign-in in the Keychain: {e}");
    }
    publish(app);
    // Back from the browser to Friends.
    let _ = crate::show_preferences(app);
    Ok(account.view())
}

/// Opens the sign-in page and waits for the browser to come back. The code and the verifier to trade
/// it with, or `None` when cancelled.
fn wait_for_browser(
    client: &Client<HttpTransport>,
    tx: Sender<Wake>,
    rx: &mpsc::Receiver<Wake>,
) -> Result<Option<(String, String)>, String> {
    // A one-off address on this Mac only, on a port the system picks.
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}{}", friends_account::CALLBACK_PATH);
    let _listening = Listening::start(listener, port, tx);
    let pkce = Pkce::new()?;
    open_in_browser(&client.authorize_url("apple", &redirect, &pkce))?;
    match rx.recv_timeout(BROWSER_TIMEOUT) {
        Ok(Wake::Browser(Callback::Code(code))) => Ok(Some((code, pkce.verifier))),
        Ok(Wake::Browser(Callback::Failed(reason))) => {
            Err(format!("Sign in with Apple didn't finish: {reason}"))
        }
        Ok(Wake::Browser(Callback::Other)) | Ok(Wake::Cancelled) => Ok(None),
        Err(RecvTimeoutError::Timeout) => Err("Signing in took too long. Try again.".into()),
        Err(RecvTimeoutError::Disconnected) => Ok(None),
    }
}

/// Accepts the browser's connections on the sign-in address until dropped. Each connection is read on
/// its own thread, so one that never says anything (a browser's preconnect) can't hold up the rest.
struct Listening {
    stopped: Arc<AtomicBool>,
    port: u16,
}

impl Listening {
    fn start(listener: TcpListener, port: u16, wake: Sender<Wake>) -> Listening {
        let stopped = Arc::new(AtomicBool::new(false));
        let flag = stopped.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if flag.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = stream else { continue };
                let wake = wake.clone();
                std::thread::spawn(move || {
                    let callback = answer_browser(stream);
                    if callback != Callback::Other {
                        let _ = wake.send(Wake::Browser(callback));
                    }
                });
            }
        });
        Listening { stopped, port }
    }
}

impl Drop for Listening {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        // The listening thread sleeps in accept(); a connection wakes it to see it should stop.
        let _ = TcpStream::connect(("127.0.0.1", self.port));
    }
}

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

/// Reads one request from the browser and answers it with a short page.
fn answer_browser(mut stream: TcpStream) -> Callback {
    let _ = stream.set_read_timeout(Some(REQUEST_TIMEOUT));
    let mut line = String::new();
    let _ = BufReader::new(&stream).read_line(&mut line);
    // "GET /auth/callback?code=... HTTP/1.1"
    let target = line.split_whitespace().nth(1).unwrap_or("");
    let callback = parse_callback(target);
    let (status, text) = match &callback {
        Callback::Code(_) => (
            "200 OK",
            "You're signed in to Friends. You can close this tab.",
        ),
        Callback::Failed(_) => (
            "200 OK",
            "Signing in to Friends didn't work. Go back to Friends to try again.",
        ),
        Callback::Other => ("404 Not Found", "Not found."),
    };
    let body = format!(
        "<!doctype html><meta charset=utf-8><title>Friends</title>\
         <body style=\"font:15px -apple-system,sans-serif;margin:4em auto;max-width:32em;text-align:center\">\
         <p>{text}</p></body>"
    );
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    callback
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
pub fn sign_out(app: AppHandle) -> Result<AccountView, String> {
    let account = app.state::<Account>();
    let old = account
        .state()
        .sign_out(save)
        .map_err(|e| format!("Couldn't remove the sign-in from the Keychain: {e}"))?;
    publish(&app);
    if let (Some(old), Ok(client)) = (old, account.client()) {
        end_on_server(client, old);
    }
    Ok(account.view())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Connects to the sign-in address and, on another thread, sends `request` and reads the answer.
    fn browse(port: u16, request: &str) -> std::thread::JoinHandle<String> {
        let request = request.to_string();
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        std::thread::spawn(move || {
            s.write_all(request.as_bytes()).unwrap();
            let mut answer = String::new();
            let _ = std::io::Read::read_to_string(&mut s, &mut answer);
            answer
        })
    }

    fn get(target: &str) -> String {
        format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
    }

    fn listen() -> (Listening, u16, mpsc::Receiver<Wake>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        (Listening::start(l, port, tx), port, rx)
    }

    fn next(rx: &mpsc::Receiver<Wake>) -> Callback {
        match rx.recv_timeout(Duration::from_secs(3)).unwrap() {
            Wake::Browser(c) => c,
            Wake::Cancelled => panic!("cancelled"),
        }
    }

    #[test]
    fn the_code_comes_back_past_an_idle_connection_and_other_requests() {
        let (_listening, port, rx) = listen();
        // A connection that never says anything (a browser preconnect) must not hold anything up.
        let _idle = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let icon = browse(port, &get("/favicon.ico"));
        let page = browse(port, &get("/auth/callback?code=abc"));
        assert_eq!(next(&rx), Callback::Code("abc".into()));
        assert!(icon.join().unwrap().starts_with("HTTP/1.1 404"));
        let page = page.join().unwrap();
        assert!(page.starts_with("HTTP/1.1 200 OK"));
        assert!(page.contains("You're signed in to Friends"));
    }

    #[test]
    fn a_refused_sign_in_comes_back_with_its_reason() {
        let (_listening, port, rx) = listen();
        let page = browse(
            port,
            &get("/auth/callback?error=access_denied&error_description=Nope"),
        );
        assert_eq!(next(&rx), Callback::Failed("Nope".into()));
        assert!(page.join().unwrap().contains("didn't work"));
    }

    #[test]
    fn stopping_closes_the_sign_in_address() {
        let (listening, port, _rx) = listen();
        drop(listening);
        // The listening thread has gone, so the port stops answering.
        let gone = (0..50).any(|_| {
            std::thread::sleep(Duration::from_millis(20));
            TcpStream::connect(("127.0.0.1", port)).is_err()
        });
        assert!(gone);
    }
}
