//! Syncing with the Friendship Reminder account: the rules for what to upload and how to take in what other
//! devices changed. No network here; friends-sync talks to the server.
//!
//! The account holds the real list; each device keeps a copy, so the app works offline. Every person,
//! blocked range and the day setting carry:
//! - a `uid` every device knows them by (the local number `id` is only this Mac's),
//! - `modified`, when they last changed (on any device), and
//! - `synced`, the `modified` the account has. A row whose `modified` is ahead waits to upload.
//!
//! When two devices change the same row, the later change wins. When both changes carry the same time,
//! the account's copy wins (see [`account_wins`]), so every device ends up with the same list. A
//! deleted row leaves a [`Tombstone`] until the account has heard about it, so other devices delete it
//! too instead of bringing it back.
//!
//! Only what the user typed is synced: names, how often, the last time in touch, blocked ranges and
//! the day setting. Each device works out targets and reminder days itself (they depend on today),
//! and keeps its own notification setting and "already notified" marks.

use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::model::{
    clean_name, sane_date, sane_target, squeeze, Blocked, Data, Person, FREQUENCIES, MAX_NAME_CHARS,
};
use crate::schedule::{read_day_setting, DaySet, DayToggle, StoredToggle};

/// A fresh random id for a new row.
pub fn new_uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// What syncing remembers between runs.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    /// The account this copy last synced with. Signing in to another one uploads everything to it.
    #[serde(default)]
    pub owner: Option<String>,
    /// The newest server change already taken in (the server's clock, in ms).
    #[serde(default)]
    pub cursor: i64,
    /// Rows deleted here that the account hasn't heard about yet.
    #[serde(default)]
    pub tombstones: Vec<Tombstone>,
    /// When the day setting last changed, and what the account has (as on rows).
    #[serde(default)]
    pub settings_modified: i64,
    #[serde(default)]
    pub settings_synced: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Person,
    Blocked,
}

/// A row deleted here, kept until the account has the deletion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tombstone {
    pub kind: Kind,
    pub uid: String,
    pub modified: i64,
}

/// A person as the account stores them. A deleted one keeps only its id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemotePerson {
    pub id: String,
    pub name: Option<String>,
    pub frequency_days: Option<u32>,
    pub last_connected: Option<NaiveDate>,
    pub deleted: bool,
    pub modified_at: i64,
    /// The server's time of the last write to this row (only coming down).
    #[serde(default, skip_serializing)]
    pub updated_ms: i64,
}

/// A blocked range as the account stores it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteBlocked {
    pub id: String,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub label: Option<String>,
    pub deleted: bool,
    pub modified_at: i64,
    /// The server's time of the last write to this row (only coming down).
    #[serde(default, skip_serializing)]
    pub updated_ms: i64,
}

/// The synced settings as the account stores them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "StoredRemoteSettings")]
pub struct RemoteSettings {
    pub day_toggle: DayToggle,
    /// The days picked under "Custom day".
    pub custom_days: DaySet,
    pub modified_at: i64,
    /// The server's time of the last write to this row (only coming down).
    #[serde(skip_serializing)]
    pub updated_ms: i64,
}

/// `RemoteSettings` as the account may hold them: "allDays" from an older build, and no days from
/// before custom days (see `read_day_setting`).
#[derive(Deserialize)]
struct StoredRemoteSettings {
    day_toggle: StoredToggle,
    #[serde(default)]
    custom_days: DaySet,
    modified_at: i64,
    #[serde(default)]
    updated_ms: i64,
}

impl From<StoredRemoteSettings> for RemoteSettings {
    fn from(s: StoredRemoteSettings) -> Self {
        let (day_toggle, custom_days) = read_day_setting(s.day_toggle, s.custom_days);
        RemoteSettings {
            day_toggle,
            custom_days,
            modified_at: s.modified_at,
            updated_ms: s.updated_ms,
        }
    }
}

