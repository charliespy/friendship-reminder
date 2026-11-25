//! The requests the app sends to the account's database, and how it reads the answers.

use std::cell::RefCell;

use chrono::Weekday;
use friends_account::{Config, Response, Transport};
use friends_core::schedule::{DaySet, DayToggle};
use friends_core::sync::{Changes, RemotePerson, RemoteSettings};
use friends_sync::{Server, SyncError, OVERLAP_MS};
use serde_json::{json, Value};

#[derive(Debug, Clone)]
struct Sent {
    method: &'static str,
    url: String,
    headers: Vec<(String, String)>,
    body: String,
}

/// Answers each request with the next queued answer, and records what was sent.
#[derive(Default)]
struct Fake {
    sent: RefCell<Vec<Sent>>,
    answers: RefCell<Vec<Result<Response, String>>>,
}

impl Fake {
    fn answer(self, status: u16, body: Value) -> Self {
        self.answers.borrow_mut().push(Ok(Response {
            status,
            body: body.to_string(),
        }));
        self
    }

    fn record(
        &self,
        method: &'static str,
        url: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<Response, String> {
        self.sent.borrow_mut().push(Sent {
            method,
            url: url.to_string(),
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body: body.to_string(),
        });
        let mut answers = self.answers.borrow_mut();
        if answers.is_empty() {
            Ok(Response {
                status: 200,
                body: "[]".into(),
            })
        } else {
            answers.remove(0)
        }
    }
}

impl Transport for &Fake {
    fn post(&self, url: &str, headers: &[(&str, &str)], body: &str) -> Result<Response, String> {
        self.record("POST", url, headers, body)
    }

    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Response, String> {
        self.record("GET", url, headers, "")
    }
}

fn config() -> Config {
    Config::new(
        Some("https://example.supabase.co"),
        Some("sb_publishable_test"),
    )
    .unwrap()
}

fn header<'a>(sent: &'a Sent, name: &str) -> Option<&'a str> {
    sent.headers
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

fn ada() -> RemotePerson {
    RemotePerson {
        id: "aaaa".into(),
        name: Some("Ada".into()),
        frequency_days: Some(30),
        last_connected: chrono_date("2026-08-01"),
        deleted: false,
        modified_at: 100,
        updated_ms: 0,
    }
}

fn chrono_date(s: &str) -> Option<chrono::NaiveDate> {
    Some(s.parse().unwrap())
}

#[test]
fn uploads_merge_into_the_users_rows() {
    let fake = Fake::default();
    let server = Server::new(config(), &fake);
    let changes = Changes {
        people: vec![ada()],
        blocked: vec![],
        settings: Some(RemoteSettings {
            day_toggle: DayToggle::Custom,
            custom_days: DaySet::of(&[Weekday::Tue, Weekday::Sat]),
            modified_at: 7,
            updated_ms: 0,
        }),
    };
    server.push("token-1", "user-1", &changes).unwrap();
    let sent = fake.sent.borrow();
    // Nothing goes up for a table with no changes.
    assert_eq!(sent.len(), 2);
    let people = &sent[0];
    assert_eq!(people.method, "POST");
    assert_eq!(
        people.url,
        "https://example.supabase.co/rest/v1/people?on_conflict=owner,id"
    );
    assert_eq!(header(people, "apikey"), Some("sb_publishable_test"));
    assert_eq!(header(people, "Authorization"), Some("Bearer token-1"));
    assert_eq!(
        header(people, "Prefer"),
        Some("resolution=merge-duplicates,return=minimal")
    );
    let body: Value = serde_json::from_str(&people.body).unwrap();
    assert_eq!(
        body,
        json!([{
            "owner": "user-1", "id": "aaaa", "name": "Ada", "frequency_days": 30,
            "last_connected": "2026-08-01", "deleted": false, "modified_at": 100
        }])
    );
    let settings = &sent[1];
    assert_eq!(
        settings.url,
        "https://example.supabase.co/rest/v1/settings?on_conflict=owner"
    );
    let body: Value = serde_json::from_str(&settings.body).unwrap();
    assert_eq!(
        body,
        json!([{
            "owner": "user-1", "day_toggle": "custom", "custom_days": ["tue", "sat"],
            "modified_at": 7
        }])
    );
}

