//! Serves the app's commands over HTTP on 127.0.0.1, so the UI can run in a plain browser (for
//! development and the Playwright end-to-end tests) against the real rules in friends-core.
//!
//! ```text
//! friends-devserver --port 1430 --data /path/to/friends.json [--today 2026-08-19]
//! ```
//!
//! - `POST /invoke` with `{"cmd": "<tauri command>", "args": {...}}` answers like the Tauri command would:
//!   `200` with the result, or `400` with the error.
//! - `POST /__test/reset` with a data file's JSON (or `{}`) replaces the data and clears launch at login.
//! - `GET /__test/health` answers 200 once the server is up.
//! - `POST /__test/today` with `"YYYY-MM-DD"` (a JSON string) moves the server's clock to that day.
//! - `POST /__test/contacts` (a JSON string) sets Contacts' access, in a made-up address book:
//!   - `"notAsked"` (at first): not asked yet; asked, the user allows all contacts.
//!   - `"notAskedLimited"`: not asked yet; asked, the user shares only Ben, Elif and Iris.
//!   - `"allowed"`: all contacts shared already.
//!   - `"limited"`: only Ben, Elif and Iris shared already; sharing more (in iOS Settings) then
//!     shares Kofi and Lena too.
//!   - `"denied"`: the user said no.
//! - `POST /__test/notifications` (a JSON string) sets whether the app may send notifications, as
//!   the iPhone answers: `"allowed"` (at first, as on the Mac), `"notAsked"` (asking allows them) or
//!   `"denied"`.
//! - `POST /__test/open-card` with a made-up card's id (a JSON string) acts as if the app's link on
//!   that card opened the app: the next window to ask shows that person. Answers their id, or null.
//!
//! It only listens on the loopback address and is never part of the app. So that a web page open in
//! the browser can't drive it, it answers only requests addressed to `127.0.0.1:<port>` or
//! `localhost:<port>` (no DNS rebinding) and only JSON POSTs (a page can't send those to another
//! origin without a CORS preflight, which this server never allows).

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{Local, NaiveDate};
use friends_core::api::App;
use friends_core::contacts::{choices, just_chosen, link_url, person_in_link, Card, Chosen};
use friends_core::model::NewPerson;
use friends_core::store::Store;
use serde_json::{json, Value};
use tiny_http::{Header, Method, Response, Server};

struct Options {
    port: u16,
    data: PathBuf,
    today: Option<NaiveDate>,
}

fn parse_args() -> Result<Options, String> {
    let mut port = 1430;
    let mut data = None;
    let mut today = None;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--port" => port = value.parse().map_err(|_| "--port must be a number")?,
            "--data" => data = Some(PathBuf::from(value)),
            "--today" => {
                today = Some(
                    NaiveDate::parse_from_str(&value, "%Y-%m-%d")
                        .map_err(|_| "--today must be YYYY-MM-DD")?,
                )
            }
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    let data =
        data.ok_or("--data <file> is required (use a scratch file, never your real data)")?;
    Ok(Options { port, data, today })
}

fn header<'a>(request: &'a tiny_http::Request, name: &'static str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv(name))
        .map(|h| h.value.as_str())
}

/// Refuses requests a web page could have sent: see the top of this file.
fn check_request(request: &tiny_http::Request, port: u16) -> Result<(), &'static str> {
    allowed(
        header(request, "Host"),
        header(request, "Content-Type"),
        *request.method() == Method::Post,
        port,
    )
}

fn allowed(
    host: Option<&str>,
    content_type: Option<&str>,
    post: bool,
    port: u16,
) -> Result<(), &'static str> {
    let host_ok =
        host.is_some_and(|h| h == format!("127.0.0.1:{port}") || h == format!("localhost:{port}"));
    if !host_ok {
        return Err("wrong Host: send requests to 127.0.0.1 or localhost on this port");
    }
    let json = content_type.is_some_and(|c| {
        c.split(';')
            .next()
            .is_some_and(|t| t.trim().eq_ignore_ascii_case("application/json"))
    });
    if post && !json {
        return Err("send JSON with Content-Type: application/json");
    }
    Ok(())
}

struct Dev {
    app: App,
    store_path: PathBuf,
    today: Option<NaiveDate>,
    launch_at_login: bool,
    /// Stands in for Sign in with Apple: there's no Apple page in the tests, so signing in always works.
    signed_in: bool,
    /// Stands in for Contacts' access: as `POST /__test/contacts` sets it.
    contacts: Access,
    /// The cards shared with the app when it may see only some.
    shared: Vec<String>,
    /// Stands in for the iPhone's notification permission: as `POST /__test/notifications` sets it.
    notifications: Notifications,
    /// The links the app wrote onto the made-up cards, by card.
    card_links: HashMap<String, Vec<String>>,
    /// The person a link from a card opened the app on, until a window takes them.
    shown_person: Option<u64>,
}

