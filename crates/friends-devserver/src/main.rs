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
//!
//! It only listens on the loopback address and is never part of the app. So that a web page open in
//! the browser can't drive it, it answers only requests addressed to `127.0.0.1:<port>` or
//! `localhost:<port>` (no DNS rebinding) and only JSON POSTs (a page can't send those to another
//! origin without a CORS preflight, which this server never allows).

use std::path::PathBuf;

use chrono::{Local, NaiveDate};
use friends_core::api::App;
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
}

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
                Ok(self.account())
            }
            "cancel_sign_in" => Ok(self.account()),
            "sign_out" => {
                self.signed_in = false;
                Ok(self.account())
            }
            // Window commands have nothing to do in a browser.
            "open_preferences" | "hide_dropdown" | "quit" => Ok(Value::Null),
            _ => Err(json!(format!("unknown command: {cmd}"))),
        }
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