/// Rows going up to the account, or coming down from it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Changes {
    pub people: Vec<RemotePerson>,
    pub blocked: Vec<RemoteBlocked>,
    pub settings: Option<RemoteSettings>,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.people.is_empty() && self.blocked.is_empty() && self.settings.is_none()
    }
}

fn person_fields(p: &Person) -> (&str, u32, NaiveDate) {
    (&p.name, p.frequency_days, p.last_connected)
}

fn blocked_fields(b: &Blocked) -> (NaiveDate, NaiveDate, &str) {
    (b.start, b.end, &b.label)
}

/// A time after both `now` and `previous`, so a change always counts as newer than the one before it
/// on this Mac, even if the clock went back.
fn later(now: i64, previous: i64) -> i64 {
    now.max(previous + 1)
}

/// Whether the account's copy of a row, changed at `remote`, replaces the one here, changed at
/// `modified` and uploaded up to `synced`. A later change wins. On a tie the account's copy wins: a
/// sync uploads before it downloads, so it is the account's newest. Unless the copy here changed after
/// that upload (while the download was on its way): then it is kept, and goes up next. The account
/// keeps the last write on a tie, so it takes it, and the other devices take it from there.
fn account_wins(remote: i64, modified: i64, synced: i64) -> bool {
    remote > modified || (remote == modified && synced >= modified)
}

impl Data {
    /// Gives every row without a `uid` one (rows from before syncing existed).
    pub fn fill_uids(&mut self) {
        for p in self.people.iter_mut().filter(|p| p.uid.is_empty()) {
            p.uid = new_uid();
        }
        for b in self.blocked.iter_mut().filter(|b| b.uid.is_empty()) {
            b.uid = new_uid();
        }
    }

    /// No two rows of a kind share a `uid`.
    pub(crate) fn uids_ok(&self) -> bool {
        let mut seen = HashSet::new();
        let people = self
            .people
            .iter()
            .filter(|p| !p.uid.is_empty())
            .all(|p| seen.insert(p.uid.as_str()));
        let mut seen = HashSet::new();
        let blocked = self
            .blocked
            .iter()
            .filter(|b| !b.uid.is_empty())
            .all(|b| seen.insert(b.uid.as_str()));
        people && blocked
    }

    /// After a change made here (`before` is the data before it): stamps every row the user changed
    /// with `now`, and leaves a tombstone for every row removed. Changes the app works out itself (a
    /// new target, a notification sent) aren't the user's and aren't synced.
    pub fn stamp(&mut self, before: &Data, now: i64) {
        self.fill_uids();
        let old_people: HashMap<&str, &Person> =
            before.people.iter().map(|p| (p.uid.as_str(), p)).collect();
        let old_blocked: HashMap<&str, &Blocked> =
            before.blocked.iter().map(|b| (b.uid.as_str(), b)).collect();
        for p in &mut self.people {
            match old_people.get(p.uid.as_str()) {
                Some(old) if person_fields(old) == person_fields(p) => {}
                old => p.modified = later(now, old.map_or(p.modified, |o| o.modified)),
            }
        }
        for b in &mut self.blocked {
            match old_blocked.get(b.uid.as_str()) {
                Some(old) if blocked_fields(old) == blocked_fields(b) => {}
                old => b.modified = later(now, old.map_or(b.modified, |o| o.modified)),
            }
        }
        if (self.settings.day_toggle, self.settings.custom_days)
            != (before.settings.day_toggle, before.settings.custom_days)
        {
            self.sync.settings_modified = later(now, self.sync.settings_modified);
        }
        // Only a list that has been synced needs to tell the account about deletions: one that never
        // was has nothing there to delete.
        if self.sync.owner.is_none() {
            return;
        }
        let people: HashSet<&str> = self.people.iter().map(|p| p.uid.as_str()).collect();
        let blocked: HashSet<&str> = self.blocked.iter().map(|b| b.uid.as_str()).collect();
        let gone_people = before
            .people
            .iter()
            .filter(|o| !people.contains(o.uid.as_str()))
            .map(|o| (Kind::Person, o.uid.clone(), o.modified));
        let gone_blocked = before
            .blocked
            .iter()
            .filter(|o| !blocked.contains(o.uid.as_str()))
            .map(|o| (Kind::Blocked, o.uid.clone(), o.modified));
        let gone: Vec<_> = gone_people.chain(gone_blocked).collect();
        for (kind, uid, modified) in gone {
            self.sync.tombstones.push(Tombstone {
                kind,
                uid,
                modified: later(now, modified),
            });
        }
    }

