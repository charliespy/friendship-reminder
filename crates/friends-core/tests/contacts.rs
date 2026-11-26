//! Adding people from the address book, the link each one keeps to their contact card, and the
//! first-run guide.

use std::fs;
use std::path::PathBuf;

use chrono::NaiveDate;
use friends_core::api::App;
use friends_core::contacts::{
    choices, just_chosen, link_url, person_in_link, uid_in_link, Card, Chosen,
};
use friends_core::model::{Data, NewPerson};
use friends_core::store::Store;
use serde_json::json;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn today() -> NaiveDate {
    date(2026, 8, 19)
}

fn temp_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "friends-contacts-test-{}-{name}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir.join("friends.json")
}

fn app(name: &str) -> (App, PathBuf) {
    let path = temp_file(name);
    (App::open(Store::new(&path)).unwrap(), path)
}

fn card(id: &str, name: &str) -> Card {
    Card {
        id: id.to_string(),
        name: name.to_string(),
        links: Vec::new(),
    }
}

/// Someone to add, monthly.
fn new_person(name: &str, contact: Option<&str>) -> NewPerson {
    every(30, name, contact)
}

fn every(frequency_days: u32, name: &str, contact: Option<&str>) -> NewPerson {
    NewPerson {
        name: name.to_string(),
        contact: contact.map(str::to_string),
        frequency_days,
    }
}

#[test]
fn adding_several_people_schedules_each_from_the_day_given() {
    let mut data = Data::default();
    let ids = data
        .add_people(
            &[every(90, "Ada", Some("card-1")), every(365, "Bo", None)],
            today(),
            today(),
        )
        .unwrap();
    assert_eq!(ids.len(), 2);
    assert_eq!(data.people.len(), 2);
    // Each keeps the frequency picked for them.
    assert_eq!(data.people[0].frequency_days, 90);
    assert_eq!(data.people[1].frequency_days, 365);
    for p in &data.people {
        assert_eq!(p.last_connected, today());
        // The same schedule adding them one at a time gives.
        let mut one = Data::default();
        one.add_person(&p.name, p.frequency_days, today(), today())
            .unwrap();
        assert_eq!(p.next_target, one.people[0].next_target);
        assert_eq!(p.notification_date, one.people[0].notification_date);
    }
    assert_eq!(data.people[0].contact.as_deref(), Some("card-1"));
    assert_eq!(data.people[1].contact, None);
    // Every one gets its own id every device knows.
    assert_ne!(data.people[0].uid, data.people[1].uid);
}

#[test]
fn adding_nobody_is_refused() {
    let mut data = Data::default();
    let err = data.add_people(&[], today(), today()).unwrap_err();
    assert_eq!(err.0, "Pick at least one person.");
}

#[test]
fn a_bad_name_in_the_batch_adds_nobody() {
    let (mut app, _) = app("bad-batch");
    let err = app
        .call(
            "add_people",
            json!({
                "people": [
                    {"name": "Ada", "contact": "card-1", "frequencyDays": 30},
                    {"name": "   ", "frequencyDays": 30},
                ],
                "lastConnected": "2026-08-19",
            }),
            today(),
        )
        .unwrap_err();
    assert_eq!(err.message, "Give them a name.");
    assert!(app.data().people.is_empty());
}

#[test]
fn a_card_already_in_the_list_isnt_added_twice() {
    let mut data = Data::default();
    data.add_people(&[new_person("Ada", Some("card-1"))], today(), today())
        .unwrap();
    let ids = data
        .add_people(
            &[
                new_person("Ada", Some("card-1")),
                new_person("Bo", Some("card-2")),
                new_person("Bo", Some("card-2")),
            ],
            today(),
            today(),
        )
        .unwrap();
    assert_eq!(ids.len(), 1);
    let names: Vec<&str> = data.people.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Ada", "Bo"]);
}

