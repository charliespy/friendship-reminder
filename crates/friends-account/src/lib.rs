//! Signing in to a Friendship Reminder account, kept in Supabase Auth so a later iPhone app can share the data.
//!
//! The app signs in with Apple in the browser: [`Client::authorize_url`] is the page to open, the
//! browser comes back to the app with a one-time code ([`parse_callback`]), and
//! [`Client::exchange_code`] trades it for a session. [`AccountState`] holds the session and decides
//! what each outcome does to it; the app keeps the session in the Keychain to stay signed in across
//! launches. All the HTTP goes through [`Transport`] and the Keychain through a `save` function, so
//! every rule here is tested without a network or a Mac.

use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// How early a session counts as expired, in seconds, so a request never goes out with a token that
/// runs out on the way.
const EXPIRY_MARGIN: i64 = 60;

/// The largest answer the app reads, in bytes: a page of a long list fits easily.
const MAX_BODY: u64 = 16 * 1024 * 1024;

/// The longest the app waits for Supabase to answer.
const TIMEOUT: Duration = Duration::from_secs(20);

/// What a query value escapes: everything but RFC 3986's unreserved characters.
const QUERY_VALUE: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// The path of the app's sign-in address, where the browser comes back.
pub const CALLBACK_PATH: &str = "/auth/callback";

/// Shown when the Keychain wouldn't save a new or refreshed session.
pub const NOT_SAVED: &str =
    "Couldn't save your sign-in in the Keychain, so you may need to sign in again after Friendship Reminder restarts.";

/// Shown when the saved session turned out to have ended but the Keychain wouldn't let it go.
pub const NOT_REMOVED: &str =
    "Your sign-in ended, but Friendship Reminder couldn't remove it from the Keychain. Sign in again.";

/// Shown when the Keychain wouldn't say whether a session was saved.
pub const NOT_READ: &str =
    "Couldn't read your sign-in from the Keychain. Restart Friendship Reminder, or sign in again.";

/// Where the account lives: the Supabase project's URL and its publishable key. The key is meant to be
/// built into apps; the database's own rules decide what each signed-in user may read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub url: String,
    pub publishable_key: String,
}

/// The setting names, in the environment and in `.env.local`.
pub const URL_SETTING: &str = "FRIENDS_SUPABASE_URL";
pub const KEY_SETTING: &str = "FRIENDS_SUPABASE_PUBLISHABLE_KEY";

impl Config {
    /// A config from build settings, or `None` when either is missing or empty (a build made without
    /// them, like the tests, has no account).
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

    /// A config from the text of a `.env` file (see [`env_file_value`]).
    pub fn from_env_file(text: &str) -> Option<Config> {
        Config::new(
            env_file_value(text, URL_SETTING).as_deref(),
            env_file_value(text, KEY_SETTING).as_deref(),
        )
    }
}

/// The value of `key` in a `.env` file: `KEY=value`, optionally after `export`, optionally quoted, with
/// `#` comments on their own line or after an unquoted value. The last one wins, as when the shell
/// reads the file (`. ./.env.local`).
pub fn env_file_value(text: &str, key: &str) -> Option<String> {
    text.lines().rev().find_map(|line| {
        let line = line.trim();
        let line = line.strip_prefix("export ").unwrap_or(line);
        let (k, v) = line.split_once('=')?;
        if k.trim() != key {
            return None;
        }
        let v = v.trim();
        let quoted = ['"', '\''].into_iter().find_map(|q| {
            v.strip_prefix(q)
                .and_then(|rest| rest.split_once(q))
                .map(|(inner, _)| inner)
        });
        let value = quoted.unwrap_or_else(|| v.split(" #").next().unwrap_or(v).trim());
        Some(value.to_string())
    })
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
    /// This build can't sign in: it was built without the account server's settings.
    Unavailable { reason: String },
    #[serde(rename_all = "camelCase")]
    SignedOut {
        /// Something the user should know, like the Keychain refusing to read a saved sign-in.
        warning: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    SignedIn {
        email: Option<String>,
        /// Something the user should know, like the Keychain refusing to save the sign-in.
        warning: Option<String>,
    },
}

/// Why talking to the account failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// Supabase said the session, token or sign-in code is no good (expired, revoked, already used).
    /// A saved session that gets this is dead, so the app signs out.
    Rejected(String),
    /// Supabase refused the request for another reason (a wrong app key, a bad request). That says
    /// nothing about the session, so a saved one is kept.
    Refused(String),
    /// Supabase couldn't be reached (offline, or it timed out). A saved session is kept, to try again.
    Unreachable(String),
    /// Supabase answered with a server error. A saved session is kept, to try again.
    Server(String),
}

