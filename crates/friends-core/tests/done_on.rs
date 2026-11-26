//! Marking someone done on the day you were really in touch, which may not be today.

use chrono::{Days, NaiveDate, Weekday};
use friends_core::api::App;
use friends_core::model::Data;
use friends_core::schedule::{DaySet, DayToggle};
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

/// Ada, monthly, last in touch Tue Sep 1 2026, under `toggle` (and `custom` days) with `blocked`
/// ranges: one list caught up only on `today`, after the app didn't run for weeks, and one caught
/// up every day from Oct 1, as on a Mac that's always on. Both should plan the same.
fn away_and_every_day(
    toggle: DayToggle,
    custom: Option<DaySet>,
    blocked: &[(NaiveDate, NaiveDate)],
    today: NaiveDate,
) -> (Data, Data, u64) {
    let start = date(2026, 9, 1);
    let make = || {
        let mut data = Data::default();
        data.set_day_setting(toggle, custom, start).unwrap();
        for (from, to) in blocked {
            data.add_blocked(*from, *to, "Away", start).unwrap();
        }
        let id = data.add_person("Ada", 30, start, start).unwrap();
        (data, id)
    };
    let (mut away, id) = make();
    away.catch_up(today);
    let (mut daily, _) = make();
    let mut day = date(2026, 10, 1);
    while day <= today {
        daily.catch_up(day);
        day = day.checked_add_days(Days::new(1)).unwrap();
    }
    (away, daily, id)
}

fn plan(data: &Data, id: u64) -> (NaiveDate, bool, Option<NaiveDate>) {
    let p = data.person(id).unwrap();
    (p.next_target, p.overdue, p.missed_target)
}

#[test]
fn after_weeks_away_the_day_suggested_is_the_latest_missed_weekday() {
    // Sep 1 + 30 = Thu Oct 1; Thu Nov 26 is the first day the app runs again.
    let thursday = date(2026, 11, 26);
    let (mut away, daily, id) = away_and_every_day(DayToggle::WeekdaysOnly, None, &[], thursday);
    assert_eq!(
        plan(&away, id),
        (date(2026, 11, 23), true, Some(date(2026, 11, 16)))
    );
    assert_eq!(plan(&away, id), plan(&daily, id));
    // Done on the day suggested clears the reminder: Nov 16 + 30 = Wed Dec 16.
    away.mark_done(id, Some(date(2026, 11, 16)), thursday)
        .unwrap();
    assert_eq!(plan(&away, id), (date(2026, 12, 16), false, None));
}

#[test]
fn after_weeks_away_the_day_suggested_is_the_latest_missed_weekend() {
    let sunday = date(2026, 11, 29);
    let (mut away, daily, id) = away_and_every_day(DayToggle::WeekendsOnly, None, &[], sunday);
    assert_eq!(
        plan(&away, id),
        (date(2026, 11, 28), true, Some(date(2026, 11, 21)))
    );
    assert_eq!(plan(&away, id), plan(&daily, id));
    away.mark_done(id, Some(date(2026, 11, 21)), sunday)
        .unwrap();
    assert!(!away.person(id).unwrap().overdue);
}

#[test]
fn after_weeks_away_the_day_suggested_is_the_latest_missed_picked_day() {
    // Thursdays only, and today is a Thursday: last Thursday is the day missed.
    let thursday = date(2026, 11, 26);
    let thursdays = Some(DaySet::of(&[Weekday::Thu]));
    let (away, daily, id) = away_and_every_day(DayToggle::Custom, thursdays, &[], thursday);
    assert_eq!(plan(&away, id), (thursday, true, Some(date(2026, 11, 19))));
    assert_eq!(plan(&away, id), plan(&daily, id));
}

