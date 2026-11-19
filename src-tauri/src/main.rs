// Keeps a console window from opening alongside the app on Windows release builds; harmless on macOS.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    friends_lib::run()
}
