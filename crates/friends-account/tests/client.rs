use std::cell::RefCell;

use friends_account::{
    AccountView, AuthError, Client, Config, Nonce, Response, Session, Transport,
};
use serde_json::{json, Value};

/// One request as sent: the URL, the headers and the body.
type Sent = (String, Vec<(String, String)>, String);

/// Answers every request with one canned response and remembers what was sent.
struct Fake {
    answer: Result<Response, String>,
    sent: RefCell<Vec<Sent>>,
}

impl Fake {
    fn new(status: u16, body: Value) -> Fake {
        Fake {
            answer: Ok(Response {
                status,
                body: body.to_string(),
            }),
            sent: RefCell::new(Vec::new()),
        }
    }

    fn offline() -> Fake {
        Fake {
            answer: Err("connection refused".to_string()),
            sent: RefCell::new(Vec::new()),
        }
    }
}

impl Transport for &Fake {
    fn post(&self, url: &str, headers: &[(&str, &str)], body: &str) -> Result<Response, String> {
        self.sent.borrow_mut().push((
            url.to_string(),
            headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body.to_string(),
        ));
        self.answer.clone()
    }
}

fn config() -> Config {
    Config::new(
        Some("https://example.supabase.co/"),
        Some("sb_publishable_test"),
    )
    .unwrap()
}

fn token_answer() -> Value {
    json!({
        "access_token": "access-1",
        "token_type": "bearer",
        "expires_in": 3600,
        "expires_at": 1_800_003_600,
        "refresh_token": "refresh-1",
        "user": { "id": "user-1", "email": "ada@example.com" }
    })
}

#[test]
fn a_build_without_both_settings_has_no_account() {
    assert_eq!(Config::new(None, Some("key")), None);
    assert_eq!(Config::new(Some("https://x.supabase.co"), None), None);
    assert_eq!(Config::new(Some(" "), Some("key")), None);
    assert_eq!(config().url, "https://example.supabase.co");
}

#[test]
fn signing_in_with_apple_sends_the_token_and_raw_nonce() {
    let fake = Fake::new(200, token_answer());
    let session = Client::new(config(), &fake)
        .sign_in_with_apple("apple-token", "raw-nonce", 1_800_000_000)
        .unwrap();
    assert_eq!(
        session,
        Session {
            access_token: "access-1".into(),
            refresh_token: "refresh-1".into(),
            expires_at: 1_800_003_600,
            user_id: "user-1".into(),
            email: Some("ada@example.com".into()),
        }
    );
    let sent = fake.sent.borrow();
    let (url, headers, body) = &sent[0];
    assert_eq!(
        url,
        "https://example.supabase.co/auth/v1/token?grant_type=id_token"
    );
    assert!(headers.contains(&("apikey".into(), "sb_publishable_test".into())));
    let body: Value = serde_json::from_str(body).unwrap();
    assert_eq!(
        body,
        json!({ "provider": "apple", "id_token": "apple-token", "nonce": "raw-nonce" })
    );
}

#[test]
fn expiry_comes_from_expires_in_when_expires_at_is_missing() {
    let mut answer = token_answer();
    answer.as_object_mut().unwrap().remove("expires_at");
    let fake = Fake::new(200, answer);
    let session = Client::new(config(), &fake)
        .refresh("refresh-0", 1_000)
        .unwrap();
    assert_eq!(session.expires_at, 4_600);
    let sent = fake.sent.borrow();
    assert!(sent[0].0.ends_with("grant_type=refresh_token"));
    assert_eq!(
        serde_json::from_str::<Value>(&sent[0].2).unwrap(),
        json!({ "refresh_token": "refresh-0" })
    );
}

#[test]
fn a_hidden_or_empty_email_is_none() {
    let mut answer = token_answer();
    answer["user"]["email"] = json!("");
    let fake = Fake::new(200, answer);
    let session = Client::new(config(), &fake).refresh("r", 0).unwrap();
    assert_eq!(session.email, None);
}

#[test]
fn a_refused_refresh_is_rejected_with_supabases_message() {
    let fake = Fake::new(
        400,
        json!({ "code": 400, "error_code": "refresh_token_not_found", "msg": "Invalid Refresh Token" }),
    );
    let err = Client::new(config(), &fake).refresh("old", 0).unwrap_err();
    assert_eq!(err, AuthError::Rejected("Invalid Refresh Token".into()));
}

#[test]
fn offline_and_server_errors_are_not_rejections() {
    let offline = Fake::offline();
    assert!(matches!(
        Client::new(config(), &offline).refresh("r", 0),
        Err(AuthError::Unreachable(_))
    ));
    for status in [500, 503, 429] {
        let fake = Fake::new(status, json!({ "message": "busy" }));
        assert_eq!(
            Client::new(config(), &fake).refresh("r", 0),
            Err(AuthError::Server("busy".into()))
        );
    }
}

#[test]
fn a_strange_answer_is_a_server_error() {
    let fake = Fake::new(200, json!({ "access_token": "a" }));
    assert!(matches!(
        Client::new(config(), &fake).refresh("r", 0),
        Err(AuthError::Server(_))
    ));
}

#[test]
fn signing_out_sends_the_access_token_and_accepts_an_already_dead_session() {
    for status in [204, 401, 403, 404] {
        let fake = Fake::new(status, json!({}));
        Client::new(config(), &fake).sign_out("access-1").unwrap();
        let sent = fake.sent.borrow();
        assert_eq!(
            sent[0].0,
            "https://example.supabase.co/auth/v1/logout?scope=local"
        );
        assert!(sent[0]
            .1
            .contains(&("Authorization".into(), "Bearer access-1".into())));
    }
    let fake = Fake::new(500, json!({}));
    assert!(Client::new(config(), &fake).sign_out("a").is_err());
}

#[test]
fn a_session_needs_a_refresh_a_minute_before_it_expires() {
    let session = Session {
        access_token: "a".into(),
        refresh_token: "r".into(),
        expires_at: 1_000,
        user_id: "u".into(),
        email: None,
    };
    assert!(!session.needs_refresh(939));
    assert!(session.needs_refresh(940));
    assert!(session.needs_refresh(2_000));
}

#[test]
fn the_nonce_hash_is_sha256_of_the_raw_value() {
    // SHA-256("abc"), a standard test vector.
    assert_eq!(
        Nonce::from_raw("abc".into()).hashed,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let a = Nonce::new().unwrap();
    let b = Nonce::new().unwrap();
    assert_eq!(a.raw.len(), 64);
    assert_ne!(a.raw, b.raw);
}

#[test]
fn the_account_view_is_tagged_for_the_ui() {
    assert_eq!(
        serde_json::to_value(AccountView::SignedOut).unwrap(),
        json!({ "status": "signedOut" })
    );
    assert_eq!(
        serde_json::to_value(AccountView::Unavailable {
            reason: "no".into()
        })
        .unwrap(),
        json!({ "status": "unavailable", "reason": "no" })
    );
    assert_eq!(
        serde_json::to_value(AccountView::SignedIn {
            email: Some("ada@example.com".into()),
            warning: None,
        })
        .unwrap(),
        json!({ "status": "signedIn", "email": "ada@example.com", "warning": null })
    );
}
