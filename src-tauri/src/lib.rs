//! The Tauri shell: the menu bar icon, the dropdown and Preferences windows, notifications and launch at
//! login. The rules and the data live in friends-core; this file only connects them to macOS.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use chrono::{Local, NaiveDate};
use friends_core::api::{ApiError, App, View};
use friends_core::store::Store;
use serde_json::Value;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, Rect, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_notification::NotificationExt;

const DROPDOWN: &str = "dropdown";
const PREFERENCES: &str = "preferences";
const VIEW_CHANGED: &str = "view-changed";
const TRAY_ID: &str = "friends";
/// How often to check for missed targets and reminders to send. Cheap, and it also covers the Mac
/// waking from sleep and the date changing at midnight.
const TICK_EVERY: Duration = Duration::from_secs(60);

struct State(Mutex<App>);

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
/// (`~/Library/Application Support/io.github.charliespy.friends`). `FRIENDS_DATA_FILE` overrides it in
/// debug builds, so trying the app never touches real data.
fn data_file(app: &AppHandle) -> Result<PathBuf, String> {
    #[cfg(debug_assertions)]
    if let Ok(path) = std::env::var("FRIENDS_DATA_FILE") {
        return Ok(PathBuf::from(path));
    }
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("friends.json"))
}

fn lock(state: &State) -> MutexGuard<'_, App> {
    // App::call undoes a change that fails, so a lock poisoned by a panic still holds whole data.
    state.0.lock().unwrap_or_else(|e| e.into_inner())
}

/// Tell every window about the new view, and update the menu bar icon's count.
fn publish(app: &AppHandle, view: &View) {
    let _ = app.emit(VIEW_CHANGED, view);
    let due = view.people.iter().filter(|p| p.due).count();
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let title = if due > 0 { Some(due.to_string()) } else { None };
        let _ = tray.set_title(title);
        let tooltip = match due {
            0 => "Friends: nobody to reach out to right now".to_string(),
            1 => "Friends: 1 person to reach out to".to_string(),
            n => format!("Friends: {n} people to reach out to"),
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

/// The one command the UI calls: runs a friends-core command and tells every window the result.
#[tauri::command]
fn call(
    app: AppHandle,
    state: tauri::State<'_, State>,
    cmd: String,
    args: Value,
) -> Result<View, ApiError> {
    let view = lock(&state).call(&cmd, args, today())?;
    publish(&app, &view);
    Ok(view)
}

#[tauri::command]
fn get_launch_at_login(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_launch_at_login(app: AppHandle, on: bool) -> Result<bool, String> {
    let launcher = app.autolaunch();
    let result = if on {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(|e| format!("macOS didn't accept the change: {e}"))?;
    launcher.is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn open_preferences(app: AppHandle) -> Result<(), String> {
    show_preferences(&app).map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_dropdown(app: AppHandle) {
    if let Some(w) = app.get_webview_window(DROPDOWN) {
        let _ = w.hide();
    }
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

fn show_preferences(app: &AppHandle) -> tauri::Result<()> {
    hide_dropdown(app.clone());
    if let Some(w) = app.get_webview_window(PREFERENCES) {
        w.unminimize()?;
        w.show()?;
        w.set_focus()?;
        return Ok(());
    }
    let w = WebviewWindowBuilder::new(app, PREFERENCES, WebviewUrl::App("preferences.html".into()))
        .title("Friends")
        .inner_size(720.0, 520.0)
        .min_inner_size(600.0, 440.0)
        .resizable(true)
        .center()
        .build()?;
    w.set_focus()?;
    Ok(())
}

/// Shows the dropdown under the menu bar icon, or hides it when it's already showing.
fn toggle_dropdown(app: &AppHandle, icon: Rect) {
    let Some(w) = app.get_webview_window(DROPDOWN) else {
        return;
    };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    let scale = w.scale_factor().unwrap_or(1.0);
    let (pos, size) = (
        icon.position.to_physical::<f64>(scale),
        icon.size.to_physical::<f64>(scale),
    );
    let width = w
        .outer_size()
        .map(|s| f64::from(s.width))
        .unwrap_or(360.0 * scale);
    // Centred under the icon, a few points below the menu bar.
    let x = pos.x + size.width / 2.0 - width / 2.0;
    let y = pos.y + size.height + 4.0 * scale;
    let _ = w.set_position(PhysicalPosition::new(x.max(0.0), y));
    let _ = w.show();
    let _ = w.set_focus();
}

/// The regular check: catch up missed targets, send due notifications, refresh the windows.
fn tick(app: &AppHandle) {
    let state = app.state::<State>();
    let day = today();
    let (due, view) = {
        let mut a = lock(&state);
        match a.tick(day) {
            Ok(due) => (due, a.view(day)),
            Err(e) => {
                eprintln!("friends: couldn't save after the regular check: {e}");
                return;
            }
        }
    };
    for person in due {
        let body = if person.overdue {
            format!(
                "You meant to reach out to {} — there's still time this round.",
                person.name
            )
        } else {
            format!("It's time to reach out to {}.", person.name)
        };
        let _ = app
            .notification()
            .builder()
            .title("Friends")
            .body(body)
            .show();
    }
    publish(app, &view);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Opening the app again (from Finder or Spotlight) opens Preferences instead of a second copy.
            let _ = show_preferences(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            call,
            get_launch_at_login,
            set_launch_at_login,
            open_preferences,
            hide_dropdown,
            quit
        ])
        .setup(|app| {
            // A menu bar app: no Dock icon, no app menu.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let store = Store::new(data_file(&handle)?);
            let core = App::open(store)?;
            app.manage(State(Mutex::new(core)));

            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(icon)
                .icon_as_template(true)
                .tooltip("Friends")
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        toggle_dropdown(tray.app_handle(), rect);
                    }
                })
                .build(app)?;

            if let Some(w) = app.get_webview_window(DROPDOWN) {
                let hide = w.clone();
                w.on_window_event(move |e| {
                    if let WindowEvent::Focused(false) = e {
                        let _ = hide.hide();
                    }
                });
            }

            // Ask once for permission to notify; macOS remembers the answer.
            let _ = handle.notification().request_permission();

            std::thread::spawn(move || loop {
                tick(&handle);
                std::thread::sleep(TICK_EVERY);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the Friends app");
}