#[test]
fn downloads_read_what_changed_since_the_cursor() {
    let row = json!({
        "id": "aaaa", "name": "Ada", "frequency_days": 30, "last_connected": "2026-08-01",
        "deleted": false, "modified_at": 100, "updated_ms": 90_000
    });
    let gone = json!({
        "id": "bbbb", "name": null, "frequency_days": null, "last_connected": null,
        "deleted": true, "modified_at": 200, "updated_ms": 95_000
    });
    let fake = Fake::default()
        .answer(200, json!([row, gone]))
        .answer(200, json!([]))
        .answer(
            200,
            json!([{"day_toggle": "allDays", "modified_at": 3, "updated_ms": 80_000}]),
        );
    let server = Server::new(config(), &fake);
    let (changes, cursor) = server.pull("token-1", 60_000).unwrap();
    assert_eq!(cursor, 95_000);
    assert_eq!(changes.people.len(), 2);
    assert_eq!(
        changes.people[0],
        RemotePerson {
            updated_ms: 90_000,
            ..ada()
        }
    );
    assert!(changes.people[1].deleted);
    assert_eq!(
        changes.settings,
        Some(RemoteSettings {
            // "Any day", from before custom days: all seven days picked.
            day_toggle: DayToggle::Custom,
            custom_days: DaySet::ALL,
            modified_at: 3,
            updated_ms: 80_000,
        })
    );
    let sent = fake.sent.borrow();
    assert_eq!(sent.len(), 3);
    assert!(sent.iter().all(|s| s.method == "GET"));
    assert_eq!(
        sent[0].url,
        format!(
            "https://example.supabase.co/rest/v1/people?select=id,name,frequency_days,last_connected,deleted,modified_at,updated_ms&updated_ms=gt.{}&order=updated_ms.asc,id.asc&limit=500",
            60_000 - OVERLAP_MS
        )
    );
    assert!(sent[1].url.contains("/rest/v1/blocked_dates?"));
    assert!(sent[2].url.contains("/rest/v1/settings?"));
}

#[test]
fn nothing_new_keeps_the_cursor() {
    let fake = Fake::default();
    let server = Server::new(config(), &fake);
    let (changes, cursor) = server.pull("t", 5).unwrap();
    assert!(changes.is_empty());
    assert_eq!(cursor, 5);
}

#[test]
fn a_full_page_asks_for_the_next() {
    let rows: Vec<Value> = (0..500)
        .map(|i| {
            json!({
                "id": format!("id-{i}"), "name": "A", "frequency_days": 30,
                "last_connected": "2026-08-01", "deleted": false, "modified_at": 1,
                "updated_ms": 1000 + i / 2
            })
        })
        .collect();
    let fake = Fake::default().answer(200, Value::Array(rows));
    let server = Server::new(config(), &fake);
    let (changes, _) = server.pull("t", 0).unwrap();
    assert_eq!(changes.people.len(), 500);
    let sent = fake.sent.borrow();
    // The next page starts after the last row seen, by time and then id (rows can share a time).
    assert!(sent[1].url.contains(
        "/rest/v1/people?select=id,name,frequency_days,last_connected,deleted,modified_at,updated_ms&or=(updated_ms.gt.1249,and(updated_ms.eq.1249,id.gt.id-499))&order=updated_ms.asc,id.asc&limit=500"
    ));
    assert!(sent[2].url.contains("/rest/v1/blocked_dates?"));
}

