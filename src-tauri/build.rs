//! Besides Tauri's own build step, builds the account server's settings into the app when they are in
//! the environment (`FRIENDS_SUPABASE_URL`, `FRIENDS_SUPABASE_PUBLISHABLE_KEY`), as release builds have
//! them. Debug builds without them read `.env.local` at the repo root when they start instead.

const KEYS: [&str; 2] = ["FRIENDS_SUPABASE_URL", "FRIENDS_SUPABASE_PUBLISHABLE_KEY"];

fn main() {
    for key in KEYS {
        println!("cargo:rerun-if-env-changed={key}");
        if let Some(value) = std::env::var(key).ok().filter(|v| !v.trim().is_empty()) {
            println!("cargo:rustc-env={key}={value}");
        }
    }
    tauri_build::build()
}
