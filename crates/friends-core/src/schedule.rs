//! The scheduling rules: when to reach out to someone next, and on which day to remind about it.
//!
//! Everything works on calendar dates (`NaiveDate`), never on times, so a reminder can't slip a day
//! because of a time zone or a daylight-saving change.
//!
//! The rules, carried over from the Swift app:
//! - The next target is the last time you were in touch plus the person's frequency, moved forward to
//!   the first day the day setting allows (weekdays only, weekends only, or the days you picked).
//! - A target inside a blocked range moves to the first allowed day after the range, again and again
//!   until it is outside every range.
//! - A weekend target is reminded about on the Monday of its week, so there is time to plan.
//! - A missed target moves to the next run of allowed days (the next weekend, for weekends only). With
//!   days you picked, each picked day is its own chance: a missed one moves to the next picked day.
//!
//! Changes from the Swift app, on purpose: a blocked range typed backwards blocks the days between its
//! ends (Swift ignored it), frequencies are capped at ten years, and "any day" became days you pick
//! (all seven at first, which works the way "any day" did).

use std::fmt;

use chrono::{Datelike, Days, NaiveDate, Weekday};
use serde::de::{self, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Which days a target may fall on, as the setting is stored. With `Custom`, the days are the ones
/// picked (`Settings::custom_days`); see [`DayRule`] for both together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum DayToggle {
    WeekdaysOnly,
    #[default]
    WeekendsOnly,
    /// "Any day" before custom days existed: the same as all seven days picked.
    #[serde(alias = "allDays")]
    Custom,
}

/// Some days of the week. Stored as a list of short names, Monday first: `["mon", "sat"]`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DaySet(u8);

const DAY_NAMES: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

impl DaySet {
    pub const ALL: DaySet = DaySet(0b111_1111);
    pub const NONE: DaySet = DaySet(0);

    pub fn of(days: &[Weekday]) -> DaySet {
        DaySet(days.iter().fold(0, |bits, d| bits | Self::bit(*d)))
    }

    fn bit(day: Weekday) -> u8 {
        1 << day.num_days_from_monday()
    }

    pub fn contains(self, day: Weekday) -> bool {
        self.0 & Self::bit(day) != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl Default for DaySet {
    fn default() -> Self {
        DaySet::ALL
    }
}

impl fmt::Debug for DaySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = DAY_NAMES
            .iter()
            .enumerate()
            .filter(|(i, _)| self.0 & (1 << i) != 0)
            .map(|(_, n)| *n);
        f.debug_list().entries(names).finish()
    }
}

impl Serialize for DaySet {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(None)?;
        for (i, name) in DAY_NAMES.iter().enumerate() {
            if self.0 & (1 << i) != 0 {
                seq.serialize_element(name)?;
            }
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for DaySet {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Names;
        impl<'de> Visitor<'de> for Names {
            type Value = DaySet;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a list of days: \"mon\" to \"sun\"")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<DaySet, A::Error> {
                let mut bits = 0u8;
                while let Some(name) = seq.next_element::<String>()? {
                    let i = DAY_NAMES
                        .iter()
                        .position(|n| *n == name)
                        .ok_or_else(|| de::Error::unknown_variant(&name, &DAY_NAMES))?;
                    bits |= 1 << i;
                }
                Ok(DaySet(bits))
            }
        }
        deserializer.deserialize_seq(Names)
    }
}

/// The day setting the rules work with: which days a target may fall on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayRule {
    WeekdaysOnly,
    WeekendsOnly,
    /// Only these days. Never empty: the setting refuses an empty set.
    Custom(DaySet),
}

impl DayRule {
    /// Every day of the week: what "any day" was.
    pub const ANY: DayRule = DayRule::Custom(DaySet::ALL);

    pub fn new(toggle: DayToggle, custom_days: DaySet) -> DayRule {
        match toggle {
            DayToggle::WeekdaysOnly => DayRule::WeekdaysOnly,
            DayToggle::WeekendsOnly => DayRule::WeekendsOnly,
            // An empty set would allow no day at all; it can't be saved, but just in case.
            DayToggle::Custom if custom_days.is_empty() => DayRule::ANY,
            DayToggle::Custom => DayRule::Custom(custom_days),
        }
    }
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
        let (lo, hi) = if self.start <= self.end {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        };
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
    date.checked_add_days(Days::new(1))
        .expect("date out of range")
}

