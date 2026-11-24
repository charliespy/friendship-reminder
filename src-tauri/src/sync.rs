//! Keeps the list in sync with the Friends account while signed in, on a thread of its own: at launch,
//! right after signing in, a few seconds after each change, and every few minutes (to pick up changes
//! from other devices). Offline, it tries again a minute later. The rules live in friends-core
//! (`sync`) and the requests in friends-sync; this file only decides when to run.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::Duration;

use friends_account::HttpTransport;
use friends_sync::{Server, SyncError};
use tauri::{AppHandle, Manager};

use crate::account::{self, Account};

/// How long after a change to upload it, so a burst of changes goes up together.
const SETTLE: Duration = Duration::from_secs(3);
/// How often to look for changes from other devices.
const EVERY: Duration = Duration::from_secs(5 * 60);
/// How soon to try again after a sync failed.
const RETRY: Duration = Duration::from_secs(60);

/// Asks the sync thread to run.
pub enum Nudge {
    /// Something changed here: sync once things settle.
    Soon,
    /// Sync right away (just signed in).
    Now,
}

/// Where to send nudges.
pub struct Syncer(Mutex<Option<Sender<Nudge>>>);

impl Syncer {
    pub fn new() -> Syncer {
        Syncer(Mutex::new(None))
    }
}

pub fn nudge(app: &AppHandle, nudge: Nudge) {
    if let Some(tx) = &*app
        .state::<Syncer>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
    {
        let _ = tx.send(nudge);
    }
}

/// Starts the sync thread. It first picks up the saved sign-in (so no sync refreshes the session at
/// the same time as the check at launch does), then syncs.
pub fn start(app: &AppHandle) {
    let (tx, rx) = mpsc::channel();
    *app.state::<Syncer>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(tx);
    let app = app.clone();
    std::thread::spawn(move || {
        account::restore(&app);
        let mut wait = Duration::ZERO;
        loop {
            match rx.recv_timeout(wait) {
                Ok(Nudge::Soon) => loop {
                    // Wait for the changes to settle; a Now cuts it short.
                    match rx.recv_timeout(SETTLE) {
                        Ok(Nudge::Soon) => continue,
                        Ok(Nudge::Now) | Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                },
                Ok(Nudge::Now) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            wait = if run(&app) { EVERY } else { RETRY };
        }
    });
}

/// One sync, if signed in. Returns whether it worked (or there was nothing to do).
fn run(app: &AppHandle) -> bool {
    let Some(config) = app.state::<Account>().config() else {
        return true;
    };
    let session = match account::session_for_sync(app, false) {
        Ok(Some(s)) => s,
        Ok(None) => return true,
        Err(e) => {
            account::set_sync(app, |v| v.error = Some(e));
            return false;
        }
    };
    account::set_sync(app, |v| v.syncing = true);
    let server = Server::new(config, HttpTransport::default());
    let state = app.state::<crate::State>();
    let result = friends_sync::sync(
        &server,
        &session,
        &state.0,
        crate::today(),
        |user| account::signed_in_user(app).as_deref() == Some(user),
        || account::session_for_sync(app, true),
    );
    match &result {
        Ok(true) => {
            let core = crate::lock(&state);
            // Under the lock, so views reach the windows in order.
            crate::publish_if_changed(app, &core.view(crate::today()));
        }
        Ok(false) | Err(SyncError::SignedOut) => {}
        Err(e) => eprintln!("friends: sync failed: {e}"),
    }
    let finished = friends_core::now_ms();
    account::set_sync(app, |v| {
        v.syncing = false;
        match &result {
            Ok(_) => {
                v.last_synced = Some(finished);
                v.error = None;
            }
            // Signed out meanwhile: the sign-out already cleared what's shown.
            Err(SyncError::SignedOut) => {}
            Err(e) => v.error = Some(e.to_string()),
        }
    });
    matches!(result, Ok(_) | Err(SyncError::SignedOut))
}
