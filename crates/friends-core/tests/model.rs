//! The data changes the UI makes, and catching up on missed reminders.

use chrono::NaiveDate;
use friends_core::model::Data;
use friends_core::schedule::DayToggle;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

// Wednesday Aug 19, 2026.
fn today() -> NaiveDate {
    date(2026, 8, 19)
}

#[test]
fn a_new_person_is_scheduled_from_the_last_time_in_touch() {
    let mut data = Data::default();
    let id = data
        .add_person("  Ada   Lovelace ", 30, date(2026, 7, 23), today())
        .unwrap();
    let p = data.person(id).unwrap();
    assert_eq!(p.name, "Ada Lovelace");
    assert_eq!(p.next_target, date(2026, 8, 22)); // Saturday
    assert_eq!(p.notification_date, date(2026, 8, 17)); // the Monday before
    assert!(p.is_due(today()));
    assert!(!p.overdue);
}

#[test]
fn adding_someone_long_out_of_touch_catches_them_up_to_today() {
    let mut data = Data::default();
    let id = data
        .add_person("Grace", 30, date(2025, 1, 1), today())
        .unwrap();
    let p = data.person(id).unwrap();
    assert!(p.next_target >= today());
    assert_eq!(p.next_target, date(2026, 8, 22)); // the first weekend from today
    assert!(p.overdue);
}

#[test]
fn bad_input_is_refused_with_a_message() {
    let mut data = Data::default();
    assert!(data.add_person("   ", 30, today(), today()).is_err());
    assert!(data.add_person("Ada", 31, today(), today()).is_err());
    assert!(data
        .add_person("Ada", 30, date(2026, 8, 20), today())
        .is_err());
    assert!(data
        .add_person(&"a".repeat(101), 30, today(), today())
        .is_err());
    assert!(data
        .add_blocked(date(2026, 8, 25), date(2026, 8, 24), "Trip", today())
        .is_err());
    assert!(data.people.is_empty() && data.blocked.is_empty());
}

#[test]
fn marking_done_counts_the_next_target_from_today() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 90, date(2026, 5, 1), today())
        .unwrap();
    data.mark_done(id, today()).unwrap();
    let p = data.person(id).unwrap();
    assert_eq!(p.last_connected, today());
    assert_eq!(p.next_target, date(2026, 11, 21)); // Aug 19 + 90 = Tue Nov 17 -> Saturday Nov 21
    assert!(!p.overdue);
}

#[test]
fn a_missed_target_is_pushed_to_the_next_weekend_after_it_goes_by() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2026, 7, 23), today())
        .unwrap(); // target Sat Aug 22
                   // On the target weekend nothing moves.
    assert!(!data.catch_up(date(2026, 8, 23)));
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 8, 22));
    // The Monday after, it was missed: the next weekend.
    assert!(data.catch_up(date(2026, 8, 24)));
    let p = data.person(id).unwrap();
    assert_eq!(p.next_target, date(2026, 8, 29));
    assert_eq!(p.notification_date, date(2026, 8, 24));
    assert!(p.overdue);
}

#[test]
fn catching_up_after_months_away_lands_on_or_after_today() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2026, 7, 23), today())
        .unwrap();
    data.catch_up(date(2027, 3, 3)); // a Wednesday, months later
    assert_eq!(data.person(id).unwrap().next_target, date(2027, 3, 6));
}

#[test]
fn a_new_blocked_range_reschedules_everyone() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2026, 7, 23), today())
        .unwrap();
    data.add_blocked(date(2026, 8, 20), date(2026, 8, 27), "Trip", today())
        .unwrap();
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 8, 29));
    let blocked_id = data.blocked[0].id;
    assert!(data.delete_blocked(blocked_id, today()));
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 8, 22));
}

#[test]
fn changing_the_day_setting_reschedules_everyone() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2026, 7, 23), today())
        .unwrap();
    data.set_day_toggle(DayToggle::WeekdaysOnly, today());
    let p = data.person(id).unwrap();
    assert_eq!(p.next_target, date(2026, 8, 24)); // Saturday -> Monday
    assert_eq!(p.notification_date, date(2026, 8, 24));
}

#[test]
fn editing_only_the_name_keeps_the_schedule() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2025, 1, 1), today())
        .unwrap();
    let before = data.person(id).unwrap().clone();
    data.update_person(id, "Ada L.", 30, date(2025, 1, 1), today())
        .unwrap();
    let after = data.person(id).unwrap();
    assert_eq!(after.name, "Ada L.");
    assert_eq!(
        (after.next_target, after.overdue),
        (before.next_target, before.overdue)
    );
}

#[test]
fn editing_the_frequency_reschedules() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2026, 7, 23), today())
        .unwrap();
    data.update_person(id, "Ada", 90, date(2026, 7, 23), today())
        .unwrap();
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 10, 24)); // Jul 23 + 90 = Wed Oct 21 -> Sat Oct 24
}

#[test]
fn unknown_ids_are_reported_or_skipped() {
    let mut data = Data::default();
    assert!(data.mark_done(42, today()).is_err());
    assert!(data.update_person(42, "Ada", 30, today(), today()).is_err());
    assert_eq!(data.delete_people(&[42]), 0);
    assert!(!data.delete_blocked(42, today()));
}

