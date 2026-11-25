//! The sync rules: what goes up to the account, and how changes from other devices come in.

use chrono::NaiveDate;
use friends_core::api::App;
use friends_core::model::Data;
use friends_core::schedule::{DaySet, DayToggle};
use friends_core::store::Store;
use friends_core::sync::{Changes, RemoteBlocked, RemotePerson, RemoteSettings};
use serde_json::json;

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn today() -> NaiveDate {
    day(2026, 8, 19)
}

const OWNER: &str = "11111111-1111-1111-1111-111111111111";

/// Runs a change the way the app does: the change, then the stamp at `now`.
fn change(data: &mut Data, now: i64, f: impl FnOnce(&mut Data)) {
    let before = data.clone();
    f(data);
    data.stamp(&before, now);
}

fn remote_person(id: &str, name: &str, modified_at: i64) -> RemotePerson {
    RemotePerson {
        id: id.to_string(),
        name: Some(name.to_string()),
        frequency_days: Some(30),
        last_connected: Some(day(2026, 8, 1)),
        deleted: false,
        modified_at,
        updated_ms: 0,
    }
}

fn deleted_person(id: &str, modified_at: i64) -> RemotePerson {
    RemotePerson {
        id: id.to_string(),
        name: None,
        frequency_days: None,
        last_connected: None,
        deleted: true,
        modified_at,
        updated_ms: 0,
    }
}

fn people(changes: Vec<RemotePerson>) -> Changes {
    Changes {
        people: changes,
        ..Changes::default()
    }
}

/// A list with Ada, synced once with OWNER.
fn synced_with_ada() -> (Data, String) {
    let mut data = Data::default();
    change(&mut data, 100, |d| {
        d.add_person("Ada", 30, day(2026, 8, 1), today()).unwrap();
    });
    let sent = data.sync_start(OWNER);
    data.sync_pushed(&sent);
    let uid = data.people[0].uid.clone();
    (data, uid)
}

#[test]
fn a_first_sync_uploads_everything_once() {
    let mut data = Data::default();
    change(&mut data, 100, |d| {
        d.add_person("Ada", 90, day(2026, 8, 1), today()).unwrap();
        d.add_blocked(day(2026, 9, 1), day(2026, 9, 7), "Trip", today())
            .unwrap();
        d.delete_people(&[]);
    });
    let sent = data.sync_start(OWNER);
    assert_eq!(sent.people.len(), 1);
    let ada = &sent.people[0];
    assert_eq!(ada.name.as_deref(), Some("Ada"));
    assert_eq!(ada.frequency_days, Some(90));
    assert_eq!(ada.last_connected, Some(day(2026, 8, 1)));
    assert_eq!((ada.deleted, ada.modified_at), (false, 100));
    assert_eq!(sent.blocked.len(), 1);
    assert_eq!(sent.blocked[0].label.as_deref(), Some("Trip"));
    // A day setting never changed stays here, so it can't override one chosen on another device.
    assert_eq!(sent.settings, None);
    data.sync_pushed(&sent);
    assert!(data.sync_start(OWNER).is_empty());
    assert!(!data.has_unsynced());
    // Once chosen, it goes up.
    change(&mut data, 200, |d| {
        d.set_day_setting(DayToggle::Custom, None, today()).unwrap()
    });
    assert!(data.has_unsynced());
    let sent = data.sync_start(OWNER);
    assert_eq!(
        sent.settings.map(|s| (s.day_toggle, s.modified_at)),
        Some((DayToggle::Custom, 200))
    );
}

#[test]
fn a_new_device_takes_the_accounts_day_setting() {
    let mut data = Data::default();
    let sent = data.sync_start(OWNER);
    assert_eq!(sent.settings, None);
    let got = Changes {
        settings: Some(RemoteSettings {
            day_toggle: DayToggle::WeekdaysOnly,
            custom_days: DaySet::ALL,
            modified_at: 1,
            updated_ms: 0,
        }),
        ..Changes::default()
    };
    data.sync_pulled(&got, 1, today());
    assert_eq!(data.settings.day_toggle, DayToggle::WeekdaysOnly);
}

