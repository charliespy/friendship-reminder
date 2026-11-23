//! Besides Tauri's own build step, passes the account server's settings to the app. They come from the
//! environment (`FRIENDS_SUPABASE_URL`, `FRIENDS_SUPABASE_PUBLISHABLE_KEY`) or, for local builds, from
//! `.env.local` at the repo root (git ignores it). A build without them has no account.

use std::path::Path;

const KEYS: [&str; 2] = ["FRIENDS_SUPABASE_URL", "FRIENDS_SUPABASE_PUBLISHABLE_KEY"];

/// The value of `key` in a `.env` file: `KEY=value`, optionally after `export`, optionally quoted,
/// with `#` comments on their own line or after an unquoted value.
fn from_env_file(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
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

fn main() {
    let env_file = Path::new("../.env.local");
    // Watching a file that doesn't exist makes Cargo rerun this script on every build, so watch the
    // repo root instead until there is one (creating the file changes the folder).
    if env_file.exists() {
        println!("cargo:rerun-if-changed=../.env.local");
    } else {
        println!("cargo:rerun-if-changed=..");
    }
    let file = std::fs::read_to_string(env_file).unwrap_or_default();
    for key in KEYS {
        println!("cargo:rerun-if-env-changed={key}");
        let from_env = std::env::var(key).ok().filter(|v| !v.trim().is_empty());
        let value = from_env.or_else(|| from_env_file(&file, key));
        if let Some(value) = value.filter(|v| !v.is_empty()) {
            println!("cargo:rustc-env={key}={value}");
        }
    }
    tauri_build::build()
}