/// Contacts' access, as the test sets it up.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Access {
    /// Not asked yet; `limited`: asked, the user shares only some contacts.
    NotAsked {
        limited: bool,
    },
    Allowed,
    Limited,
    Denied,
}

/// The iPhone's notification permission, as the test sets it up (and as `notification_access`
/// answers it).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Notifications {
    Allowed,
    NotAsked,
    Denied,
}

impl Notifications {
    const ALL: [Notifications; 3] = [Self::Allowed, Self::NotAsked, Self::Denied];

    /// As the app's command answers it.
    fn name(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::NotAsked => "notAsked",
            Self::Denied => "denied",
        }
    }
}

/// Who a user shares when they share only some contacts, and who "share more" adds.
const SHARED_FIRST: [&str; 3] = ["card-02", "card-05", "card-09"];
const SHARED_MORE: [&str; 2] = ["card-11", "card-12"];

/// A made-up address book: (card id, name). The app reads only names.
const ADDRESS_BOOK: [(&str, &str); 12] = [
    ("card-01", "Ana Torres"),
    ("card-02", "Ben Okafor"),
    ("card-03", "Chloé Martin"),
    ("card-04", "Dev Patel"),
    ("card-05", "Elif Yılmaz"),
    ("card-06", "Felix Wagner"),
    ("card-07", "Grace Lee"),
    ("card-08", "Hana Sato"),
    ("card-09", "Iris Novak"),
    ("card-10", "Jonah Reyes"),
    ("card-11", "Kofi Mensah"),
    ("card-12", "Lena Fischer"),
];

impl Dev {
    fn today(&self) -> NaiveDate {
        self.today.unwrap_or_else(|| Local::now().date_naive())
    }

    /// One Tauri command, as `src-tauri` answers it.
    fn invoke(&mut self, cmd: &str, args: Value) -> Result<Value, Value> {
        let to_value = |v| serde_json::to_value(v).map_err(|e| json!(e.to_string()));
        match cmd {
            "call" => {
                let name = args
                    .get("cmd")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let inner = args.get("args").cloned().unwrap_or(Value::Null);
                let today = self.today();
                match self.app.call(&name, inner, today) {
                    Ok(view) => to_value(view),
                    Err(e) => Err(serde_json::to_value(e).unwrap_or(Value::Null)),
                }
            }
            "get_launch_at_login" => Ok(json!(self.launch_at_login)),
            "set_launch_at_login" => {
                self.launch_at_login = args.get("on").and_then(Value::as_bool).unwrap_or(false);
                Ok(json!(self.launch_at_login))
            }
            "get_account" => Ok(self.account()),
            "sign_in_with_apple" => {
                self.signed_in = true;
                // As the app does once signed in: the guide's sign-in step is behind you.
                let today = self.today();
                let _ = self
                    .app
                    .call("finish_guide", json!({ "step": "signIn" }), today);
                Ok(self.account())
            }
            "cancel_sign_in" => Ok(self.account()),
            "sign_out" => {
                self.signed_in = false;
                Ok(self.account())
            }
            "list_contacts" => {
                let ask = args.get("ask").and_then(Value::as_bool).unwrap_or(false);
                let mut chosen = Chosen::Nobody;
                match self.contacts {
                    Access::NotAsked { .. } if !ask => {}
                    Access::NotAsked { limited: false } => self.set_access(Access::Allowed),
                    Access::NotAsked { limited: true } => {
                        self.set_access(Access::Limited);
                        chosen = Chosen::Everyone;
                    }
                    _ => {}
                }
                Ok(self.contact_list(&chosen))
            }
            "share_more_contacts" => {
                let mut shared = Vec::new();
                if self.contacts == Access::Limited {
                    for id in SHARED_MORE {
                        if !self.shared.iter().any(|s| s == id) {
                            self.shared.push(id.to_string());
                            shared.push(id.to_string());
                        }
                    }
                }
                Ok(self.contact_list(&Chosen::Cards(shared)))
            }
            "import_contacts" => {
                let people: Vec<NewPerson> =
                    serde_json::from_value(args.get("people").cloned().unwrap_or(Value::Null))
                        .map_err(|e| json!(e.to_string()))?;
                let today = self.today();
                let (view, links) = self
                    .app
                    .import_contacts(people, today)
                    .map_err(|e| serde_json::to_value(e).unwrap_or(Value::Null))?;
                // As the app does: the link goes onto each card.
                for (card, uid) in links {
                    let on_card = self.card_links.entry(card).or_default();
                    let url = link_url(&uid);
                    if !on_card.contains(&url) {
                        on_card.push(url);
                    }
                }
                to_value(view)
            }
            "notification_access" => Ok(json!(self.notifications.name())),
            "allow_notifications" => {
                if self.notifications == Notifications::NotAsked {
                    self.notifications = Notifications::Allowed;
                }
                Ok(json!(self.notifications.name()))
            }
            "take_shown_person" => Ok(json!(self.shown_person.take())),
            // Window commands have nothing to do in a browser.
            "open_preferences" | "hide_dropdown" | "quit" => Ok(Value::Null),
            _ => Err(json!(format!("unknown command: {cmd}"))),
        }
    }

