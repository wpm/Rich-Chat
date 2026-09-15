//! Tauri's build step, plus an Info.plist embedded in the macOS debug binary.
//!
//! `cargo tauri dev` runs that binary bare rather than inside an app bundle,
//! so macOS has no Info.plist to read and labels the Dock tile and the
//! application menu with the executable's name, `rich-chat-desktop`. A plist
//! linked into the binary's `__info_plist` section gives it the product name
//! instead. Release builds go into the bundle `cargo tauri build` makes, and
//! the bundle's own Info.plist takes over there.

use std::{env, fs, path::PathBuf};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
        && env::var("PROFILE").as_deref() == Ok("debug")
    {
        embed_dev_info_plist();
    }
    tauri_build::build()
}

fn embed_dev_info_plist() {
    println!("cargo:rerun-if-changed=tauri.conf.json");
    let config = fs::read_to_string("tauri.conf.json").expect("tauri.conf.json is readable");
    let config: serde_json::Value =
        serde_json::from_str(&config).expect("tauri.conf.json is valid JSON");
    let name = config["productName"]
        .as_str()
        .expect("tauri.conf.json sets productName");
    let name = name
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{name}</string>
    <key>CFBundleDisplayName</key>
    <string>{name}</string>
</dict>
</plist>
"#
    );
    let path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set")).join("Info.plist");
    fs::write(&path, plist).expect("the dev Info.plist is written to OUT_DIR");
    println!(
        "cargo:rustc-link-arg-bins=-Wl,-sectcreate,__TEXT,__info_plist,{}",
        path.display()
    );
}