impl AuthError {
    /// Whether the session this came from is gone for good.
    pub fn session_is_dead(&self) -> bool {
        matches!(self, AuthError::Rejected(_))
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Rejected(m) | AuthError::Refused(m) => write!(f, "Sign-in was refused: {m}"),
            AuthError::Unreachable(_) => f.write_str(
                "Couldn't reach the Friendship Reminder account. Check your internet connection.",
            ),
            AuthError::Server(_) => f.write_str(
                "The Friendship Reminder account isn't answering right now. Try again later.",
            ),
        }
    }
}

impl std::error::Error for AuthError {}

/// Supabase's error codes that mean the session, refresh token or sign-in code itself is no good.
const DEAD_SESSION_CODES: [&str; 10] = [
    "refresh_token_not_found",
    "refresh_token_already_used",
    "session_not_found",
    "session_expired",
    "user_not_found",
    "user_banned",
    "invalid_grant",
    "flow_state_not_found",
    "flow_state_expired",
    "bad_code_verifier",
];

/// An HTTP answer: its status code and body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body: String,
}

/// Sends a request. `Err` means no answer came back at all.
pub trait Transport {
    fn post(&self, url: &str, headers: &[(&str, &str)], body: &str) -> Result<Response, String>;

    /// Sends a GET. Sign-in never needs one, so a transport for it alone may leave this out.
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Response, String> {
        let _ = (url, headers);
        Err("this transport can't send a GET".to_string())
    }
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
        read(req.send(body).map_err(|e| e.to_string())?)
    }

    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Response, String> {
        let mut req = self.agent.get(url);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        read(req.call().map_err(|e| e.to_string())?)
    }
}