    /// Whether syncing with account `owner` would replace this list: it last synced with another
    /// account. Each account's list stays its own, so this one isn't uploaded into the other.
    pub fn belongs_to_another_account(&self, owner: &str) -> bool {
        self.sync.owner.as_deref().is_some_and(|o| o != owner)
    }

    /// Whether anything here waits to upload.
    pub fn has_unsynced(&self) -> bool {
        self.sync.owner.is_some()
            && (!self.sync.tombstones.is_empty()
                || self.sync.settings_modified > self.sync.settings_synced
                || self.people.iter().any(|p| p.modified > p.synced)
                || self.blocked.iter().any(|b| b.modified > b.synced))
    }

    /// Starts a sync with account `owner`: what to upload.
    ///
    /// The first sync of a list uploads all of it, so signing in keeps what was entered before. A list
    /// that last synced with another account is that account's: it is cleared here (the caller keeps a
    /// copy) and this account's list comes down instead. The day setting only goes up if it was ever
    /// changed, so a new device with the default doesn't override the account's.
    pub fn sync_start(&mut self, owner: &str) -> Changes {
        if self.belongs_to_another_account(owner) {
            self.people.clear();
            self.blocked.clear();
            self.settings.day_toggle = DayToggle::default();
            self.settings.custom_days = DaySet::default();
            self.sync.settings_modified = 0;
        }
        if self.sync.owner.as_deref() != Some(owner) {
            self.sync = SyncState {
                owner: Some(owner.to_string()),
                settings_modified: self.sync.settings_modified,
                ..SyncState::default()
            };
            for p in &mut self.people {
                p.modified = p.modified.max(1);
                p.synced = 0;
            }
            for b in &mut self.blocked {
                b.modified = b.modified.max(1);
                b.synced = 0;
            }
        }
        self.fill_uids();
        let mut people: Vec<RemotePerson> = self
            .people
            .iter()
            .filter(|p| p.modified > p.synced)
            .map(|p| RemotePerson {
                id: p.uid.clone(),
                name: Some(p.name.clone()),
                frequency_days: Some(p.frequency_days),
                last_connected: Some(p.last_connected),
                deleted: false,
                modified_at: p.modified,
                updated_ms: 0,
            })
            .collect();
        let mut blocked: Vec<RemoteBlocked> = self
            .blocked
            .iter()
            .filter(|b| b.modified > b.synced)
            .map(|b| RemoteBlocked {
                id: b.uid.clone(),
                starts_on: Some(b.start),
                ends_on: Some(b.end),
                label: Some(b.label.clone()),
                deleted: false,
                modified_at: b.modified,
                updated_ms: 0,
            })
            .collect();
        for t in &self.sync.tombstones {
            match t.kind {
                Kind::Person => people.push(RemotePerson {
                    id: t.uid.clone(),
                    name: None,
                    frequency_days: None,
                    last_connected: None,
                    deleted: true,
                    modified_at: t.modified,
                    updated_ms: 0,
                }),
                Kind::Blocked => blocked.push(RemoteBlocked {
                    id: t.uid.clone(),
                    starts_on: None,
                    ends_on: None,
                    label: None,
                    deleted: true,
                    modified_at: t.modified,
                    updated_ms: 0,
                }),
            }
        }
        let settings =
            (self.sync.settings_modified > self.sync.settings_synced).then_some(RemoteSettings {
                day_toggle: self.settings.day_toggle,
                custom_days: self.settings.custom_days,
                modified_at: self.sync.settings_modified,
                updated_ms: 0,
            });
        Changes {
            people,
            blocked,
            settings,
        }
    }

