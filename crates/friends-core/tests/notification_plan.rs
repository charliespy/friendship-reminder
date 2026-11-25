//! Booking reminders ahead, for a device that can't check every minute (the iPhone app is stopped
//! when it isn't on screen): which reminders to hand to the system, for which day, with what text.

use std::fs;
use std::path::PathBuf;

use chrono::NaiveDate;
use friends_core::api::{reminder_text, App, MAX_PLANNED};
use friends_core::store::Store;
use serde_json::json;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

/// A Wednesday.
fn today() -> NaiveDate {
    date(2026, 8, 19)
}

fn app(name: &str) -> App {
    let dir = std::env::temp_dir().join(format!("friends-plan-test-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path: PathBuf = dir.join("friends.json");
    let mut app = App::open(Store::new(path)).unwrap();
    app.call("set_day_toggle", json!({"dayToggle": "allDays"}), today())
        .unwrap();
    app
}

fn add(app: &mut App, name: &str, freq: u32, last: &str) -> u64 {
    let view = app
        .call(
            "add_person",
            json!({"name": name, "frequencyDays": freq, "lastConnected": last}),
            today(),
        )
        .unwrap();
    view.people
        .iter()
        .find(|p| p.person.name == name)
        .unwrap()
        .person
        .id
}

#[test]
fn upcoming_reminders_are_booked_soonest_first() {
    let mut app = app("upcoming");
    // Monthly: aim for next Wednesday, reminded that day.
    let bea = add(&mut app, "Bea", 30, "2026-07-27");
    // Quarterly: aim for next Wednesday too.
    let amy = add(&mut app, "amy", 90, "2026-05-28");
    // Monthly, last in touch today: much later.
    let cy = add(&mut app, "Cy", 30, "2026-08-19");

    let plan = app.plan_notifications(today()).unwrap();
    let got: Vec<(u64, NaiveDate)> = plan.iter().map(|p| (p.person, p.date)).collect();
    // Same day: by name, ignoring case.
    assert_eq!(
        got,
        vec![
            (amy, date(2026, 8, 26)),
            (bea, date(2026, 8, 26)),
            (cy, date(2026, 9, 18)),
        ]
    );
    assert_eq!(plan[1].body, "It's time to reach out to Bea.");
}

#[test]
fn todays_reminder_is_booked_and_the_shell_picks_the_hour() {
    let mut app = app("today");
    let ada = add(&mut app, "Ada", 30, "2026-07-20");
    let plan = app.plan_notifications(today()).unwrap();
    assert_eq!(plan.len(), 1);
    assert_eq!((plan[0].person, plan[0].date), (ada, today()));
    // Booking doesn't use the reminder up: the same plan comes back next time.
    assert_eq!(app.plan_notifications(today()).unwrap(), plan);
}

#[test]
fn reminders_from_earlier_days_are_not_booked_and_count_as_sent() {
    let mut app = app("past");
    // Aiming for Saturday, so reminded on Monday: booked back then, or the person just shows in
    // the list.
    add(&mut app, "Ada", 30, "2026-07-23");
    assert!(app.plan_notifications(today()).unwrap().is_empty());
    // Handled, so the regular check doesn't send it again either.
    assert!(app.tick(today()).unwrap().is_empty());
    assert!(app.view(today()).people[0].due);
}

#[test]
fn a_reminder_already_sent_today_is_not_booked_again() {
    let mut app = app("sent");
    add(&mut app, "Ada", 30, "2026-07-20");
    assert_eq!(app.tick(today()).unwrap().len(), 1);
    assert!(app.plan_notifications(today()).unwrap().is_empty());
}

#[test]
fn missed_targets_are_caught_up_and_booked_with_the_missed_text() {
    let mut app = app("missed");
    let ada = add(&mut app, "Ada", 30, "2026-07-20");
    // Two days after the target went by unmarked.
    let later = date(2026, 8, 21);
    let plan = app.plan_notifications(later).unwrap();
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].person, ada);
    assert!(plan[0].date >= later);
    assert_eq!(
        plan[0].body,
        "You meant to reach out to Ada — there's still time this round."
    );
    let person = &app.view(later).people[0].person;
    assert!(person.overdue);
    assert_eq!(plan[0].date, person.notification_date);
}

#[test]
fn nothing_is_booked_while_notifications_are_off() {
    let mut app = app("off");
    add(&mut app, "Ada", 30, "2026-08-19");
    app.call("set_notifications", json!({"on": false}), today())
        .unwrap();
    assert!(app.plan_notifications(today()).unwrap().is_empty());
}

#[test]
fn the_plan_stays_within_what_the_system_keeps() {
    let mut app = app("cap");
    for i in 0..(MAX_PLANNED + 10) {
        add(&mut app, &format!("Friend {i:03}"), 30, "2026-08-19");
    }
    let plan = app.plan_notifications(today()).unwrap();
    assert_eq!(plan.len(), MAX_PLANNED);
    assert_eq!(plan[0].body, "It's time to reach out to Friend 000.");
}

#[test]
fn the_text_says_when_a_target_was_missed() {
    let mut app = app("text");
    add(&mut app, "Ada", 30, "2026-08-19");
    let mut person = app.view(today()).people[0].person.clone();
    assert_eq!(reminder_text(&person), "It's time to reach out to Ada.");
    person.overdue = true;
    assert_eq!(
        reminder_text(&person),
        "You meant to reach out to Ada — there's still time this round."
    );
}
