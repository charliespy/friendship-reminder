//! The Friends account: Sign in with Apple, traded for a Supabase session that is kept in the Keychain.
//! The sign-in rules live in friends-account; this file connects them to macOS and the windows.
//!
//! Signing in happens in the browser: the app opens Supabase's Sign in with Apple page, and when it's
//! done the browser comes back to a one-off address on this Mac (`http://127.0.0.1:<port>/auth/callback`)
//! with a code the app trades for a session. (Apple's own sign-in sheet needs the Mac App Store: apps
//! signed with a Developer ID can't use it.)
//!
//! Signing in is optional. A build without the Supabase settings (`FRIENDS_SUPABASE_URL` and
//! `FRIENDS_SUPABASE_PUBLISHABLE_KEY` at build time) shows the Account section as unavailable, and
//! touches neither the network nor the Keychain.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use friends_account::{
    parse_callback, AccountView, AuthError, Callback, Client, Config, HttpTransport, Pkce, Session,
};
use tauri::{AppHandle, Emitter, Manager};

/// Sent to every window when the account changes, with the new [`AccountView`].
const ACCOUNT_CHANGED: &str = "account-changed";

/// The Keychain item holding the session, as JSON.
const KEYCHAIN_SERVICE: &str = "io.github.charliespy.friends";
const KEYCHAIN_ACCOUNT: &str = "account-session";

/// Shown when a refreshed session couldn't be saved: Supabase replaced the saved refresh token, so the
/// next launch will have to sign in again.
const NOT_SAVED: &str =
    "Couldn't save your sign-in in the Keychain, so you may need to sign in again after Friends restarts.";

/// The signed-in state. `generation` goes up on every change, so work that started from an older state
/// (a refresh at launch that finishes after the user signed out) can tell and drop its result.
struct Signed {
    generation: u64,
    session: Option<Session>,
    /// Set when the Keychain no longer holds the current session.
    warning: Option<String>,
}

pub struct Account {
    /// Why this build can't sign in, or the project to sign in to.
    config: Result<Config, String>,
    state: Mutex<Signed>,
    /// Goes up with every sign-in started or cancelled, so a sign-in still waiting for the browser
    /// knows it was replaced and stops.
    attempt: AtomicU64,
}

impl Account {
    pub fn new() -> Account {
        Account {
            config: available(),
            state: Mutex::new(Signed {
                generation: 0,
                session: None,
                warning: None,
            }),
            attempt: AtomicU64::new(0),
        }
    }

    fn state(&self) -> MutexGuard<'_, Signed> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn view(&self) -> AccountView {
        match &self.config {
            Err(reason) => AccountView::Unavailable {
                reason: reason.clone(),
            },
            Ok(_) => {
                let state = self.state();
                match &state.session {
                    Some(s) => AccountView::SignedIn {
                        email: s.email.clone(),
                        warning: state.warning.clone(),
                    },
                    None => AccountView::SignedOut,
                }
            }
        }
    }

    fn client(&self) -> Result<Client<HttpTransport>, String> {
        let config = self.config.clone()?;
        Ok(Client::new(config, HttpTransport::default()))
    }

    /// Replaces the session, in memory and in the Keychain, unless it changed since `generation`
    /// (pass `None` to replace it whatever happened meanwhile). Returns the Keychain's error, if any,
    /// or `Ok(false)` when the change was dropped as stale.
    fn replace(&self, generation: Option<u64>, session: Option<Session>) -> Result<bool, String> {
        let mut state = self.state();
        if generation.is_some_and(|g| g != state.generation) {
            return Ok(false);
        }
        // Saved while the lock is held, so the Keychain and memory change together.
        let saved = save(session.as_ref());
        state.generation += 1;
        state.warning = match (&saved, &session) {
            (Err(_), Some(_)) => Some(NOT_SAVED.to_string()),
            _ => None,
        };
        state.session = session;
        saved.map(|()| true)
    }
}

