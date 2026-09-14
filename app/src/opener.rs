//! Opens links outside the window.
//!
//! Inside Tauri the webview must not navigate, so links go to the system
//! browser through the opener plugin. In a plain browser (`trunk serve`)
//! they open a new tab.

use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

fn in_tauri() -> bool {
    web_sys::window()
        .map(|window| {
            js_sys::Reflect::has(&window, &JsValue::from_str("__TAURI__")).unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Opens `url` in the system browser under Tauri, or a new tab otherwise.
pub fn open_url(url: String) {
    if in_tauri() {
        spawn_local(async move {
            let args = js_sys::Object::new();
            let _ =
                js_sys::Reflect::set(&args, &JsValue::from_str("url"), &JsValue::from_str(&url));
            let _ = invoke("plugin:opener|open_url", args.into()).await;
        });
    } else if let Some(window) = web_sys::window() {
        let _ = window.open_with_url_and_target(&url, "_blank");
    }
}
