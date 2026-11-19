//! Saving and loading the data as one JSON file.
//!
//! Writes go to a temporary file next to the real one and are then renamed over it, so a crash mid-write
//! never leaves half a file. A file that can't be read is never overwritten: it is moved aside to
//! `<name>.unreadable-<n>` first, so nothing the user entered is lost.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::model::Data;

/// The newest file format this version of the app understands.
pub const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct Store {
    path: PathBuf,
}

/// What loading found.
#[derive(Debug, PartialEq, Eq)]
pub enum Loaded {
    /// The file was there and read fine.
    Existing(Data),
    /// No file yet: a fresh start.
    New(Data),
    /// The file couldn't be read; it was moved to this path and the app starts fresh.
    MovedAside(Data, PathBuf),
}

impl Loaded {
    pub fn into_data(self) -> Data {
        match self {
            Loaded::Existing(d) | Loaded::New(d) | Loaded::MovedAside(d, _) => d,
        }
    }
}

impl Store {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Store { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> io::Result<Loaded> {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(Loaded::New(Data::default()))
            }
            Err(e) if e.kind() == io::ErrorKind::InvalidData => return self.move_aside(),
            Err(e) => return Err(e),
        };
        // Check the version before anything else: a file from a newer version of the app may hold things
        // this one can't read or would drop on save, so it is refused and left as it is.
        #[derive(serde::Deserialize)]
        struct Version {
            version: u32,
        }
        if let Ok(Version { version }) = serde_json::from_str::<Version>(&text) {
            if version > CURRENT_VERSION {
                return Err(io::Error::other(format!(
                    "{} was saved by a newer version of Friends (format {version}). Update the app to open it.",
                    self.path.display()
                )));
            }
        }
        match serde_json::from_str::<Data>(&text) {
            Ok(mut data) if data.is_sane() => {
                data.repair_ids();
                // Saved from now on in this version's format.
                data.version = CURRENT_VERSION;
                Ok(Loaded::Existing(data))
            }
            // Not JSON, the wrong shape, or values the app could never have saved.
            Ok(_) | Err(_) => self.move_aside(),
        }
    }

    fn move_aside(&self) -> io::Result<Loaded> {
        let name = self
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("friends.json");
        let mut n = 1;
        let aside = loop {
            let candidate = self.path.with_file_name(format!("{name}.unreadable-{n}"));
            if !candidate.exists() {
                break candidate;
            }
            n += 1;
        };
        fs::rename(&self.path, &aside)?;
        Ok(Loaded::MovedAside(Data::default(), aside))
    }

    pub fn save(&self, data: &Data) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(data).map_err(io::Error::other)?;
        // One temporary name per process, so two processes saving at once can't write into the same
        // temporary file. (The last save still wins: the app allows one copy of itself, and the test server
        // uses its own file.)
        let tmp = self
            .path
            .with_extension(format!("json.tmp-{}", std::process::id()));
        let written = (|| {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(&json)?;
            f.write_all(b"\n")?;
            f.sync_all()?;
            fs::rename(&tmp, &self.path)
        })();
        if written.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        written
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("friends-store-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_missing_file_is_a_fresh_start() {
        let store = Store::new(temp_dir("missing").join("friends.json"));
        assert_eq!(store.load().unwrap(), Loaded::New(Data::default()));
    }

    #[test]
    fn saved_data_loads_back() {
        let dir = temp_dir("roundtrip");
        let store = Store::new(dir.join("sub").join("friends.json"));
        let mut data = Data::default();
        let today = NaiveDate::from_ymd_opt(2026, 8, 19).unwrap();
        data.add_person("Ada", 30, today, today).unwrap();
        store.save(&data).unwrap();
        assert_eq!(store.load().unwrap(), Loaded::Existing(data));
        assert_eq!(fs::read_dir(dir.join("sub")).unwrap().count(), 1); // no temporary file left
    }

    #[test]
    fn an_unreadable_file_is_moved_aside_not_overwritten() {
        let dir = temp_dir("corrupt");
        let path = dir.join("friends.json");
        fs::write(&path, "{ not json").unwrap();
        fs::write(dir.join("friends.json.unreadable-1"), "older").unwrap();
        let store = Store::new(&path);
        match store.load().unwrap() {
            Loaded::MovedAside(data, aside) => {
                assert_eq!(data, Data::default());
                assert_eq!(aside, dir.join("friends.json.unreadable-2"));
                assert_eq!(fs::read_to_string(&aside).unwrap(), "{ not json");
                assert_eq!(
                    fs::read_to_string(dir.join("friends.json.unreadable-1")).unwrap(),
                    "older"
                );
                assert!(!path.exists());
            }
            other => panic!("expected MovedAside, got {other:?}"),
        }
    }

    #[test]
    fn a_file_with_impossible_values_is_moved_aside() {
        let dir = temp_dir("insane");
        let path = dir.join("friends.json");
        let text = r#"{"version":1,"people":[{"id":1,"name":"A","frequencyDays":7,"lastConnected":"2026-01-01","nextTarget":"2026-01-08","notificationDate":"2026-01-08"}]}"#;
        fs::write(&path, text).unwrap();
        assert!(matches!(
            Store::new(&path).load().unwrap(),
            Loaded::MovedAside(..)
        ));
    }

    #[test]
    fn repeated_ids_make_a_file_unreadable_and_a_stale_last_id_is_repaired() {
        let dir = temp_dir("ids");
        let path = dir.join("friends.json");
        let person = |id: u64| {
            format!(
                r#"{{"id":{id},"name":"A","frequencyDays":30,"lastConnected":"2026-01-01","nextTarget":"2026-01-31","notificationDate":"2026-01-26"}}"#
            )
        };
        fs::write(
            &path,
            format!(r#"{{"version":1,"people":[{},{}]}}"#, person(3), person(3)),
        )
        .unwrap();
        assert!(matches!(
            Store::new(&path).load().unwrap(),
            Loaded::MovedAside(..)
        ));
        fs::write(
            &path,
            format!(r#"{{"version":1,"lastId":1,"people":[{}]}}"#, person(7)),
        )
        .unwrap();
        assert_eq!(Store::new(&path).load().unwrap().into_data().last_id, 7);
    }

    #[test]
    fn a_file_from_a_newer_version_is_left_alone() {
        let dir = temp_dir("newer");
        let path = dir.join("friends.json");
        for text in [
            r#"{"version": 99}"#,
            r#"{"version":2,"settings":{"dayToggle":"custom"}}"#,
        ] {
            fs::write(&path, text).unwrap();
            assert!(Store::new(&path).load().is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), text);
        }
    }
}