#[test]
fn deleting_people_removes_only_those_and_ids_are_never_reused() {
    let mut data = Data::default();
    let a = data.add_person("A", 30, today(), today()).unwrap();
    let b = data.add_person("B", 30, today(), today()).unwrap();
    assert_eq!(data.delete_people(&[a]), 1);
    assert_eq!(data.people.len(), 1);
    assert_eq!(data.people[0].id, b);
    let c = data.add_person("C", 30, today(), today()).unwrap();
    assert!(c > b);
}

#[test]
fn each_reminder_is_notified_once() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2026, 7, 23), today())
        .unwrap(); // notify Mon Aug 17
    let first = data.take_notifications(today());
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].id, id);
    assert!(data.take_notifications(today()).is_empty());
    // Missed and pushed: a new notification date, so a new notification.
    data.catch_up(date(2026, 8, 24));
    assert_eq!(data.take_notifications(date(2026, 8, 24)).len(), 1);
}

#[test]
fn reminders_not_yet_due_are_not_notified() {
    let mut data = Data::default();
    data.add_person("Ada", 365, today(), today()).unwrap();
    assert!(data.take_notifications(today()).is_empty());
}

#[test]
fn data_survives_a_json_round_trip_and_old_files_get_defaults() {
    let mut data = Data::default();
    data.add_person("Ada", 30, date(2026, 7, 23), today())
        .unwrap();
    data.add_blocked(date(2026, 9, 1), date(2026, 9, 3), "Trip", today())
        .unwrap();
    let json = serde_json::to_string(&data).unwrap();
    assert_eq!(serde_json::from_str::<Data>(&json).unwrap(), data);
    let minimal: Data = serde_json::from_str(r#"{"version":1}"#).unwrap();
    assert_eq!(minimal.settings.day_toggle, DayToggle::WeekendsOnly);
    assert!(minimal.settings.notifications);
}

#[test]
fn a_weekday_target_stays_up_through_that_working_week() {
    let mut data = Data::default();
    data.set_day_toggle(DayToggle::WeekdaysOnly, today());
    let id = data
        .add_person("Ada", 30, date(2026, 7, 18), today())
        .unwrap(); // Jul 18 + 30 = Mon Aug 17
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 8, 17));
    assert!(!data.catch_up(date(2026, 8, 21))); // still that week on Friday
    assert!(data.catch_up(date(2026, 8, 22))); // the week is over
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 8, 24));
}

#[test]
fn decades_of_missed_days_catch_up_fully_under_all_days() {
    let mut data = Data::default();
    data.set_day_toggle(DayToggle::AllDays, today());
    let id = data
        .add_person("Ada", 30, date(1950, 1, 1), today())
        .unwrap();
    let p = data.person(id).unwrap();
    assert_eq!(p.next_target, today());
    assert!(p.overdue);
    // A later check the same day changes nothing, so nothing is notified twice.
    assert!(!data.catch_up(today()));
}

#[test]
fn a_long_backlog_under_weekends_only_lands_on_this_weekend() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 365, date(1900, 1, 6), today())
        .unwrap();
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 8, 22));
}

#[test]
fn blocked_dates_more_than_ten_years_out_are_refused() {
    let mut data = Data::default();
    assert!(data
        .add_blocked(date(2036, 8, 1), date(2036, 8, 10), "Fine", today())
        .is_ok());
    assert!(data
        .add_blocked(date(2026, 8, 25), date(2200, 12, 31), "Typo", today())
        .is_err());
}

#[test]
fn targets_pushed_past_a_far_range_still_make_a_loadable_file() {
    let mut data = Data::default();
    data.add_person("Ada", 30, date(2026, 7, 23), today())
        .unwrap();
    data.add_blocked(date(2026, 8, 20), date(2036, 8, 1), "Away", today())
        .unwrap();
    assert!(data.is_sane());
}

#[test]
fn a_missed_weekday_target_stays_up_over_the_weekend_without_an_early_notification() {
    let mut data = Data::default();
    data.set_day_toggle(DayToggle::WeekdaysOnly, today());
    let id = data
        .add_person("Ada", 30, date(2026, 7, 18), today())
        .unwrap(); // target Mon Aug 17
    data.take_notifications(today());
    data.catch_up(date(2026, 8, 22)); // Saturday: pushed to Mon Aug 24
    let p = data.person(id).unwrap();
    assert!(p.is_due(date(2026, 8, 22)));
    assert!(data.take_notifications(date(2026, 8, 22)).is_empty());
    assert_eq!(data.take_notifications(date(2026, 8, 24)).len(), 1);
}

#[test]
fn huge_ids_make_a_file_unreadable() {
    let mut data = Data::default();
    data.add_person("Ada", 30, today(), today()).unwrap();
    data.last_id = u64::MAX;
    assert!(!data.is_sane());
}

#[test]
fn ids_stop_short_of_the_limit_instead_of_saving_an_unloadable_file() {
    let mut data = Data::default();
    data.last_id = friends_core::model::MAX_ID - 2;
    assert!(data.add_person("A", 30, today(), today()).is_ok());
    assert!(data.add_person("B", 30, today(), today()).is_err());
    assert!(data.is_sane());
}

#[test]
fn renaming_works_when_the_stored_date_is_after_today() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2026, 8, 20), date(2026, 8, 20))
        .unwrap();
    // A day earlier, after a time zone change: the unchanged date isn't re-checked.
    data.update_person(id, "Ada L.", 30, date(2026, 8, 20), today())
        .unwrap();
    assert_eq!(data.person(id).unwrap().name, "Ada L.");
    // A changed date still is.
    assert!(data
        .update_person(id, "Ada L.", 30, date(2026, 8, 21), today())
        .is_err());
}