    /// The account took `sent`. Rows changed again while it was on its way stay waiting.
    pub fn sync_pushed(&mut self, sent: &Changes) {
        for r in &sent.people {
            if let Some(p) = self.people.iter_mut().find(|p| p.uid == r.id) {
                if !r.deleted && p.modified == r.modified_at {
                    p.synced = r.modified_at;
                }
            }
        }
        for r in &sent.blocked {
            if let Some(b) = self.blocked.iter_mut().find(|b| b.uid == r.id) {
                if !r.deleted && b.modified == r.modified_at {
                    b.synced = r.modified_at;
                }
            }
        }
        let sent_deleted = |kind: Kind, uid: &str, modified: i64| match kind {
            Kind::Person => sent
                .people
                .iter()
                .any(|r| r.deleted && r.id == uid && r.modified_at == modified),
            Kind::Blocked => sent
                .blocked
                .iter()
                .any(|r| r.deleted && r.id == uid && r.modified_at == modified),
        };
        self.sync
            .tombstones
            .retain(|t| !sent_deleted(t.kind, &t.uid, t.modified));
        if let Some(s) = &sent.settings {
            if self.sync.settings_modified == s.modified_at {
                self.sync.settings_synced = s.modified_at;
            }
        }
    }

    /// Takes in what the account sent (`got`, changed since the cursor), keeping whichever side changed
    /// last, and moves the cursor to `cursor`. Rows this version can't use (a frequency it doesn't
    /// offer, a date out of range) are skipped, and the cursor stays at the first of them so they come
    /// down again: an update that understands them picks them up. Returns whether the list changed.
    pub fn sync_pulled(&mut self, got: &Changes, cursor: i64, today: NaiveDate) -> bool {
        let before = self.clone();
        let mut schedules_changed = false;
        let mut skipped: Option<i64> = None;
        let mut skip = |ms: i64| skipped = Some(skipped.map_or(ms, |s: i64| s.min(ms)));
        for r in &got.people {
            if self.take_person(r, today).is_err() {
                skip(r.updated_ms);
            }
        }
        for r in &got.blocked {
            match self.take_blocked(r) {
                Ok(moved) => schedules_changed |= moved,
                Err(()) => skip(r.updated_ms),
            }
        }
        if let Some(s) = &got.settings {
            let (modified, synced) = (self.sync.settings_modified, self.sync.settings_synced);
            if account_wins(s.modified_at, modified, synced) {
                let before = self.settings.rule();
                self.settings.day_toggle = s.day_toggle;
                self.settings.custom_days = s.custom_days;
                // Only a change to the days in use moves anyone.
                schedules_changed |= self.settings.rule() != before;
                self.sync.settings_modified = s.modified_at;
                self.sync.settings_synced = s.modified_at;
            }
        }
        if schedules_changed {
            self.blocked.sort_by_key(|b| (b.start, b.end, b.id));
            self.recompute_all(today);
        }
        let changed = *self != before;
        self.sync.cursor = match skipped {
            Some(first) => first.min(cursor),
            None => self.sync.cursor.max(cursor),
        };
        changed
    }

