//! The scheduling rules: when to reach out to someone next, and on which day to remind about it.
//!
//! Everything works on calendar dates (`NaiveDate`), never on times, so a reminder can't slip a day
//! because of a time zone or a daylight-saving change.
//!
//! The rules, carried over from the Swift app:
//! - The next target is the last time you were in touch plus the person's frequency, moved forward to
//!   the first day the day setting allows (all days, weekdays only or weekends only).
//! - A target inside a blocked range moves to the first allowed day after the range, again and again
//!   until it is outside every range.
//! - A weekend target is reminded about on the Monday of its week, so there is time to plan.
//! - A missed target moves to the next run of allowed days (the next weekend, for weekends only).

use chrono::{Datelike, Days, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

/// Which days a target may fall on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum DayToggle {
    AllDays,
    WeekdaysOnly,
    #[default]
    WeekendsOnly,
}

/// Days when no target may land (a trip, a busy stretch). Both ends are included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl BlockedRange {
    pub fn contains(&self, date: NaiveDate) -> bool {
        // A range typed in backwards still means the days between its two ends.
        let (lo, hi) = if self.start <= self.end { (self.start, self.end) } else { (self.end, self.start) };
        lo <= date && date <= hi
    }

    fn last_day(&self) -> NaiveDate {
        self.start.max(self.end)
    }
}

/// When to aim to be in touch, and when to be reminded of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleResult {
    pub next_target_date: NaiveDate,
    pub notification_date: NaiveDate,
}

fn next_day(date: NaiveDate) -> NaiveDate {
    date.checked_add_days(Days::new(1)).expect("date out of range")
}

fn is_weekend(date: NaiveDate) -> bool {
    matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

/// Whether `date` is a day the setting allows.
pub fn matches(date: NaiveDate, toggle: DayToggle) -> bool {
    match toggle {
        DayToggle::AllDays => true,
        DayToggle::WeekendsOnly => is_weekend(date),
        DayToggle::WeekdaysOnly => !is_weekend(date),
    }
}

/// The first allowed day on or after `date`.
pub fn snap_to_toggle(date: NaiveDate, toggle: DayToggle) -> NaiveDate {
    let mut candidate = date;
    // Every setting allows at least two days a week, so this ends within a week.
    while !matches(candidate, toggle) {
        candidate = next_day(candidate);
    }
    candidate
}

/// The first allowed day on or after `date` that is outside every blocked range.
pub fn avoid_blocked_ranges(date: NaiveDate, ranges: &[BlockedRange], toggle: DayToggle) -> NaiveDate {
    let mut candidate = date;
    // Each move goes past the end of a range the candidate was in, so the candidate only moves forward
    // and can be inside each range at most once: this ends after at most one pass per range.
    loop {
        match ranges.iter().find(|r| r.contains(candidate)) {
            Some(r) => candidate = snap_to_toggle(next_day(r.last_day()), toggle),
            None => return candidate,
        }
    }
}

/// The Monday of the ISO week (Monday to Sunday) that holds `date`.
pub fn monday_of_week(date: NaiveDate) -> NaiveDate {
    let back = u64::from(date.weekday().num_days_from_monday());
    date.checked_sub_days(Days::new(back)).expect("date out of range")
}

fn with_notification(target: NaiveDate) -> ScheduleResult {
    let notification_date = if is_weekend(target) { monday_of_week(target) } else { target };
    ScheduleResult { next_target_date: target, notification_date }
}

/// The schedule after being in touch on `last_connected`.
pub fn compute_schedule(
    last_connected: NaiveDate,
    frequency_days: u32,
    toggle: DayToggle,
    blocked: &[BlockedRange],
) -> ScheduleResult {
    let raw = last_connected
        .checked_add_days(Days::new(u64::from(frequency_days)))
        .expect("date out of range");
    let target = avoid_blocked_ranges(snap_to_toggle(raw, toggle), blocked, toggle);
    with_notification(target)
}

/// The schedule after the target `current_target` went by without being done: the next run of allowed
/// days after the current one (for weekends only, the next weekend, not the Sunday after a missed
/// Saturday). With all days allowed, it is simply the next day.
pub fn push_missed_schedule(
    current_target: NaiveDate,
    toggle: DayToggle,
    blocked: &[BlockedRange],
) -> ScheduleResult {
    let mut candidate = current_target;
    if toggle == DayToggle::AllDays {
        candidate = next_day(candidate);
    } else {
        while matches(candidate, toggle) {
            candidate = next_day(candidate);
        }
    }
    let target = avoid_blocked_ranges(snap_to_toggle(candidate, toggle), blocked, toggle);
    with_notification(target)
}

#[cfg(test)]
mod tests {
    //! Cases the Swift tests didn't cover. The ported ones are in `tests/schedule_engine.rs`.
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn a_range_typed_backwards_still_blocks_its_days() {
        let blocked = [BlockedRange { start: date(2026, 8, 27), end: date(2026, 8, 20) }];
        assert_eq!(avoid_blocked_ranges(date(2026, 8, 22), &blocked, DayToggle::WeekendsOnly), date(2026, 8, 29));
    }

    #[test]
    fn ranges_listed_out_of_order_are_all_skipped() {
        let blocked = [
            BlockedRange { start: date(2026, 8, 28), end: date(2026, 8, 30) },
            BlockedRange { start: date(2026, 8, 20), end: date(2026, 8, 27) },
        ];
        assert_eq!(avoid_blocked_ranges(date(2026, 8, 22), &blocked, DayToggle::WeekendsOnly), date(2026, 9, 5));
    }

    #[test]
    fn a_target_on_the_last_day_of_a_range_moves_past_it() {
        let blocked = [BlockedRange { start: date(2026, 8, 20), end: date(2026, 8, 22) }];
        assert_eq!(avoid_blocked_ranges(date(2026, 8, 22), &blocked, DayToggle::AllDays), date(2026, 8, 23));
    }

    #[test]
    fn monday_of_week_for_a_monday_is_that_day() {
        assert_eq!(monday_of_week(date(2026, 8, 17)), date(2026, 8, 17));
    }

    #[test]
    fn the_monday_lead_can_cross_a_year_boundary() {
        // Saturday Jan 2, 2027 belongs to the week starting Monday Dec 28, 2026.
        let result = compute_schedule(date(2026, 12, 3), 30, DayToggle::WeekendsOnly, &[]);
        assert_eq!(result.next_target_date, date(2027, 1, 2));
        assert_eq!(result.notification_date, date(2026, 12, 28));
    }

    #[test]
    fn weekdays_only_missed_friday_moves_to_monday() {
        // A missed Friday: the run of weekdays it belongs to ends that day, so the next is Monday.
        let result = push_missed_schedule(date(2026, 8, 21), DayToggle::WeekdaysOnly, &[]);
        assert_eq!(result.next_target_date, date(2026, 8, 24));
        assert_eq!(result.notification_date, date(2026, 8, 24));
    }

    #[test]
    fn weekend_targets_under_all_days_still_get_the_monday_lead() {
        let result = compute_schedule(date(2026, 7, 23), 30, DayToggle::AllDays, &[]);
        assert_eq!(result.next_target_date, date(2026, 8, 22));
        assert_eq!(result.notification_date, date(2026, 8, 17));
    }

    #[test]
    fn day_toggle_is_stored_in_camel_case() {
        assert_eq!(serde_json::to_string(&DayToggle::WeekendsOnly).unwrap(), "\"weekendsOnly\"");
    }
}
