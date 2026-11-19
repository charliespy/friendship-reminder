//! Port of the Swift app's `SchedulingEngineTests.swift`, one test per Swift test, same dates and same
//! expectations. The Swift tests ran in UTC on whole days; the Rust engine works on calendar dates
//! (`NaiveDate`), so there is no time zone to set.

use chrono::NaiveDate;
use friends_core::schedule::{
    avoid_blocked_ranges, compute_schedule, matches, monday_of_week, push_missed_schedule,
    snap_to_toggle, BlockedRange, DayToggle,
};

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn range(start: NaiveDate, end: NaiveDate) -> BlockedRange {
    BlockedRange { start, end }
}

#[test]
fn matches_weekends_only_true_for_saturday() {
    assert!(matches(date(2026, 8, 22), DayToggle::WeekendsOnly));
}

#[test]
fn matches_weekends_only_false_for_wednesday() {
    assert!(!matches(date(2026, 8, 19), DayToggle::WeekendsOnly));
}

#[test]
fn matches_all_days_always_true() {
    assert!(matches(date(2026, 8, 19), DayToggle::AllDays));
}

#[test]
fn matches_weekdays_only_false_for_sunday() {
    assert!(!matches(date(2026, 8, 23), DayToggle::WeekdaysOnly));
}

#[test]
fn snap_to_toggle_weekends_only_wednesday_snaps_to_saturday() {
    assert_eq!(snap_to_toggle(date(2026, 8, 19), DayToggle::WeekendsOnly), date(2026, 8, 22));
}

#[test]
fn snap_to_toggle_weekends_only_saturday_stays_saturday() {
    assert_eq!(snap_to_toggle(date(2026, 8, 22), DayToggle::WeekendsOnly), date(2026, 8, 22));
}

#[test]
fn snap_to_toggle_weekdays_only_saturday_snaps_to_monday() {
    assert_eq!(snap_to_toggle(date(2026, 8, 22), DayToggle::WeekdaysOnly), date(2026, 8, 24));
}

#[test]
fn avoid_blocked_ranges_date_inside_range_moves_to_next_valid_day_after_range() {
    let blocked = [range(date(2026, 8, 20), date(2026, 8, 27))];
    // The next Saturday or Sunday strictly after Aug 27 is Aug 29.
    assert_eq!(
        avoid_blocked_ranges(date(2026, 8, 22), &blocked, DayToggle::WeekendsOnly),
        date(2026, 8, 29)
    );
}

#[test]
fn avoid_blocked_ranges_date_outside_any_range_unchanged() {
    let blocked = [range(date(2026, 9, 1), date(2026, 9, 5))];
    assert_eq!(
        avoid_blocked_ranges(date(2026, 8, 22), &blocked, DayToggle::WeekendsOnly),
        date(2026, 8, 22)
    );
}

#[test]
fn avoid_blocked_ranges_multiple_overlapping_ranges_skips_all_of_them() {
    // After the first range ends on Aug 27, the next weekend day (Aug 29) is inside the second range
    // (through Aug 30), so the search goes on past it too: the next Saturday or Sunday is Sep 5.
    let blocked = [
        range(date(2026, 8, 20), date(2026, 8, 27)),
        range(date(2026, 8, 28), date(2026, 8, 30)),
    ];
    assert_eq!(
        avoid_blocked_ranges(date(2026, 8, 22), &blocked, DayToggle::WeekendsOnly),
        date(2026, 9, 5)
    );
}

#[test]
fn monday_of_week_for_saturday_returns_monday_of_same_week() {
    assert_eq!(monday_of_week(date(2026, 8, 22)), date(2026, 8, 17));
}

#[test]
fn monday_of_week_for_sunday_returns_monday_of_same_week() {
    // ISO weeks: Sunday Aug 23 belongs to the week that started on Monday Aug 17.
    assert_eq!(monday_of_week(date(2026, 8, 23)), date(2026, 8, 17));
}

#[test]
fn compute_schedule_all_days_toggle_weekday_target_notification_equals_target() {
    // Saturday Aug 1 + 30 days = Monday Aug 31: a weekday target, so no shift.
    let result = compute_schedule(date(2026, 8, 1), 30, DayToggle::AllDays, &[]);
    assert_eq!(result.next_target_date, date(2026, 8, 31));
    assert_eq!(result.notification_date, date(2026, 8, 31));
}

#[test]
fn compute_schedule_weekends_only_toggle_weekend_target_notification_is_monday() {
    // Thursday Jul 23 + 30 days = Saturday Aug 22, already a weekend day.
    let result = compute_schedule(date(2026, 7, 23), 30, DayToggle::WeekendsOnly, &[]);
    assert_eq!(result.next_target_date, date(2026, 8, 22));
    assert_eq!(result.notification_date, date(2026, 8, 17)); // Monday of that week
}

#[test]
fn compute_schedule_blocked_range_shifts_target() {
    // The raw target, Saturday Aug 22, is inside the blocked range.
    let blocked = [range(date(2026, 8, 20), date(2026, 8, 27))];
    let result = compute_schedule(date(2026, 7, 23), 30, DayToggle::WeekendsOnly, &blocked);
    assert_eq!(result.next_target_date, date(2026, 8, 29)); // the next Saturday after the range
    assert_eq!(result.notification_date, date(2026, 8, 24)); // Monday of that week
}

#[test]
fn push_missed_schedule_weekends_only_moves_to_next_weekend() {
    // Saturday Aug 22 is in the past and was not done.
    let result = push_missed_schedule(date(2026, 8, 22), DayToggle::WeekendsOnly, &[]);
    assert_eq!(result.next_target_date, date(2026, 8, 29)); // the next Saturday
    assert_eq!(result.notification_date, date(2026, 8, 24)); // Monday of that week
}

#[test]
fn push_missed_schedule_respects_blocked_ranges() {
    // The next Saturday after Aug 22 is Aug 29, inside the blocked range, so it goes on to the next
    // Saturday or Sunday after Sep 4: Sep 5.
    let blocked = [range(date(2026, 8, 28), date(2026, 9, 4))];
    let result = push_missed_schedule(date(2026, 8, 22), DayToggle::WeekendsOnly, &blocked);
    assert_eq!(result.next_target_date, date(2026, 9, 5));
}

#[test]
fn push_missed_schedule_all_days_moves_to_next_day() {
    let result = push_missed_schedule(date(2026, 8, 22), DayToggle::AllDays, &[]);
    assert_eq!(result.next_target_date, date(2026, 8, 23));
}
