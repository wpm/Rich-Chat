//! The desktop shell. Nothing but a window and the opener plugin, which
//! the frontend uses to hand links to the system browser.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("the Tauri application runs");
}
