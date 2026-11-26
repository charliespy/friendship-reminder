//! The app's data: the people, the blocked ranges and the settings, and every change the UI can make
//! to them. All of it is plain data plus pure functions of "today", so it can be tested without a clock.

use chrono::{Datelike, Days, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::schedule::{
    compute_schedule, last_day_of_run, push_missed_schedule, read_day_setting, BlockedRange,
    DayRule, DaySet, DayToggle, ScheduleResult, StoredToggle,
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
    /// While overdue: the latest target that went by, the day you last meant to reach out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missed_target: Option<NaiveDate>,
    /// The notification date the app last sent a notification for, so each one goes out once.
    #[serde(default)]
    pub notified_for: Option<NaiveDate>,
    /// The id every device knows this row by (the local `id` is this Mac's own number for it).
    #[serde(default)]
    pub uid: String,
    /// When this row last changed, here or on another device, in milliseconds since 1970: when two
    /// devices change the same row, the later change wins (on a tie, see `account_wins` in `sync.rs`).
    #[serde(default)]
    pub modified: i64,
    /// The `modified` the account has. Behind `modified`, a change here is waiting to upload.
    #[serde(default)]
    pub synced: i64,
    /// This device's id for the person's card in Contacts, when they were added from there. Each
    /// device has its own id for the same card, so this stays here and never goes to the account
    /// (the card itself carries a link other devices find them by: see [`crate::contacts`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
}

/// Someone to add in a batch (from Contacts): their name, this device's id for their card, and how
/// often to be in touch with them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewPerson {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
    pub frequency_days: u32,
}

/// The first-run guide, on this device: which steps are behind you. Each is done once, by doing it
/// or by skipping it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Guide {
    /// The sign-in page: signed in, or "Not now".
    pub sign_in: bool,
    /// Adding people from Contacts: added, skipped, or the list already had people.
    pub import: bool,
}

impl Guide {
    /// Every step is behind you.
    pub fn is_done(&self) -> bool {
        self.sign_in && self.import
    }

    /// A file from before the guide existed belongs to someone who has used the app already.
    pub(crate) fn finished() -> Self {
        Guide {
            sign_in: true,
            import: true,
        }
    }
}

impl Person {
    /// The reminder is showing: its day has come and you haven't marked it done.
    /// A missed target stays up too, even before its new reminder day (under weekdays only, a missed
    /// week is pushed on the Saturday, and the person shouldn't vanish for the weekend).
    pub fn is_due(&self, today: NaiveDate) -> bool {
        self.overdue || self.notification_date <= today
    }

    /// The earliest day Done can say you were in touch: the last time, or today when the stored day
    /// is "tomorrow" after a time zone change.
    pub fn earliest_done(&self, today: NaiveDate) -> NaiveDate {
        self.last_connected.min(today)
    }

    /// The day the app suggested, once it has come: the target that went by for someone overdue, or
    /// the target itself from its day on. `None` while the target is still ahead.
    pub fn suggested_day(&self, today: NaiveDate) -> Option<NaiveDate> {
        match self.missed_target {
            Some(missed) if self.overdue => Some(missed),
            _ => (self.next_target <= today).then_some(self.next_target),
        }
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
    /// The id every device knows this row by (the local `id` is this Mac's own number for it).
    #[serde(default)]
    pub uid: String,
    /// When this row last changed, here or on another device, in milliseconds since 1970: when two
    /// devices change the same row, the later change wins (on a tie, see `account_wins` in `sync.rs`).
    #[serde(default)]
    pub modified: i64,
    /// The `modified` the account has. Behind `modified`, a change here is waiting to upload.
    #[serde(default)]
    pub synced: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", from = "StoredSettings")]
pub struct Settings {
    pub day_toggle: DayToggle,
    /// The days picked under "Custom day" (all seven at first). Kept while another setting is on, so
    /// coming back to it brings them back. Never empty.
    pub custom_days: DaySet,
    /// Send a macOS notification when a reminder comes up.
    pub notifications: bool,
}

/// `Settings` as a file holds them, from any version.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredSettings {
    #[serde(default)]
    day_toggle: StoredToggle,
    #[serde(default)]
    custom_days: DaySet,
    #[serde(default = "yes")]
    notifications: bool,
}

impl From<StoredSettings> for Settings {
    fn from(s: StoredSettings) -> Self {
        let (day_toggle, custom_days) = read_day_setting(s.day_toggle, s.custom_days);
        Settings {
            day_toggle,
            custom_days,
            notifications: s.notifications,
        }
    }
}

impl Settings {
    /// The day setting as the scheduling rules use it.
    pub fn rule(&self) -> DayRule {
        DayRule::new(self.day_toggle, self.custom_days)
    }
}

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            day_toggle: DayToggle::WeekendsOnly,
            custom_days: DaySet::ALL,
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
    /// What syncing with the Friendship Reminder account needs to remember.
    #[serde(default)]
    pub sync: crate::sync::SyncState,
    /// The first-run guide on this device (not synced).
    #[serde(default = "Guide::finished")]
    pub guide: Guide,
}

