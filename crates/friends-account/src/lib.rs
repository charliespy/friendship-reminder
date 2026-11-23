//! Signing in to a Friends account, kept in Supabase Auth so a later iPhone app can share the data.
//!
//! The app signs in with Apple: macOS hands it an identity token from Apple, and [`Client`] trades that
//! token for a Supabase session. The session's refresh token is what the app keeps (in the Keychain) to
//! stay signed in across launches. All the HTTP goes through [`Transport`], so the rules here are tested
//! without a network.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// How early a session counts as expired, in seconds, so a request never goes out with a token that
/// runs out on the way.
const EXPIRY_MARGIN: i64 = 60;

/// The longest the app waits for Supabase to answer.
const TIMEOUT: Duration = Duration::from_secs(20);

/// Where the account lives: the Supabase project's URL and its publishable key. The key is meant to be
/// built into apps; the database's own rules decide what each signed-in user may read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub url: String,
    pub publishable_key: String,
}

impl Config {
    /// A config from build settings, or `None` when either is missing or empty (a build made without
    /// them, like the tests and most development builds, has no account).
    pub fn new(url: Option<&str>, publishable_key: Option<&str>) -> Option<Config> {
        let url = url?.trim().trim_end_matches('/');
        let publishable_key = publishable_key?.trim();
        if url.is_empty() || publishable_key.is_empty() {
            return None;
        }
        Some(Config {
            url: url.to_string(),
            publishable_key: publishable_key.to_string(),
        })
    }
}

/// A signed-in session. Saved as JSON in the Keychain between launches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    /// When the access token runs out, in seconds since 1970.
    pub expires_at: i64,
    pub user_id: String,
    /// The email Apple shared: the real one, or a private relay address if the user chose to hide it.
    pub email: Option<String>,
}

impl Session {
    /// Whether the access token has run out (or is about to) and needs a refresh before use.
    pub fn needs_refresh(&self, now: i64) -> bool {
        now >= self.expires_at - EXPIRY_MARGIN
    }
}

/// What the Account section shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum AccountView {
    /// This build can't sign in (no Supabase settings, or not signed for Sign in with Apple).
    Unavailable {
        reason: String,
    },
    SignedOut,
    #[serde(rename_all = "camelCase")]
    SignedIn {
        email: Option<String>,
        /// Something the user should know about the sign-in, like the Keychain refusing to save it.
        warning: Option<String>,
    },
}

/// Why talking to the account failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// Supabase said no: the token or session is invalid or expired. A saved session that gets this
    /// is no good any more, so the app signs out.
    Rejected(String),
    /// Supabase couldn't be reached (offline, or it timed out). A saved session is kept, to try again.
    Unreachable(String),
    /// Supabase answered with a server error. A saved session is kept, to try again.
    Server(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Rejected(m) => write!(f, "Sign-in was refused: {m}"),
            AuthError::Unreachable(_) => {
                f.write_str("Couldn't reach the Friends account. Check your internet connection.")
            }
            AuthError::Server(_) => {
                f.write_str("The Friends account isn't answering right now. Try again later.")
            }
        }
    }
}

impl std::error::Error for AuthError {}

/// An HTTP answer: its status code and body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body: String,
}

/// Sends a POST. `Err` means no answer came back at all.
pub trait Transport {
    fn post(&self, url: &str, headers: &[(&str, &str)], body: &str) -> Result<Response, String>;
}

/// The real transport, over HTTPS.
pub struct HttpTransport {
    agent: ureq::Agent,
}

impl Default for HttpTransport {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(TIMEOUT))
            .build()
            .new_agent();
        HttpTransport { agent }
    }
}

impl Transport for HttpTransport {
    fn post(&self, url: &str, headers: &[(&str, &str)], body: &str) -> Result<Response, String> {
        let mut req = self.agent.post(url);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let mut res = req.send(body).map_err(|e| e.to_string())?;
        let status = res.status().as_u16();
        let body = res.body_mut().read_to_string().unwrap_or_default();
        Ok(Response { status, body })
    }
}

/// Talks to Supabase Auth.
pub struct Client<T: Transport> {
    config: Config,
    transport: T,
}

