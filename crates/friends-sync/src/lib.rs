//! Talks to the account's database (Supabase's REST API) to upload and download the list. What to
//! upload and how to take in what comes down are friends-core's rules ([`friends_core::sync`]); the
//! tables and their privacy rules are in `supabase/migrations/`.
//!
//! All the HTTP goes through friends-account's [`Transport`], so this is tested without a network.

use std::sync::Mutex;

use chrono::NaiveDate;
use friends_account::{Config, Response, Session, Transport};
use friends_core::api::App;
use friends_core::sync::{Changes, RemoteBlocked, RemotePerson, RemoteSettings};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

/// How far back each download reaches before the cursor, in ms. A write the server stamped just before
/// another but saved just after it would otherwise be missed; taking a change in twice is harmless.
pub const OVERLAP_MS: i64 = 30_000;

/// Rows per request, up and down.
const PAGE: usize = 500;

const PEOPLE: &str = "people";
const BLOCKED: &str = "blocked_dates";
const SETTINGS: &str = "settings";

/// Why a sync failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    /// The sign-in's access token ran out or was refused: refresh it and try again.
    Expired,
    /// The account couldn't be reached (offline, or it timed out).
    Unreachable(String),
    /// The account answered with an error, or the list couldn't be saved.
    Failed(String),
    /// Renewing the sign-in didn't work; says why.
    Renew(String),
    /// Signed out (or into another account) while syncing: the sync stopped.
    SignedOut,
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::Expired => {
                f.write_str("Your sign-in needs renewing. Friends will try again.")
            }
            SyncError::Unreachable(_) => {
                f.write_str("Couldn't reach your Friends account. Friends will try again.")
            }
            SyncError::Failed(m) => write!(f, "Couldn't sync with your Friends account: {m}"),
            SyncError::Renew(m) => f.write_str(m),
            SyncError::SignedOut => f.write_str("Signed out before syncing finished."),
        }
    }
}

impl std::error::Error for SyncError {}

/// What the Account section shows about syncing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncView {
    /// A sync is running now.
    pub syncing: bool,
    /// When the last sync finished, in ms since 1970.
    pub last_synced: Option<i64>,
    /// Why the last sync failed, if it did.
    pub error: Option<String>,
}

/// The account's database.
pub struct Server<T: Transport> {
    config: Config,
    transport: T,
}

/// A downloaded row's place in the download order: the server's time of its last write, then its id.
trait Keyed {
    fn key(&self) -> (i64, &str);
}

impl Keyed for RemotePerson {
    fn key(&self) -> (i64, &str) {
        (self.updated_ms, &self.id)
    }
}

impl Keyed for RemoteBlocked {
    fn key(&self) -> (i64, &str) {
        (self.updated_ms, &self.id)
    }
}

impl<T: Transport> Server<T> {
    pub fn new(config: Config, transport: T) -> Self {
        Server { config, transport }
    }

    /// Uploads `changes` as user `owner` (signed in with `token`). The server keeps whichever change is
    /// newer, so uploading something it already has, or something older, is harmless.
    pub fn push(&self, token: &str, owner: &str, changes: &Changes) -> Result<(), SyncError> {
        for page in changes.people.chunks(PAGE) {
            self.upsert(token, PEOPLE, "owner,id", owner, page)?;
        }
        for page in changes.blocked.chunks(PAGE) {
            self.upsert(token, BLOCKED, "owner,id", owner, page)?;
        }
        if let Some(s) = &changes.settings {
            self.upsert(token, SETTINGS, "owner", owner, std::slice::from_ref(s))?;
        }
        Ok(())
    }

    /// Downloads everything that changed since `cursor` (minus [`OVERLAP_MS`]), with the new cursor
    /// (the newest server write seen).
    pub fn pull(&self, token: &str, cursor: i64) -> Result<(Changes, i64), SyncError> {
        let since = cursor.saturating_sub(OVERLAP_MS);
        let people: Vec<RemotePerson> = self.select_all(
            token,
            PEOPLE,
            "id,name,frequency_days,last_connected,deleted,modified_at,updated_ms",
            since,
        )?;
        let blocked: Vec<RemoteBlocked> = self.select_all(
            token,
            BLOCKED,
            "id,starts_on,ends_on,label,deleted,modified_at,updated_ms",
            since,
        )?;
        // One row per account, so one request.
        let url = format!(
            "{}/rest/v1/{SETTINGS}?select=day_toggle,custom_days,modified_at,updated_ms&updated_ms=gt.{since}",
            self.config.url
        );
        let settings: Vec<RemoteSettings> = self.get(token, &url)?;
        let newest = people
            .iter()
            .map(|r| r.updated_ms)
            .chain(blocked.iter().map(|r| r.updated_ms))
            .chain(settings.iter().map(|r| r.updated_ms))
            .fold(cursor, i64::max);
        let changes = Changes {
            people,
            blocked,
            settings: settings.into_iter().next(),
        };
        Ok((changes, newest))
    }

