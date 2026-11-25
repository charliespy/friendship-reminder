//! The Tauri shell, shared by the Mac and iPhone apps: the data, the one command the UI calls, and the
//! account and sync. The rules and the data live in friends-core; this file only connects them to the
//! windows. `desktop.rs` holds the Mac's menu bar app and `mobile.rs` the iPhone's.

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
compile_error!("Friends is a Mac and iPhone app: build it for macOS or iOS.");

mod account;
#[cfg(target_os = "macos")]
mod desktop;
#[cfg(target_os = "ios")]
mod mobile;
mod sync;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use chrono::{Local, NaiveDate};
use friends_core::api::{ApiError, App, ErrorKind, View};
use friends_core::store::Store;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

const VIEW_CHANGED: &str = "view-changed";

struct State(Mutex<App>);

/// Why the data couldn't be opened, on iPhone, where the app stays up to say so (every command
/// answers with it, and the window shows it) instead of quitting without a word.
#[cfg(target_os = "ios")]
struct CannotOpen(String);

/// The local calendar date. `FRIENDS_TODAY=YYYY-MM-DD` overrides it in debug builds, to try the app on
/// a given day.
fn today() -> NaiveDate {
    #[cfg(debug_assertions)]
    if let Some(d) = std::env::var("FRIENDS_TODAY")
        .ok()
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
    {
        return d;
    }
    Local::now().date_naive()
}

/// Where the data is saved: `friends.json` in the app's data folder
/// (`~/Library/Application Support/io.github.charliespy.friends` on the Mac). `FRIENDS_DATA_FILE`
/// overrides it in debug builds, so trying the app never touches real data.
fn data_file(app: &AppHandle) -> Result<PathBuf, String> {
    #[cfg(debug_assertions)]
    if let Ok(path) = std::env::var("FRIENDS_DATA_FILE") {
        return Ok(PathBuf::from(path));
    }
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("friends.json"))
}

/// Loads the data file. A damaged file is moved aside by the core; this fails only when the file
/// or its folder can't be read or created (permissions, a full or missing disk).
fn open_data(app: &AppHandle) -> Result<App, String> {
    let path = data_file(app).map_err(|e| format!("Couldn't find the app data folder: {e}"))?;
    App::open(Store::new(path.clone())).map_err(|e| {
        // The core's error may name the file again, so the whole message is cleaned.
        friends_core::hide_home(
            &format!("Couldn't read {}: {e}", path.display()),
            friends_core::home_dir().as_deref(),
        )
    })
}

fn lock(state: &State) -> MutexGuard<'_, App> {
    // A panic while the lock is held poisons it. The data may then hold half a change, but every
    // command validates what it saves, and refusing all commands from then on would be worse.
    state.0.lock().unwrap_or_else(|e| e.into_inner())
}

/// The last view sent to the windows, as JSON, so the regular check can skip sending an unchanged one.
struct LastPublished(Mutex<Option<String>>);

/// Sends `view` to the windows (and the menu bar). Callers hold the data lock, so views go out in order.
fn publish(app: &AppHandle, view: &View) {
    send(app, view, serde_json::to_string(view).ok());
}

/// Like `publish`, but does nothing when the windows already show this exact view (the regular
/// check runs every minute and usually changes nothing).
fn publish_if_changed(app: &AppHandle, view: &View) {
    let json = serde_json::to_string(view).ok();
    let last = app.state::<LastPublished>();
    let unchanged = json.is_some() && *last.0.lock().unwrap_or_else(|e| e.into_inner()) == json;
    if !unchanged {
        send(app, view, json);
    }
}

/// Tell every window about the new view; on the Mac, update the menu bar icon's count, and on the
/// iPhone, book the reminders again.
fn send(app: &AppHandle, view: &View, json: Option<String>) {
    *app.state::<LastPublished>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = json;
    let _ = app.emit(VIEW_CHANGED, view);
    #[cfg(target_os = "macos")]
    desktop::show_count(app, view.people.iter().filter(|p| p.due).count());
    #[cfg(target_os = "ios")]
    mobile::views_changed(app);
}

/// The one command the UI calls: runs a friends-core command and tells every window the result.
/// Async, so Tauri runs it off the main thread and a slow save never freezes the window.
#[tauri::command]
async fn call(app: AppHandle, cmd: String, args: Value) -> Result<View, ApiError> {
    let Some(state) = app.try_state::<State>() else {
        #[cfg(target_os = "ios")]
        let reason = app
            .try_state::<CannotOpen>()
            .map_or_else(String::new, |c| c.0.clone());
        #[cfg(not(target_os = "ios"))]
        let reason = String::new();
        return Err(ApiError::new(
            ErrorKind::Storage,
            format!("Friends can't open its data. {reason}"),
        ));
    };
    let mut core = lock(&state);
    let day = today();
    let result = core.call(&cmd, args, day);
    // Every command (get_view too, and one that fails) may have caught up missed targets first, so
    // every one tells the windows. Sent while the lock is held, so two commands' views reach the
    // windows in the order they ran.
    match &result {
        Ok(view) => publish(&app, view),
        Err(_) => publish(&app, &core.view(day)),
    }
    let unsynced = core.has_unsynced();
    drop(core);
    if unsynced {
        sync::nudge(&app, sync::Nudge::Soon);
    }
    result
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // The Mac's single-instance plugin has to come first.
    #[cfg(target_os = "macos")]
    let builder = desktop::plugins(tauri::Builder::default())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            call,
            desktop::get_launch_at_login,
            desktop::set_launch_at_login,
            desktop::open_preferences,
            desktop::hide_dropdown,
            desktop::quit,
            account::get_account,
            account::sign_in_with_apple,
            account::cancel_sign_in,
            account::sign_out
        ]);
    #[cfg(target_os = "ios")]
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_web_auth::init())
        .invoke_handler(tauri::generate_handler![
            call,
            account::get_account,
            account::sign_in_with_apple,
            account::cancel_sign_in,
            account::sign_out
        ]);
    builder
        .setup(|app| {
            let handle = app.handle().clone();
            app.manage(LastPublished(Mutex::new(None)));
            app.manage(account::Account::new());
            app.manage(sync::Syncer::new());
            let core = match open_data(&handle) {
                Ok(core) => core,
                Err(e) => {
                    eprintln!("friends: {e}");
                    #[cfg(target_os = "macos")]
                    desktop::cannot_open(&e);
                    #[cfg(target_os = "ios")]
                    {
                        app.manage(CannotOpen(e));
                        return Ok(());
                    }
                }
            };
            app.manage(State(Mutex::new(core)));
            #[cfg(target_os = "macos")]
            desktop::setup(app)?;
            #[cfg(target_os = "ios")]
            mobile::setup(app)?;
            sync::start(&handle);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the Friends app")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            desktop::on_event(app, &event);
            #[cfg(target_os = "ios")]
            mobile::on_event(app, &event);
        });
}
