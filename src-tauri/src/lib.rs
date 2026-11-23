//! The Tauri shell: the menu bar icon, the dropdown and Preferences windows, notifications and launch at
//! login. The rules and the data live in friends-core; this file only connects them to macOS.

mod account;

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use chrono::{Local, NaiveDate};
use friends_core::api::{ApiError, App, View};
use friends_core::store::Store;
use serde_json::Value;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, LogicalPosition, Manager, Rect, WebviewUrl, WebviewWindowBuilder,
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

/// Sends `view` to the windows and the menu bar. Callers hold the data lock, so views go out in order.
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

/// Tell every window about the new view, and update the menu bar icon's count.
fn send(app: &AppHandle, view: &View, json: Option<String>) {
    *app.state::<LastPublished>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = json;
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
    drop(core);
    result
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

/// A screen in points (macOS's global coordinates, top-left origin) and its scale (2 on Retina).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Screen {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    scale: f64,
}

/// Where the dropdown goes, in points: centred under the icon, a few points below the menu bar, and
/// kept inside the screen the icon is on.
///
/// The units on macOS are a trap. The tray reports the icon's rect as points times the scale of the
/// icon's own screen, so on a Retina laptop next to a plain monitor the same pixel x can mean either
/// screen. The cursor (which is on the icon: the user just pressed it) is reported as points times the
/// main screen's scale, one scale for everything, so it tells which screen it is. Then the icon's
/// pixels are turned back into points with that screen's scale, and everything is done in points.
fn dropdown_origin(
    icon_px: (f64, f64, f64, f64),
    cursor_pt: Option<(f64, f64)>,
    screens: &[Screen],
    fallback_scale: f64,
) -> (f64, f64) {
    let contains = |s: &Screen, x: f64, y: f64| {
        x >= s.left && x < s.left + s.width && y >= s.top && y < s.top + s.height
    };
    let screen = cursor_pt
        .and_then(|(cx, cy)| screens.iter().copied().find(|s| contains(s, cx, cy)))
        // No cursor to go by (rare): a screen whose menu bar holds the icon at that screen's scale.
        // The pixels can fit two screens, and with "Displays have separate Spaces" (the default)
        // each has a menu bar, so this is a guess: the main one (at 0, 0).
        .or_else(|| {
            let fits: Vec<Screen> = screens
                .iter()
                .copied()
                .filter(|s| {
                    let (x, y) = (icon_px.0 / s.scale, icon_px.1 / s.scale);
                    contains(s, x, y) && y < s.top + 40.0
                })
                .collect();
            let main = fits.iter().find(|s| s.left == 0.0 && s.top == 0.0);
            main.or(fits.first()).copied()
        });
    let scale = screen.map_or(fallback_scale, |s| s.scale);
    let (x, y, w, h) = (
        icon_px.0 / scale,
        icon_px.1 / scale,
        icon_px.2 / scale,
        icon_px.3 / scale,
    );
    let left = x + w / 2.0 - DROPDOWN_WIDTH / 2.0;
    let left = match screen {
        Some(s) => left.min(s.left + s.width - DROPDOWN_WIDTH).max(s.left),
        None => left,
    };
    (left, y + h + 4.0)
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
    let window_scale = w.scale_factor().unwrap_or(1.0);
    let (pos, size) = (
        icon.position.to_physical::<f64>(window_scale),
        icon.size.to_physical::<f64>(window_scale),
    );
    let main_scale = app
        .primary_monitor()
        .ok()
        .flatten()
        .map_or(window_scale, |m| m.scale_factor());
    let cursor_pt = app
        .cursor_position()
        .ok()
        .map(|c| (c.x / main_scale, c.y / main_scale));
    // Each monitor's position and size are its points times its own scale.
    let screens: Vec<Screen> = app
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let (p, s, scale) = (m.position(), m.size(), m.scale_factor());
            Screen {
                left: f64::from(p.x) / scale,
                top: f64::from(p.y) / scale,
                width: f64::from(s.width) / scale,
                height: f64::from(s.height) / scale,
                scale,
            }
        })
        .collect();
    let (x, y) = dropdown_origin(
        (pos.x, pos.y, size.width, size.height),
        cursor_pt,
        &screens,
        window_scale,
    );
    // In points, which macOS places as they are, whatever screen the dropdown was on before.
    let _ = w.set_position(LogicalPosition::new(x, y));
    let _ = w.show();
    let _ = w.set_focus();
}