    /// Takes in one person. `Err` when this version can't use the row and it would have mattered.
    fn take_person(&mut self, r: &RemotePerson, today: NaiveDate) -> Result<(), ()> {
        // `None` for a deletion, `Err` for a row this version can't use.
        let fields = if r.deleted {
            Ok(None)
        } else {
            match (&r.name, r.frequency_days, r.last_connected) {
                (Some(name), Some(freq), Some(last))
                    if FREQUENCIES.contains(&freq) && sane_date(last) =>
                {
                    clean_name(name)
                        .map(|name| Some((name, freq, last)))
                        .map_err(|_| ())
                }
                _ => Err(()),
            }
        };
        let rule = self.settings.rule();
        let ranges = self.ranges();
        if let Some(i) = self.people.iter().position(|p| p.uid == r.id) {
            let p = &mut self.people[i];
            if !account_wins(r.modified_at, p.modified, p.synced) {
                return Ok(());
            }
            let Some((name, freq, last)) = fields? else {
                self.people.remove(i);
                return Ok(());
            };
            let reschedule = p.frequency_days != freq || p.last_connected != last;
            p.name = name;
            p.frequency_days = freq;
            p.last_connected = last;
            p.modified = r.modified_at;
            p.synced = r.modified_at;
            if reschedule {
                Data::reschedule(p, rule, &ranges, today);
            }
            return Ok(());
        }
        // Deleted here (not uploaded yet): the deletion stands unless the other device changed the row
        // after it. On a tie it goes up next, as an edit here would (see [`account_wins`]).
        if let Some(t) = self.tombstone(Kind::Person, &r.id) {
            if r.deleted {
                self.drop_tombstone(Kind::Person, &r.id);
            }
            if r.deleted || r.modified_at <= t.modified {
                return Ok(());
            }
        }
        let Some((name, freq, last)) = fields? else {
            return Ok(());
        };
        let id = self.next_id().map_err(|_| ())?;
        self.drop_tombstone(Kind::Person, &r.id);
        let mut person = Person {
            id,
            name,
            frequency_days: freq,
            last_connected: last,
            next_target: last,
            notification_date: last,
            overdue: false,
            missed_target: None,
            notified_for: None,
            uid: r.id.clone(),
            modified: r.modified_at,
            synced: r.modified_at,
            contact: None,
        };
        Data::reschedule(&mut person, rule, &ranges, today);
        self.people.push(person);
        Ok(())
    }

    /// Takes in one blocked range: whether the ranges changed (so everyone's schedule needs working
    /// out again), or `Err` when this version can't use the row and it would have mattered.
    fn take_blocked(&mut self, r: &RemoteBlocked) -> Result<bool, ()> {
        let fields = if r.deleted {
            Ok(None)
        } else {
            match (r.starts_on, r.ends_on, &r.label) {
                (Some(start), Some(end), Some(label))
                    if sane_date(start)
                        && sane_target(end)
                        && start <= end
                        && squeeze(label).chars().count() <= MAX_NAME_CHARS =>
                {
                    Ok(Some((start, end, squeeze(label))))
                }
                _ => Err(()),
            }
        };
        if let Some(i) = self.blocked.iter().position(|b| b.uid == r.id) {
            let b = &mut self.blocked[i];
            if !account_wins(r.modified_at, b.modified, b.synced) {
                return Ok(false);
            }
            let Some((start, end, label)) = fields? else {
                self.blocked.remove(i);
                return Ok(true);
            };
            let moved = b.start != start || b.end != end;
            b.start = start;
            b.end = end;
            b.label = label;
            b.modified = r.modified_at;
            b.synced = r.modified_at;
            return Ok(moved);
        }
        if let Some(t) = self.tombstone(Kind::Blocked, &r.id) {
            if r.deleted {
                self.drop_tombstone(Kind::Blocked, &r.id);
            }
            if r.deleted || r.modified_at <= t.modified {
                return Ok(false);
            }
        }
        let Some((start, end, label)) = fields? else {
            return Ok(false);
        };
        let id = self.next_id().map_err(|_| ())?;
        self.drop_tombstone(Kind::Blocked, &r.id);
        self.blocked.push(Blocked {
            id,
            start,
            end,
            label,
            uid: r.id.clone(),
            modified: r.modified_at,
            synced: r.modified_at,
        });
        Ok(true)
    }

    fn tombstone(&self, kind: Kind, uid: &str) -> Option<Tombstone> {
        self.sync
            .tombstones
            .iter()
            .find(|t| t.kind == kind && t.uid == uid)
            .cloned()
    }

    fn drop_tombstone(&mut self, kind: Kind, uid: &str) {
        self.sync
            .tombstones
            .retain(|t| !(t.kind == kind && t.uid == uid));
    }
}
