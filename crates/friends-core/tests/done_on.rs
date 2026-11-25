//! Marking someone done on the day you were really in touch, which may not be today.

use chrono::NaiveDate;
use friends_core::api::App;
use friends_core::model::Data;
use friends_core::store::Store;
use serde_json::json;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

// Wednesday Aug 19, 2026.
fn today() -> NaiveDate {
    date(2026, 8, 19)
}

#[test]
fn done_on_an_earlier_day_counts_from_that_day() {
    let mut data = Data::default(); // weekends only
    let id = data
        .add_person("Ada", 30, date(2026, 7, 1), today())
        .unwrap();
    data.mark_done(id, Some(date(2026, 8, 16)), today())
        .unwrap();
    let p = data.person(id).unwrap();
    assert_eq!(p.last_connected, date(2026, 8, 16));
    // Aug 16 + 30 = Tue Sep 15, so the weekend after: Sat Sep 19.
    assert_eq!(p.next_target, date(2026, 9, 19));
    assert!(!p.overdue);
    assert_eq!(p.missed_target, None);
}

#[test]
fn done_without_a_day_is_today() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2026, 7, 1), today())
        .unwrap();
    data.mark_done(id, None, today()).unwrap();
    assert_eq!(data.person(id).unwrap().last_connected, today());
}

#[test]
fn done_can_be_neither_in_the_future_nor_before_the_last_time() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2026, 7, 1), today())
        .unwrap();
    let err = data
        .mark_done(id, Some(date(2026, 8, 20)), today())
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "The last time you were in touch can't be in the future."
    );
    let err = data
        .mark_done(id, Some(date(2026, 6, 30)), today())
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "You were last in touch on Wed, Jul 1, 2026. Pick that day or later."
    );
    assert_eq!(data.person(id).unwrap().last_connected, date(2026, 7, 1));
}

#[test]
fn a_missed_target_is_remembered_as_the_day_suggested() {
    let mut data = Data::default(); // weekends only
                                    // Jul 22 + 30 = Fri Aug 21 -> Sat Aug 22.
    let id = data
        .add_person("Ada", 30, date(2026, 7, 22), today())
        .unwrap();
    assert_eq!(data.person(id).unwrap().suggested_day(today()), None);
    // The Saturday itself: suggested.
    assert_eq!(
        data.person(id).unwrap().suggested_day(date(2026, 8, 22)),
        Some(date(2026, 8, 22))
    );
    // The weekend goes by, and the next one too: still that first Saturday.
    assert!(data.catch_up(date(2026, 8, 24)));
    assert!(data.catch_up(date(2026, 8, 31)));
    let p = data.person(id).unwrap();
    assert!(p.overdue);
    assert_eq!(p.next_target, date(2026, 9, 5));
    assert_eq!(p.missed_target, Some(date(2026, 8, 22)));
    assert_eq!(p.suggested_day(date(2026, 8, 31)), Some(date(2026, 8, 22)));
    // Done on that Saturday: counted from it, and nothing is missed any more.
    data.mark_done(id, Some(date(2026, 8, 22)), date(2026, 8, 31))
        .unwrap();
    let p = data.person(id).unwrap();
    assert_eq!(p.last_connected, date(2026, 8, 22));
    assert!(!p.overdue);
    assert_eq!(p.missed_target, None);
}

#[test]
fn the_view_gives_the_day_suggested_and_done_takes_a_day() {
    let dir = std::env::temp_dir().join(format!("friends-done-on-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut app = App::open(Store::new(dir.join("friends.json"))).unwrap();
    let sunday = date(2026, 8, 23);
    let view = app
        .call(
            "add_person",
            json!({"name": "Ada", "frequencyDays": 30, "lastConnected": "2026-07-22"}),
            sunday,
        )
        .unwrap();
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["people"][0]["suggestedDay"], "2026-08-22");
    let id = view.people[0].person.id;
    let view = app
        .call("mark_done", json!({"id": id, "on": "2026-08-22"}), sunday)
        .unwrap();
    assert_eq!(view.people[0].person.last_connected, date(2026, 8, 22));
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["people"][0]["suggestedDay"], serde_json::Value::Null);
}
