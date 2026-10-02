//! The engine for the Android app.
//!
//! The app's Kotlin side talks to the game through the same JSON interface
//! as the browser tools (`scraped_web::handle`), so every client plays
//! identically. Jb's content is built in: requests that need the pack get
//! it added here. Game state lives in thread-local storage, so the app
//! makes every call from one engine thread.
//!
//! The library also paints the app's backdrop (`atmosphere`): a quiet,
//! slowly drifting texture behind the text. It is decoration only and never
//! depends on the game: the game itself has no graphics.

pub mod atmosphere;

use jni::objects::{JClass, JIntArray, JString};
use jni::sys::{jboolean, jfloat, jint, jstring};
use jni::JNIEnv;
use serde_json::{json, Value};

include!(concat!(env!("OUT_DIR"), "/content.rs"));

/// Commands that read the content pack.
const NEEDS_PACK: &[&str] = &[
    "play_new",
    "play_load",
    "play_pack",
    "app_labels",
    "ui_labels",
    "registry",
    "lint",
    "preview",
    "coverage",
    "voice",
    "transcript_hash",
    "storylet_preview",
];

/// Handles one JSON request, adding the built-in content where needed.
pub fn call(request: &str) -> String {
    let mut req: Value = match serde_json::from_str(request) {
        Ok(v) => v,
        Err(e) => return json!({ "error": format!("bad request: {e}") }).to_string(), // DEBUG-TEXT
    };
    let needs = req["cmd"].as_str().is_some_and(|c| NEEDS_PACK.contains(&c));
    if needs && req.get("files").is_none() && req.get("pack").is_none() {
        req["files"] = Value::Array(
            CONTENT
                .iter()
                .map(|(path, text)| json!({ "path": path, "text": text }))
                .collect(),
        );
    }
    scraped_web::handle(&req.to_string())
}

/// `Engine.call(request: String): String`
#[no_mangle]
pub extern "system" fn Java_org_scrapedagain_Engine_call<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    request: JString<'l>,
) -> jstring {
    let req: String = match env.get_string(&request) {
        Ok(s) => s.into(),
        Err(_) => String::new(),
    };
    let out = call(&req);
    env.new_string(out)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// `Engine.atmosphere(pixels: IntArray, width: Int, height: Int, time: Float, dark: Boolean)`:
/// paints one frame of the backdrop into `pixels` (ARGB, row by row).
#[no_mangle]
pub extern "system" fn Java_org_scrapedagain_Engine_atmosphere<'l>(
    env: JNIEnv<'l>,
    _class: JClass<'l>,
    pixels: JIntArray<'l>,
    width: jint,
    height: jint,
    time: jfloat,
    dark: jboolean,
) {
    let (w, h) = (width.max(1) as usize, height.max(1) as usize);
    let mut buf = vec![0u32; w * h];
    atmosphere::paint(&mut buf, w, h, time, dark != 0);
    let ints: Vec<i32> = buf.into_iter().map(|p| p as i32).collect();
    let _ = env.set_int_array_region(&pixels, 0, &ints);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_is_built_in_and_used() {
        assert!(CONTENT.iter().any(|(p, _)| *p == "app.toml"));
        let labels: Value = serde_json::from_str(&call(r#"{"cmd":"app_labels"}"#)).unwrap();
        assert!(labels["app_name"].as_str().is_some_and(|s| !s.is_empty()));
        let first: Value =
            serde_json::from_str(&call(r#"{"cmd":"play_new","seed":"42"}"#)).unwrap();
        assert!(first["text"].as_str().is_some_and(|s| !s.is_empty()));
        let next: Value = serde_json::from_str(&call(r#"{"cmd":"play","line":"look"}"#)).unwrap();
        assert!(next["state"]["things"].is_array());
        let bad: Value = serde_json::from_str(&call("not json")).unwrap();
        assert!(bad["error"].is_string());
    }
}
