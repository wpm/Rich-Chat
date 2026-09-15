//! The desktop shell: a window, the opener plugin (the frontend uses it to
//! hand links to the system browser), and Tauri's default menu.

#[cfg(all(target_os = "macos", debug_assertions))]
mod dev_bundle;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();
    #[cfg(all(target_os = "macos", debug_assertions))]
    dev_bundle::relaunch_in_bundle(&dev_bundle::App {
        name: &context.package_info().name,
        version: &context.package_info().version.to_string(),
        identifier: &context.config().identifier,
    });
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(context)
        .expect("the Tauri application runs");
}