impl Default for Data {
    fn default() -> Self {
        Data {
            version: crate::store::CURRENT_VERSION,
            people: Vec::new(),
            blocked: Vec::new(),
            settings: Settings::default(),
            last_id: 0,
            sync: Default::default(),
            // A new list: the guide starts.
            guide: Guide {
                sign_in: false,
                import: false,
            },
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
pub(crate) fn squeeze(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn clean_name(name: &str) -> Result<String, Invalid> {
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
                    .chain(p.missed_target)
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
        people_ok && blocked_ok && ids_ok && self.uids_ok()
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

    /// A new id, refused at MAX_ID so the saved file stays one the app will load.
    pub(crate) fn next_id(&mut self) -> Result<u64, Invalid> {
        if self.last_id + 1 >= MAX_ID {
            return invalid("Friendship Reminder has run out of ids.");
        }
        self.last_id += 1;
        Ok(self.last_id)
    }

    pub(crate) fn ranges(&self) -> Vec<BlockedRange> {
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

    /// A fresh schedule for one person from their last time in touch, caught up to `today`. Returns
    /// whether its first target was already over (then they're missed).
    pub(crate) fn reschedule(
        person: &mut Person,
        rule: DayRule,
        ranges: &[BlockedRange],
        today: NaiveDate,
    ) -> bool {
        person.apply(compute_schedule(
            person.last_connected,
            person.frequency_days,
            rule,
            ranges,
        ));
        person.overdue = false;
        person.missed_target = None;
        // Behind already (someone added long out of touch, or a Done on a day more than a round
        // ago): caught up to today, and missed. Every device makes the same plan from the same data,
        // whichever way the last time in touch changed.
        Self::catch_up_person(person, rule, ranges, today)
    }

    /// Pushes a target that went by without being done to the next allowed days, again and again
    /// until it is current. A target went by once the whole run of allowed days it starts is over (the
    /// weekend, for a Saturday target under weekends only). Returns whether it moved.
    fn catch_up_person(
        person: &mut Person,
        rule: DayRule,
        ranges: &[BlockedRange],
        today: NaiveDate,
    ) -> bool {
        let mut moved = false;
        let mut pushes = 0;
        // Pushing from a target years back would take one step per missed run; start from the week
        // before today instead, which lands on the same day.
        let week_ago = today.checked_sub_days(Days::new(7)).unwrap_or(today);
        let mut last_missed = None;
        while last_day_of_run(person.next_target, rule) < today && pushes < MAX_PUSHES {
            last_missed = Some(person.next_target);
            let from = person.next_target.max(week_ago);
            person.apply(push_missed_schedule(from, rule, ranges));
            pushes += 1;
            moved = true;
        }
        if moved {
            person.overdue = true;
            // The latest target that went by: the day you'd most likely have been in touch on.
            person.missed_target = last_missed;
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
        let id = self.next_id()?;
        let mut person = Person {
            id,
            name,
            frequency_days,
            last_connected,
            next_target: last_connected,
            notification_date: last_connected,
            overdue: false,
            missed_target: None,
            notified_for: None,
            uid: crate::sync::new_uid(),
            modified: 0,
            synced: 0,
            contact: None,
        };
        Self::reschedule(&mut person, self.settings.rule(), &self.ranges(), today);
        self.people.push(person);
        Ok(id)
    }

    /// Adds several people at once (from Contacts), each with their own frequency, all with the
    /// same last time in touch. Someone whose card is already in the list (or twice in the batch) is left out. When any
    /// name can't be used, nobody is added. Returns the ids of the people added.
    pub fn add_people(
        &mut self,
        people: &[NewPerson],
        last_connected: NaiveDate,
        today: NaiveDate,
    ) -> Result<Vec<u64>, Invalid> {
        if people.is_empty() {
            return invalid("Pick at least one person.");
        }
        for p in people {
            check_frequency(p.frequency_days)?;
        }
        check_last_connected(last_connected, today)?;
        let names = people
            .iter()
            .map(|p| clean_name(&p.name))
            .collect::<Result<Vec<_>, _>>()?;
        // Nobody already in the list: the same card here, or the same name (the picker can be behind,
        // with a person synced down since). Errors after the checks above can't happen short of
        // running out of ids; App::call undoes the whole batch then.
        let mut ids = Vec::new();
        for (p, name) in people.iter().zip(names) {
            let fold = |n: &str| n.to_lowercase();
            let listed = self.people.iter().any(|q| {
                (p.contact.is_some() && q.contact == p.contact) || fold(&q.name) == fold(&name)
            });
            if listed {
                continue;
            }
            let id = self.add_person(&name, p.frequency_days, last_connected, today)?;
            if let Some(person) = self.people.last_mut() {
                person.contact = p.contact.clone();
            }
            ids.push(id);
        }
        Ok(ids)
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
        let (rule, ranges) = (self.settings.rule(), self.ranges());
        let person = self.person_mut(id)?;
        // Only a changed date is checked: a stored one can be "tomorrow" after a time zone change.
        if person.last_connected != last_connected {
            check_last_connected(last_connected, today)?;
        }
        let schedule_changed =
            person.frequency_days != frequency_days || person.last_connected != last_connected;
        person.name = name;
        person.frequency_days = frequency_days;
        person.last_connected = last_connected;
        if schedule_changed {
            Self::reschedule(person, rule, &ranges, today);
        }
        Ok(())
    }

    /// Removes people. Ids that are already gone are skipped. Returns how many were removed.
    pub fn delete_people(&mut self, ids: &[u64]) -> usize {
        let before = self.people.len();
        self.people.retain(|p| !ids.contains(&p.id));
        before - self.people.len()
    }

    /// You were in touch on `on` (today, when not given): their next target counts from that day.
    pub fn mark_done(
        &mut self,
        id: u64,
        on: Option<NaiveDate>,
        today: NaiveDate,
    ) -> Result<(), Invalid> {
        let on = on.unwrap_or(today);
        check_last_connected(on, today)?;
        let (rule, ranges) = (self.settings.rule(), self.ranges());
        let person = self.person_mut(id)?;
        let earliest = person.earliest_done(today);
        if on < earliest {
            return invalid(format!(
                "Pick {} or later: the last time you were in touch.",
                earliest.format("%a, %b %-d, %Y")
            ));
        }
        person.last_connected = on;
        Self::reschedule(person, rule, &ranges, today);
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
        let id = self.next_id()?;
        self.blocked.push(Blocked {
            id,
            start,
            end,
            label,
            uid: crate::sync::new_uid(),
            modified: 0,
            synced: 0,
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

    /// The day setting and, when given, the days picked for "Custom day", as one change: everyone is
    /// rescheduled once. Nothing changes when the days are refused.
    pub fn set_day_setting(
        &mut self,
        toggle: DayToggle,
        custom_days: Option<DaySet>,
        today: NaiveDate,
    ) -> Result<(), Invalid> {
        let days = custom_days.unwrap_or(self.settings.custom_days);
        // The days only count under Custom day; kept as they were otherwise.
        if toggle == DayToggle::Custom && days.is_empty() {
            return invalid("Pick at least one day.");
        }
        let days = if days.is_empty() {
            self.settings.custom_days
        } else {
            days
        };
        let before = self.settings.rule();
        self.settings.day_toggle = toggle;
        self.settings.custom_days = days;
        if self.settings.rule() != before {
            self.recompute_all(today);
        }
        Ok(())
    }

    /// A list with people in it is past the guide's import step (people added, or synced down).
    pub fn settle_guide(&mut self) {
        if !self.people.is_empty() {
            self.guide.import = true;
        }
    }

    pub fn set_notifications(&mut self, on: bool) {
        self.settings.notifications = on;
    }

    /// Everyone's schedule worked out again from their last time in touch, under the current day
    /// setting and blocked ranges (after a blocked range or the day setting changed).
    pub fn recompute_all(&mut self, today: NaiveDate) {
        let (rule, ranges) = (self.settings.rule(), self.ranges());
        for person in &mut self.people {
            // A new plan can't make anyone "missed": a target the old plan never aimed for may already
            // be over (a blocked range removed after the fact). Someone already missed stays missed
            // while their new target is still behind them.
            let was_overdue = person.overdue;
            let behind = Self::reschedule(person, rule, &ranges, today);
            person.overdue = was_overdue && behind;
            // Still missed: the day missed is the new plan's, the same on every device.
            if !person.overdue {
                person.missed_target = None;
            }
        }
    }

    /// Pushes every missed target forward to today or later. Run on launch, on wake and every hour.
    /// Returns whether anything changed.
    pub fn catch_up(&mut self, today: NaiveDate) -> bool {
        let (rule, ranges) = (self.settings.rule(), self.ranges());
        let mut changed = false;
        for person in &mut self.people {
            changed |= Self::catch_up_person(person, rule, &ranges, today);
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
