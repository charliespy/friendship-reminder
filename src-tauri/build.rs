/// The account server's settings that get built into the app (read with `option_env!`).
const SETTINGS: [&str; 2] = ["FRIENDS_SUPABASE_URL", "FRIENDS_SUPABASE_PUBLISHABLE_KEY"];

fn main() {
    build_in_account_settings();
    tauri_build::build()
}

/// Release builds take the account server's settings from the environment. The iPhone build runs
/// the Rust compile from inside Xcode, which doesn't pass the environment on, so a setting that
/// isn't there is read from `.env.local` at the repo root instead (git ignores it).
fn build_in_account_settings() {
    let env_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.env.local");
    println!("cargo:rerun-if-changed={}", env_file.display());
    for name in SETTINGS {
        println!("cargo:rerun-if-env-changed={name}");
    }
    let Ok(text) = std::fs::read_to_string(&env_file) else {
        return;
    };
    for name in SETTINGS {
        if std::env::var_os(name).is_some() {
            continue;
        }
        if let Some(value) = setting(&text, name) {
            println!("cargo:rustc-env={name}={value}");
        }
    }
}

/// `NAME=value` from a .env file, without quotes; the last line for a name wins.
fn setting(text: &str, name: &str) -> Option<String> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            let line = line.strip_prefix("export ").unwrap_or(line);
            let (key, value) = line.split_once('=')?;
            (key.trim() == name).then(|| {
                let value = value.trim();
                value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                    .unwrap_or(value)
                    .to_string()
            })
        })
        .filter(|value| !value.is_empty())
        .last()
}
