use std::cell::RefCell;

use friends_account::{
    parse_callback, AccountView, AuthError, Callback, Client, Config, Pkce, Response, Session,
    Transport,
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
fn the_sign_in_page_carries_the_provider_return_address_and_challenge() {
    let fake = Fake::new(200, json!({}));
    let pkce = Pkce::from_verifier("verifier".into());
    let url = Client::new(config(), &fake).authorize_url(
        "apple",
        "http://127.0.0.1:50123/auth/callback",
        &pkce,
    );
    assert_eq!(
        url,
        format!(
            "https://example.supabase.co/auth/v1/authorize?provider=apple\
             &redirect_to=http%3A%2F%2F127.0.0.1%3A50123%2Fauth%2Fcallback\
             &code_challenge={}&code_challenge_method=s256",
            pkce.challenge
        )
    );
    assert!(fake.sent.borrow().is_empty());
}

#[test]
fn the_code_is_traded_with_the_verifier() {
    let fake = Fake::new(200, token_answer());
    let session = Client::new(config(), &fake)
        .exchange_code("code-1", "verifier-1", 1_800_000_000)
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
        "https://example.supabase.co/auth/v1/token?grant_type=pkce"
    );
    assert!(headers.contains(&("apikey".into(), "sb_publishable_test".into())));
    assert_eq!(
        serde_json::from_str::<Value>(body).unwrap(),
        json!({ "auth_code": "code-1", "code_verifier": "verifier-1" })
    );
}

#[test]
fn the_browser_comes_back_with_a_code_or_a_reason() {
    assert_eq!(
        parse_callback("/auth/callback?code=ab%2Dc+d"),
        Callback::Code("ab-c d".into())
    );
    assert_eq!(
        parse_callback("/auth/callback?error=access_denied&error_description=User+cancelled%20it"),
        Callback::Failed("User cancelled it".into())
    );
    assert_eq!(
        parse_callback("/auth/callback?error=server_error"),
        Callback::Failed("server_error".into())
    );
    assert_eq!(
        parse_callback("/auth/callback"),
        Callback::Failed("no sign-in code came back".into())
    );
    assert_eq!(parse_callback("/favicon.ico"), Callback::Other);
    // A stray % is kept, not an error.
    assert_eq!(
        parse_callback("/auth/callback?code=50%"),
        Callback::Code("50%".into())
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
fn only_a_dead_session_is_a_rejection() {
    // A wrong or rotated app key is refused, but says nothing about the session.
    let fake = Fake::new(401, json!({ "message": "Invalid API key" }));
    let err = Client::new(config(), &fake).refresh("r", 0).unwrap_err();
    assert_eq!(err, AuthError::Refused("Invalid API key".into()));
    assert!(!err.session_is_dead());
    let fake = Fake::new(
        400,
        json!({ "error_code": "session_expired", "msg": "Session expired" }),
    );
    assert!(Client::new(config(), &fake)
        .refresh("r", 0)
        .unwrap_err()
        .session_is_dead());
}

#[test]
fn ending_a_session_refreshes_an_expired_access_token_first() {
    let expired = Session {
        access_token: "old-access".into(),
        refresh_token: "refresh-0".into(),
        expires_at: 100,
        user_id: "u".into(),
        email: None,
    };
    // The fake answers the refresh with a fresh session, then the logout (which ignores the body).
    let fake = Fake::new(200, token_answer());
    Client::new(config(), &fake)
        .end_session(&expired, 1_000)
        .unwrap();
    let sent = fake.sent.borrow();
    assert!(sent[0].0.ends_with("grant_type=refresh_token"));
    assert!(sent[1].0.ends_with("/logout?scope=local"));
    assert!(sent[1]
        .1
        .contains(&("Authorization".into(), "Bearer access-1".into())));

    // A live one is ended with its own token.
    let fake = Fake::new(204, json!({}));
    Client::new(config(), &fake)
        .end_session(&expired, 0)
        .unwrap();
    assert!(fake.sent.borrow()[0]
        .1
        .contains(&("Authorization".into(), "Bearer old-access".into())));

    // An already dead one has nothing left to end.
    let fake = Fake::new(400, json!({ "error_code": "refresh_token_not_found" }));
    Client::new(config(), &fake)
        .end_session(&expired, 1_000)
        .unwrap();
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
fn the_challenge_is_the_rfc_7636_example() {
    // The worked example in RFC 7636, appendix B.
    assert_eq!(
        Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".into()).challenge,
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
    let a = Pkce::new().unwrap();
    let b = Pkce::new().unwrap();
    assert_eq!(a.verifier.len(), 43);
    assert_ne!(a.verifier, b.verifier);
}

#[test]
fn the_account_view_is_tagged_for_the_ui() {
    assert_eq!(
        serde_json::to_value(AccountView::SignedOut { warning: None }).unwrap(),
        json!({ "status": "signedOut", "warning": null })
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
