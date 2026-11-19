//! The Tauri shell: the menu bar icon, the dropdown and Preferences windows, notifications and launch at
//! login. The rules and the data live in friends-core; this file only connects them to macOS.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

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
/// The dropdown's width in points (tauri.conf.json).
const DROPDOWN_WIDTH: f64 = 360.0;
/// Pressing the menu bar icon takes focus from the open dropdown, which hides it; a press on the icon
/// within this time of that is the same press, and must not show the dropdown again.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

struct State(Mutex<App>);

/// When the dropdown last hid itself because it lost focus.
struct DropdownHiddenAt(Mutex<Option<Instant>>);

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
    // A panic while the lock is held poisons it. The data may then hold half a change, but every
    // command validates what it saves, and refusing all commands from then on would be worse.
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
/// Async, so Tauri runs it off the main thread and a slow save never freezes the menu bar.
#[tauri::command]
async fn call(
    app: AppHandle,
    state: tauri::State<'_, State>,
    cmd: String,
    args: Value,
) -> Result<View, ApiError> {
    let view = lock(&state).call(&cmd, args, today())?;
    // Every command (get_view too) may have caught up missed targets, so every one tells the windows.
    publish(&app, &view);
    Ok(view)
}

#[tauri::command]
async fn get_launch_at_login(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_launch_at_login(app: AppHandle, on: bool) -> Result<bool, String> {
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

/// Whether a click on the icon should show the dropdown: not when the dropdown hid itself because of
/// that same click.
fn should_show(hidden_at: Option<Instant>, now: Instant) -> bool {
    hidden_at.is_none_or(|t| now.saturating_duration_since(t) >= REOPEN_GUARD)
}

/// Which of the screens (left, top, width, height, in pixels) holds the point (x, y), also in pixels.
/// The tray reports the icon in pixels and each monitor's position and size are in pixels at its own
/// scale, so the comparison needs no conversion (monitor_from_point would want points on macOS).
fn screen_at(x: f64, y: f64, screens: &[(f64, f64, f64, f64)]) -> Option<usize> {
    screens
        .iter()
        .position(|&(left, top, w, h)| x >= left && x < left + w && y >= top && y < top + h)
}

/// The dropdown's left edge: centred under the icon, kept inside the screen the icon is on.
/// Everything in physical pixels; `screen` is that screen's left edge and width.
fn dropdown_x(icon_x: f64, icon_width: f64, width: f64, screen: Option<(f64, f64)>) -> f64 {
    let x = icon_x + icon_width / 2.0 - width / 2.0;
    match screen {
        Some((left, screen_width)) => x.min(left + screen_width - width).max(left),
        None => x,
    }
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
    let hidden_at = *app
        .state::<DropdownHiddenAt>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if !should_show(hidden_at, Instant::now()) {
        return;
    }
    // The screen under the icon decides the scale (a Retina laptop next to a plain monitor differ).
    let window_scale = w.scale_factor().unwrap_or(1.0);
    let (pos, size) = (
        icon.position.to_physical::<f64>(window_scale),
        icon.size.to_physical::<f64>(window_scale),
    );
    let monitors = app.available_monitors().unwrap_or_default();
    let rects: Vec<_> = monitors
        .iter()
        .map(|m| {
            let (p, s) = (m.position(), m.size());
            (
                f64::from(p.x),
                f64::from(p.y),
                f64::from(s.width),
                f64::from(s.height),
            )
        })
        .collect();
    let found = screen_at(pos.x, pos.y, &rects);
    let scale = found.map_or(window_scale, |i| monitors[i].scale_factor());
    let screen = found.map(|i| (rects[i].0, rects[i].2));
    let x = dropdown_x(pos.x, size.width, DROPDOWN_WIDTH * scale, screen);
    // A few points below the menu bar.
    let y = pos.y + size.height + 4.0 * scale;
    let _ = w.set_position(PhysicalPosition::new(x, y));
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
        // macOS delivers it later; the person also stays in the dropdown and the menu bar count.
        if let Err(e) = app
            .notification()
            .builder()
            .title("Friends")
            .body(body)
            .show()
        {
            eprintln!("friends: couldn't send a notification: {e}");
        }
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
            app.manage(DropdownHiddenAt(Mutex::new(None)));

            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(icon)
                .icon_as_template(true)
                .tooltip("Friends")
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        // On press, like other menu bar items, and the same event that takes focus
                        // from an open dropdown (so the reopen guard is not a race with a long press).
                        button_state: MouseButtonState::Down,
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
                let app = handle.clone();
                w.on_window_event(move |e| {
                    // Cmd+W (the default menu's Close Window) would destroy the dropdown for good.
                    if let WindowEvent::CloseRequested { api, .. } = e {
                        api.prevent_close();
                        let _ = hide.hide();
                    }
                    if let WindowEvent::Focused(false) = e {
                        if hide.is_visible().unwrap_or(false) {
                            *app.state::<DropdownHiddenAt>()
                                .0
                                .lock()
                                .unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
                        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropdown_is_centred_under_the_icon() {
        assert_eq!(dropdown_x(1000.0, 40.0, 720.0, Some((0.0, 3000.0))), 660.0);
    }

    #[test]
    fn dropdown_stays_on_the_icons_screen() {
        // Near the right edge: pushed left so it ends at the screen's edge.
        assert_eq!(dropdown_x(2950.0, 40.0, 720.0, Some((0.0, 3000.0))), 2280.0);
        // Near the left edge of a screen placed left of the main one.
        assert_eq!(
            dropdown_x(-1990.0, 40.0, 720.0, Some((-2000.0, 2000.0))),
            -2000.0
        );
        // Unknown screen: just centred.
        assert_eq!(dropdown_x(10.0, 40.0, 720.0, None), -330.0);
    }

    #[test]
    fn finds_the_screen_that_holds_the_icon() {
        // A Retina laptop (3024 px wide) with a plain monitor to its right and one to its left.
        let screens = [
            (0.0, 0.0, 3024.0, 1964.0),
            (3024.0, 0.0, 1920.0, 1080.0),
            (-2560.0, 0.0, 2560.0, 1440.0),
        ];
        assert_eq!(screen_at(2800.0, 10.0, &screens), Some(0));
        assert_eq!(screen_at(3100.0, 10.0, &screens), Some(1));
        assert_eq!(screen_at(-5.0, 10.0, &screens), Some(2));
        assert_eq!(screen_at(9000.0, 10.0, &screens), None);
    }

    #[test]
    fn the_click_that_hid_the_dropdown_does_not_show_it_again() {
        let now = Instant::now();
        assert!(should_show(None, now));
        assert!(!should_show(Some(now), now + Duration::from_millis(50)));
        assert!(should_show(Some(now), now + Duration::from_millis(400)));
    }
}
