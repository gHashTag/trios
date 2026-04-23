//! SILVER-RING-EXT-02 — Background Service Worker
//!
//! Chrome extension background initialization via WASM.

use js_sys::Function;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn background_init() -> Result<(), JsValue> {
    let global = js_sys::global();

    let chrome = js_sys::Reflect::get(&global, &JsValue::from_str("chrome"))
        .map_err(|_| JsValue::from_str("no chrome global"))?;

    if js_sys::Reflect::has(&chrome, &JsValue::from_str("sidePanel"))? {
        let side_panel = js_sys::Reflect::get(&chrome, &JsValue::from_str("sidePanel"))?;
        let set_panel_behavior =
            js_sys::Reflect::get(&side_panel, &JsValue::from_str("setPanelBehavior"))?;

        if !set_panel_behavior.is_undefined() && !set_panel_behavior.is_null() {
            let behavior = js_sys::Object::new();
            js_sys::Reflect::set(
                &behavior,
                &JsValue::from_str("openPanelOnActionClick"),
                &JsValue::TRUE,
            )?;
            let args = js_sys::Array::new();
            args.push(&behavior);
            let _ = Function::from(set_panel_behavior).apply(&side_panel, &args);
        }
    }

    if js_sys::Reflect::has(&chrome, &JsValue::from_str("runtime"))? {
        let runtime = js_sys::Reflect::get(&chrome, &JsValue::from_str("runtime"))?;
        let on_installed = js_sys::Reflect::get(&runtime, &JsValue::from_str("onInstalled"))?;

        if js_sys::Reflect::has(&on_installed, &JsValue::from_str("addListener"))? {
            let add_listener =
                js_sys::Reflect::get(&on_installed, &JsValue::from_str("addListener"))?;
            let closure = Closure::<dyn Fn()>::new(|| {
                let global = js_sys::global();
                if let Ok(chrome) = js_sys::Reflect::get(&global, &JsValue::from_str("chrome")) {
                    if let Ok(sp) = js_sys::Reflect::get(&chrome, &JsValue::from_str("sidePanel")) {
                        if let Ok(set_pb) =
                            js_sys::Reflect::get(&sp, &JsValue::from_str("setPanelBehavior"))
                        {
                            let behavior = js_sys::Object::new();
                            let _ = js_sys::Reflect::set(
                                &behavior,
                                &JsValue::from_str("openPanelOnActionClick"),
                                &JsValue::TRUE,
                            );
                            let args = js_sys::Array::new();
                            args.push(&behavior);
                            let _ = Function::from(set_pb).apply(&sp, &args);
                        }
                    }
                }
            });
            let args = js_sys::Array::new();
            args.push(closure.as_ref());
            let _ = Function::from(add_listener).apply(&on_installed, &args);
            closure.forget();
        }
    }

    Ok(())
}
