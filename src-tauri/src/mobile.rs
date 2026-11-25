//! The iPhone app: one window with the phone layout, and reminders booked ahead with iOS.
//!
//! iOS stops the app soon after it leaves the screen, so it can't check every minute the way the Mac
//! app does. Instead, whenever the list changes (and when the app comes back on screen), it hands iOS
//! the coming reminders, each for 9:00 in the morning of its day, replacing what it booked before.
//! friends-core decides which reminders and what they say; this file only books them.

use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::time::Duration;

use chrono::{Local, NaiveDate, NaiveTime, TimeZone};
use friends_core::api::Planned;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::{NotificationExt, PermissionState, Schedule};

use crate::{lock, publish_if_changed, today, State};

/// The hour of the morning a reminder comes, local time.
const MORNING: u32 = 9;
/// How often to look again while the app is on screen, for the date changing at midnight.
const TICK_EVERY: Duration = Duration::from_secs(60);

/// Where to ask for the reminders to be booked again.
struct Booker(Mutex<Option<Sender<()>>>);

/// A reminder as booked: its id, the second it fires (since 1970) and its text.
type Booked = (i32, i64, String);

/// The list changed: book the reminders again (on the booking thread, which takes the data lock).
pub fn views_changed(app: &AppHandle) {
    if let Some(tx) = &*app
        .state::<Booker>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
    {
        let _ = tx.send(());
    }
}

/// After launch: start booking reminders, and look again every minute while on screen.
pub fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel::<()>();
    app.manage(Booker(Mutex::new(Some(tx))));
    let handle = app.handle().clone();
    std::thread::spawn(move || {
        // Asking shows iOS's prompt once and waits for the answer; iOS remembers it.
        let allowed = match handle.notification().request_permission() {
            Ok(state) => state == PermissionState::Granted,
            Err(e) => {
                eprintln!("friends: couldn't ask to send notifications: {e}");
                false
            }
        };
        let mut last: Option<Vec<Booked>> = None;
        loop {
            book(&handle, allowed, &mut last);
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

/// Back on screen: the day may have moved on while the app was stopped.
pub fn on_event(app: &AppHandle, event: &tauri::RunEvent) {
    if let tauri::RunEvent::Resumed = event {
        views_changed(app);
    }
}

/// When a reminder for `date` comes: 9:00 that morning, local time (or the first moment after, on a
/// day the clocks change then).
fn morning_of(date: NaiveDate) -> Option<i64> {
    let at = date.and_time(NaiveTime::from_hms_opt(MORNING, 0, 0)?);
    let local = Local.from_local_datetime(&at);
    let local = local.earliest().or_else(|| {
        Local
            .from_local_datetime(&(at + chrono::Duration::hours(1)))
            .earliest()
    })?;
    Some(local.timestamp())
}

/// The reminders to book now: the plan's, at their morning, leaving out the ones whose morning has
/// passed (today's, once 9:00 has gone by: the person shows in Today instead).
fn to_book(plan: &[Planned], now: i64) -> Vec<Booked> {
    plan.iter()
        .filter_map(|p| {
            let at = morning_of(p.date)?;
            // Person ids start at 1 and stay small; the modulo only keeps the id a valid i32.
            let id = i32::try_from(p.person % i32::MAX as u64).ok()?;
            (at > now).then(|| (id, at, p.body.clone()))
        })
        .collect()
}

/// Works out the reminders, and replaces the booked ones when they changed.
fn book(app: &AppHandle, allowed: bool, last: &mut Option<Vec<Booked>>) {
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
    let wanted = if allowed {
        to_book(&plan, chrono::Utc::now().timestamp())
    } else {
        Vec::new()
    };
    if last.as_ref() == Some(&wanted) {
        return;
    }
    let notification = app.notification();
    if let Err(e) = notification.cancel_all() {
        eprintln!("friends: couldn't clear the booked reminders: {e}");
    }
    for (id, at, body) in &wanted {
        let Ok(date) = time::OffsetDateTime::from_unix_timestamp(*at) else {
            continue;
        };
        if let Err(e) = notification
            .builder()
            .id(*id)
            .title("Friends")
            .body(body)
            .schedule(Schedule::At {
                date,
                repeating: false,
                allow_while_idle: false,
            })
            .show()
        {
            eprintln!("friends: couldn't book a reminder: {e}");
        }
    }
    #[cfg(debug_assertions)]
    eprintln!("friends: booked {} reminders", wanted.len());
    *last = Some(wanted);
}
