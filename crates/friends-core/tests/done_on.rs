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
        "Pick Wed, Jul 1, 2026 or later: the last time you were in touch."
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
    // The weekend goes by: that Saturday is the day missed.
    assert!(data.catch_up(date(2026, 8, 24)));
    assert_eq!(
        data.person(id).unwrap().missed_target,
        Some(date(2026, 8, 22))
    );
    // The next one too: now the latest Saturday, the day you'd most likely have met.
    assert!(data.catch_up(date(2026, 8, 31)));
    let p = data.person(id).unwrap();
    assert!(p.overdue);
    assert_eq!(p.next_target, date(2026, 9, 5));
    assert_eq!(p.missed_target, Some(date(2026, 8, 29)));
    assert_eq!(p.suggested_day(date(2026, 8, 31)), Some(date(2026, 8, 29)));
    // Done on that Saturday: counted from it, and nothing is missed any more.
    data.mark_done(id, Some(date(2026, 8, 29)), date(2026, 8, 31))
        .unwrap();
    let p = data.person(id).unwrap();
    assert_eq!(p.last_connected, date(2026, 8, 29));
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
    // Sunday: Saturday's target is still on (the weekend isn't over), and was yesterday.
    assert_eq!(json["people"][0]["suggestedEarlier"], "2026-08-22");
    assert_eq!(json["people"][0]["suggestedToday"], false);
    let id = view.people[0].person.id;
    let view = app
        .call("mark_done", json!({"id": id, "on": "2026-08-22"}), sunday)
        .unwrap();
    assert_eq!(view.people[0].person.last_connected, date(2026, 8, 22));
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(
        json["people"][0]["suggestedEarlier"],
        serde_json::Value::Null
    );
}

#[test]
fn done_on_a_day_more_than_a_round_ago_is_still_behind_on_every_device() {
    // Weekends only. Jul 22 + 30 -> Sat Aug 22, missed; today is Mon Sep 28.
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2026, 7, 22), today())
        .unwrap();
    let monday = date(2026, 9, 28);
    data.catch_up(monday);
    // In touch on Aug 22: monthly, so the next was due by Sat Sep 26, which went by too. Ada is
    // behind again, truthfully, aiming for the next weekend.
    data.mark_done(id, Some(date(2026, 8, 22)), monday).unwrap();
    let mac = data.person(id).unwrap().clone();
    assert_eq!(mac.last_connected, date(2026, 8, 22));
    assert!(mac.overdue);
    assert_eq!(mac.missed_target, Some(date(2026, 9, 26)));
    assert_eq!(mac.next_target, date(2026, 10, 3));
    // Another device that only learns the new last time in touch makes the same plan.
    let mut phone = Data::default();
    let other = phone
        .add_person("Ada", 30, date(2026, 8, 22), monday)
        .unwrap();
    let p = phone.person(other).unwrap();
    assert_eq!(
        (p.next_target, p.overdue, p.missed_target),
        (mac.next_target, mac.overdue, mac.missed_target)
    );
}

#[test]
fn done_today_works_when_the_last_time_is_tomorrow_after_a_time_zone_change() {
    let mut data = Data::default();
    let id = data.add_person("Ada", 30, today(), today()).unwrap();
    // Recorded on a device a day ahead.
    let yesterday = date(2026, 8, 18);
    data.mark_done(id, None, yesterday).unwrap();
    assert_eq!(data.person(id).unwrap().last_connected, yesterday);
}

#[test]
fn a_new_plan_works_out_the_day_missed_the_same_way_everywhere() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2026, 7, 22), today())
        .unwrap();
    let monday = date(2026, 8, 31);
    data.catch_up(monday);
    assert_eq!(
        data.person(id).unwrap().missed_target,
        Some(date(2026, 8, 29))
    );
    // A blocked range changes the plan; Ada is still missed, and still missed Aug 29.
    data.add_blocked(date(2026, 9, 5), date(2026, 9, 6), "Away", monday)
        .unwrap();
    let p = data.person(id).unwrap();
    assert!(p.overdue);
    assert_eq!(p.missed_target, Some(date(2026, 8, 29)));
}

#[test]
fn the_view_says_when_today_is_the_day_suggested() {
    let dir = std::env::temp_dir().join(format!("friends-done-today-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut app = App::open(Store::new(dir.join("friends.json"))).unwrap();
    let saturday = date(2026, 8, 22);
    let view = app
        .call(
            "add_person",
            json!({"name": "Ada", "frequencyDays": 30, "lastConnected": "2026-07-22"}),
            saturday,
        )
        .unwrap();
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(
        json["people"][0]["suggestedEarlier"],
        serde_json::Value::Null
    );
    assert_eq!(json["people"][0]["suggestedToday"], true);
}