/// The regular check: catch up missed targets, send due notifications, refresh the windows.
fn tick(app: &AppHandle) {
    let state = app.state::<State>();
    let day = today();
    let due = {
        let mut a = lock(&state);
        match a.tick(day) {
            Ok(due) => {
                // Under the lock, for the same reason as in `call`.
                publish_if_changed(app, &a.view(day));
                due
            }
            Err(e) => {
                // The data was rolled back; still refresh the count and dates (the day may have moved on).
                eprintln!("friends: couldn't save after the regular check: {e}");
                publish_if_changed(app, &a.view(day));
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
            quit,
            account::get_account,
            account::sign_in_with_apple,
            account::cancel_sign_in,
            account::sign_out
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let core = match open_data(&handle) {
                Ok(core) => core,
                Err(e) => {
                    // A menu bar app that quits at launch would vanish without a word; say why.
                    eprintln!("friends: {e}");
                    rfd::MessageDialog::new()
                        .set_level(rfd::MessageLevel::Error)
                        .set_title("Friends can't open its data")
                        .set_description(format!(
                            "{e}\n\nCheck that you can read and write this folder, then open Friends again."
                        ))
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show();
                    std::process::exit(1);
                }
            };
            app.manage(State(Mutex::new(core)));
            app.manage(account::Account::new());
            let restoring = handle.clone();
            std::thread::spawn(move || account::restore(&restoring));

            // A menu bar app: no Dock icon, no app menu. Switched after the data opens, so the alert
            // above still comes to the front as a normal app's would.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            app.manage(DropdownHiddenAt(Mutex::new(None)));
            app.manage(LastPublished(Mutex::new(None)));

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
        .build(tauri::generate_context!())
        .expect("error while building the Friends app")
        .run(|app, event| {
            // Opening Friends again from Finder or Spotlight while it runs doesn't start a second
            // copy on macOS; the system sends the running one a "reopen". Show Preferences, so the
            // app is reachable even when its menu bar icon is hidden (a full menu bar, the notch).
            if let tauri::RunEvent::Reopen { .. } = event {
                let _ = show_preferences(app);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Retina laptop (1512 x 982 pt, scale 2) with a plain 1920 x 1080 monitor to its right.
    const LAPTOP: Screen = Screen {
        left: 0.0,
        top: 0.0,
        width: 1512.0,
        height: 982.0,
        scale: 2.0,
    };
    const EXTERNAL: Screen = Screen {
        left: 1512.0,
        top: 0.0,
        width: 1920.0,
        height: 1080.0,
        scale: 1.0,
    };

    #[test]
    fn dropdown_is_centred_under_the_icon() {
        // Icon at 1000 pt on the laptop, 20 pt wide, 24 pt tall: reported doubled.
        let at = dropdown_origin(
            (2000.0, 0.0, 40.0, 48.0),
            Some((1005.0, 5.0)),
            &[LAPTOP, EXTERNAL],
            2.0,
        );
        assert_eq!(at, (1010.0 - 180.0, 28.0));
    }

    #[test]
    fn the_cursor_tells_which_screen_the_icon_is_on() {
        // Icon at 3000 pt on the plain monitor, reported as 3000 px: the same number the laptop
        // would give for an icon at 1500 pt. The cursor says it's the monitor.
        let at = dropdown_origin(
            (3000.0, 0.0, 20.0, 24.0),
            Some((3005.0, 5.0)),
            &[LAPTOP, EXTERNAL],
            2.0,
        );
        assert_eq!(at, (3010.0 - 180.0, 28.0));
        // Icon at 1500 pt on the laptop, also reported as 3000 px: kept inside the laptop's edge.
        let at = dropdown_origin(
            (3000.0, 0.0, 40.0, 48.0),
            Some((1505.0, 5.0)),
            &[LAPTOP, EXTERNAL],
            1.0,
        );
        assert_eq!(at, (1512.0 - 360.0, 28.0));
    }

    #[test]
    fn dropdown_stays_inside_the_screen() {
        // Near the left edge of a screen placed left of the laptop.
        let left = Screen {
            left: -1920.0,
            ..EXTERNAL
        };
        let at = dropdown_origin(
            (-1910.0, 0.0, 20.0, 24.0),
            Some((-1905.0, 5.0)),
            &[LAPTOP, left],
            2.0,
        );
        assert_eq!(at.0, -1920.0);
        // No cursor: the screen whose menu bar holds the icon (here the laptop, at its scale 2).
        let at = dropdown_origin((2000.0, 0.0, 40.0, 48.0), None, &[LAPTOP], 1.0);
        assert_eq!(at, (830.0, 28.0));
        // No cursor, and the pixels fit both screens: the main one (the laptop here).
        let at = dropdown_origin((3000.0, 0.0, 20.0, 24.0), None, &[EXTERNAL, LAPTOP], 1.0);
        assert_eq!(at, (1512.0 - 360.0, 16.0));
        // No cursor and no screen fits: centred, using the fallback scale.
        let at = dropdown_origin((2000.0, 0.0, 40.0, 48.0), None, &[], 2.0);
        assert_eq!(at, (830.0, 28.0));
    }

    #[test]
    fn the_click_that_hid_the_dropdown_does_not_show_it_again() {
        let now = Instant::now();
        assert!(should_show(None, now));
        assert!(!should_show(Some(now), now + Duration::from_millis(50)));
        assert!(should_show(Some(now), now + Duration::from_millis(400)));
    }
}
