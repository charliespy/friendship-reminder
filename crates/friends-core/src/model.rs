//! The app's data: the people, the blocked ranges and the settings, and every change the UI can make
//! to them. All of it is plain data plus pure functions of "today", so it can be tested without a clock.

use chrono::{Datelike, Days, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::schedule::{
    compute_schedule, last_day_of_run, push_missed_schedule, BlockedRange, DayToggle,
    ScheduleResult,
};

/// The frequencies the app offers, in days.
pub const FREQUENCIES: [u32; 4] = [30, 90, 180, 365];

/// The longest name the app stores, in characters.
pub const MAX_NAME_CHARS: usize = 100;

/// Most times a missed target is pushed in one catch-up. A long backlog first jumps to the week before
/// today, so a few pushes always suffice; the cap only guards against a future bug looping forever.
const MAX_PUSHES: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: u64,
    pub name: String,
    pub frequency_days: u32,
    /// The last day you were in touch.
    pub last_connected: NaiveDate,
    /// The day to aim for.
    pub next_target: NaiveDate,
    /// The day the reminder shows up (the Monday before a weekend target).
    pub notification_date: NaiveDate,
    /// A target went by without being marked done, so this one is a catch-up.
    #[serde(default)]
    pub overdue: bool,
    /// The notification date the app last sent a notification for, so each one goes out once.
    #[serde(default)]
    pub notified_for: Option<NaiveDate>,
}

impl Person {
    /// The reminder is showing: its day has come and you haven't marked it done.
    /// A missed target stays up too, even before its new reminder day (under weekdays only, a missed
    /// week is pushed on the Saturday, and the person shouldn't vanish for the weekend).
    pub fn is_due(&self, today: NaiveDate) -> bool {
        self.overdue || self.notification_date <= today
    }

    fn apply(&mut self, s: ScheduleResult) {
        self.next_target = s.next_target_date;
        self.notification_date = s.notification_date;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Blocked {
    pub id: u64,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub day_toggle: DayToggle,
    /// Send a macOS notification when a reminder comes up.
    #[serde(default = "yes")]
    pub notifications: bool,
}

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            day_toggle: DayToggle::WeekendsOnly,
            notifications: true,
        }
    }
}

/// Everything the app stores. Saved as one JSON file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    /// The file format's version, for later changes to it.
    pub version: u32,
    #[serde(default)]
    pub people: Vec<Person>,
    #[serde(default)]
    pub blocked: Vec<Blocked>,
    #[serde(default)]
    pub settings: Settings,
    /// The last id handed out, so a deleted id is never reused.
    #[serde(default)]
    pub last_id: u64,
}

impl Default for Data {
    fn default() -> Self {
        Data {
            version: crate::store::CURRENT_VERSION,
            people: Vec::new(),
            blocked: Vec::new(),
            settings: Settings::default(),
            last_id: 0,
        }
    }
}

/// A change the user asked for that can't be made, with a message fit to show them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalid(pub String);

impl std::fmt::Display for Invalid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Invalid {}

fn invalid<T>(msg: impl Into<String>) -> Result<T, Invalid> {
    Err(Invalid(msg.into()))
}