#[test]
fn rows_this_version_cant_use_come_down_again() {
    let mut data = Data::default();
    let sent = data.sync_start(OWNER);
    data.sync_pushed(&sent);
    let mut sixty = remote_person("a", "A", 1);
    sixty.frequency_days = Some(60);
    sixty.updated_ms = 5_000;
    let mut fine = remote_person("b", "B", 1);
    fine.updated_ms = 9_000;
    data.sync_pulled(&people(vec![sixty.clone(), fine]), 9_000, today());
    assert_eq!(data.people.len(), 1);
    // The cursor stays at the row it skipped, so the next download includes it again.
    assert_eq!(data.sync.cursor, 5_000);
    // A row it can't use but that lost to a newer change here anyway doesn't hold the cursor.
    let (mut data, uid) = synced_with_ada();
    let mut stale = remote_person(&uid, "A", 50);
    stale.frequency_days = Some(60);
    data.sync_pulled(&people(vec![stale]), 9_000, today());
    assert_eq!(data.sync.cursor, 9_000);
}

#[test]
fn only_what_the_user_typed_counts_as_a_change() {
    let (mut data, _) = synced_with_ada();
    // The app working out a new schedule, or sending a notification, isn't a change to sync.
    change(&mut data, 200, |d| {
        d.catch_up(day(2027, 1, 1));
        d.take_notifications(day(2027, 1, 1));
        d.set_notifications(false);
    });
    assert!(data.sync_start(OWNER).is_empty());
    let id = data.people[0].id;
    change(&mut data, 300, |d| d.mark_done(id, None, today()).unwrap());
    let sent = data.sync_start(OWNER);
    assert_eq!(sent.people.len(), 1);
    assert_eq!(sent.people[0].modified_at, 300);
}

#[test]
fn a_change_made_while_uploading_waits_for_the_next_upload() {
    let (mut data, _) = synced_with_ada();
    let id = data.people[0].id;
    change(&mut data, 200, |d| {
        d.update_person(id, "Ada L", 30, today(), today()).unwrap()
    });
    let sent = data.sync_start(OWNER);
    change(&mut data, 300, |d| {
        d.update_person(id, "Ada Lovelace", 30, today(), today())
            .unwrap()
    });
    data.sync_pushed(&sent);
    let again = data.sync_start(OWNER);
    assert_eq!(again.people[0].name.as_deref(), Some("Ada Lovelace"));
}

#[test]
fn a_change_is_always_newer_than_the_last_even_if_the_clock_went_back() {
    let (mut data, _) = synced_with_ada();
    let id = data.people[0].id;
    change(&mut data, 50, |d| {
        d.update_person(id, "Ada L", 30, today(), today()).unwrap()
    });
    assert_eq!(data.people[0].modified, 101);
}

#[test]
fn deletions_go_up_once_the_list_has_synced() {
    let mut data = Data::default();
    change(&mut data, 100, |d| {
        let id = d.add_person("Ada", 30, today(), today()).unwrap();
        d.delete_people(&[id]);
    });
    // Never synced: nothing on the account to delete.
    assert!(data.sync.tombstones.is_empty());

    let (mut data, uid) = synced_with_ada();
    let id = data.people[0].id;
    change(&mut data, 200, |d| {
        d.delete_people(&[id]);
    });
    let sent = data.sync_start(OWNER);
    assert_eq!(sent.people, vec![deleted_person(&uid, 200)]);
    data.sync_pushed(&sent);
    assert!(data.sync.tombstones.is_empty());
    assert!(data.sync_start(OWNER).is_empty());
}

