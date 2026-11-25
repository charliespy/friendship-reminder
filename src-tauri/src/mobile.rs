//! The iPhone app: one window with the phone layout, and reminders booked ahead with iOS.
//!
//! iOS stops the app soon after it leaves the screen, so it can't check every minute the way the Mac
//! app does. Instead, whenever the list changes (and when the app comes back on screen), it hands iOS
//! the coming reminders, replacing the ones that changed. friends-core decides which reminders, when
//! and what they say (`plan_notifications`, `bookings`); this file only books them.

use std::collections::HashMap;
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::time::Duration;

use chrono::{Datelike, Local, NaiveDateTime, Timelike};
use friends_core::api::bookings;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::{NotificationExt, PermissionState, Schedule};

use crate::{lock, publish_if_changed, today, State};

/// How often to look again while the app is on screen, for the date changing at midnight.
const TICK_EVERY: Duration = Duration::from_secs(60);

/// Where to ask for the reminders to be booked again.
struct Booker(Mutex<Sender<()>>);

/// What iOS holds now: each booked reminder's id, with when it comes and what it says.
type Booked = HashMap<i32, (NaiveDateTime, String)>;

/// The list changed: book the reminders again (on the booking thread, which takes the data lock).
pub fn views_changed(app: &AppHandle) {
    if let Some(booker) = app.try_state::<Booker>() {
        let _ = booker.0.lock().unwrap_or_else(|e| e.into_inner()).send(());
    }
}

/// After launch: start booking reminders, and look again every minute while on screen.
pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel::<()>();
    app.manage(Booker(Mutex::new(tx)));
    let handle = app.handle().clone();
    std::thread::spawn(move || {
        // Shows iOS's prompt the first time and waits for the answer; iOS remembers it. Turning
        // notifications on or off later in iOS Settings is read again at every booking.
        if let Err(e) = handle.notification().request_permission() {
            eprintln!("friends: couldn't ask to send notifications: {e}");
        }
        // Unknown at first: whatever an earlier run booked is cleared before booking.
        let mut booked: Option<Booked> = None;
        loop {
            book(&handle, &mut booked);
            // Many changes at once book once.
            if rx.recv().is_err() {
                return;
            }
            while rx.try_recv().is_ok() {}
        }
    });
    let ticker = app.handle().clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(TICK_EVERY);
        views_changed(&ticker);
    });
    Ok(())
}

/// Back on screen: the day may have moved on, or notifications been allowed, while the app was
/// stopped.
pub fn on_event(app: &AppHandle, event: &tauri::RunEvent) {
    if let tauri::RunEvent::Resumed = event {
        views_changed(app);
    }
}

/// A reminder's id for iOS: the person's id (they start at 1 and stay small; the modulo only keeps
/// it a valid i32).
fn id_of(person: u64) -> i32 {
    i32::try_from(person % i32::MAX as u64).unwrap_or(0)
}

/// The plugin reads the time it is given as the device's own clock, whatever offset it carries, so
/// the wall-clock time goes over as if it were UTC.
fn as_plugin_time(at: NaiveDateTime) -> Option<time::OffsetDateTime> {
    let month = time::Month::try_from(u8::try_from(at.month()).ok()?).ok()?;
    let date =
        time::Date::from_calendar_date(at.year(), month, u8::try_from(at.day()).ok()?).ok()?;
    let clock = time::Time::from_hms(
        u8::try_from(at.hour()).ok()?,
        u8::try_from(at.minute()).ok()?,
        0,
    )
    .ok()?;
    Some(time::PrimitiveDateTime::new(date, clock).assume_utc())
}

/// Works out the reminders, and books the ones that changed: what's no longer wanted, or changed,
/// is cancelled, and what's new, or changed, is booked. A booking that fails is left out of
/// `booked`, so the next round tries it again.
fn book(app: &AppHandle, booked: &mut Option<Booked>) {
    let plan = {
        let state = app.state::<State>();
        let mut core = lock(&state);
        let day = today();
        let plan = core.plan_notifications(day);
        // Missed targets may have moved, or the day changed: show it. Under the lock, so views go
        // out in order.
        publish_if_changed(app, &core.view(day));
        match plan {
            Ok(plan) => plan,
            Err(e) => {
                eprintln!("friends: couldn't save while booking reminders: {e}");
                return;
            }
        }
    };
    let notification = app.notification();
    let allowed = matches!(
        notification.permission_state(),
        Ok(PermissionState::Granted)
    );
    let wanted: Booked = if allowed {
        bookings(&plan, Local::now().naive_local())
            .into_iter()
            .map(|b| (id_of(b.person), (b.at, b.body)))
            .collect()
    } else {
        HashMap::new()
    };
    let current = match booked {
        Some(current) => current,
        None => {
            if let Err(e) = notification.cancel_all() {
                eprintln!("friends: couldn't clear the booked reminders: {e}");
                return;
            }
            booked.insert(HashMap::new())
        }
    };
    let gone: Vec<i32> = current
        .iter()
        .filter(|(id, b)| wanted.get(id) != Some(b))
        .map(|(id, _)| *id)
        .collect();
    if !gone.is_empty() {
        if let Err(e) = notification.cancel(gone.clone()) {
            eprintln!("friends: couldn't cancel booked reminders: {e}");
            // Unsure what iOS still holds: start from a clean slate next time.
            *booked = None;
            return;
        }
        for id in &gone {
            current.remove(id);
        }
    }
    for (id, (at, body)) in wanted {
        if current.contains_key(&id) {
            continue;
        }
        let Some(date) = as_plugin_time(at) else {
            continue;
        };
        let result = notification
            .builder()
            .id(id)
            .title("Friends")
            .body(&body)
            .schedule(Schedule::At {
                date,
                repeating: false,
                allow_while_idle: false,
            })
            .show();
        match result {
            Ok(()) => {
                current.insert(id, (at, body));
            }
            Err(e) => eprintln!("friends: couldn't book a reminder: {e}"),
        }
    }
    #[cfg(debug_assertions)]
    eprintln!("friends: {} reminders booked", current.len());
}