/// Trims a name or label and squeezes runs of spaces into one.
fn squeeze(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clean_name(name: &str) -> Result<String, Invalid> {
    let name = squeeze(name);
    if name.is_empty() {
        return invalid("Give them a name.");
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return invalid(format!("Keep the name under {MAX_NAME_CHARS} characters."));
    }
    Ok(name)
}

fn check_frequency(days: u32) -> Result<(), Invalid> {
    if FREQUENCIES.contains(&days) {
        Ok(())
    } else {
        invalid("Pick how often to be in touch.")
    }
}

/// Whether a date is one the app works with: from 1900 through 2200. Anything else is a typo or a
/// damaged file, and dates far out would only make the scheduling loops do needless work.
pub fn sane_date(date: NaiveDate) -> bool {
    (1900..=2200).contains(&date.year())
}

/// Whether a worked-out date (a target, a reminder day, a range's end) is one a saved file may hold:
/// from 1900 to 2300, so targets pushed past a range ending ten years out still load.
pub fn sane_target(date: NaiveDate) -> bool {
    (1900..=2300).contains(&date.year())
}

/// Ids stay below this, so they never overflow and stay exact as JavaScript numbers.
pub const MAX_ID: u64 = 1 << 53;

fn check_last_connected(date: NaiveDate, today: NaiveDate) -> Result<(), Invalid> {
    if !sane_date(date) {
        return invalid("Pick a date between 1900 and today.");
    }
    if date > today {
        invalid("The last time you were in touch can't be in the future.")
    } else {
        Ok(())
    }
}

impl Data {
    /// Whether loaded data holds only values the app could have saved: known frequencies and sane
    /// dates. A file that fails this is treated as unreadable.
    pub fn is_sane(&self) -> bool {
        let people_ok = self.people.iter().all(|p| {
            FREQUENCIES.contains(&p.frequency_days)
                && sane_date(p.last_connected)
                && [p.next_target, p.notification_date]
                    .into_iter()
                    .all(sane_target)
        });
        let blocked_ok = self
            .blocked
            .iter()
            .all(|b| sane_date(b.start) && sane_target(b.end));
        let mut ids: Vec<u64> = self.ids().collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        let ids_ok =
            ids.len() == count && self.last_id < MAX_ID && ids.last().is_none_or(|&id| id < MAX_ID);
        people_ok && blocked_ok && ids_ok
    }

    fn ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.people
            .iter()
            .map(|p| p.id)
            .chain(self.blocked.iter().map(|b| b.id))
    }

    /// Makes sure new ids come after every id in use, for a file whose `lastId` is missing or behind.
    pub fn repair_ids(&mut self) {
        self.last_id = self.last_id.max(self.ids().max().unwrap_or(0));
    }

    fn next_id(&mut self) -> u64 {
        // is_sane keeps ids under MAX_ID, so this can't overflow.
        self.last_id += 1;
        self.last_id
    }

    fn ranges(&self) -> Vec<BlockedRange> {
        self.blocked
            .iter()
            .map(|b| BlockedRange {
                start: b.start,
                end: b.end,
            })
            .collect()
    }

    pub fn person(&self, id: u64) -> Option<&Person> {
        self.people.iter().find(|p| p.id == id)
    }

    fn person_mut(&mut self, id: u64) -> Result<&mut Person, Invalid> {
        match self.people.iter_mut().find(|p| p.id == id) {
            Some(p) => Ok(p),
            None => invalid("That person isn't in your list any more."),
        }
    }

    /// A fresh schedule for one person from their last time in touch, caught up to `today`.
    fn reschedule(
        person: &mut Person,
        toggle: DayToggle,
        ranges: &[BlockedRange],
        today: NaiveDate,
    ) {
        person.apply(compute_schedule(
            person.last_connected,
            person.frequency_days,
            toggle,
            ranges,
        ));
        person.overdue = false;
        Self::catch_up_person(person, toggle, ranges, today);
    }

    /// Pushes a target that went by without being done to the next allowed days, again and again
    /// until it is current. A target went by once the whole run of allowed days it starts is over (the
    /// weekend, for a Saturday target under weekends only). Returns whether it moved.
    fn catch_up_person(
        person: &mut Person,
        toggle: DayToggle,
        ranges: &[BlockedRange],
        today: NaiveDate,
    ) -> bool {
        let mut moved = false;
        let mut pushes = 0;
        // Pushing from a target years back would take one step per missed run; start from the week
        // before today instead, which lands on the same day.
        let week_ago = today.checked_sub_days(Days::new(7)).unwrap_or(today);
        while last_day_of_run(person.next_target, toggle) < today && pushes < MAX_PUSHES {
            let from = person.next_target.max(week_ago);
            person.apply(push_missed_schedule(from, toggle, ranges));
            pushes += 1;
            moved = true;
        }
        if moved {
            person.overdue = true;
        }
        moved
    }

    pub fn add_person(
        &mut self,
        name: &str,
        frequency_days: u32,
        last_connected: NaiveDate,
        today: NaiveDate,
    ) -> Result<u64, Invalid> {
        let name = clean_name(name)?;
        check_frequency(frequency_days)?;
        check_last_connected(last_connected, today)?;
        let id = self.next_id();
        let mut person = Person {
            id,
            name,
            frequency_days,
            last_connected,
            next_target: last_connected,
            notification_date: last_connected,
            overdue: false,
            notified_for: None,
        };
        Self::reschedule(&mut person, self.settings.day_toggle, &self.ranges(), today);
        self.people.push(person);
        Ok(id)
    }

    pub fn update_person(
        &mut self,
        id: u64,
        name: &str,
        frequency_days: u32,
        last_connected: NaiveDate,
        today: NaiveDate,
    ) -> Result<(), Invalid> {
        let name = clean_name(name)?;
        check_frequency(frequency_days)?;
        check_last_connected(last_connected, today)?;
        let (toggle, ranges) = (self.settings.day_toggle, self.ranges());
        let person = self.person_mut(id)?;
        let schedule_changed =
            person.frequency_days != frequency_days || person.last_connected != last_connected;
        person.name = name;
        person.frequency_days = frequency_days;
        person.last_connected = last_connected;
        if schedule_changed {
            Self::reschedule(person, toggle, &ranges, today);
        }
        Ok(())
    }

    /// Removes people. Ids that are already gone are skipped. Returns how many were removed.
    pub fn delete_people(&mut self, ids: &[u64]) -> usize {
        let before = self.people.len();
        self.people.retain(|p| !ids.contains(&p.id));
        before - self.people.len()
    }

    /// You were in touch today: their next target counts from today.
    pub fn mark_done(&mut self, id: u64, today: NaiveDate) -> Result<(), Invalid> {
        let (toggle, ranges) = (self.settings.day_toggle, self.ranges());
        let person = self.person_mut(id)?;
        person.last_connected = today;
        Self::reschedule(person, toggle, &ranges, today);
        Ok(())
    }

    pub fn add_blocked(
        &mut self,
        start: NaiveDate,
        end: NaiveDate,
        label: &str,
        today: NaiveDate,
    ) -> Result<u64, Invalid> {
        let label = squeeze(label);
        if label.chars().count() > MAX_NAME_CHARS {
            return invalid(format!("Keep the label under {MAX_NAME_CHARS} characters."));
        }
        // Ten years ahead at most: targets get pushed past the range, and they must stay inside the
        // dates a saved file may hold.
        let horizon = today.checked_add_days(Days::new(3653)).unwrap_or(today);
        if !sane_date(start) || end > horizon {
            return invalid("Pick dates from 1900 up to ten years from now.");
        }
        if end < start {
            return invalid("The range has to end on or after the day it starts.");
        }
        let id = self.next_id();
        self.blocked.push(Blocked {
            id,
            start,
            end,
            label,
        });
        self.blocked.sort_by_key(|b| (b.start, b.end, b.id));
        self.recompute_all(today);
        Ok(id)
    }

    pub fn delete_blocked(&mut self, id: u64, today: NaiveDate) -> bool {
        let before = self.blocked.len();
        self.blocked.retain(|b| b.id != id);
        let removed = self.blocked.len() != before;
        if removed {
            self.recompute_all(today);
        }
        removed
    }

    pub fn set_day_toggle(&mut self, toggle: DayToggle, today: NaiveDate) {
        if self.settings.day_toggle != toggle {
            self.settings.day_toggle = toggle;
            self.recompute_all(today);
        }
    }

    pub fn set_notifications(&mut self, on: bool) {
        self.settings.notifications = on;
    }

    /// Everyone's schedule worked out again from their last time in touch, under the current day
    /// setting and blocked ranges (after a blocked range or the day setting changed).
    pub fn recompute_all(&mut self, today: NaiveDate) {
        let (toggle, ranges) = (self.settings.day_toggle, self.ranges());
        for person in &mut self.people {
            Self::reschedule(person, toggle, &ranges, today);
        }
    }

    /// Pushes every missed target forward to today or later. Run on launch, on wake and every hour.
    /// Returns whether anything changed.
    pub fn catch_up(&mut self, today: NaiveDate) -> bool {
        let (toggle, ranges) = (self.settings.day_toggle, self.ranges());
        let mut changed = false;
        for person in &mut self.people {
            changed |= Self::catch_up_person(person, toggle, &ranges, today);
        }
        changed
    }

    /// The people whose reminder came up and hasn't been notified yet, marked as notified now.
    /// The caller sends the notifications, or not when they're off: reminders that came up while they
    /// were off count as handled, so turning them back on doesn't send a backlog all at once (the
    /// dropdown still lists those people).
    pub fn take_notifications(&mut self, today: NaiveDate) -> Vec<Person> {
        let mut out = Vec::new();
        for person in &mut self.people {
            if person.notification_date <= today
                && person.notified_for != Some(person.notification_date)
            {
                person.notified_for = Some(person.notification_date);
                out.push(person.clone());
            }
        }
        out
    }
}