    /// Sets Contacts' access; with only some contacts shared, those are Ben, Elif and Iris.
    fn set_access(&mut self, access: Access) {
        self.contacts = access;
        self.shared = if access == Access::Limited {
            SHARED_FIRST.map(String::from).to_vec()
        } else {
            Vec::new()
        };
    }

    /// The cards the app may see, as `list_contacts` answers, with `chosen` in Apple's sheets.
    fn contact_list(&self, chosen: &Chosen) -> Value {
        let limited = match self.contacts {
            Access::Denied => return json!({ "status": "denied" }),
            Access::NotAsked { .. } => return json!({ "status": "notAsked" }),
            Access::Allowed => false,
            Access::Limited => true,
        };
        let cards = ADDRESS_BOOK
            .iter()
            .filter(|(id, _)| !limited || self.shared.iter().any(|s| s == id))
            .map(|(id, name)| Card {
                id: (*id).to_string(),
                name: (*name).to_string(),
                links: self.card_links.get(*id).cloned().unwrap_or_default(),
            })
            .collect();
        let contacts = choices(self.app.data(), cards);
        let picked = just_chosen(&contacts, chosen);
        json!({ "status": "allowed", "contacts": contacts, "limited": limited, "picked": picked })
    }

    fn account(&self) -> Value {
        if self.signed_in {
            // Synced two minutes ago, as the app shows it most of the time.
            let two_minutes_ago = friends_core::now_ms() - 120_000;
            json!({
                "status": "signedIn",
                "email": "ana.torres@example.com",
                "warning": null,
                "sync": { "syncing": false, "lastSynced": two_minutes_ago, "error": null }
            })
        } else {
            json!({ "status": "signedOut", "warning": null })
        }
    }

    fn reset(&mut self, body: &str) -> Result<(), String> {
        if let Some(dir) = self.store_path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(
            &self.store_path,
            if body.trim() == "{}" {
                r#"{"version":1}"#
            } else {
                body
            },
        )
        .map_err(|e| e.to_string())?;
        self.app = App::open(Store::new(&self.store_path)).map_err(|e| e.to_string())?;
        self.launch_at_login = false;
        self.signed_in = false;
        self.set_access(Access::NotAsked { limited: false });
        self.notifications = Notifications::Allowed;
        self.card_links.clear();
        self.shown_person = None;
        Ok(())
    }
}

fn json_response(status: u16, body: &Value) -> Response<std::io::Cursor<Vec<u8>>> {
    let header = Header::from_bytes("Content-Type", "application/json").expect("valid header");
    Response::from_data(body.to_string().into_bytes())
        .with_status_code(status)
        .with_header(header)
}

