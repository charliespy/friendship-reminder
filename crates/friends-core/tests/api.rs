//! The commands the UI calls, through the same entry point the app uses.

use std::fs;
use std::path::PathBuf;

use chrono::NaiveDate;
use friends_core::api::{App, ErrorKind};
use friends_core::store::Store;
use serde_json::json;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn today() -> NaiveDate {
    date(2026, 8, 19)
}

fn temp_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("friends-api-test-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir.join("friends.json")
}

fn app(name: &str) -> (App, PathBuf) {
    let path = temp_file(name);
    (App::open(Store::new(&path)).unwrap(), path)
}

#[test]
fn adding_a_person_saves_and_answers_with_the_view() {
    let (mut app, path) = app("add");
    let view = app
        .call(
            "add_person",
            json!({"name": "Ada", "frequencyDays": 30, "lastConnected": "2026-07-23"}),
            today(),
        )
        .unwrap();
    assert_eq!(view.people.len(), 1);
    assert!(view.people[0].due);
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["people"][0]["name"], "Ada");
    assert_eq!(json["people"][0]["nextTarget"], "2026-08-22");
    assert_eq!(json["people"][0]["notificationDate"], "2026-08-17");
    assert_eq!(json["settings"]["dayToggle"], "weekendsOnly");
    // Saved: a fresh app on the same file sees it.
    let again = App::open(Store::new(&path)).unwrap();
    assert_eq!(again.data().people.len(), 1);
}

#[test]
fn people_come_soonest_first_then_by_name() {
    let (mut app, _) = app("order");
    for (name, freq) in [("zed", 30), ("Bea", 365), ("amy", 30)] {
        app.call(
            "add_person",
            json!({"name": name, "frequencyDays": freq, "lastConnected": "2026-08-19"}),
            today(),
        )
        .unwrap();
    }
    let view = app.call("get_view", json!(null), today()).unwrap();
    let names: Vec<_> = view.people.iter().map(|p| p.person.name.as_str()).collect();
    assert_eq!(names, ["amy", "zed", "Bea"]);
}

#[test]
fn invalid_input_is_an_invalid_error_and_changes_nothing() {
    let (mut app, path) = app("invalid");
    let err = app
        .call(
            "add_person",
            json!({"name": " ", "frequencyDays": 30, "lastConnected": "2026-07-23"}),
            today(),
        )
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Invalid);
    assert_eq!(err.message, "Give them a name.");
    assert!(app.data().people.is_empty());
    assert!(!path.exists());
}

#[test]
fn malformed_calls_are_bad_call_errors() {
    let (mut app, _) = app("badcall");
    assert_eq!(
        app.call("rm_rf", json!({}), today()).unwrap_err().kind,
        ErrorKind::BadCall
    );
    assert_eq!(
        app.call("mark_done", json!({"id": "x"}), today())
            .unwrap_err()
            .kind,
        ErrorKind::BadCall
    );
    assert_eq!(
        app.call("mark_done", json!({"id": 1, "extra": 1}), today())
            .unwrap_err()
            .kind,
        ErrorKind::BadCall
    );
}

#[test]
fn every_command_works_end_to_end() {
    let (mut app, _) = app("all");
    app.call(
        "add_person",
        json!({"name": "Ada", "frequencyDays": 30, "lastConnected": "2026-07-23"}),
        today(),
    )
    .unwrap();
    app.call(
        "add_person",
        json!({"name": "Bo", "frequencyDays": 90, "lastConnected": "2026-08-01"}),
        today(),
    )
    .unwrap();
    let id = app.data().people[0].id;
    let view = app.call("mark_done", json!({"id": id}), today()).unwrap();
    assert!(!view.people.iter().any(|p| p.due));
    app.call(
        "update_person",
        json!({"id": id, "name": "Ada L", "frequencyDays": 90, "lastConnected": "2026-08-19"}),
        today(),
    )
    .unwrap();
    let view = app
        .call(
            "add_blocked",
            json!({"start": "2026-11-01", "end": "2026-11-30", "label": "Trip"}),
            today(),
        )
        .unwrap();
    assert_eq!(view.blocked.len(), 1);
    // No one's target is inside the trip any more.
    assert!(view.people.iter().all(|p| {
        p.person.next_target < date(2026, 11, 1) || p.person.next_target > date(2026, 11, 30)
    }));
    let blocked_id = view.blocked[0].id;
    app.call("delete_blocked", json!({"id": blocked_id}), today())
        .unwrap();
    app.call("set_day_toggle", json!({"dayToggle": "allDays"}), today())
        .unwrap();
    let view = app
        .call("set_notifications", json!({"on": false}), today())
        .unwrap();
    assert!(!view.settings.notifications);
    let view = app
        .call("delete_people", json!({"ids": [id]}), today())
        .unwrap();
    assert_eq!(view.people.len(), 1);
}