#[test]
fn people_from_other_devices_come_in_with_a_schedule() {
    let mut data = Data::default();
    let sent = data.sync_start(OWNER);
    data.sync_pushed(&sent);
    let got = people(vec![remote_person("aaaa", "Grace", 500)]);
    assert!(data.sync_pulled(&got, 900, today()));
    let grace = &data.people[0];
    assert_eq!(grace.name, "Grace");
    assert_eq!(grace.uid, "aaaa");
    // A month after the last time in touch, moved to the weekend.
    assert_eq!(grace.next_target, day(2026, 9, 5));
    assert!(grace.id > 0);
    assert_eq!(data.sync.cursor, 900);
    // Already synced: nothing to send back.
    assert!(data.sync_start(OWNER).is_empty());
    // Taking the same change in again does nothing.
    assert!(!data.sync_pulled(&got, 900, today()));
}

#[test]
fn the_later_change_wins() {
    let (mut data, uid) = synced_with_ada();
    // An older change from another device loses.
    data.sync_pulled(&people(vec![remote_person(&uid, "Old", 50)]), 1, today());
    assert_eq!(data.people[0].name, "Ada");
    // A newer one wins, and the schedule follows its date.
    let mut newer = remote_person(&uid, "Ada L", 500);
    newer.last_connected = Some(day(2026, 8, 10));
    data.sync_pulled(&people(vec![newer]), 2, today());
    assert_eq!(data.people[0].last_connected, day(2026, 8, 10));
    assert_eq!(data.people[0].next_target, day(2026, 9, 12));
    // A change here after it wins over it: it stays waiting to upload.
    let id = data.people[0].id;
    change(&mut data, 600, |d| {
        d.update_person(id, "Ada Lovelace", 30, today(), today())
            .unwrap()
    });
    data.sync_pulled(&people(vec![remote_person(&uid, "Ada L", 500)]), 3, today());
    assert_eq!(data.people[0].name, "Ada Lovelace");
    assert_eq!(data.sync_start(OWNER).people.len(), 1);
}

#[test]
fn deletions_from_other_devices_and_from_here() {
    // Deleted on another device after the last change here: gone here too.
    let (mut data, uid) = synced_with_ada();
    data.sync_pulled(&people(vec![deleted_person(&uid, 500)]), 1, today());
    assert!(data.people.is_empty());
    assert!(data.sync.tombstones.is_empty());

    // Changed here after the other device deleted them: kept, and uploaded again.
    let (mut data, uid) = synced_with_ada();
    let id = data.people[0].id;
    change(&mut data, 600, |d| d.mark_done(id, None, today()).unwrap());
    data.sync_pulled(&people(vec![deleted_person(&uid, 500)]), 1, today());
    assert_eq!(data.people.len(), 1);

    // Deleted here, then an older change arrives: stays deleted.
    let (mut data, uid) = synced_with_ada();
    let id = data.people[0].id;
    change(&mut data, 600, |d| {
        d.delete_people(&[id]);
    });
    data.sync_pulled(&people(vec![remote_person(&uid, "Ada", 500)]), 1, today());
    assert!(data.people.is_empty());
    assert_eq!(data.sync.tombstones.len(), 1);

    // Deleted here, then a newer change arrives: back, and the deletion is dropped.
    data.sync_pulled(&people(vec![remote_person(&uid, "Ada L", 700)]), 2, today());
    assert_eq!(data.people[0].name, "Ada L");
    assert!(data.sync.tombstones.is_empty());
}

#[test]
fn rows_that_make_no_sense_are_skipped() {
    let mut data = Data::default();
    data.sync_start(OWNER);
    let mut odd_frequency = remote_person("a", "A", 1);
    odd_frequency.frequency_days = Some(7);
    let mut blank = remote_person("b", "   ", 1);
    blank.frequency_days = Some(30);
    let mut ancient = remote_person("c", "C", 1);
    ancient.last_connected = Some(day(1200, 1, 1));
    let mut missing = remote_person("d", "D", 1);
    missing.name = None;
    let backwards = RemoteBlocked {
        id: "e".into(),
        starts_on: Some(day(2026, 9, 7)),
        ends_on: Some(day(2026, 9, 1)),
        label: Some(String::new()),
        deleted: false,
        modified_at: 1,
        updated_ms: 0,
    };
    let got = Changes {
        people: vec![odd_frequency, blank, ancient, missing],
        blocked: vec![backwards],
        settings: None,
    };
    data.sync_pulled(&got, 5, today());
    assert!(data.people.is_empty() && data.blocked.is_empty());
    assert!(data.is_sane());
}

