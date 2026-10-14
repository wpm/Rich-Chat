//! The settings kept in the browser's local storage, and the system's
//! color scheme read from it. Run with `.github/scripts/browser-tests.sh`;
//! the tests in `super::tests` cover the reading and writing natively.

use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

use super::*;

wasm_bindgen_test_configure!(run_in_browser);

/// The page's local storage, emptied, so that each test starts from
/// nothing stored.
fn cleared() -> web_sys::Storage {
    let storage = storage().expect("a browser has local storage");
    storage.clear().unwrap();
    storage
}

#[wasm_bindgen_test]
fn with_nothing_stored_the_defaults_load() {
    cleared();
    assert_eq!(Settings::load(), Settings::default());
}

#[wasm_bindgen_test]
fn what_is_stored_loads_again() {
    cleared();
    let settings = Settings {
        theme: Some(Theme::Dark),
        show_names: true,
        input_height: Some(120.0),
        background: Some("#202020".to_string()),
        ..Settings::default()
    };
    settings.store();
    assert_eq!(Settings::load(), settings);
}

#[wasm_bindgen_test]
fn a_setting_unset_is_removed_so_it_follows_the_system_again() {
    let storage = cleared();
    let mut settings = Settings {
        theme: Some(Theme::Dark),
        ..Settings::default()
    };
    settings.store();
    assert_eq!(
        storage.get_item(THEME_KEY).unwrap().as_deref(),
        Some("dark")
    );
    settings.theme = None;
    settings.store();
    assert_eq!(storage.get_item(THEME_KEY).unwrap(), None);
}

#[wasm_bindgen_test]
fn the_system_scheme_is_the_browsers() {
    let dark = web_sys::window()
        .unwrap()
        .match_media("(prefers-color-scheme: dark)")
        .unwrap()
        .unwrap()
        .matches();
    assert_eq!(system_prefers_dark(), dark);
}