#[test]
fn the_add_people_command_saves_the_link_on_this_device() {
    let (mut app, path) = app("command");
    let view = app
        .call(
            "add_people",
            json!({
                "people": [{"name": "Ada Park", "contact": "card-1", "frequencyDays": 180}],
                "lastConnected": "2026-08-19",
            }),
            today(),
        )
        .unwrap();
    assert_eq!(view.people.len(), 1);
    let again = App::open(Store::new(&path)).unwrap();
    assert_eq!(again.data().people[0].contact.as_deref(), Some("card-1"));
    assert_eq!(again.data().people[0].frequency_days, 180);
}

#[test]
fn the_link_to_the_card_never_goes_to_the_account() {
    let mut data = Data::default();
    data.add_people(
        &[new_person("Ada", Some("card-secret-id"))],
        today(),
        today(),
    )
    .unwrap();
    let changes = data.sync_start("11111111-1111-1111-1111-111111111111");
    assert_eq!(changes.people.len(), 1);
    let sent = serde_json::to_string(&changes.people).unwrap();
    assert!(!sent.contains("card-secret-id"), "sent: {sent}");
}

#[test]
fn the_link_stays_when_another_device_changes_the_person() {
    use friends_core::sync::{Changes, RemotePerson};
    let mut data = Data::default();
    data.add_people(&[new_person("Ada", Some("card-1"))], today(), today())
        .unwrap();
    let uid = data.people[0].uid.clone();
    data.sync_start("11111111-1111-1111-1111-111111111111");
    let got = Changes {
        people: vec![RemotePerson {
            id: uid,
            name: Some("Ada Park".into()),
            frequency_days: Some(90),
            last_connected: Some(today()),
            deleted: false,
            modified_at: i64::MAX / 2,
            updated_ms: 1,
        }],
        ..Default::default()
    };
    data.sync_pulled(&got, 1, today());
    assert_eq!(data.people[0].name, "Ada Park");
    assert_eq!(data.people[0].contact.as_deref(), Some("card-1"));
}

#[test]
fn a_link_on_the_card_names_the_person_every_device_knows() {
    let url = link_url("3f2a9c4e-0000-4000-8000-000000000001");
    assert_eq!(
        url,
        "io.github.charliespy.friends://person/3f2a9c4e-0000-4000-8000-000000000001"
    );
    assert_eq!(
        uid_in_link(&url),
        Some("3f2a9c4e-0000-4000-8000-000000000001")
    );
    assert_eq!(uid_in_link("https://example.com/person/x"), None);
    assert_eq!(uid_in_link("io.github.charliespy.friends://person/"), None);
}

#[test]
fn the_picker_lists_cards_by_name_and_marks_who_is_already_added() {
    let mut data = Data::default();
    data.add_people(&[new_person("Ada Park", Some("card-1"))], today(), today())
        .unwrap();
    data.add_person("bo  chen", 30, today(), today()).unwrap();
    data.add_person("Kit", 30, today(), today()).unwrap();
    let kit = data.people[2].uid.clone();
    let mut linked_elsewhere = card("card-9", "Kit Lane");
    linked_elsewhere.links = vec![link_url(&kit)];
    let list = choices(
        &data,
        vec![
            card("card-3", "  Zoe   Ray "),
            card("card-1", "Ada Park (work)"),
            card("card-2", "Bo Chen"),
            card("card-4", "   "),
            linked_elsewhere,
            card("card-5", "cal Moss"),
        ],
    );
    let shown: Vec<(&str, &str, bool)> = list
        .iter()
        .map(|c| (c.id.as_str(), c.name.as_str(), c.added))
        .collect();
    assert_eq!(
        shown,
        [
            // Linked from this device.
            ("card-1", "Ada Park (work)", true),
            // The same name, whatever the case and spacing.
            ("card-2", "Bo Chen", true),
            ("card-5", "cal Moss", false),
            // Linked from another device: the card carries the link.
            ("card-9", "Kit Lane", true),
            ("card-3", "Zoe Ray", false),
        ]
    );
}