#[test]
fn blocked_ranges_and_the_day_setting_from_other_devices_move_targets() {
    let (mut data, _) = synced_with_ada();
    let target = data.people[0].next_target; // a Saturday a month out
    let got = Changes {
        people: vec![],
        blocked: vec![RemoteBlocked {
            id: "bbbb".into(),
            starts_on: Some(target),
            ends_on: Some(target),
            label: Some("Trip".into()),
            deleted: false,
            modified_at: 500,
            updated_ms: 0,
        }],
        settings: None,
    };
    data.sync_pulled(&got, 1, today());
    assert_eq!(data.blocked.len(), 1);
    assert!(data.people[0].next_target > target);
    let all_days = Changes {
        settings: Some(RemoteSettings {
            day_toggle: DayToggle::Custom,
            custom_days: DaySet::ALL,
            modified_at: 500,
            updated_ms: 0,
        }),
        ..Changes::default()
    };
    data.sync_pulled(&all_days, 1, today());
    assert_eq!(data.settings.day_toggle, DayToggle::Custom);
    assert_eq!(data.people[0].next_target, day(2026, 8, 31));
    // An older day setting from another device loses to one changed here later.
    change(&mut data, 900, |d| {
        d.set_day_setting(DayToggle::WeekdaysOnly, None, today())
            .unwrap()
    });
    let older = Changes {
        settings: Some(RemoteSettings {
            day_toggle: DayToggle::Custom,
            custom_days: DaySet::ALL,
            modified_at: 800,
            updated_ms: 0,
        }),
        ..Changes::default()
    };
    data.sync_pulled(&older, 2, today());
    assert_eq!(data.settings.day_toggle, DayToggle::WeekdaysOnly);
}

#[test]
fn another_accounts_list_isnt_uploaded_into_this_one() {
    let (mut data, _) = synced_with_ada();
    change(&mut data, 200, |d| {
        d.set_day_setting(DayToggle::Custom, None, today()).unwrap()
    });
    let id = data.people[0].id;
    change(&mut data, 300, |d| {
        d.delete_people(&[id]);
        d.add_person("Grace", 30, today(), today()).unwrap();
    });
    data.sync_pulled(&Changes::default(), 1000, today());
    let other = "22222222-2222-2222-2222-222222222222";
    assert!(data.belongs_to_another_account(other));
    // The list here is the first account's: it is cleared, and nothing of it goes up.
    let sent = data.sync_start(other);
    assert!(sent.is_empty());
    assert!(data.people.is_empty());
    assert_eq!(data.settings.day_toggle, DayToggle::default());
    assert_eq!(data.sync.cursor, 0);
    assert!(data.sync.tombstones.is_empty());
    // The other account's list comes down instead.
    data.sync_pulled(&people(vec![remote_person("cccc", "Alan", 5)]), 7, today());
    assert_eq!(data.people[0].name, "Alan");
}

#[test]
fn the_first_sign_in_keeps_the_list_entered_before() {
    let mut data = Data::default();
    change(&mut data, 100, |d| {
        d.add_person("Ada", 30, today(), today()).unwrap();
    });
    assert!(!data.belongs_to_another_account(OWNER));
    // Signed out, nothing waits to upload: there's no account yet.
    assert!(!data.has_unsynced());
    assert_eq!(data.sync_start(OWNER).people.len(), 1);
}

