//! Talks to the account's database (Supabase's REST API) to upload and download the list. What to
//! upload and how to take in what comes down are friends-core's rules ([`friends_core::sync`]); the
//! tables and their privacy rules are in `supabase/migrations/`.
//!
//! All the HTTP goes through friends-account's [`Transport`], so this is tested without a network.

use friends_account::{Config, Response, Transport};
use friends_core::sync::{Changes, RemoteBlocked, RemotePerson, RemoteSettings};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
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
    /// The account answered with an error.
    Failed(String),
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

#[derive(Deserialize)]
struct Row<T> {
    #[serde(flatten)]
    row: T,
    updated_ms: i64,
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

    /// Downloads everything that changed since `cursor` (minus [`OVERLAP_MS`]), with the new cursor.
    pub fn pull(&self, token: &str, cursor: i64) -> Result<(Changes, i64), SyncError> {
        let since = cursor.saturating_sub(OVERLAP_MS);
        let mut newest = cursor;
        let people: Vec<Row<RemotePerson>> = self.select(
            token,
            PEOPLE,
            "id,name,frequency_days,last_connected,deleted,modified_at,updated_ms",
            since,
        )?;
        let blocked: Vec<Row<RemoteBlocked>> = self.select(
            token,
            BLOCKED,
            "id,starts_on,ends_on,label,deleted,modified_at,updated_ms",
            since,
        )?;
        let settings: Vec<Row<RemoteSettings>> =
            self.select(token, SETTINGS, "day_toggle,modified_at,updated_ms", since)?;
        let mut take = |ms: i64| newest = newest.max(ms);
        people.iter().for_each(|r| take(r.updated_ms));
        blocked.iter().for_each(|r| take(r.updated_ms));
        settings.iter().for_each(|r| take(r.updated_ms));
        let changes = Changes {
            people: people.into_iter().map(|r| r.row).collect(),
            blocked: blocked.into_iter().map(|r| r.row).collect(),
            settings: settings.into_iter().map(|r| r.row).next(),
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

    fn select<R: DeserializeOwned>(
        &self,
        token: &str,
        table: &str,
        columns: &str,
        since: i64,
    ) -> Result<Vec<R>, SyncError> {
        let bearer = format!("Bearer {token}");
        let headers = self.headers(&bearer);
        let mut out = Vec::new();
        loop {
            let url = format!(
                "{}/rest/v1/{table}?select={columns}&updated_ms=gt.{since}&order=updated_ms.asc&limit={PAGE}&offset={}",
                self.config.url,
                out.len()
            );
            let res = self
                .transport
                .get(&url, &headers)
                .map_err(SyncError::Unreachable)?;
            check(&res)?;
            let page: Vec<R> = serde_json::from_str(&res.body).map_err(|e| {
                SyncError::Failed(format!("the account sent something unexpected ({e})"))
            })?;
            let full = page.len() == PAGE;
            out.extend(page);
            if !full {
                return Ok(out);
            }
        }
    }
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
