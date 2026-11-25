//! The data changes the UI makes, and catching up on missed reminders.

use chrono::NaiveDate;
use friends_core::model::{Data, MAX_NAME_CHARS};
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

/// The message `add_person` refuses this input with.
fn person_refusal(data: &mut Data, name: &str, frequency: u32, last: NaiveDate) -> String {
    data.add_person(name, frequency, last, today())
        .unwrap_err()
        .0
}

/// The message `add_blocked` refuses this range with.
fn blocked_refusal(data: &mut Data, start: NaiveDate, end: NaiveDate, label: &str) -> String {
    data.add_blocked(start, end, label, today()).unwrap_err().0
}

#[test]
fn bad_input_is_refused_with_a_message() {
    let mut data = Data::default();
    let d = &mut data;
    let too_long = "a".repeat(MAX_NAME_CHARS + 1);
    for blank in ["", "   ", "\t\n"] {
        assert_eq!(person_refusal(d, blank, 30, today()), "Give them a name.");
    }
    assert_eq!(
        person_refusal(d, &too_long, 30, today()),
        format!("Keep the name under {MAX_NAME_CHARS} characters.")
    );
    assert_eq!(
        person_refusal(d, "Ada", 31, today()),
        "Pick how often to be in touch."
    );
    assert_eq!(
        person_refusal(d, "Ada", 30, date(2026, 8, 20)),
        "The last time you were in touch can't be in the future."
    );
    assert_eq!(
        person_refusal(d, "Ada", 30, date(1899, 12, 31)),
        "Pick a date between 1900 and today."
    );

    assert_eq!(
        blocked_refusal(d, date(2026, 8, 25), date(2026, 8, 24), "Trip"),
        "The range has to end on or after the day it starts."
    );
    assert_eq!(
        blocked_refusal(d, date(1899, 12, 31), date(1900, 1, 2), "Trip"),
        "Pick dates from 1900 up to ten years from now."
    );
    assert_eq!(
        blocked_refusal(d, date(2026, 8, 25), date(2200, 12, 31), "Typo"),
        "Pick dates from 1900 up to ten years from now."
    );
    assert_eq!(
        blocked_refusal(d, date(2026, 8, 25), date(2026, 8, 26), &too_long),
        format!("Keep the label under {MAX_NAME_CHARS} characters.")
    );

    assert!(data.people.is_empty() && data.blocked.is_empty());
}

#[test]
fn an_edit_is_refused_with_the_same_messages() {
    let mut data = Data::default();
    let id = data.add_person("Ada", 30, today(), today()).unwrap();
    let mut edit = |name: &str, frequency: u32, last: NaiveDate| {
        data.update_person(id, name, frequency, last, today())
            .unwrap_err()
            .0
    };
    assert_eq!(edit(" ", 30, today()), "Give them a name.");
    assert_eq!(edit("Ada", 31, today()), "Pick how often to be in touch.");
    assert_eq!(
        edit("Ada", 30, date(2026, 8, 20)),
        "The last time you were in touch can't be in the future."
    );
    assert_eq!(data.person(id).unwrap().name, "Ada");
}

#[test]
fn the_edges_of_the_limits_are_accepted() {
    let mut data = Data::default();
    let longest = "a".repeat(MAX_NAME_CHARS);
    let id = data.add_person(&longest, 30, today(), today()).unwrap();
    assert_eq!(data.person(id).unwrap().name, longest);
    data.add_person("Ada", 30, date(1900, 1, 1), today())
        .unwrap();
    data.add_blocked(date(1900, 1, 1), date(1900, 1, 2), &longest, today())
        .unwrap();
    assert_eq!(data.blocked[0].label, longest);
}

#[test]
fn marking_done_counts_the_next_target_from_today() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 90, date(2026, 5, 1), today())
        .unwrap();
    data.mark_done(id, None, today()).unwrap();
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
fn removing_a_blocked_range_doesnt_mark_anyone_as_missed() {
    // Weekends only, monthly. The natural target, Sat Nov 14, is blocked, so the plan is Sat Nov 21.
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2026, 10, 10), date(2026, 10, 12))
        .unwrap();
    data.add_blocked(
        date(2026, 11, 13),
        date(2026, 11, 15),
        "Trip",
        date(2026, 10, 12),
    )
    .unwrap();
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 11, 21));
    // Removed once it's over: Nov 14 has gone by, but nobody was asked to aim for it.
    let blocked_id = data.blocked[0].id;
    assert!(data.delete_blocked(blocked_id, date(2026, 11, 18)));
    let p = data.person(id).unwrap();
    assert_eq!(p.next_target, date(2026, 11, 21));
    assert!(!p.overdue);
}

#[test]
fn someone_already_missed_stays_missed_after_a_recompute() {
    let mut data = Data::default();
    let id = data
        .add_person("Grace", 30, date(2025, 1, 1), today())
        .unwrap();
    assert!(data.person(id).unwrap().overdue);
    data.add_blocked(date(2026, 9, 1), date(2026, 9, 3), "Trip", today())
        .unwrap();
    assert!(data.person(id).unwrap().overdue);
}

#[test]
fn a_recompute_clears_missed_when_the_new_target_is_still_ahead() {
    let mut data = Data::default();
    // Jul 10 + 30 = Sun Aug 9, gone by on Wed Aug 19: pushed to Sat Aug 22, missed.
    let id = data
        .add_person("Ada", 30, date(2026, 7, 10), today())
        .unwrap();
    let p = data.person(id).unwrap();
    assert_eq!(p.next_target, date(2026, 8, 22));
    assert!(p.overdue);
    // A trip over the old target: the new plan aims for Sat Aug 22, which is still ahead.
    data.add_blocked(date(2026, 8, 1), date(2026, 8, 20), "Trip", today())
        .unwrap();
    let p = data.person(id).unwrap();
    assert_eq!(p.next_target, date(2026, 8, 22));
    assert_eq!(p.notification_date, date(2026, 8, 17));
    assert!(!p.overdue);
}