#[test]
fn the_app_stamps_changes_and_upgrades_old_files() {
    let dir = std::env::temp_dir().join(format!("friends-sync-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("friends.json");
    std::fs::write(
        &path,
        r#"{"version":1,"lastId":1,"settings":{"dayToggle":"allDays"},"people":[{"id":1,"name":"Ada","frequencyDays":30,"lastConnected":"2026-08-01","nextTarget":"2026-08-31","notificationDate":"2026-08-31"}]}"#,
    )
    .unwrap();
    let mut app = App::open(Store::new(&path)).unwrap();
    assert!(!app.data().people[0].uid.is_empty());
    // A day setting chosen before syncing existed counts as chosen, so it goes up.
    assert!(app.data().sync.settings_modified > 0);
    app.set_clock(|| 4242);
    app.call(
        "add_person",
        json!({"name": "Grace", "frequencyDays": 30, "lastConnected": "2026-08-19"}),
        today(),
    )
    .unwrap();
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved["version"], 3);
    assert_eq!(saved["settings"]["dayToggle"], "custom");
    assert_eq!(saved["people"][1]["modified"], 4242);
    let sent = app.sync_start(OWNER).unwrap();
    assert_eq!(sent.people.len(), 2);
    app.sync_pushed(&sent).unwrap();
    let got = people(vec![remote_person("cccc", "Alan", 5000)]);
    assert!(app.sync_pulled(&got, 77, today()).unwrap());
    assert_eq!(app.sync_cursor(), 77);
    let reopened = App::open(Store::new(&path)).unwrap();
    assert_eq!(reopened.data(), app.data());

    // Another account: the list goes to a copy first, then is cleared.
    app.sync_start("22222222-2222-2222-2222-222222222222")
        .unwrap();
    assert!(app.data().people.is_empty());
    let copy = App::open(Store::new(dir.join("friends.json.other-account-1"))).unwrap();
    assert_eq!(copy.data().people.len(), 3);
}

/// A list as every device should agree on it: each person's and range's id and name, and the day
/// setting.
type List = (
    Vec<(String, String)>,
    Vec<(String, String)>,
    Option<DayToggle>,
);

/// A small stand-in for the account: like `friends_stamp()` in the Supabase migration, it keeps a
/// write unless the row it has is newer (so on a tie, the last write wins), and stamps each write
/// with its own clock so devices can ask for what changed since their cursor.
#[derive(Default)]
struct Account {
    clock: i64,
    people: Vec<RemotePerson>,
    blocked: Vec<RemoteBlocked>,
    settings: Option<RemoteSettings>,
}

fn write_row<T: Clone>(
    rows: &mut Vec<T>,
    clock: &mut i64,
    row: &T,
    key: impl Fn(&T) -> (String, i64),
    stamp: impl Fn(&mut T, i64),
) {
    let (id, modified_at) = key(row);
    let at = rows.iter().position(|r| key(r).0 == id);
    if at.is_some_and(|i| modified_at < key(&rows[i]).1) {
        return;
    }
    *clock += 1;
    let mut row = row.clone();
    stamp(&mut row, *clock);
    match at {
        Some(i) => rows[i] = row,
        None => rows.push(row),
    }
}

impl Account {
    /// One sync of `data`, the way friends-sync runs it: upload, then download what changed.
    fn sync(&mut self, data: &mut Data) {
        self.upload(data);
        self.download(data);
    }

    fn upload(&mut self, data: &mut Data) {
        let sent = data.sync_start(OWNER);
        for r in &sent.people {
            let key = |r: &RemotePerson| (r.id.clone(), r.modified_at);
            write_row(&mut self.people, &mut self.clock, r, key, |r, ms| {
                r.updated_ms = ms
            });
        }
        for r in &sent.blocked {
            let key = |r: &RemoteBlocked| (r.id.clone(), r.modified_at);
            write_row(&mut self.blocked, &mut self.clock, r, key, |r, ms| {
                r.updated_ms = ms
            });
        }
        if let Some(s) = &sent.settings {
            if self
                .settings
                .as_ref()
                .is_none_or(|o| s.modified_at >= o.modified_at)
            {
                self.clock += 1;
                self.settings = Some(RemoteSettings {
                    updated_ms: self.clock,
                    ..s.clone()
                });
            }
        }
        data.sync_pushed(&sent);
    }

    fn download(&mut self, data: &mut Data) {
        let since = data.sync.cursor;
        let got = Changes {
            people: self
                .people
                .iter()
                .filter(|r| r.updated_ms > since)
                .cloned()
                .collect(),
            blocked: self
                .blocked
                .iter()
                .filter(|r| r.updated_ms > since)
                .cloned()
                .collect(),
            settings: self.settings.clone().filter(|s| s.updated_ms > since),
        };
        data.sync_pulled(&got, self.clock, today());
    }

    /// What every device should end up with: the live rows' fields, and the day setting.
    fn list(&self) -> List {
        let mut people: Vec<_> = (self.people.iter())
            .filter(|r| !r.deleted)
            .map(|r| (r.id.clone(), r.name.clone().unwrap()))
            .collect();
        let mut blocked: Vec<_> = (self.blocked.iter())
            .filter(|r| !r.deleted)
            .map(|r| (r.id.clone(), r.label.clone().unwrap()))
            .collect();
        people.sort();
        blocked.sort();
        (
            people,
            blocked,
            self.settings.as_ref().map(|s| s.day_toggle),
        )
    }
}

fn list(data: &Data) -> List {
    let mut people: Vec<_> = (data.people.iter())
        .map(|p| (p.uid.clone(), p.name.clone()))
        .collect();
    let mut blocked: Vec<_> = (data.blocked.iter())
        .map(|b| (b.uid.clone(), b.label.clone()))
        .collect();
    people.sort();
    blocked.sort();
    (people, blocked, Some(data.settings.day_toggle))
}

/// Device A adds Bo, a trip and a day setting at 1000; all three devices sync. Then B and C, whose
/// clocks read 500, each change the same row: both changes are stamped 1001, a tie.
fn three_devices() -> (Account, [Data; 3]) {
    let mut account = Account::default();
    let mut devices: [Data; 3] = Default::default();
    change(&mut devices[0], 1000, |d| {
        d.add_person("Bo", 30, day(2026, 7, 21), today()).unwrap();
        d.add_blocked(day(2026, 9, 1), day(2026, 9, 7), "Trip", today())
            .unwrap();
        d.set_day_setting(DayToggle::Custom, Some(DaySet::ALL), today())
            .unwrap();
    });
    for d in &mut devices {
        account.sync(d);
    }
    assert_eq!(list(&devices[2]), account.list());
    (account, devices)
}

/// C syncs, then B, then everyone a few times: every device must end with the account's list.
fn settle(account: &mut Account, devices: &mut [Data; 3]) {
    let [_, b, c] = devices;
    account.sync(c);
    account.sync(b);
    for _ in 0..3 {
        for d in devices.iter_mut() {
            account.sync(d);
        }
    }
    for d in devices.iter() {
        assert_eq!(list(d), account.list());
        assert!(!d.has_unsynced());
    }
}

#[test]
fn a_tie_between_a_deletion_and_an_edit_ends_the_same_everywhere() {
    let (mut account, mut devices) = three_devices();
    let [_, b, c] = &mut devices;
    let (bo_b, bo_c) = (b.people[0].id, c.people[0].id);
    change(b, 500, |d| {
        d.delete_people(&[bo_b]);
    });
    change(c, 500, |d| {
        d.update_person(bo_c, "Cleo", 30, day(2026, 7, 21), today())
            .unwrap()
    });
    assert_eq!(b.sync.tombstones[0].modified, c.people[0].modified);
    settle(&mut account, &mut devices);
    assert!(devices[2].people.is_empty());
}

#[test]
fn a_tie_between_two_edits_ends_the_same_everywhere() {
    let (mut account, mut devices) = three_devices();
    let [_, b, c] = &mut devices;
    let (bo_b, bo_c) = (b.people[0].id, c.people[0].id);
    change(b, 500, |d| {
        d.update_person(bo_b, "Bea", 30, day(2026, 7, 21), today())
            .unwrap()
    });
    change(c, 500, |d| {
        d.update_person(bo_c, "Cleo", 30, day(2026, 7, 21), today())
            .unwrap()
    });
    assert_eq!(b.people[0].modified, c.people[0].modified);
    settle(&mut account, &mut devices);
    assert_eq!(devices[2].people[0].name, "Bea");
}

#[test]
fn a_tied_blocked_range_ends_the_same_everywhere() {
    let (mut account, mut devices) = three_devices();
    let [_, b, c] = &mut devices;
    let trip_b = b.blocked[0].id;
    change(b, 500, |d| {
        d.delete_blocked(trip_b, today());
    });
    // Ranges can't be edited in the app yet; a changed label stands for an edit made elsewhere.
    change(c, 500, |d| d.blocked[0].label = "Holiday".into());
    assert_eq!(b.sync.tombstones[0].modified, c.blocked[0].modified);
    settle(&mut account, &mut devices);
    assert!(devices[2].blocked.is_empty());
}

#[test]
fn a_tied_day_setting_ends_the_same_everywhere() {
    let (mut account, mut devices) = three_devices();
    let [_, b, c] = &mut devices;
    change(b, 500, |d| {
        d.set_day_setting(DayToggle::WeekdaysOnly, None, today())
            .unwrap();
    });
    change(c, 500, |d| {
        d.set_day_setting(DayToggle::WeekendsOnly, None, today())
            .unwrap();
    });
    assert_eq!(b.sync.settings_modified, c.sync.settings_modified);
    settle(&mut account, &mut devices);
    assert_eq!(devices[2].settings.day_toggle, DayToggle::WeekdaysOnly);
}

#[test]
fn a_tie_with_a_change_made_during_the_download_keeps_the_change() {
    // B deletes Bo and syncs. C uploads, then renames Bo while the download is on its way: the
    // rename isn't lost, it goes up next and wins.
    let (mut account, mut devices) = three_devices();
    let [_, b, c] = &mut devices;
    let (bo_b, bo_c) = (b.people[0].id, c.people[0].id);
    change(b, 500, |d| {
        d.delete_people(&[bo_b]);
    });
    account.sync(b);
    account.upload(c);
    change(c, 500, |d| {
        d.update_person(bo_c, "Cleo", 30, day(2026, 7, 21), today())
            .unwrap()
    });
    account.download(c);
    assert_eq!(c.people[0].name, "Cleo");
    assert!(c.has_unsynced());
    settle(&mut account, &mut devices);
    assert_eq!(devices[1].people[0].name, "Cleo");

    // The other way round: B renames Bo and syncs; C deletes Bo during the download. The deletion
    // stands, goes up next, and wins.
    let (mut account, mut devices) = three_devices();
    let [_, b, c] = &mut devices;
    let (bo_b, bo_c) = (b.people[0].id, c.people[0].id);
    change(b, 500, |d| {
        d.update_person(bo_b, "Bea", 30, day(2026, 7, 21), today())
            .unwrap()
    });
    account.sync(b);
    account.upload(c);
    change(c, 500, |d| {
        d.delete_people(&[bo_c]);
    });
    account.download(c);
    assert!(c.people.is_empty());
    settle(&mut account, &mut devices);
    assert!(devices[1].people.is_empty());

    // The same for the day setting.
    let (mut account, mut devices) = three_devices();
    let [_, b, c] = &mut devices;
    change(b, 500, |d| {
        d.set_day_setting(DayToggle::WeekdaysOnly, None, today())
            .unwrap();
    });
    account.sync(b);
    account.upload(c);
    change(c, 500, |d| {
        d.set_day_setting(DayToggle::WeekendsOnly, None, today())
            .unwrap();
    });
    account.download(c);
    assert_eq!(c.settings.day_toggle, DayToggle::WeekendsOnly);
    settle(&mut account, &mut devices);
    assert_eq!(devices[1].settings.day_toggle, DayToggle::WeekendsOnly);
}