#[test]
fn errors_say_what_to_do() {
    let fake = Fake::default().answer(401, json!({"code": "PGRST303", "message": "JWT expired"}));
    let server = Server::new(config(), &fake);
    assert_eq!(server.pull("t", 0), Err(SyncError::Expired));

    let fake = Fake::default().answer(
        404,
        json!({"code": "PGRST205", "message": "Could not find the table 'public.people'"}),
    );
    let server = Server::new(config(), &fake);
    let err = server.push(
        "t",
        "u",
        &Changes {
            people: vec![ada()],
            ..Changes::default()
        },
    );
    assert_eq!(
        err.unwrap_err().to_string(),
        "Couldn't sync with your Friends account: the account's database isn't set up yet"
    );

    let fake = Fake::default();
    fake.answers.borrow_mut().push(Err("offline".into()));
    let server = Server::new(config(), &fake);
    assert!(matches!(
        server.pull("t", 0),
        Err(SyncError::Unreachable(_))
    ));

    let fake = Fake::default().answer(200, json!({"not": "a list"}));
    let server = Server::new(config(), &fake);
    assert!(matches!(server.pull("t", 0), Err(SyncError::Failed(_))));
}

mod whole_sync {
    use super::*;
    use friends_account::Session;
    use friends_core::api::App;
    use friends_core::store::Store;
    use std::sync::Mutex;

    fn session(token: &str) -> Session {
        Session {
            access_token: token.into(),
            refresh_token: "r".into(),
            expires_at: i64::MAX,
            user_id: "user-1".into(),
            email: None,
        }
    }

    fn app(name: &str) -> Mutex<App> {
        let dir =
            std::env::temp_dir().join(format!("friends-sync-run-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut app = App::open(Store::new(dir.join("friends.json"))).unwrap();
        app.call(
            "add_person",
            json!({"name": "Ada", "frequencyDays": 30, "lastConnected": "2026-08-01"}),
            today(),
        )
        .unwrap();
        Mutex::new(app)
    }

    fn today() -> chrono::NaiveDate {
        "2026-08-19".parse().unwrap()
    }

    #[test]
    fn uploads_then_downloads_and_renews_a_refused_token_once() {
        let app = app("renew");
        let grace = json!([{
            "id": "cccc", "name": "Grace", "frequency_days": 90, "last_connected": "2026-08-02",
            "deleted": false, "modified_at": 5, "updated_ms": 70_000
        }]);
        // The first upload is refused (token ran out); after renewing, upload, then three downloads.
        let fake = Fake::default()
            .answer(401, json!({"message": "JWT expired"}))
            .answer(201, json!(null))
            .answer(200, grace);
        let server = Server::new(config(), &fake);
        let renewed = std::cell::Cell::new(0);
        let changed = friends_sync::sync(
            &server,
            &session("old"),
            &app,
            today(),
            |user| user == "user-1",
            || {
                renewed.set(renewed.get() + 1);
                Ok(Some(session("new")))
            },
        )
        .unwrap();
        assert!(changed);
        assert_eq!(renewed.get(), 1);
        let sent = fake.sent.borrow();
        assert_eq!(header(&sent[0], "Authorization"), Some("Bearer old"));
        assert!(sent[1..]
            .iter()
            .all(|s| header(s, "Authorization") == Some("Bearer new")));
        assert_eq!(sent.len(), 5);
        let app = app.lock().unwrap();
        let names: Vec<_> = app.data().people.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Ada", "Grace"]);
        assert!(!app.has_unsynced());
        assert_eq!(app.sync_cursor(), 70_000);
    }

    #[test]
    fn a_sign_out_meanwhile_stops_the_sync() {
        let app = app("signed-out");
        let fake = Fake::default();
        let server = Server::new(config(), &fake);
        let result = friends_sync::sync(
            &server,
            &session("t"),
            &app,
            today(),
            |_| false,
            || unreachable!(),
        );
        assert_eq!(result, Err(SyncError::SignedOut));
        assert!(fake.sent.borrow().is_empty());

        // A refused token whose renewal finds the account signed out isn't a sync that worked.
        let fake = Fake::default().answer(401, json!({}));
        let server = Server::new(config(), &fake);
        let result =
            friends_sync::sync(&server, &session("t"), &app, today(), |_| true, || Ok(None));
        assert_eq!(result, Err(SyncError::SignedOut));
    }
}
