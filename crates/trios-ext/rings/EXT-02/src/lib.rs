//! EXT-02 — Settings
//!
//! chrome.storage.local wrapper for API keys and preferences.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::cell::RefCell;

thread_local! {
    static API_KEY: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Get the cached API key (set from chrome.storage.local or settings UI).
pub fn get_api_key() -> Option<String> {
    API_KEY.with(|k| k.borrow().clone())
}

/// Save API key to chrome.storage.local and in-memory cache.
pub fn save_api_key(key: &str) -> Result<(), JsValue> {
    let local = storage_local()?;
    let set_fn: js_sys::Function = js_sys::Reflect::get(&local, &JsValue::from_str("set"))?
        .dyn_into().map_err(|_| JsValue::from_str("storage.local.set not a function"))?;
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &JsValue::from_str("zai_key"), &JsValue::from_str(key))?;
    set_fn.call1(&local, &obj)?;
    API_KEY.with(|k| *k.borrow_mut() = Some(key.to_string()));
    Ok(())
}

/// Load API key from chrome.storage.local (call once on startup).
pub fn load_api_key() {
    let local = match storage_local() {
        Ok(l) => l,
        Err(_) => return,
    };
    let get_fn: js_sys::Function = match js_sys::Reflect::get(&local, &JsValue::from_str("get"))
        .ok()
        .and_then(|f| f.dyn_into().ok())
    {
        Some(f) => f,
        None => return,
    };

    let cb = Closure::<dyn Fn(js_sys::Object)>::new(|result: js_sys::Object| {
        if let Ok(val) = js_sys::Reflect::get(&result, &JsValue::from_str("zai_key")) {
            if let Some(s) = val.as_string() {
                if !s.is_empty() {
                    API_KEY.with(|k| *k.borrow_mut() = Some(s));
                }
            }
        }
    });

    let keys = js_sys::Array::new();
    keys.push(&JsValue::from_str("zai_key"));
    let _ = get_fn.call2(&local, &keys.into(), cb.as_ref().unchecked_ref());
    cb.forget();
}

/// Navigate `js_sys::global().chrome.storage.local`
fn storage_local() -> Result<JsValue, JsValue> {
    let g = js_sys::global();
    let chrome = js_sys::Reflect::get(&g, &JsValue::from_str("chrome"))
        .map_err(|_| JsValue::from_str("chrome not available"))?;
    let storage = js_sys::Reflect::get(&chrome, &JsValue::from_str("storage"))?;
    let local = js_sys::Reflect::get(&storage, &JsValue::from_str("local"))?;
    Ok(local)
}

#[wasm_bindgen]
pub fn settings_save_key(key: &str) -> Result<(), JsValue> {
    save_api_key(key)
}