#[test]
fn a_save_that_fails_is_undone_and_reported() {
    let path = temp_file("readonly");
    let mut app = App::open(Store::new(&path)).unwrap();
    // Make the save fail: the folder can't be written to.
    use std::os::unix::fs::PermissionsExt;
    let dir = path.parent().unwrap().to_path_buf();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    let err = app
        .call(
            "add_person",
            json!({"name": "Ada", "frequencyDays": 30, "lastConnected": "2026-07-23"}),
            today(),
        )
        .unwrap_err();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(err.kind, ErrorKind::Storage);
    assert!(app.data().people.is_empty());
}

#[test]
fn tick_catches_up_and_notifies_each_reminder_once() {
    let (mut app, path) = app("tick");
    app.call(
        "add_person",
        json!({"name": "Ada", "frequencyDays": 30, "lastConnected": "2026-07-23"}),
        today(),
    )
    .unwrap();
    let due = app.tick(today()).unwrap();
    assert_eq!(due.len(), 1);
    assert!(app.tick(today()).unwrap().is_empty());
    // Saved, so a restart doesn't notify again.
    let mut again = App::open(Store::new(&path)).unwrap();
    assert!(again.tick(today()).unwrap().is_empty());
    // The weekend went by: pushed, overdue, notified again.
    let due = again.tick(date(2026, 8, 24)).unwrap();
    assert_eq!(due.len(), 1);
    assert!(due[0].overdue);
}

#[test]
fn tick_sends_nothing_when_notifications_are_off() {
    let (mut app, _) = app("tick-off");
    app.call("set_notifications", json!({"on": false}), today())
        .unwrap();
    app.call(
        "add_person",
        json!({"name": "Ada", "frequencyDays": 30, "lastConnected": "2026-07-23"}),
        today(),
    )
    .unwrap();
    assert!(app.tick(today()).unwrap().is_empty());
    // Turning them back on doesn't replay the old ones.
    app.call("set_notifications", json!({"on": true}), today())
        .unwrap();
    assert!(app.tick(today()).unwrap().is_empty());
}

#[test]
fn an_unreadable_file_is_reported_in_the_view() {
    let path = temp_file("unreadable");
    fs::write(&path, "garbage").unwrap();
    let app = App::open(Store::new(&path)).unwrap();
    let view = app.view(today());
    assert!(view
        .unreadable_file
        .unwrap()
        .ends_with("friends.json.unreadable-1"));
}

#[test]
fn any_command_catches_up_before_the_regular_check_runs() {
    let (mut app, _) = app("call-catches-up");
    app.call(
        "add_person",
        json!({"name": "Ada", "frequencyDays": 30, "lastConnected": "2026-07-23"}),
        today(),
    )
    .unwrap();
    // Monday after the target weekend, before any tick.
    let view = app
        .call("get_view", json!(null), date(2026, 8, 24))
        .unwrap();
    assert_eq!(view.people[0].person.next_target, date(2026, 8, 29));
    assert!(view.people[0].person.overdue);
}

#[test]
fn the_view_still_loads_when_saving_the_catch_up_fails() {
    use std::os::unix::fs::PermissionsExt;
    let (mut app, path) = app("catch-up-save-fails");
    app.call(
        "add_person",
        json!({"name": "Ada", "frequencyDays": 30, "lastConnected": "2026-07-23"}),
        today(),
    )
    .unwrap();
    let dir = path.parent().unwrap().to_path_buf();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    let view = app.call("get_view", json!(null), date(2026, 8, 24));
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    let view = view.unwrap();
    assert_eq!(view.people[0].person.next_target, date(2026, 8, 29));
}
