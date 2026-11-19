//! The Tauri shell: windows, the menu bar icon and the OS integrations. The rules live in friends-core.

pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running the Friends app");
}