fn read(mut res: ureq::http::Response<ureq::Body>) -> Result<Response, String> {
    let status = res.status().as_u16();
    // A body cut off halfway is a failed request, not an empty answer: a refresh that lost its
    // answer must not look like one Supabase refused.
    let body = res
        .body_mut()
        .with_config()
        .limit(MAX_BODY)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    Ok(Response { status, body })
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

    /// The page that starts a sign-in with `provider` (like `"apple"`) in the browser. When it's done,
    /// Supabase sends the browser to `redirect_to` with a one-time `code` for [`Client::exchange_code`].
    /// `redirect_to` must be on the project's list of allowed redirect URLs.
    pub fn authorize_url(&self, provider: &str, redirect_to: &str, pkce: &Pkce) -> String {
        format!(
            "{}/auth/v1/authorize?provider={}&redirect_to={}&code_challenge={}&code_challenge_method=s256",
            self.config.url,
            utf8_percent_encode(provider, QUERY_VALUE),
            utf8_percent_encode(redirect_to, QUERY_VALUE),
            pkce.challenge,
        )
    }

    /// Trades the one-time code from the browser for a session. Only the app that started the sign-in
    /// holds `verifier`, so a code that leaks on the way is no use to anyone else.
    pub fn exchange_code(
        &self,
        code: &str,
        verifier: &str,
        now: i64,
    ) -> Result<Session, AuthError> {
        let body = json!({ "auth_code": code, "code_verifier": verifier });
        self.token("pkce", &body, now)
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

    /// Ends a session the app is letting go of. Its access token may have run out while the app ran
    /// (a menu bar app runs for days), and Supabase won't end a session with an expired one, so that is
    /// refreshed first.
    pub fn end_session(&self, session: &Session, now: i64) -> Result<(), AuthError> {
        if session.needs_refresh(now) {
            match self.refresh(&session.refresh_token, now) {
                Ok(fresh) => self.sign_out(&fresh.access_token),
                // Already dead: nothing left to end.
                Err(e) if e.session_is_dead() => Ok(()),
                Err(e) => Err(e),
            }
        } else {
            self.sign_out(&session.access_token)
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
    let body = serde_json::from_str::<Value>(&res.body).ok();
    let text = |keys: &[&str]| {
        body.as_ref().and_then(|v| {
            keys.iter()
                .find_map(|k| v.get(*k).and_then(Value::as_str).map(str::to_string))
        })
    };
    let message = text(&["msg", "error_description", "message", "error"])
        .unwrap_or_else(|| format!("HTTP {}", res.status));
    let code = text(&["error_code", "error"]).unwrap_or_default();
    match res.status {
        408 | 429 | 500..=599 => AuthError::Server(message),
        400..=499 if DEAD_SESSION_CODES.contains(&code.as_str()) => AuthError::Rejected(message),
        400..=499 => AuthError::Refused(message),
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

/// What to do after [`AccountState::start_restore`].
#[derive(Debug, PartialEq, Eq)]
pub enum Restore {
    /// Nothing more.
    Done,
    /// Refresh this session, then pass the result to [`AccountState::finish_restore`].
    Refresh { generation: u64, session: Session },
    /// End this old session on the server.
    End(Session),
}

/// Who is signed in, and the rules for changing it. Every change bumps `generation`, so work that
/// started from an older state (a refresh at launch that finishes after the user signed in or out) can
/// tell, and its result is dropped. Each change is saved with the given `save` function (the Keychain)
/// together with the change in memory, so the two never disagree.
#[derive(Debug, Default)]
pub struct AccountState {
    generation: u64,
    session: Option<Session>,
    warning: Option<String>,
}

impl AccountState {
    pub fn view(&self) -> AccountView {
        let warning = self.warning.clone();
        match &self.session {
            Some(s) => AccountView::SignedIn {
                email: s.email.clone(),
                warning,
            },
            None => AccountView::SignedOut { warning },
        }
    }

    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// At launch, what the Keychain held, read after noting [`AccountState::generation`] as `before`.
    /// Shows a saved session as signed in right away, to be refreshed. If the account changed while
    /// the Keychain was read (a sign-in or a sign-out), what it held no longer speaks for it: a saved
    /// session other than the current one is handed back to end on the server.
    pub fn start_restore(
        &mut self,
        before: u64,
        saved: Result<Option<Session>, String>,
    ) -> Restore {
        if before != self.generation || self.session.is_some() {
            return match saved {
                Ok(Some(old)) if self.session.as_ref() != Some(&old) => Restore::End(old),
                _ => Restore::Done,
            };
        }
        match saved {
            Err(_) => {
                self.warning = Some(NOT_READ.to_string());
                self.generation += 1;
                Restore::Done
            }
            Ok(None) => Restore::Done,
            Ok(Some(session)) => {
                self.session = Some(session.clone());
                self.generation += 1;
                Restore::Refresh {
                    generation: self.generation,
                    session,
                }
            }
        }
    }

    /// The refresh started by [`AccountState::start_restore`] came back. A fresh session replaces the
    /// saved one; a dead one signs out; anything else (offline, a server error) keeps the saved one to
    /// try again at the next launch. Dropped if the account changed meanwhile (a sign-in or a
    /// sign-out); a fresh session dropped that way comes back as `Ok(Some(_))`, to end on the server,
    /// since the refresh replaced the tokens a sign-out would have ended it with.
    pub fn finish_restore(
        &mut self,
        generation: u64,
        refreshed: Result<Session, AuthError>,
        save: impl FnOnce(Option<&Session>) -> Result<(), String>,
    ) -> Result<Option<Session>, String> {
        if generation != self.generation {
            return Ok(refreshed.ok());
        }
        match refreshed {
            Ok(fresh) => self.keep(Some(fresh), save).map(|_| None),
            // Signed out either way: a dead session can't do anything. If the Keychain keeps it, the
            // next launch finds it dead again.
            Err(e) if e.session_is_dead() => match self.sign_out(save) {
                Ok(_) => Ok(None),
                Err(e) => {
                    self.session = None;
                    self.warning = Some(NOT_REMOVED.to_string());
                    self.generation += 1;
                    Err(e)
                }
            },
            Err(_) => Ok(None),
        }
    }

    /// A sign-in finished. If the Keychain won't save it, the session is kept for now with a warning.
    /// Returns the session it replaced (the one restored at launch, say), to end on the server, and how
    /// saving went.
    pub fn signed_in(
        &mut self,
        session: Session,
        save: impl FnOnce(Option<&Session>) -> Result<(), String>,
    ) -> (Option<Session>, Result<(), String>) {
        let replaced = self.session.take();
        (replaced, self.keep(Some(session), save))
    }

    /// Signs out, but only once the Keychain has let go of the session: one left behind would sign the
    /// user back in at the next launch. On an error nothing changes. Returns the session that was
    /// signed in, to end on the server.
    pub fn sign_out(
        &mut self,
        save: impl FnOnce(Option<&Session>) -> Result<(), String>,
    ) -> Result<Option<Session>, String> {
        save(None)?;
        self.warning = None;
        self.generation += 1;
        Ok(self.session.take())
    }

    fn keep(
        &mut self,
        session: Option<Session>,
        save: impl FnOnce(Option<&Session>) -> Result<(), String>,
    ) -> Result<(), String> {
        let saved = save(session.as_ref());
        self.warning = saved.as_ref().err().map(|_| NOT_SAVED.to_string());
        self.session = session;
        self.generation += 1;
        saved
    }
}

/// The browser sign-ins: which one the browser may come back to. The window names each attempt, so a
/// Cancel that arrives before its sign-in starts waiting still stops it, and starting another stops the
/// one waiting. `T` is how to wake a waiting sign-in.
#[derive(Debug)]
pub struct Attempts<T> {
    current: Option<(String, T)>,
    /// An attempt cancelled before it started waiting.
    cancelled: Option<String>,
}

impl<T> Default for Attempts<T> {
    fn default() -> Self {
        Attempts {
            current: None,
            cancelled: None,
        }
    }
}

/// What [`Attempts::start`] made of a sign-in.
#[derive(Debug, PartialEq, Eq)]
pub enum Started<T> {
    /// Cancelled before it began: don't open the browser.
    Cancelled,
    /// Now the one waiting; wake the one it replaced, if any, so it stops.
    Waiting { replaced: Option<T> },
}

impl<T> Attempts<T> {
    pub fn start(&mut self, attempt: &str, wake: T) -> Started<T> {
        if self.cancelled.as_deref() == Some(attempt) {
            self.cancelled = None;
            return Started::Cancelled;
        }
        let replaced = self
            .current
            .replace((attempt.to_string(), wake))
            .map(|(_, w)| w);
        Started::Waiting { replaced }
    }

    /// Cancels `attempt`: returns how to wake it if it's waiting, or remembers it for when it starts.
    pub fn cancel(&mut self, attempt: &str) -> Option<T> {
        if self.is_waiting(attempt) {
            return self.current.take().map(|(_, w)| w);
        }
        self.cancelled = Some(attempt.to_string());
        None
    }

    /// Cancels whichever attempt is waiting (Preferences, which has its Cancel, closed): returns how
    /// to wake it.
    pub fn cancel_all(&mut self) -> Option<T> {
        self.current.take().map(|(_, w)| w)
    }

    pub fn is_waiting(&self, attempt: &str) -> bool {
        self.current.as_ref().is_some_and(|(a, _)| a == attempt)
    }

    /// Ends `attempt`'s wait: `true` if it was still the one the user wants (not cancelled or
    /// replaced), so its session should be kept; `false` means end that session instead.
    pub fn finish(&mut self, attempt: &str) -> bool {
        if self.cancelled.as_deref() == Some(attempt) {
            self.cancelled = None;
        }
        let wanted = self.is_waiting(attempt);
        if wanted {
            self.current = None;
        }
        wanted
    }
}

/// The secret pair that ties a browser sign-in to this app (PKCE): Supabase gets the challenge when the
/// sign-in starts, and the code it hands back only works together with the verifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pkce {
    pub verifier: String,
    /// SHA-256 of `verifier`, in URL-safe base64 without padding.
    pub challenge: String,
}

impl Pkce {
    pub fn new() -> Result<Pkce, String> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
        Ok(Pkce::from_verifier(URL_SAFE_NO_PAD.encode(bytes)))
    }

    pub fn from_verifier(verifier: String) -> Pkce {
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Pkce {
            verifier,
            challenge,
        }
    }
}

/// What came back to the app's sign-in address from the browser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Callback {
    /// The sign-in worked: the one-time code to trade for a session.
    Code(String),
    /// The sign-in didn't work (refused, or cancelled on Apple's page), with the reason given.
    Failed(String),
    /// Some other request (the browser asking for an icon), to answer and ignore.
    Other,
}

/// The address an app's own sign-in sheet comes back to: `<scheme>://auth/callback`. (Supabase's list
/// of allowed addresses only takes ones written with `//`.)
pub fn app_redirect(scheme: &str) -> String {
    format!("{scheme}:/{CALLBACK_PATH}")
}

/// Reads the address the sign-in sheet came back to, made by [`app_redirect`].
pub fn parse_app_callback(scheme: &str, back: &str) -> Callback {
    match back.strip_prefix(scheme).and_then(|t| t.strip_prefix(":/")) {
        Some(target) => parse_callback(target),
        None => Callback::Other,
    }
}

/// Reads the path and query of a request to the app's sign-in address, `/auth/callback?...`.
pub fn parse_callback(target: &str) -> Callback {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path != CALLBACK_PATH {
        return Callback::Other;
    }
    let param = |name: &str| {
        form_urlencoded::parse(query.as_bytes())
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
    };
    if let Some(code) = param("code").filter(|c| !c.is_empty()) {
        return Callback::Code(code);
    }
    // Only an answer that says it failed ends the sign-in; anything else (a browser fetching the page
    // ahead of time, a reload without its query) is ignored, and the real answer can still come.
    match param("error") {
        Some(error) => Callback::Failed(param("error_description").unwrap_or(error)),
        None => Callback::Other,
    }
}
