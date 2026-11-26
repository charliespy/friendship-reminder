//! The Friendship Reminder app's logic, kept free of Tauri and macOS so it can be tested on its own.

pub mod api;
pub mod model;
pub mod schedule;
pub mod store;
pub mod sync;

/// The time now, in milliseconds since 1970: what sync stamps changes with.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// The user's home folder, from `$HOME` (what macOS sets for every app).
pub fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(std::path::PathBuf::from)
}

/// The text with the home folder shown as ~, so an error pasted into a bug report doesn't carry the
/// user's account name.
pub fn hide_home(text: &str, home: Option<&std::path::Path>) -> String {
    match home.map(|h| h.display().to_string()) {
        Some(h) if !h.is_empty() && h != "/" => {
            text.replace(&format!("{}/", h.trim_end_matches('/')), "~/")
        }
        _ => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hide_home_hides_every_copy_of_the_home_folder() {
        let home = std::path::Path::new("/Users/you");
        let text = "Couldn't read /Users/you/Library/x/friends.json: \
                    /Users/you/Library/x/friends.json was saved by a newer version";
        assert_eq!(
            hide_home(text, Some(home)),
            "Couldn't read ~/Library/x/friends.json: \
             ~/Library/x/friends.json was saved by a newer version"
        );
        // Only the home folder itself, not a longer name that starts the same way.
        let vol = std::path::Path::new("/Volumes/d/you");
        assert_eq!(
            hide_home("/Volumes/d/youth/x and /Volumes/d/you/x", Some(vol)),
            "/Volumes/d/youth/x and ~/x"
        );
        assert_eq!(hide_home("/Users/you/x", None), "/Users/you/x");
        assert_eq!(hide_home("/x", Some(std::path::Path::new("/"))), "/x");
    }
}