#[test]
fn a_very_long_name_on_a_card_is_cut_to_fit() {
    let long = "A".repeat(150);
    let list = choices(&Data::default(), vec![card("card-1", &long)]);
    assert_eq!(list[0].name.chars().count(), 100);
}

#[test]
fn a_new_list_starts_the_guide_and_an_old_one_has_finished_it() {
    let (app, _) = app("guide-new");
    let view = serde_json::to_value(app.view(today())).unwrap();
    assert_eq!(view["guide"], json!({"signIn": false, "import": false}));

    // A file saved before the guide existed: its user has been here before.
    let path = temp_file("guide-old");
    fs::write(&path, r#"{"version": 3, "people": [], "blocked": []}"#).unwrap();
    let old = App::open(Store::new(&path)).unwrap();
    let view = serde_json::to_value(old.view(today())).unwrap();
    assert_eq!(view["guide"], json!({"signIn": true, "import": true}));
}

#[test]
fn each_step_of_the_guide_is_remembered_once_done() {
    let (mut app, path) = app("guide-steps");
    let view = app
        .call("finish_guide", json!({"step": "signIn"}), today())
        .unwrap();
    let json = serde_json::to_value(&view).unwrap();
    assert_eq!(json["guide"], json!({"signIn": true, "import": false}));
    app.call("finish_guide", json!({"step": "import"}), today())
        .unwrap();
    let again = App::open(Store::new(&path)).unwrap();
    let json = serde_json::to_value(again.view(today())).unwrap();
    assert_eq!(json["guide"], json!({"signIn": true, "import": true}));
    assert!(app
        .call("finish_guide", json!({"step": "other"}), today())
        .is_err());
}

#[test]
fn finishing_the_guide_isnt_a_change_to_sync() {
    let mut data = Data::default();
    data.sync_start("11111111-1111-1111-1111-111111111111");
    let before = data.clone();
    data.guide.sign_in = true;
    data.stamp(&before, 1_000);
    assert!(!data.has_unsynced());
}

#[test]
fn a_list_started_over_after_a_damaged_file_skips_the_guide() {
    // Someone who used the app before: the "couldn't be read" notice must show, not the guide.
    let path = temp_file("guide-damaged");
    fs::write(&path, "not json").unwrap();
    let app = App::open(Store::new(&path)).unwrap();
    let view = serde_json::to_value(app.view(today())).unwrap();
    assert!(view["unreadableFile"].is_string());
    assert_eq!(view["guide"], json!({"signIn": true, "import": true}));
}

#[test]
fn someone_with_the_same_name_isnt_added_again_from_contacts() {
    // The picker can be behind: a person synced down meanwhile, whose card it didn't mark.
    let mut data = Data::default();
    data.add_person("Ana  torres", 30, today(), today())
        .unwrap();
    let ids = data
        .add_people(
            &[
                new_person("Ana Torres", Some("card-1")),
                new_person("Bo", Some("card-2")),
            ],
            today(),
            today(),
        )
        .unwrap();
    assert_eq!(ids.len(), 1);
    assert_eq!(data.people.len(), 2);
}

#[test]
fn people_in_the_list_finish_the_import_step() {
    let (mut app, _) = app("guide-people");
    let view = app
        .call(
            "add_person",
            json!({"name": "Ada", "frequencyDays": 30, "lastConnected": "2026-08-19"}),
            today(),
        )
        .unwrap();
    assert!(view.guide.import);
    assert!(!view.guide.sign_in);
}

#[test]
fn people_synced_down_finish_the_import_step() {
    use friends_core::sync::{Changes, RemotePerson};
    let (mut app, _) = app("guide-synced");
    app.sync_start("11111111-1111-1111-1111-111111111111")
        .unwrap();
    let got = Changes {
        people: vec![RemotePerson {
            id: "3f2a9c4e-0000-4000-8000-000000000001".into(),
            name: Some("Ada".into()),
            frequency_days: Some(30),
            last_connected: Some(today()),
            deleted: false,
            modified_at: 5,
            updated_ms: 5,
        }],
        ..Default::default()
    };
    app.sync_pulled(&got, 5, today()).unwrap();
    assert!(app.view(today()).guide.import);
}

#[test]
fn importing_answers_which_cards_take_a_link() {
    let (mut app, _) = app("import-links");
    app.call(
        "add_people",
        json!({"people": [{"name": "Ada", "contact": "card-1", "frequencyDays": 30}], "lastConnected": "2026-08-19"}),
        today(),
    )
    .unwrap();
    let (view, links) = app
        .import_contacts(
            vec![
                new_person("Ada", Some("card-1")),
                new_person("Bo", Some("card-2")),
            ],
            today(),
        )
        .unwrap();
    assert_eq!(view.people.len(), 2);
    let bo = app.data().people.iter().find(|p| p.name == "Bo").unwrap();
    // Only the person added now: Ada's card has its link already.
    assert_eq!(links, vec![("card-2".to_string(), bo.uid.clone())]);
}

#[test]
fn a_frequency_the_app_doesnt_offer_adds_nobody() {
    let mut data = Data::default();
    let err = data
        .add_people(
            &[every(30, "Ada", None), every(45, "Bo", None)],
            today(),
            today(),
        )
        .unwrap_err();
    assert_eq!(err.0, "Pick how often to be in touch.");
    assert!(data.people.is_empty());
}

#[test]
fn a_link_from_a_card_finds_the_person_on_this_device() {
    let mut data = Data::default();
    data.add_people(&[new_person("Ada", Some("card-1"))], today(), today())
        .unwrap();
    let (id, uid) = (data.people[0].id, data.people[0].uid.clone());
    assert_eq!(person_in_link(&data, &link_url(&uid)), Some(id));
    // Someone deleted since, another app's link, or a link with nothing after it.
    assert_eq!(person_in_link(&data, &link_url("gone")), None);
    assert_eq!(person_in_link(&data, "https://example.com"), None);
    assert_eq!(
        person_in_link(&data, "io.github.charliespy.friends://auth/callback?code=x"),
        None
    );
}

#[test]
fn people_just_chosen_in_the_system_sheet_go_straight_to_how_often() {
    let mut data = Data::default();
    data.add_people(&[new_person("Ben Okafor", None)], today(), today())
        .unwrap();
    let list = choices(
        &data,
        vec![
            card("c3", "Iris Novak"),
            card("c1", "Ben Okafor"),
            card("c2", "Elif Yilmaz"),
        ],
    );
    // A fresh "share only some": everyone the app can see, except who's in the list already, by name.
    assert_eq!(just_chosen(&list, &Chosen::Everyone), ["c2", "c3"]);
    // Shared more: only those cards, still without anyone added.
    assert_eq!(
        just_chosen(&list, &Chosen::Cards(vec!["c1".into(), "c3".into()])),
        ["c3"]
    );
    assert!(just_chosen(&list, &Chosen::Nobody).is_empty());
    // A card that isn't among those the app can see is left out.
    assert!(just_chosen(&list, &Chosen::Cards(vec!["c9".into()])).is_empty());
}

#[test]
fn cards_shared_in_settings_are_the_ones_new_to_the_app() {
    let ids = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let before = ids(&["c1", "c2"]);
    // Still sharing only some: whoever the app can see now and couldn't before was picked there
    // (c1 un-shared meanwhile doesn't count).
    assert_eq!(
        Chosen::shared_in_settings(&before, &ids(&["c2", "c3", "c4"]), true),
        Chosen::Cards(ids(&["c3", "c4"]))
    );
    assert_eq!(
        Chosen::shared_in_settings(&before, &ids(&["c2"]), true),
        Chosen::Cards(vec![])
    );
    // Full access now: every card is new to the app, but nobody was picked; the list shows instead.
    assert_eq!(
        Chosen::shared_in_settings(&before, &ids(&["c1", "c2", "c3", "c4"]), false),
        Chosen::Nobody
    );
}
