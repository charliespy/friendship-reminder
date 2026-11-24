//! The commands the UI calls, by name with JSON arguments, and what they answer.
//!
//! The Tauri shell and the end-to-end test server both call [`App::call`], so the tests exercise the same
//! code the app runs. Every command answers with the whole [`View`], so the UI never has to work out a
//! schedule itself.

use std::io;
use std::path::PathBuf;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::{Blocked, Data, Person, Settings};
use crate::schedule::DayToggle;
use crate::store::{Loaded, Store};
use crate::sync::Changes;

/// What the UI shows: everything, worked out for `today`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub today: NaiveDate,
    /// Soonest reminder first, then by name.
    pub people: Vec<PersonView>,
    pub blocked: Vec<Blocked>,
    pub settings: Settings,
    /// When the data file couldn't be read at launch: where it was moved, so the UI can say so.
    pub unreadable_file: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonView {
    #[serde(flatten)]
    pub person: Person,
    /// The reminder is showing now.
    pub due: bool,
}

/// Why a command failed, with a message fit to show the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    /// The input can't be used (an empty name, a range that ends before it starts).
    Invalid,
    /// The data couldn't be saved.
    Storage,
    /// No such command, or arguments of the wrong shape: a bug in the UI.
    BadCall,
}

impl ApiError {
    fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        ApiError {
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ApiError {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersonArgs {
    name: String,
    frequency_days: u32,
    last_connected: NaiveDate,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdatePersonArgs {
    id: u64,
    name: String,
    frequency_days: u32,
    /// Left out when the form didn't change it, so a "Done" made meanwhile isn't undone.
    #[serde(default)]
    last_connected: Option<NaiveDate>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdArgs {
    id: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdsArgs {
    ids: Vec<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BlockedArgs {
    start: NaiveDate,
    end: NaiveDate,
    #[serde(default)]
    label: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ToggleArgs {
    day_toggle: DayToggle,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OnArgs {
    on: bool,
}

fn args<T: for<'de> Deserialize<'de>>(cmd: &str, value: Value) -> Result<T, ApiError> {
    // A command with no arguments may be called with `null`.
    let value = if value.is_null() {
        Value::Object(Default::default())
    } else {
        value
    };
    serde_json::from_value(value)
        .map_err(|e| ApiError::new(ErrorKind::BadCall, format!("{cmd}: {e}")))
}

/// The app's data and where it is saved.
pub struct App {
    store: Store,
    data: Data,
    unreadable_file: Option<PathBuf>,
    /// The time now, in milliseconds since 1970, to stamp changes with (see [`crate::sync`]).
    clock: fn() -> i64,
}

impl App {
    /// Loads the data from `store` (see [`Store::load`] for what happens to a file it can't read).
    pub fn open(store: Store) -> io::Result<App> {
        let (data, unreadable_file) = match store.load()? {
            Loaded::MovedAside(d, aside) => (d, Some(aside)),
            loaded => (loaded.into_data(), None),
        };
        Ok(App {
            store,
            data,
            unreadable_file,
            clock: crate::now_ms,
        })
    }

    /// Uses `clock` for the time changes are stamped with, for tests.
    pub fn set_clock(&mut self, clock: fn() -> i64) {
        self.clock = clock;
    }

    pub fn data(&self) -> &Data {
        &self.data
    }

    pub fn view(&self, today: NaiveDate) -> View {
        let mut people: Vec<PersonView> = self
            .data
            .people
            .iter()
            .map(|p| PersonView {
                person: p.clone(),
                due: p.is_due(today),
            })
            .collect();
        people.sort_by_cached_key(|p| {
            (
                p.person.notification_date,
                p.person.name.to_lowercase(),
                p.person.id,
            )
        });
        View {
            today,
            people,
            blocked: self.data.blocked.clone(),
            settings: self.data.settings.clone(),
            unreadable_file: self.unreadable_file.as_ref().map(|p| {
                // Shown in the app, where it may end up in a screenshot for a bug report.
                crate::hide_home(&p.display().to_string(), crate::home_dir().as_deref())
            }),
        }
    }

    /// Runs one command and saves. If the change can't be saved, it is undone, so what the app shows
    /// always matches the file.
    pub fn call(&mut self, cmd: &str, a: Value, today: NaiveDate) -> Result<View, ApiError> {
        // Catch up first, so the view is right even before the regular check has run today. Catching up
        // can always be worked out again, so if saving it fails the app keeps going and the next save
        // writes it.
        if self.data.catch_up(today) {
            let _ = self.store.save(&self.data);
        }
        let before = self.data.clone();
        let result = self.apply(cmd, a, today);
        if let Err(e) = result {
            self.data = before;
            return Err(e);
        }
        self.data.stamp(&before, (self.clock)());
        if self.data != before {
            if let Err(e) = self.store.save(&self.data) {
                self.data = before;
                return Err(ApiError::new(
                    ErrorKind::Storage,
                    format!("Couldn't save your changes: {e}"),
                ));
            }
        }
        Ok(self.view(today))
    }

    fn apply(&mut self, cmd: &str, a: Value, today: NaiveDate) -> Result<(), ApiError> {
        let invalid = |e: crate::model::Invalid| ApiError::new(ErrorKind::Invalid, e.0);
        let d = &mut self.data;
        match cmd {
            "get_view" => {
                args::<serde_json::Map<String, Value>>(cmd, a)?;
            }
            "add_person" => {
                let p: PersonArgs = args(cmd, a)?;
                d.add_person(&p.name, p.frequency_days, p.last_connected, today)
                    .map_err(invalid)?;
            }
            "update_person" => {
                let p: UpdatePersonArgs = args(cmd, a)?;
                let last_connected = match p.last_connected {
                    Some(day) => day,
                    // An unknown id falls through to update_person, which reports it.
                    None => d.person(p.id).map_or(today, |x| x.last_connected),
                };
                d.update_person(p.id, &p.name, p.frequency_days, last_connected, today)
                    .map_err(invalid)?;
            }
            "delete_people" => {
                let p: IdsArgs = args(cmd, a)?;
                d.delete_people(&p.ids);
            }
            "mark_done" => {
                let p: IdArgs = args(cmd, a)?;
                d.mark_done(p.id, today).map_err(invalid)?;
            }
            "add_blocked" => {
                let p: BlockedArgs = args(cmd, a)?;
                d.add_blocked(p.start, p.end, &p.label, today)
                    .map_err(invalid)?;
            }
            "delete_blocked" => {
                let p: IdArgs = args(cmd, a)?;
                d.delete_blocked(p.id, today);
            }
            "set_day_toggle" => {
                let p: ToggleArgs = args(cmd, a)?;
                d.set_day_toggle(p.day_toggle, today);
            }
            "set_notifications" => {
                let p: OnArgs = args(cmd, a)?;
                d.set_notifications(p.on);
            }
            _ => {
                return Err(ApiError::new(
                    ErrorKind::BadCall,
                    format!("unknown command: {cmd}"),
                ))
            }
        }
        Ok(())
    }

    /// Starts a sync with account `owner`: what to upload (see [`Data::sync_start`]). Saved, since a
    /// new account resets what was synced.
    /// A list that last synced with another account is that account's: it is saved to a copy next to
    /// the data file (`friends.json.other-account-<n>`) before it is cleared, so nothing is lost.
    pub fn sync_start(&mut self, owner: &str) -> io::Result<Changes> {
        if self.data.belongs_to_another_account(owner) {
            self.store.save_copy(&self.data, "other-account")?;
        }
        let before = self.data.clone();
        let changes = self.data.sync_start(owner);
        self.save_or_undo(before)?;
        Ok(changes)
    }

    /// The account took `sent` (see [`Data::sync_pushed`]).
    pub fn sync_pushed(&mut self, sent: &Changes) -> io::Result<()> {
        let before = self.data.clone();
        self.data.sync_pushed(sent);
        self.save_or_undo(before)
    }

    /// Whether a change here waits to upload.
    pub fn has_unsynced(&self) -> bool {
        self.data.has_unsynced()
    }

    /// Where the next download starts: the newest server change already taken in.
    pub fn sync_cursor(&self) -> i64 {
        self.data.sync.cursor
    }

    /// Takes in what the account sent, up to `cursor` (see [`Data::sync_pulled`]). Returns whether
    /// anything here changed, so the windows need the new view.
    pub fn sync_pulled(
        &mut self,
        got: &Changes,
        cursor: i64,
        today: NaiveDate,
    ) -> io::Result<bool> {
        let before = self.data.clone();
        let caught_up = self.data.catch_up(today);
        let changed = self.data.sync_pulled(got, cursor, today) || caught_up;
        self.save_or_undo(before)?;
        Ok(changed)
    }

    fn save_or_undo(&mut self, before: Data) -> io::Result<()> {
        if self.data != before {
            if let Err(e) = self.store.save(&self.data) {
                self.data = before;
                return Err(e);
            }
        }
        Ok(())
    }

    /// The regular check (on launch, on wake and every hour): push missed targets forward, and return
    /// the people to send a notification for now (none when notifications are off). Saves when
    /// anything changed.
    pub fn tick(&mut self, today: NaiveDate) -> io::Result<Vec<Person>> {
        let before = self.data.clone();
        self.data.catch_up(today);
        let due = self.data.take_notifications(today);
        if self.data != before {
            if let Err(e) = self.store.save(&self.data) {
                self.data = before;
                return Err(e);
            }
        }
        Ok(if self.data.settings.notifications {
            due
        } else {
            Vec::new()
        })
    }
}
