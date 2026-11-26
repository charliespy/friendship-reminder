//! The Tauri shell, shared by the Mac and iPhone apps: the data, the one command the UI calls, and the
//! account and sync. The rules and the data live in friends-core; this file only connects them to the
//! windows. `desktop.rs` holds the Mac's menu bar app and `mobile.rs` the iPhone's.

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
compile_error!("Friendship Reminder is a Mac and iPhone app: build it for macOS or iOS.");

mod account;
mod contacts;
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

/// Sent to the windows when a link from a contact card opened the app on someone: take them with
/// `take_shown_person`.
const SHOW_PERSON: &str = "show-person";

/// The person a link from a contact card opened the app on, until a window shows them. Kept, not
/// only sent, since the window may still be loading.
struct ShownPerson(Mutex<Option<u64>>);

/// The app was opened from a link: the line Friendship Reminder writes on a contact card. Shows the
/// app (Preferences on the Mac), and the person, if they're still in the list. Other links (the
/// iPhone's sign-in comes back on the same address) are left alone.
fn open_links(app: &AppHandle, urls: &[tauri::Url]) {
    let Some(state) = app.try_state::<State>() else {
        return;
    };
    let person = {
        let core = lock(&state);
        let data = core.data();
        let ours = urls
            .iter()
            .any(|u| friends_core::contacts::uid_in_link(u.as_str()).is_some());
        if !ours {
            return;
        }
        urls.iter()
            .find_map(|u| friends_core::contacts::person_in_link(data, u.as_str()))
    };
    if let Some(id) = person {
        *app.state::<ShownPerson>()
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(id);
    }
    #[cfg(target_os = "macos")]
    let _ = desktop::show_preferences(app);
    if let Some(id) = person {
        let _ = app.emit(SHOW_PERSON, id);
    }
}

/// The person a link opened the app on, once: the window that asks first shows them.
#[tauri::command]
fn take_shown_person(app: AppHandle) -> Option<u64> {
    app.state::<ShownPerson>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
}

/// Whether the app may send notifications, as the system answers.
#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NotificationAccess {
    Allowed,
    /// Not asked yet (iPhone): the app asks only when the user taps to turn reminders on.
    NotAsked,
    Denied,
}

/// Whether the app may send notifications. The Mac's always may: macOS asks at launch.
#[tauri::command]
async fn notification_access(app: AppHandle) -> Result<NotificationAccess, String> {
    // Off the async runtime: the plugin's call waits on the main thread.
    #[cfg(target_os = "ios")]
    return tauri::async_runtime::spawn_blocking(move || mobile::notification_access(&app))
        .await
        .map_err(|e| e.to_string())?;
    #[cfg(not(target_os = "ios"))]
    {
        let _ = app;
        Ok(NotificationAccess::Allowed)
    }
}

/// Asks the system whether the app may send notifications (iOS's question, the first time), and
/// answers what the user chose.
#[tauri::command]
async fn allow_notifications(app: AppHandle) -> Result<NotificationAccess, String> {
    #[cfg(target_os = "ios")]
    return tauri::async_runtime::spawn_blocking(move || mobile::allow_notifications(&app))
        .await
        .map_err(|e| e.to_string())?;
    #[cfg(not(target_os = "ios"))]
    {
        let _ = app;
        Ok(NotificationAccess::Allowed)
    }
}

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
    with_core(&app, |core, day| Ok((core.call(&cmd, args, day)?, ()))).map(|(view, ())| view)
}

/// Runs `f` on the data and tells every window the new view, even when `f` fails (it may have
/// caught up missed targets first), then nudges sync when a change waits to upload.
fn with_core<T>(
    app: &AppHandle,
    f: impl FnOnce(&mut App, NaiveDate) -> Result<(View, T), ApiError>,
) -> Result<(View, T), ApiError> {
    let Some(state) = app.try_state::<State>() else {
        #[cfg(target_os = "ios")]
        let reason = app
            .try_state::<CannotOpen>()
            .map_or_else(String::new, |c| c.0.clone());
        #[cfg(not(target_os = "ios"))]
        let reason = String::new();
        return Err(ApiError::new(
            ErrorKind::Storage,
            format!("Friendship Reminder can't open its data. {reason}"),
        ));
    };
    let mut core = lock(&state);
    let day = today();
    let result = f(&mut core, day);
    // Sent while the lock is held, so two commands' views reach the windows in the order they ran.
    match &result {
        Ok((view, _)) => publish(app, view),
        Err(_) => publish(app, &core.view(day)),
    }
    let unsynced = core.has_unsynced();
    drop(core);
    if unsynced {
        sync::nudge(app, sync::Nudge::Soon);
    }
    result
}

/// The first-run guide's sign-in step is behind you: signed in, or there's no account server to
/// sign in to. Runs on its own task, so it never waits on a lock its caller holds.
fn finish_guide_sign_in(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let done = app
            .try_state::<State>()
            .is_some_and(|s| lock(&s).data().guide.sign_in);
        if !done {
            let finish = serde_json::json!({ "step": "signIn" });
            if let Err(e) = with_core(&app, |core, day| {
                Ok((core.call("finish_guide", finish, day)?, ()))
            }) {
                eprintln!("friends: couldn't save the guide's progress: {e}");
            }
        }
    });
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
            account::sign_out,
            contacts::list_contacts,
            contacts::share_more_contacts,
            contacts::import_contacts,
            notification_access,
            allow_notifications,
            take_shown_person
        ]);
    #[cfg(target_os = "ios")]
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_phone_sheets::init())
        .invoke_handler(tauri::generate_handler![
            call,
            account::get_account,
            account::sign_in_with_apple,
            account::cancel_sign_in,
            account::sign_out,
            contacts::list_contacts,
            contacts::share_more_contacts,
            contacts::import_contacts,
            notification_access,
            allow_notifications,
            take_shown_person
        ]);
    builder
        .setup(|app| {
            let handle = app.handle().clone();
            app.manage(LastPublished(Mutex::new(None)));
            app.manage(ShownPerson(Mutex::new(None)));
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
            // Without an account server there's no sign-in step in the guide.
            if app.state::<account::Account>().config().is_none() {
                finish_guide_sign_in(&handle);
            }
            #[cfg(target_os = "macos")]
            desktop::setup(app)?;
            #[cfg(target_os = "ios")]
            mobile::setup(app)?;
            sync::start(&handle);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the Friendship Reminder app")
        .run(|app, event| {
            if let tauri::RunEvent::Opened { urls } = &event {
                open_links(app, urls);
            }
            #[cfg(target_os = "macos")]
            desktop::on_event(app, &event);
            #[cfg(target_os = "ios")]
            mobile::on_event(app, &event);
        });
}