#[test]
fn changing_the_day_setting_reschedules_everyone() {
    let mut data = Data::default();
    let id = data
        .add_person("Ada", 30, date(2026, 7, 23), today())
        .unwrap();
    data.set_day_setting(DayToggle::WeekdaysOnly, None, today())
        .unwrap();
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
    assert!(data.mark_done(42, None, today()).is_err());
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
    data.set_day_setting(DayToggle::WeekdaysOnly, None, today())
        .unwrap();
    let id = data
        .add_person("Ada", 30, date(2026, 7, 18), today())
        .unwrap(); // Jul 18 + 30 = Mon Aug 17
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 8, 17));
    assert!(!data.catch_up(date(2026, 8, 21))); // still that week on Friday
    assert!(data.catch_up(date(2026, 8, 22))); // the week is over
    assert_eq!(data.person(id).unwrap().next_target, date(2026, 8, 24));
}

#[test]
fn decades_of_missed_days_catch_up_fully_under_custom_all_days() {
    let mut data = Data::default();
    data.set_day_setting(DayToggle::Custom, None, today())
        .unwrap();
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
fn a_long_backlog_under_weekdays_only_lands_on_the_next_monday() {
    let mut data = Data::default();
    let saturday = date(2026, 8, 22);
    data.set_day_setting(DayToggle::WeekdaysOnly, None, saturday)
        .unwrap();
    let id = data
        .add_person("Ada", 30, date(2020, 1, 1), saturday)
        .unwrap();
    let p = data.person(id).unwrap();
    assert_eq!(p.next_target, date(2026, 8, 24));
    assert_eq!(p.notification_date, date(2026, 8, 24));
    assert!(p.overdue);
    assert!(p.is_due(saturday));
    // A later check the same day changes nothing.
    assert!(!data.catch_up(saturday));
    // Shown over the weekend, but notified on Monday, once.
    assert!(data.take_notifications(saturday).is_empty());
    assert_eq!(data.take_notifications(date(2026, 8, 24)).len(), 1);
    assert!(data.take_notifications(date(2026, 8, 24)).is_empty());
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
    data.set_day_setting(DayToggle::WeekdaysOnly, None, today())
        .unwrap();
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
    let mut data = Data {
        last_id: friends_core::model::MAX_ID - 2,
        ..Data::default()
    };
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

/// One person and one blocked range, with `edit` applied: is the result a file Friends would load?
fn loads_with(edit: impl FnOnce(&mut Data)) -> bool {
    let mut data = Data::default();
    data.add_person("Ada", 30, date(2026, 7, 23), today())
        .unwrap();
    data.add_blocked(date(2026, 9, 1), date(2026, 9, 5), "Trip", today())
        .unwrap();
    assert!(data.is_sane());
    edit(&mut data);
    data.is_sane()
}

// Targets and reminder days are worked out, and get pushed past blocked ranges, which may end ten
// years out. So they, and range ends, may run to 2300.
#[test]
fn a_loaded_file_may_hold_targets_reminder_days_and_range_ends_up_to_2300() {
    for (day, loads) in [(date(2300, 12, 31), true), (date(2301, 1, 1), false)] {
        assert_eq!(
            loads_with(|d| d.people[0].next_target = day),
            loads,
            "next target {day}"
        );
        assert_eq!(
            loads_with(|d| d.people[0].notification_date = day),
            loads,
            "reminder day {day}"
        );
        assert_eq!(
            loads_with(|d| d.blocked[0].end = day),
            loads,
            "range end {day}"
        );
    }
}

// Last contact dates and range starts are entered by hand, and the app refuses them past 2200.
#[test]
fn a_loaded_file_may_hold_last_contact_dates_and_range_starts_only_up_to_2200() {
    for (day, loads) in [(date(2200, 12, 31), true), (date(2201, 1, 1), false)] {
        assert_eq!(
            loads_with(|d| d.people[0].last_connected = day),
            loads,
            "last contact {day}"
        );
        let range_from = |d: &mut Data| {
            d.blocked[0].start = day;
            d.blocked[0].end = date(2250, 1, 1);
        };
        assert_eq!(loads_with(range_from), loads, "range start {day}");
    }
    // The same day is fine as a target.
    assert!(loads_with(|d| d.people[0].next_target = date(2201, 1, 1)));
}

#[test]
fn a_loaded_file_may_hold_no_person_or_range_date_before_1900() {
    for (day, loads) in [(date(1900, 1, 1), true), (date(1899, 12, 31), false)] {
        assert_eq!(
            loads_with(|d| d.people[0].last_connected = day),
            loads,
            "last contact {day}"
        );
        assert_eq!(
            loads_with(|d| d.people[0].next_target = day),
            loads,
            "next target {day}"
        );
        assert_eq!(
            loads_with(|d| d.people[0].notification_date = day),
            loads,
            "reminder day {day}"
        );
        // A range ends on or after its start, so the start is the one to try.
        let range_from = |d: &mut Data| {
            d.blocked[0].start = day;
            d.blocked[0].end = date(1900, 1, 5);
        };
        assert_eq!(loads_with(range_from), loads, "range start {day}");
    }
}