fn main() {
    let opts = match parse_args() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("friends-devserver: {e}");
            std::process::exit(2);
        }
    };
    let app = App::open(Store::new(&opts.data)).unwrap_or_else(|e| {
        eprintln!("friends-devserver: can't open {}: {e}", opts.data.display());
        std::process::exit(1);
    });
    let mut dev = Dev {
        app,
        store_path: opts.data.clone(),
        today: opts.today,
        launch_at_login: false,
        signed_in: false,
        contacts: Access::NotAsked { limited: false },
        shared: Vec::new(),
        notifications: Notifications::Allowed,
        card_links: HashMap::new(),
        shown_person: None,
    };
    let server = Server::http(("127.0.0.1", opts.port)).unwrap_or_else(|e| {
        eprintln!(
            "friends-devserver: can't listen on 127.0.0.1:{}: {e}",
            opts.port
        );
        std::process::exit(1);
    });
    println!(
        "friends-devserver listening on http://127.0.0.1:{}",
        opts.port
    );

    for mut request in server.incoming_requests() {
        if let Err(why) = check_request(&request, opts.port) {
            let _ = request.respond(json_response(403, &json!(why)));
            continue;
        }
        let mut body = String::new();
        if request.as_reader().read_to_string(&mut body).is_err() {
            let _ = request.respond(json_response(400, &json!("unreadable body")));
            continue;
        }
        let response = match (request.method(), request.url()) {
            (Method::Post, "/invoke") => match serde_json::from_str::<Value>(&body) {
                Ok(v) => {
                    let cmd = v
                        .get("cmd")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let args = v.get("args").cloned().unwrap_or(Value::Null);
                    match dev.invoke(&cmd, args) {
                        Ok(result) => json_response(200, &result),
                        Err(err) => json_response(400, &err),
                    }
                }
                Err(e) => json_response(400, &json!(e.to_string())),
            },
            (Method::Get, "/__test/health") => json_response(200, &json!("ok")),
            (Method::Post, "/__test/reset") => match dev.reset(&body) {
                Ok(()) => json_response(200, &Value::Null),
                Err(e) => json_response(500, &json!(e)),
            },
            (Method::Post, "/__test/today") => match serde_json::from_str::<String>(&body)
                .ok()
                .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
            {
                Some(d) => {
                    dev.today = Some(d);
                    json_response(200, &Value::Null)
                }
                None => json_response(400, &json!("send \"YYYY-MM-DD\"")),
            },
            (Method::Post, "/__test/open-card") => match serde_json::from_str::<String>(&body) {
                Ok(card) => {
                    let links = dev.card_links.get(&card).cloned().unwrap_or_default();
                    dev.shown_person = links
                        .iter()
                        .find_map(|url| person_in_link(dev.app.data(), url));
                    json_response(200, &json!(dev.shown_person))
                }
                Err(_) => json_response(400, &json!("send the card's id as a JSON string")),
            },
            (Method::Post, "/__test/contacts") => {
                let access = match serde_json::from_str::<String>(&body).as_deref() {
                    Ok("notAsked") => Some(Access::NotAsked { limited: false }),
                    Ok("notAskedLimited") => Some(Access::NotAsked { limited: true }),
                    Ok("allowed") => Some(Access::Allowed),
                    Ok("limited") => Some(Access::Limited),
                    Ok("denied") => Some(Access::Denied),
                    _ => None,
                };
                match access {
                    Some(access) => {
                        dev.set_access(access);
                        json_response(200, &Value::Null)
                    }
                    None => json_response(
                        400,
                        &json!("send \"notAsked\", \"notAskedLimited\", \"allowed\", \"limited\" or \"denied\""),
                    ),
                }
            }
            (Method::Post, "/__test/notifications") => {
                let asked = serde_json::from_str::<String>(&body).ok();
                match Notifications::ALL
                    .into_iter()
                    .find(|n| asked.as_deref() == Some(n.name()))
                {
                    Some(answer) => {
                        dev.notifications = answer;
                        json_response(200, &Value::Null)
                    }
                    None => {
                        json_response(400, &json!("send \"allowed\", \"notAsked\" or \"denied\""))
                    }
                }
            }
            _ => json_response(404, &json!("not found")),
        };
        let _ = request.respond(response);
    }
}

#[cfg(test)]
mod tests {
    use super::allowed;

    #[test]
    fn accepts_json_posts_to_the_loopback_host() {
        assert!(allowed(Some("127.0.0.1:1430"), Some("application/json"), true, 1430).is_ok());
        assert!(allowed(
            Some("localhost:1430"),
            Some("application/json; charset=utf-8"),
            true,
            1430
        )
        .is_ok());
        assert!(allowed(Some("localhost:1430"), None, false, 1430).is_ok());
    }

    #[test]
    fn refuses_what_a_web_page_could_send() {
        // A plain form or no-cors fetch: no JSON content type.
        assert!(allowed(Some("127.0.0.1:1430"), Some("text/plain"), true, 1430).is_err());
        assert!(allowed(Some("127.0.0.1:1430"), None, true, 1430).is_err());
        // DNS rebinding: the page's own host name pointed at 127.0.0.1.
        assert!(allowed(
            Some("evil.example:1430"),
            Some("application/json"),
            true,
            1430
        )
        .is_err());
        assert!(allowed(Some("127.0.0.1:9999"), Some("application/json"), true, 1430).is_err());
        assert!(allowed(None, Some("application/json"), true, 1430).is_err());
    }
}