fn available() -> Result<Config, String> {
    Config::new(
        option_env!("FRIENDS_SUPABASE_URL"),
        option_env!("FRIENDS_SUPABASE_PUBLISHABLE_KEY"),
    )
    .ok_or_else(|| "This copy of Friends was built without an account server.".to_string())
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn publish(app: &AppHandle) {
    let _ = app.emit(ACCOUNT_CHANGED, app.state::<Account>().view());
}

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
                // errSecItemNotFound: nothing was saved.
                Err(e) if e.code() != -25300 => Err(e.to_string()),
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

fn load() -> Option<Session> {
    #[cfg(target_os = "macos")]
    {
        let bytes =
            security_framework::passwords::get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
                .ok()?;
        serde_json::from_slice(&bytes).ok()
    }
    #[cfg(not(target_os = "macos"))]
    None
}

/// At launch: picks up a saved session and refreshes it, which also checks it's still good. Offline,
/// the saved session is kept (it shows as signed in) and refreshed at the next launch; a session
/// Supabase refuses (revoked, or signed out elsewhere) is dropped. A sign-in or sign-out made while the
/// refresh is on its way wins over it.
pub fn restore(app: &AppHandle) {
    let account = app.state::<Account>();
    let Ok(client) = account.client() else {
        return;
    };
    let Some(saved) = load() else {
        return;
    };
    let generation = {
        let mut state = account.state();
        if state.session.is_some() {
            return; // Signed in already, in the moment since launch.
        }
        state.session = Some(saved.clone());
        state.generation += 1;
        state.generation
    };
    publish(app);
    let outcome = match client.refresh(&saved.refresh_token, now()) {
        Ok(fresh) => account.replace(Some(generation), Some(fresh)),
        Err(AuthError::Rejected(e)) => {
            eprintln!("friends: the saved session was refused, signing out: {e}");
            account.replace(Some(generation), None)
        }
        Err(e) => {
            eprintln!("friends: couldn't refresh the session: {e:?}");
            Ok(false)
        }
    };
    if let Err(e) = outcome {
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
    account.attempt.fetch_add(1, Ordering::SeqCst);
    account.view()
}

/// How long a sign-in waits for the browser to come back.
const BROWSER_TIMEOUT: Duration = Duration::from_secs(600);

fn sign_in(app: &AppHandle) -> Result<AccountView, String> {
    let account = app.state::<Account>();
    let client = account.client()?;
    let attempt = account.attempt.fetch_add(1, Ordering::SeqCst) + 1;
    let current = || account.attempt.load(Ordering::SeqCst) == attempt;

    // A one-off address on this Mac only, on a port the system picks.
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect = format!("http://127.0.0.1:{port}{}", friends_account::CALLBACK_PATH);
    let pkce = Pkce::new()?;
    open_in_browser(&client.authorize_url("apple", &redirect, &pkce))?;

    let Some(code) = wait_for_code(&listener, &current)? else {
        // Cancelled, or replaced by a newer sign-in: nothing to report.
        return Ok(account.view());
    };
    let session = client
        .exchange_code(&code, &pkce.verifier, now())
        .map_err(|e| e.to_string())?;
    if !current() {
        // Cancelled while the code was being traded: don't keep the session.
        end_on_server(client, session);
        return Ok(account.view());
    }
    if let Err(e) = save(Some(&session)) {
        // Without the Keychain the app can't keep this session, so end it rather than leave it live.
        end_on_server(client, session);
        return Err(format!("Couldn't save the sign-in in the Keychain: {e}"));
    }
    {
        let mut state = account.state();
        state.generation += 1;
        state.session = Some(session);
        state.warning = None;
    }
    publish(app);
    // Back from the browser to Friends.
    let _ = crate::show_preferences(app);
    Ok(account.view())
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

/// Waits for the browser to come back with a code. `None` when this sign-in was cancelled or replaced.
fn wait_for_code(
    listener: &TcpListener,
    current: &dyn Fn() -> bool,
) -> Result<Option<String>, String> {
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + BROWSER_TIMEOUT;
    while current() {
        if Instant::now() > deadline {
            return Err("Signing in took too long. Try again.".into());
        }
        let stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => return Err(e.to_string()),
        };
        match answer_browser(stream) {
            Callback::Code(code) => return Ok(Some(code)),
            Callback::Failed(reason) => {
                return Err(format!("Sign in with Apple didn't finish: {reason}"))
            }
            Callback::Other => {}
        }
    }
    Ok(None)
}

/// Reads one request from the browser and answers it with a short page.
fn answer_browser(mut stream: TcpStream) -> Callback {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
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

/// Ends a session on Supabase's side, in the background. The access token kept in memory may have run
/// out while the app ran (a menu bar app runs for days), and Supabase would then refuse to end the
/// session with it, so an expired one is refreshed first. Offline, the session just stays unused.
fn end_on_server(client: Client<HttpTransport>, session: Session) {
    std::thread::spawn(move || {
        let access = if session.needs_refresh(now()) {
            match client.refresh(&session.refresh_token, now()) {
                Ok(fresh) => fresh.access_token,
                Err(e) => {
                    eprintln!("friends: couldn't refresh the session to end it: {e:?}");
                    return;
                }
            }
        } else {
            session.access_token
        };
        if let Err(e) = client.sign_out(&access) {
            eprintln!("friends: couldn't end the session on the server: {e:?}");
        }
    });
}

/// Signs out on this Mac at once, then ends the session on Supabase's side in the background.
#[tauri::command]
pub fn sign_out(app: AppHandle) -> Result<AccountView, String> {
    let account = app.state::<Account>();
    let old = account.state().session.clone();
    // Signed out in memory even if the Keychain refuses; the session is ended on the server too, so
    // a copy left in the Keychain is refused and dropped at the next launch.
    let removed = account.replace(None, None);
    publish(&app);
    if let (Some(old), Ok(client)) = (old, account.client()) {
        end_on_server(client, old);
    }
    removed.map_err(|e| format!("Couldn't remove the sign-in from the Keychain: {e}"))?;
    Ok(account.view())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sends one request to the listener the way a browser would, and returns the answer.
    fn browse(port: u16, target: &str) -> std::thread::JoinHandle<String> {
        let target = target.to_string();
        // Connected before returning, so requests reach the listener in the order they were made.
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        std::thread::spawn(move || {
            write!(s, "GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
            let mut answer = String::new();
            let _ = std::io::Read::read_to_string(&mut s, &mut answer);
            answer
        })
    }

    fn listener() -> (TcpListener, u16) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        (l, port)
    }

    #[test]
    fn the_browser_brings_back_the_code_after_other_requests() {
        let (l, port) = listener();
        let icon = browse(port, "/favicon.ico");
        let page = browse(port, "/auth/callback?code=abc");
        // The icon request is answered and ignored; the callback ends the wait.
        assert_eq!(wait_for_code(&l, &|| true), Ok(Some("abc".to_string())));
        assert!(icon.join().unwrap().starts_with("HTTP/1.1 404"));
        let page = page.join().unwrap();
        assert!(page.starts_with("HTTP/1.1 200 OK"));
        assert!(page.contains("You're signed in to Friends"));
    }

    #[test]
    fn a_refused_sign_in_is_an_error_and_the_page_says_so() {
        let (l, port) = listener();
        let page = browse(
            port,
            "/auth/callback?error=access_denied&error_description=Nope",
        );
        assert_eq!(
            wait_for_code(&l, &|| true),
            Err("Sign in with Apple didn't finish: Nope".to_string())
        );
        assert!(page.join().unwrap().contains("didn't work"));
    }

    #[test]
    fn a_cancelled_sign_in_stops_waiting() {
        let (l, _) = listener();
        assert_eq!(wait_for_code(&l, &|| false), Ok(None));
    }
}