#[test]
fn after_weeks_away_a_blocked_range_still_leaves_the_latest_missed_day() {
    let thursday = date(2026, 11, 26);
    // A week off just before today, and a long range that ends close to it.
    for blocked in [
        [(date(2026, 11, 16), date(2026, 11, 20))],
        [(date(2026, 10, 20), date(2026, 11, 17))],
    ] {
        let (away, daily, id) =
            away_and_every_day(DayToggle::WeekdaysOnly, None, &blocked, thursday);
        assert_eq!(plan(&away, id), plan(&daily, id), "blocked {blocked:?}");
        assert!(away.person(id).unwrap().overdue);
    }
}

#[test]
fn after_weeks_away_blocked_days_a_few_weeks_apart_still_leave_the_latest_missed_day() {
    // Every day allowed, single blocked days three weeks apart, and the app runs again on Tue Jun 16.
    let mut data = Data::default();
    let start = date(2026, 1, 1);
    let all = Some(DaySet::ALL);
    data.set_day_setting(DayToggle::Custom, all, start).unwrap();
    for (m, d) in [(2, 10), (3, 3), (3, 24), (4, 14), (5, 5), (5, 26)] {
        data.add_blocked(date(2026, m, d), date(2026, m, d), "Away", start)
            .unwrap();
    }
    let id = data.add_person("Ada", 30, start, start).unwrap();
    data.catch_up(date(2026, 6, 16));
    assert_eq!(
        plan(&data, id),
        (date(2026, 6, 16), true, Some(date(2026, 6, 15)))
    );
}

#[test]
fn after_months_away_a_long_blocked_range_still_leaves_the_latest_missed_day() {
    // Weekdays only, Jul 1 + 30 = Fri Jul 31, then blocked from Sep 1 to Nov 20 (one range, or two
    // back to back). Every day from Aug 1 the app would have pushed Jul 31 to Mon Aug 3, ... Mon
    // Aug 31, then past the range to Mon Nov 23: Aug 31 is the day missed.
    let start = date(2026, 7, 1);
    let thursday = date(2026, 11, 26);
    for blocked in [
        vec![(date(2026, 9, 1), date(2026, 11, 20))],
        vec![
            (date(2026, 9, 1), date(2026, 10, 20)),
            (date(2026, 10, 21), date(2026, 11, 20)),
        ],
    ] {
        let mut data = Data::default();
        data.set_day_setting(DayToggle::WeekdaysOnly, None, start)
            .unwrap();
        for (from, to) in &blocked {
            data.add_blocked(*from, *to, "Away", start).unwrap();
        }
        let id = data.add_person("Ada", 30, start, start).unwrap();
        data.catch_up(thursday);
        assert_eq!(
            plan(&data, id),
            (date(2026, 11, 23), true, Some(date(2026, 8, 31))),
            "blocked {blocked:?}"
        );
    }
}

#[test]
fn after_weeks_away_the_order_of_blocked_ranges_does_not_change_the_plan() {
    // The same ranges, listed in both orders (another device may hold them the other way round).
    let start = date(2026, 7, 1);
    let thursday = date(2026, 11, 26);
    let ranges = [
        (date(2026, 9, 1), date(2026, 10, 20)),
        (date(2026, 10, 21), date(2026, 11, 20)),
        (date(2026, 11, 23), date(2026, 11, 23)),
    ];
    let plan_with = |ranges: &[(NaiveDate, NaiveDate)]| {
        let mut data = Data::default();
        data.set_day_setting(DayToggle::WeekdaysOnly, None, start)
            .unwrap();
        for (from, to) in ranges {
            data.add_blocked(*from, *to, "Away", start).unwrap();
        }
        let id = data.add_person("Ada", 30, start, start).unwrap();
        data.catch_up(thursday);
        plan(&data, id)
    };
    let reversed: Vec<_> = ranges.iter().rev().copied().collect();
    // Mon Aug 31 was the day missed; Mon Nov 23 is blocked, so Tue Nov 24 is the target.
    let expected = (date(2026, 11, 24), true, Some(date(2026, 8, 31)));
    assert_eq!(plan_with(&ranges), expected);
    assert_eq!(plan_with(&reversed), expected);
}
