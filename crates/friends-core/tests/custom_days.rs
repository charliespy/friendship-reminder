//! "Custom day": aiming only for the days of the week you picked.

use chrono::{NaiveDate, Weekday};
use friends_core::api::App;
use friends_core::model::Data;
use friends_core::schedule::{
    compute_schedule, last_day_of_run, push_missed_schedule, BlockedRange, DayRule, DaySet,
    DayToggle,
};
use friends_core::store::Store;
use serde_json::json;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

// Wednesday Aug 19, 2026.
fn today() -> NaiveDate {
    date(2026, 8, 19)
}

fn tue_thu() -> DayRule {
    DayRule::Custom(DaySet::of(&[Weekday::Tue, Weekday::Thu]))
}

#[test]
fn a_target_moves_to_the_next_picked_day() {
    // Jul 21 + 30 = Thu Aug 20: picked, so it stays.
    let r = compute_schedule(date(2026, 7, 21), 30, tue_thu(), &[]);
    assert_eq!(r.next_target_date, date(2026, 8, 20));
    assert_eq!(r.notification_date, date(2026, 8, 20));
    // Jul 22 + 30 = Fri Aug 21: next picked day is Tue Aug 25.
    let r = compute_schedule(date(2026, 7, 22), 30, tue_thu(), &[]);
    assert_eq!(r.next_target_date, date(2026, 8, 25));
}

#[test]
fn a_picked_weekend_day_still_gets_the_monday_heads_up() {
    let sundays = DayRule::Custom(DaySet::of(&[Weekday::Sun]));
    let r = compute_schedule(date(2026, 7, 22), 30, sundays, &[]); // Fri Aug 21 -> Sun Aug 23
    assert_eq!(r.next_target_date, date(2026, 8, 23));
    assert_eq!(r.notification_date, date(2026, 8, 17));
}

#[test]
fn each_picked_day_is_its_own_chance() {
    // A missed Tuesday counts as missed the next day, and moves to Thursday.
    assert_eq!(
        last_day_of_run(date(2026, 8, 18), tue_thu()),
        date(2026, 8, 18)
    );
    let r = push_missed_schedule(date(2026, 8, 18), tue_thu(), &[]);
    assert_eq!(r.next_target_date, date(2026, 8, 20));
    // Picked Saturday and Sunday: a missed Saturday moves to Sunday, not the next weekend.
    let weekend = DayRule::Custom(DaySet::of(&[Weekday::Sat, Weekday::Sun]));
    let r = push_missed_schedule(date(2026, 8, 22), weekend, &[]);
    assert_eq!(r.next_target_date, date(2026, 8, 23));
}

#[test]
fn blocked_dates_skip_to_the_next_picked_day_after_them() {
    let blocked = [BlockedRange {
        start: date(2026, 8, 19),
        end: date(2026, 8, 26),
    }];
    let r = compute_schedule(date(2026, 7, 21), 30, tue_thu(), &blocked); // Thu Aug 20, blocked
    assert_eq!(r.next_target_date, date(2026, 8, 27));
}

#[test]
fn all_seven_days_work_like_any_day_did() {
    let r = push_missed_schedule(date(2026, 8, 19), DayRule::ANY, &[]);
    assert_eq!(r.next_target_date, date(2026, 8, 20));
}

#[test]
fn picking_days_reschedules_everyone_only_while_custom_is_on() {
    let mut data = Data::default(); // weekends only
    let id = data
        .add_person("Ada", 30, date(2026, 7, 22), today())
        .unwrap(); // Fri Aug 21 -> Sat Aug 22
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 8, 22));
    let tue_thu = DaySet::of(&[Weekday::Tue, Weekday::Thu]);
    data.set_custom_days(tue_thu, today()).unwrap();
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 8, 22));
    data.set_day_toggle(DayToggle::Custom, today());
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 8, 25));
    data.set_custom_days(DaySet::of(&[Weekday::Sun]), today())
        .unwrap();
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 8, 23));
}

#[test]
fn at_least_one_day_has_to_be_picked() {
    let mut data = Data::default();
    let err = data.set_custom_days(DaySet::NONE, today()).unwrap_err();
    assert_eq!(err.to_string(), "Pick at least one day.");
    assert_eq!(data.settings.custom_days, DaySet::ALL);
}

fn app(name: &str) -> App {
    let dir =
        std::env::temp_dir().join(format!("friends-custom-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    App::open(Store::new(dir.join("friends.json"))).unwrap()
}

#[test]
fn the_ui_picks_days_by_short_name_and_sees_them_in_the_view() {
    let mut app = app("api");
    app.call("set_day_toggle", json!({"dayToggle": "custom"}), today())
        .unwrap();
    let view = app
        .call("set_custom_days", json!({"days": ["thu", "tue"]}), today())
        .unwrap();
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["settings"]["dayToggle"], "custom");
    assert_eq!(json["settings"]["customDays"], json!(["tue", "thu"]));
    let err = app
        .call("set_custom_days", json!({"days": []}), today())
        .unwrap_err();
    assert_eq!(err.message, "Pick at least one day.");
    assert!(app
        .call("set_custom_days", json!({"days": ["someday"]}), today())
        .is_err());
}

#[test]
fn a_file_from_before_custom_days_reads_any_day_as_all_seven() {
    let data: Data =
        serde_json::from_str(r#"{"version":2,"settings":{"dayToggle":"allDays"}}"#).unwrap();
    assert_eq!(data.settings.day_toggle, DayToggle::Custom);
    assert_eq!(data.settings.custom_days, DaySet::ALL);
    assert_eq!(data.settings.rule(), DayRule::ANY);
}
