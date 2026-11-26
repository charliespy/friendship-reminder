//! The Mac app: the menu bar icon and its dropdown, the Preferences window, notifications as reminders
//! come up, open at login, and the Dock icon.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, LogicalPosition, Manager, Rect, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};
use tauri_plugin_notification::NotificationExt;

use crate::{account, lock, publish_if_changed, today, State};

const DROPDOWN: &str = "dropdown";
const PREFERENCES: &str = "preferences";
const TRAY_ID: &str = "friends";
/// How often to check for missed targets and reminders to send. Cheap, and it also covers the Mac
/// waking from sleep and the date changing at midnight.
const TICK_EVERY: Duration = Duration::from_secs(60);
/// The dropdown's width in points (tauri.conf.json).
const DROPDOWN_WIDTH: f64 = 360.0;
/// Pressing the menu bar icon takes focus from the open dropdown, which hides it; a press on the icon
/// within this time of that is the same press, and must not show the dropdown again.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

/// When the dropdown last hid itself because it lost focus.
struct DropdownHiddenAt(Mutex<Option<Instant>>);

/// The Mac's plugins: one copy at a time, and open at login.
pub fn plugins(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Opening the app again (from Finder or Spotlight) opens Preferences instead of a second copy.
            let _ = show_preferences(app);
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .macos_launcher(MacosLauncher::LaunchAgent)
                // The login item's file keeps the name it had when the app was called Friends
                // (`~/Library/LaunchAgents/Friends.plist`), so "Open at login" stays on after the rename.
                .app_name(LOGIN_ITEM)
                .build(),
        )
}

/// The name of the login item's file, from when the app was called Friends. Debug builds share it:
/// turning "Open at login" on or off in one changes the installed app's login item too.
const LOGIN_ITEM: &str = "Friends";

/// A login item made before the rename still opens the old `Friends.app`. Once, point it at this
/// copy, and only when this copy is the one in Applications (not a .dmg or a moved download that
/// will soon be gone, and not a build of the code). Nothing is written when the login item is off
/// or already opens this app.
fn move_login_item_from_old_app(app: &AppHandle) {
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let file = std::path::Path::new(&home)
        .join("Library/LaunchAgents")
        .join(format!("{LOGIN_ITEM}.plist"));
    let Ok(item) = std::fs::read_to_string(&file) else {
        return; // Open at login is off.
    };
    let from_old_app = item.contains("/Friends.app/Contents/MacOS/");
    let in_applications = std::env::current_exe()
        .map(|exe| exe.starts_with("/Applications"))
        .unwrap_or(false);
    if from_old_app && in_applications {
        if let Err(e) = app.autolaunch().enable() {
            eprintln!("friends: couldn't move Open at login over from Friends.app: {e}");
        }
    }
}

/// The data folder can't be read: a menu bar app that quits at launch would vanish without a word, so
/// say why, then quit.
pub fn cannot_open(e: &str) -> ! {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Friendship Reminder can't open its data")
        .set_description(format!(
            "{e}\n\nCheck that you can read and write this folder, then open Friendship Reminder again."
        ))
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
    std::process::exit(1);
}

/// Shows how many people are due on the menu bar icon.
pub fn show_count(app: &AppHandle, due: usize) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let title = if due > 0 { Some(due.to_string()) } else { None };
        let _ = tray.set_title(title);
        let tooltip = match due {
            0 => "Friendship Reminder: nobody to reach out to right now".to_string(),
            1 => "Friendship Reminder: 1 person to reach out to".to_string(),
            n => format!("Friendship Reminder: {n} people to reach out to"),
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

#[tauri::command]
pub async fn get_launch_at_login(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_launch_at_login(app: AppHandle, on: bool) -> Result<bool, String> {
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
pub fn open_preferences(app: AppHandle) -> Result<(), String> {
    show_preferences(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hide_dropdown(app: AppHandle) {
    if let Some(w) = app.get_webview_window(DROPDOWN) {
        let _ = w.hide();
    }
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

pub fn show_preferences(app: &AppHandle) -> tauri::Result<()> {
    hide_dropdown(app.clone());
    if let Some(w) = app.get_webview_window(PREFERENCES) {
        w.unminimize()?;
        w.show()?;
        w.set_focus()?;
        return Ok(());
    }
    let w = WebviewWindowBuilder::new(app, PREFERENCES, WebviewUrl::App("preferences.html".into()))
        .title("Friendship Reminder")
        .inner_size(720.0, 520.0)
        .min_inner_size(600.0, 440.0)
        .resizable(true)
        .center()
        .build()?;
    let closing = app.clone();
    w.on_window_event(move |e| {
        if let WindowEvent::Destroyed = e {
            account::cancel_waiting(&closing);
        }
    });
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
        let body = friends_core::api::reminder_text(&person);
        // macOS delivers it later; the person also stays in the dropdown and the menu bar count.
        if let Err(e) = app
            .notification()
            .builder()
            .title("Friendship Reminder")
            .body(body)
            .show()
        {
            eprintln!("friends: couldn't send a notification: {e}");
        }
    }
}

/// After launch: the menu bar icon, the dropdown, notifications and the regular check.
pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    app.manage(DropdownHiddenAt(Mutex::new(None)));
    move_login_item_from_old_app(&handle);

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Friendship Reminder")
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

    // A first launch shows the guide (signing in, adding people): the menu bar icon alone is easy
    // to miss.
    let new_here = handle
        .try_state::<crate::State>()
        .is_some_and(|s| !crate::lock(&s).data().guide.is_done());
    if new_here {
        let _ = show_preferences(&handle);
    }

    // Ask once for permission to notify; macOS remembers the answer.
    let _ = handle.notification().request_permission();

    std::thread::spawn(move || loop {
        tick(&handle);
        std::thread::sleep(TICK_EVERY);
    });
    Ok(())
}

/// Friendship Reminder lives in both the menu bar and the Dock. Clicking its Dock icon, or opening it again from
/// Finder or Spotlight while it runs (macOS doesn't start a second copy), sends the running one a
/// "reopen". Show Preferences, so the app is reachable even when its menu bar icon is hidden (a full
/// menu bar, the notch).
pub fn on_event(app: &AppHandle, event: &tauri::RunEvent) {
    if let tauri::RunEvent::Reopen { .. } = event {
        let _ = show_preferences(app);
    }
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