    fn headers<'a>(&'a self, bearer: &'a str) -> Vec<(&'a str, &'a str)> {
        vec![
            ("apikey", self.config.publishable_key.as_str()),
            ("Authorization", bearer),
        ]
    }

    fn upsert<R: Serialize>(
        &self,
        token: &str,
        table: &str,
        conflict: &str,
        owner: &str,
        rows: &[R],
    ) -> Result<(), SyncError> {
        let rows: Vec<Value> = rows
            .iter()
            .map(|r| {
                let mut v = serde_json::to_value(r).expect("rows serialize");
                v["owner"] = Value::String(owner.to_string());
                v
            })
            .collect();
        let url = format!("{}/rest/v1/{table}?on_conflict={conflict}", self.config.url);
        let bearer = format!("Bearer {token}");
        let mut headers = self.headers(&bearer);
        headers.push(("Content-Type", "application/json"));
        headers.push(("Prefer", "resolution=merge-duplicates,return=minimal"));
        let res = self
            .transport
            .post(&url, &headers, &Value::Array(rows).to_string())
            .map_err(SyncError::Unreachable)?;
        check(&res)
    }

    /// Every row of `table` written after `since`, a page at a time in (server time, id) order. Each
    /// page starts after the last row of the one before, so a row written meanwhile moves to the end
    /// instead of shifting the pages and making one get skipped.
    fn select_all<R: DeserializeOwned + Keyed>(
        &self,
        token: &str,
        table: &str,
        columns: &str,
        since: i64,
    ) -> Result<Vec<R>, SyncError> {
        let mut out: Vec<R> = Vec::new();
        loop {
            let after = match out.last().map(Keyed::key) {
                None => format!("updated_ms=gt.{since}"),
                Some((ms, id)) => {
                    format!("or=(updated_ms.gt.{ms},and(updated_ms.eq.{ms},id.gt.{id}))")
                }
            };
            let url = format!(
                "{}/rest/v1/{table}?select={columns}&{after}&order=updated_ms.asc,id.asc&limit={PAGE}",
                self.config.url
            );
            let page: Vec<R> = self.get(token, &url)?;
            let full = page.len() == PAGE;
            out.extend(page);
            if !full {
                return Ok(out);
            }
        }
    }

    fn get<R: DeserializeOwned>(&self, token: &str, url: &str) -> Result<Vec<R>, SyncError> {
        let bearer = format!("Bearer {token}");
        let res = self
            .transport
            .get(url, &self.headers(&bearer))
            .map_err(SyncError::Unreachable)?;
        check(&res)?;
        serde_json::from_str(&res.body)
            .map_err(|e| SyncError::Failed(format!("the account sent something unexpected ({e})")))
    }
}

fn lock(app: &Mutex<App>) -> std::sync::MutexGuard<'_, App> {
    app.lock().unwrap_or_else(|e| e.into_inner())
}

/// One sync of `app` with the account as `session`: uploads what changed here, then takes in what
/// changed elsewhere. If the server refuses the access token, `renew` gets a fresh session (`None`
/// when that signed out) and the sync runs once more with it. `still_signed_in(user)` is asked before
/// each step, so a sign-out (or a switch to another account) stops the sync. Returns whether the list
/// changed, so the windows need the new view.
pub fn sync<T: Transport>(
    server: &Server<T>,
    session: &Session,
    app: &Mutex<App>,
    today: NaiveDate,
    still_signed_in: impl Fn(&str) -> bool,
    renew: impl FnOnce() -> Result<Option<Session>, String>,
) -> Result<bool, SyncError> {
    match sync_once(server, session, app, today, &still_signed_in) {
        Err(SyncError::Expired) => match renew() {
            Ok(Some(fresh)) => sync_once(server, &fresh, app, today, &still_signed_in),
            Ok(None) => Err(SyncError::SignedOut),
            Err(e) => Err(SyncError::Renew(e)),
        },
        other => other,
    }
}

fn sync_once<T: Transport>(
    server: &Server<T>,
    session: &Session,
    app: &Mutex<App>,
    today: NaiveDate,
    still_signed_in: &impl Fn(&str) -> bool,
) -> Result<bool, SyncError> {
    let user = session.user_id.as_str();
    let token = session.access_token.as_str();
    let saving = |e: std::io::Error| SyncError::Failed(format!("couldn't save the list ({e})"));
    let current = || {
        if still_signed_in(user) {
            Ok(())
        } else {
            Err(SyncError::SignedOut)
        }
    };
    current()?;
    let changes = lock(app).sync_start(user).map_err(saving)?;
    if !changes.is_empty() {
        server.push(token, user, &changes)?;
        lock(app).sync_pushed(&changes).map_err(saving)?;
    }
    let cursor = lock(app).sync_cursor();
    let (got, newest) = server.pull(token, cursor)?;
    // Asked again before taking the download in, so a sign-out while it came down stops it.
    let mut core = lock(app);
    current()?;
    core.sync_pulled(&got, newest, today).map_err(saving)
}

fn check(res: &Response) -> Result<(), SyncError> {
    if (200..300).contains(&res.status) {
        return Ok(());
    }
    if res.status == 401 {
        return Err(SyncError::Expired);
    }
    let body = serde_json::from_str::<Value>(&res.body).ok();
    let field = |k: &str| {
        body.as_ref()
            .and_then(|v| v.get(k))
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    // PostgREST's code for a table it doesn't know: the database setup hasn't been run.
    if field("code").as_deref() == Some("PGRST205") {
        return Err(SyncError::Failed(
            "the account's database isn't set up yet".to_string(),
        ));
    }
    let message = field("message").unwrap_or_else(|| format!("HTTP {}", res.status));
    Err(SyncError::Failed(message))
}
