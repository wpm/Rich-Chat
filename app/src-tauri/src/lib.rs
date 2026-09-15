//! The desktop shell: a window, the opener plugin (the frontend uses it to
//! hand links to the system browser), and on macOS an application menu
//! whose About panel shows the app's own icon.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());
    #[cfg(target_os = "macos")]
    let builder = builder.menu(macos_menu);
    builder
        .run(tauri::generate_context!())
        .expect("the Tauri application runs");
}

/// Tauri's default macOS menu, with the About panel handed the app icon.
///
/// The standard About panel takes its icon from the app bundle, and
/// `cargo tauri dev` runs a bare binary with no bundle, so the panel falls
/// back to the generic executable icon. Passing the icon in the About
/// metadata sidesteps the bundle. The icon is Tauri's default window icon,
/// which it takes from the first PNG under `bundle.icon` in
/// `tauri.conf.json`; that is why the list there leads with a large one.
#[cfg(target_os = "macos")]
fn macos_menu<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    use tauri::menu::{AboutMetadata, Menu, MenuItemKind, PredefinedMenuItem};

    let menu = Menu::default(app)?;
    let package = app.package_info();
    let about = PredefinedMenuItem::about(
        app,
        None,
        Some(AboutMetadata {
            name: Some(package.name.clone()),
            version: Some(package.version.to_string()),
            copyright: app.config().bundle.copyright.clone(),
            icon: app.default_window_icon().cloned(),
            ..Default::default()
        }),
    )?;
    // In the default menu the first submenu is the application menu and its
    // first item is About.
    if let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.first() {
        app_menu.remove_at(0)?;
        app_menu.insert(&about, 0)?;
    }
    Ok(menu)
}