fn is_weekend(date: NaiveDate) -> bool {
    matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

/// Whether `date` is a day the setting allows.
pub fn matches(date: NaiveDate, rule: DayRule) -> bool {
    match rule {
        DayRule::WeekendsOnly => is_weekend(date),
        DayRule::WeekdaysOnly => !is_weekend(date),
        DayRule::Custom(days) => days.contains(date.weekday()),
    }
}

/// The first allowed day on or after `date`.
pub fn snap_to_toggle(date: NaiveDate, rule: DayRule) -> NaiveDate {
    let mut candidate = date;
    // Every setting allows at least one day a week (an empty set becomes all days), so this ends
    // within a week.
    while !matches(candidate, rule) {
        candidate = next_day(candidate);
    }
    candidate
}

/// The first allowed day on or after `date` that is outside every blocked range.
pub fn avoid_blocked_ranges(date: NaiveDate, ranges: &[BlockedRange], rule: DayRule) -> NaiveDate {
    let mut candidate = snap_to_toggle(date, rule);
    // Each move goes past the end of a range the candidate was in, so the candidate only moves forward
    // and can be inside each range at most once: this ends after at most one pass per range.
    loop {
        match ranges.iter().find(|r| r.contains(candidate)) {
            Some(r) => candidate = snap_to_toggle(next_day(r.last_day()), rule),
            None => return candidate,
        }
    }
}

/// The Monday of the ISO week (Monday to Sunday) that holds `date`.
pub fn monday_of_week(date: NaiveDate) -> NaiveDate {
    let back = u64::from(date.weekday().num_days_from_monday());
    date.checked_sub_days(Days::new(back))
        .expect("date out of range")
}

fn with_notification(target: NaiveDate) -> ScheduleResult {
    let notification_date = if is_weekend(target) {
        monday_of_week(target)
    } else {
        target
    };
    ScheduleResult {
        next_target_date: target,
        notification_date,
    }
}

/// The longest frequency the engine accepts, in days (ten years). Longer ones are treated as this, so
/// a corrupt or absurd value can't push the date past what the date type holds.
pub const MAX_FREQUENCY_DAYS: u32 = 3650;

/// The schedule after being in touch on `last_connected`.
///
/// Dates must be in a sane range (before the year 200,000): `model.rs` only lets in dates from 1900 to
/// 2200 and the four offered frequencies, typed in or loaded from the file.
pub fn compute_schedule(
    last_connected: NaiveDate,
    frequency_days: u32,
    rule: DayRule,
    blocked: &[BlockedRange],
) -> ScheduleResult {
    let raw = last_connected
        .checked_add_days(Days::new(u64::from(frequency_days.min(MAX_FREQUENCY_DAYS))))
        .expect("date out of range");
    let target = avoid_blocked_ranges(raw, blocked, rule);
    with_notification(target)
}

/// The last day of the run of allowed days that starts on `target`: the Sunday for a Saturday target
/// under weekends only, the Friday for a weekday target under weekdays only, and the day itself for
/// days you picked (each picked day is its own chance). A target counts as missed only once this day
/// has gone by.
pub fn last_day_of_run(target: NaiveDate, rule: DayRule) -> NaiveDate {
    if matches!(rule, DayRule::Custom(_)) || !matches(target, rule) {
        return target;
    }
    let mut last = target;
    while matches(next_day(last), rule) {
        last = next_day(last);
    }
    last
}

/// The schedule after the target `current_target` went by without being done: the next run of allowed
/// days after the current one (for weekends only, the next weekend, not the Sunday after a missed
/// Saturday). With days you picked, it is the next picked day.
pub fn push_missed_schedule(
    current_target: NaiveDate,
    rule: DayRule,
    blocked: &[BlockedRange],
) -> ScheduleResult {
    let mut candidate = current_target;
    if matches!(rule, DayRule::Custom(_)) {
        candidate = next_day(candidate);
    } else {
        while matches(candidate, rule) {
            candidate = next_day(candidate);
        }
    }
    let target = avoid_blocked_ranges(candidate, blocked, rule);
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
        let blocked = [BlockedRange {
            start: date(2026, 8, 27),
            end: date(2026, 8, 20),
        }];
        assert_eq!(
            avoid_blocked_ranges(date(2026, 8, 22), &blocked, DayRule::WeekendsOnly),
            date(2026, 8, 29)
        );
    }

    #[test]
    fn ranges_listed_out_of_order_are_all_skipped() {
        let blocked = [
            BlockedRange {
                start: date(2026, 8, 28),
                end: date(2026, 8, 30),
            },
            BlockedRange {
                start: date(2026, 8, 20),
                end: date(2026, 8, 27),
            },
        ];
        assert_eq!(
            avoid_blocked_ranges(date(2026, 8, 22), &blocked, DayRule::WeekendsOnly),
            date(2026, 9, 5)
        );
    }

    #[test]
    fn a_target_on_the_last_day_of_a_range_moves_past_it() {
        let blocked = [BlockedRange {
            start: date(2026, 8, 20),
            end: date(2026, 8, 22),
        }];
        assert_eq!(
            avoid_blocked_ranges(date(2026, 8, 22), &blocked, DayRule::ANY),
            date(2026, 8, 23)
        );
    }

    #[test]
    fn monday_of_week_for_a_monday_is_that_day() {
        assert_eq!(monday_of_week(date(2026, 8, 17)), date(2026, 8, 17));
    }

    #[test]
    fn the_monday_lead_can_cross_a_year_boundary() {
        // Saturday Jan 2, 2027 belongs to the week starting Monday Dec 28, 2026.
        let result = compute_schedule(date(2026, 12, 3), 30, DayRule::WeekendsOnly, &[]);
        assert_eq!(result.next_target_date, date(2027, 1, 2));
        assert_eq!(result.notification_date, date(2026, 12, 28));
    }

    #[test]
    fn weekdays_only_missed_friday_moves_to_monday() {
        // A missed Friday: the run of weekdays it belongs to ends that day, so the next is Monday.
        let result = push_missed_schedule(date(2026, 8, 21), DayRule::WeekdaysOnly, &[]);
        assert_eq!(result.next_target_date, date(2026, 8, 24));
        assert_eq!(result.notification_date, date(2026, 8, 24));
    }

    #[test]
    fn weekend_targets_under_all_days_still_get_the_monday_lead() {
        let result = compute_schedule(date(2026, 7, 23), 30, DayRule::ANY, &[]);
        assert_eq!(result.next_target_date, date(2026, 8, 22));
        assert_eq!(result.notification_date, date(2026, 8, 17));
    }

    #[test]
    fn a_run_of_allowed_days_ends_on_sunday_friday_or_the_same_day() {
        assert_eq!(
            last_day_of_run(date(2026, 8, 22), DayRule::WeekendsOnly),
            date(2026, 8, 23)
        );
        assert_eq!(
            last_day_of_run(date(2026, 8, 23), DayRule::WeekendsOnly),
            date(2026, 8, 23)
        );
        assert_eq!(
            last_day_of_run(date(2026, 8, 17), DayRule::WeekdaysOnly),
            date(2026, 8, 21)
        );
        assert_eq!(
            last_day_of_run(date(2026, 8, 22), DayRule::ANY),
            date(2026, 8, 22)
        );
    }

    #[test]
    fn avoiding_ranges_also_snaps_to_an_allowed_day() {
        assert_eq!(
            avoid_blocked_ranges(date(2026, 8, 19), &[], DayRule::WeekendsOnly),
            date(2026, 8, 22)
        );
    }

    #[test]
    fn an_absurd_frequency_is_capped_instead_of_crashing() {
        let result = compute_schedule(date(2026, 8, 19), u32::MAX, DayRule::ANY, &[]);
        assert_eq!(result.next_target_date, date(2036, 8, 16)); // 3650 days later
    }

    #[test]
    fn day_toggle_is_stored_in_camel_case() {
        assert_eq!(
            serde_json::to_string(&DayToggle::WeekendsOnly).unwrap(),
            "\"weekendsOnly\""
        );
    }
}