impl<T: Transport> Client<T> {
    pub fn new(config: Config, transport: T) -> Self {
        Client { config, transport }
    }

    /// Trades Apple's identity token for a session. `nonce` is the raw nonce whose hash went to Apple,
    /// so Supabase can check the token was made for this sign-in.
    pub fn sign_in_with_apple(
        &self,
        id_token: &str,
        nonce: &str,
        now: i64,
    ) -> Result<Session, AuthError> {
        let body = json!({ "provider": "apple", "id_token": id_token, "nonce": nonce });
        self.token("id_token", &body, now)
    }

    /// A fresh session from a saved refresh token. Supabase hands out a new refresh token each time and
    /// the old one stops working soon after, so the caller must save the returned session.
    pub fn refresh(&self, refresh_token: &str, now: i64) -> Result<Session, AuthError> {
        let body = json!({ "refresh_token": refresh_token });
        self.token("refresh_token", &body, now)
    }

    /// Ends the session on Supabase's side (this device only). An already invalid session counts as
    /// signed out.
    pub fn sign_out(&self, access_token: &str) -> Result<(), AuthError> {
        let url = format!("{}/auth/v1/logout?scope=local", self.config.url);
        let bearer = format!("Bearer {access_token}");
        let res = self.send(&url, &[("Authorization", &bearer)], "")?;
        match res.status {
            200..=299 | 401 | 403 | 404 => Ok(()),
            _ => Err(error_for(&res)),
        }
    }

    fn token(&self, grant_type: &str, body: &Value, now: i64) -> Result<Session, AuthError> {
        let url = format!("{}/auth/v1/token?grant_type={grant_type}", self.config.url);
        let res = self.send(&url, &[], &body.to_string())?;
        if !(200..300).contains(&res.status) {
            return Err(error_for(&res));
        }
        parse_session(&res.body, now)
            .ok_or_else(|| AuthError::Server("unexpected answer from Supabase".to_string()))
    }

    fn send(&self, url: &str, extra: &[(&str, &str)], body: &str) -> Result<Response, AuthError> {
        let mut headers = vec![
            ("apikey", self.config.publishable_key.as_str()),
            ("Content-Type", "application/json"),
        ];
        headers.extend_from_slice(extra);
        self.transport
            .post(url, &headers, body)
            .map_err(AuthError::Unreachable)
    }
}

fn error_for(res: &Response) -> AuthError {
    let message = serde_json::from_str::<Value>(&res.body)
        .ok()
        .and_then(|v| {
            ["msg", "error_description", "message", "error"]
                .iter()
                .find_map(|k| v.get(*k).and_then(Value::as_str).map(str::to_string))
        })
        .unwrap_or_else(|| format!("HTTP {}", res.status));
    match res.status {
        400..=499 if res.status != 408 && res.status != 429 => AuthError::Rejected(message),
        _ => AuthError::Server(message),
    }
}

fn parse_session(body: &str, now: i64) -> Option<Session> {
    let v: Value = serde_json::from_str(body).ok()?;
    let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    let expires_at = v
        .get("expires_at")
        .and_then(Value::as_i64)
        .or_else(|| Some(now + v.get("expires_in")?.as_i64()?))?;
    let user = v.get("user")?;
    Some(Session {
        access_token: text("access_token")?,
        refresh_token: text("refresh_token")?,
        expires_at,
        user_id: user.get("id")?.as_str()?.to_string(),
        email: user
            .get("email")
            .and_then(Value::as_str)
            .filter(|e| !e.is_empty())
            .map(str::to_string),
    })
}

/// A one-time value tying Apple's token to this sign-in: Apple gets the hash, Supabase the raw value,
/// and Supabase checks they match, so a token taken from somewhere else can't be replayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nonce {
    pub raw: String,
    /// SHA-256 of `raw`, as lowercase hex.
    pub hashed: String,
}

impl Nonce {
    pub fn new() -> Result<Nonce, String> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
        Ok(Nonce::from_raw(hex(&bytes)))
    }

    pub fn from_raw(raw: String) -> Nonce {
        let hashed = hex(&Sha256::digest(raw.as_bytes()));
        Nonce { raw, hashed }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
