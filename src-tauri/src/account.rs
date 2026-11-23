//! The Friends account: Sign in with Apple, traded for a Supabase session that is kept in the Keychain.
//! The sign-in rules live in friends-account; this file connects them to macOS and the windows.
//!
//! Signing in is optional. A build without the Supabase settings (`FRIENDS_SUPABASE_URL` and
//! `FRIENDS_SUPABASE_PUBLISHABLE_KEY` at build time) or without the Sign in with Apple entitlement
//! shows the Account section as unavailable, and touches neither the network nor the Keychain.

use std::sync::{Mutex, MutexGuard};

use friends_account::{AccountView, AuthError, Client, Config, HttpTransport, Nonce, Session};
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
    /// Held for the whole of a sign-in, so a second click can't start another one.
    signing_in: Mutex<()>,
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
            signing_in: Mutex::new(()),
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
    let config = Config::new(
        option_env!("FRIENDS_SUPABASE_URL"),
        option_env!("FRIENDS_SUPABASE_PUBLISHABLE_KEY"),
    )
    .ok_or("This copy of Friends was built without an account server.")?;
    #[cfg(target_os = "macos")]
    if !crate::apple::entitled() {
        return Err("This copy of Friends isn't signed for Sign in with Apple.".into());
    }
    #[cfg(not(target_os = "macos"))]
    return Err("Sign in with Apple works on macOS only.".into());
    #[allow(unreachable_code)]
    Ok(config)
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

/// Shows Apple's sign-in sheet over Preferences, then trades its token for a session.
#[tauri::command]
pub async fn sign_in_with_apple(app: AppHandle) -> Result<AccountView, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || sign_in(&handle))
        .await
        .map_err(|e| e.to_string())?
}

fn sign_in(app: &AppHandle) -> Result<AccountView, String> {
    let account = app.state::<Account>();
    let client = account.client()?;
    let Ok(_busy) = account.signing_in.try_lock() else {
        return Err("Signing in already.".into());
    };
    let Some((token, nonce)) = apple_token(app)? else {
        // Closed the sheet: nothing changes and there's nothing to report.
        return Ok(account.view());
    };
    let session = client
        .sign_in_with_apple(&token, &nonce.raw, now())
        .map_err(|e| e.to_string())?;
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
    Ok(account.view())
}

/// Apple's identity token and the nonce it was made for, or `None` when the user closed the sheet.
#[cfg(target_os = "macos")]
fn apple_token(app: &AppHandle) -> Result<Option<(String, Nonce)>, String> {
    use crate::apple::{self, AppleError};
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Duration;

    /// Apple's sheet always answers, but if it never did, sign-in would stay stuck until a relaunch.
    const SHEET_TIMEOUT: Duration = Duration::from_secs(600);

    let nonce = Nonce::new()?;
    let window = app
        .get_webview_window(crate::PREFERENCES)
        .ok_or("Open Preferences to sign in.")?;
    // A raw pointer isn't Send; it is only turned back into a window on the main thread.
    let ns_window = window.ns_window().map_err(|e| e.to_string())? as usize;
    let (tx, rx) = std::sync::mpsc::channel();
    let hashed = nonce.hashed.clone();
    app.run_on_main_thread(move || apple::start(ns_window as *mut _, &hashed, tx))
        .map_err(|e| e.to_string())?;
    let answer = rx.recv_timeout(SHEET_TIMEOUT);
    // Let go of the sheet's controller and window, after its callback has returned.
    let _ = app.run_on_main_thread(apple::finish);
    match answer {
        Ok(Ok(token)) => Ok(Some((token, nonce))),
        Ok(Err(AppleError::Canceled)) => Ok(None),
        Ok(Err(AppleError::Failed(e))) => Err(format!("Sign in with Apple didn't finish: {e}")),
        Err(RecvTimeoutError::Timeout) => {
            Err("Sign in with Apple took too long. Try again.".into())
        }
        Err(RecvTimeoutError::Disconnected) => {
            Err("Sign in with Apple stopped without an answer.".into())
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn apple_token(_app: &AppHandle) -> Result<Option<(String, Nonce)>, String> {
    Err("Sign in with Apple works on macOS only.".into())
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
