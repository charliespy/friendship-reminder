//! The sync rules: what goes up to the account, and how changes from other devices come in.

use chrono::NaiveDate;
use friends_core::api::App;
use friends_core::model::Data;
use friends_core::schedule::DayToggle;
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
    // The day setting goes up too, so other devices start from this Mac's.
    assert_eq!(
        sent.settings.as_ref().map(|s| s.day_toggle),
        Some(DayToggle::WeekendsOnly)
    );
    data.sync_pushed(&sent);
    assert!(data.sync_start(OWNER).is_empty());
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
    change(&mut data, 300, |d| d.mark_done(id, today()).unwrap());
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
    change(&mut data, 600, |d| d.mark_done(id, today()).unwrap());
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
        }],
        settings: None,
    };
    data.sync_pulled(&got, 1, today());
    assert_eq!(data.blocked.len(), 1);
    assert!(data.people[0].next_target > target);
    let all_days = Changes {
        settings: Some(RemoteSettings {
            day_toggle: DayToggle::AllDays,
            modified_at: 500,
        }),
        ..Changes::default()
    };
    data.sync_pulled(&all_days, 1, today());
    assert_eq!(data.settings.day_toggle, DayToggle::AllDays);
    assert_eq!(data.people[0].next_target, day(2026, 8, 31));
    // An older day setting from another device loses to one changed here later.
    change(&mut data, 900, |d| {
        d.set_day_toggle(DayToggle::WeekdaysOnly, today())
    });
    let older = Changes {
        settings: Some(RemoteSettings {
            day_toggle: DayToggle::AllDays,
            modified_at: 800,
        }),
        ..Changes::default()
    };
    data.sync_pulled(&older, 2, today());
    assert_eq!(data.settings.day_toggle, DayToggle::WeekdaysOnly);
}

#[test]
fn another_account_gets_everything_and_not_the_old_accounts_deletions() {
    let (mut data, _) = synced_with_ada();
    change(&mut data, 200, |d| {
        d.add_person("Grace", 30, today(), today()).unwrap();
    });
    let grace = data.people[1].id;
    change(&mut data, 300, |d| {
        d.delete_people(&[grace]);
    });
    data.sync_pulled(&Changes::default(), 1000, today());
    let other = "22222222-2222-2222-2222-222222222222";
    let sent = data.sync_start(other);
    assert_eq!(sent.people.len(), 1);
    assert_eq!(sent.people[0].name.as_deref(), Some("Ada"));
    assert!(!sent.people[0].deleted);
    assert!(sent.settings.is_some());
    assert_eq!(data.sync.cursor, 0);
}

#[test]
fn the_app_stamps_changes_and_upgrades_old_files() {
    let dir = std::env::temp_dir().join(format!("friends-sync-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("friends.json");
    std::fs::write(
        &path,
        r#"{"version":1,"lastId":1,"people":[{"id":1,"name":"Ada","frequencyDays":30,"lastConnected":"2026-08-01","nextTarget":"2026-08-31","notificationDate":"2026-08-31"}]}"#,
    )
    .unwrap();
    let mut app = App::open(Store::new(&path)).unwrap();
    assert!(!app.data().people[0].uid.is_empty());
    app.set_clock(|| 4242);
    app.call(
        "add_person",
        json!({"name": "Grace", "frequencyDays": 30, "lastConnected": "2026-08-19"}),
        today(),
    )
    .unwrap();
    let saved: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(saved["version"], 2);
    assert_eq!(saved["people"][1]["modified"], 4242);
    let sent = app.sync_start(OWNER).unwrap();
    assert_eq!(sent.people.len(), 2);
    app.sync_pushed(&sent).unwrap();
    let got = people(vec![remote_person("cccc", "Alan", 5000)]);
    assert!(app.sync_pulled(&got, 77, today()).unwrap());
    assert_eq!(app.sync_cursor(), 77);
    let reopened = App::open(Store::new(&path)).unwrap();
    assert_eq!(reopened.data(), app.data());
}
