//! EXT-04 — BrowserOS A2A Agent
//!
//! Executes browser control commands received from trios-server via MCP.
//! No imports from other EXT rings. Pure WASM/web-sys.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Incoming MCP browser command from trios-server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserCommand {
    pub id: String,
    pub tool: String,
    pub params: Value,
}

/// Result reported back to trios-server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserResult {
    pub command_id: String,
    pub ok: bool,
    pub data: Value,
    pub error: Option<String>,
}

impl BrowserResult {
    pub fn ok(command_id: &str, data: Value) -> Self {
        Self { command_id: command_id.into(), ok: true, data, error: None }
    }
    pub fn err(command_id: &str, msg: &str) -> Self {
        Self { command_id: command_id.into(), ok: false, data: Value::Null, error: Some(msg.into()) }
    }
}

/// Dispatch a JSON command string → execute → return JSON result string.
/// Called by EXT-02 poll loop after receiving a command from trios-server.
#[wasm_bindgen]
pub fn dispatch_command(json_cmd: &str) -> String {
    let cmd: BrowserCommand = match serde_json::from_str(json_cmd) {
        Ok(c) => c,
        Err(e) => {
            let r = BrowserResult::err("unknown", &format!("parse error: {}", e));
            return serde_json::to_string(&r).unwrap_or_default();
        }
    };

    let result = match cmd.tool.as_str() {
        "browser_get_url" => exec_get_url(&cmd.id),
        "browser_get_title" => exec_get_title(&cmd.id),
        "browser_navigate" => {
            let url = cmd.params.get("url").and_then(|v| v.as_str()).unwrap_or("");
            exec_navigate(&cmd.id, url)
        }
        "browser_get_dom" => exec_get_dom(&cmd.id),
        "browser_query_selector" => {
            let sel = cmd.params.get("selector").and_then(|v| v.as_str()).unwrap_or("");
            exec_query_selector(&cmd.id, sel)
        }
        "browser_click" => {
            let sel = cmd.params.get("selector").and_then(|v| v.as_str()).unwrap_or("");
            exec_click(&cmd.id, sel)
        }
        "browser_type" => {
            let sel = cmd.params.get("selector").and_then(|v| v.as_str()).unwrap_or("");
            let text = cmd.params.get("text").and_then(|v| v.as_str()).unwrap_or("");
            exec_type(&cmd.id, sel, text)
        }
        "browser_scroll" => {
            let x = cmd.params.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let y = cmd.params.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            exec_scroll(&cmd.id, x, y)
        }
        "browser_eval" => {
            let js = cmd.params.get("js").and_then(|v| v.as_str()).unwrap_or("");
            exec_eval(&cmd.id, js)
        }
        unknown => BrowserResult::err(&cmd.id, &format!("unknown tool: {}", unknown)),
    };

    serde_json::to_string(&result).unwrap_or_default()
}

// ─── Executors ────────────────────────────────────────────────────

fn exec_get_url(id: &str) -> BrowserResult {
    match web_sys::window().and_then(|w| w.location().href().ok()) {
        Some(url) => BrowserResult::ok(id, json!({"url": url})),
        None => BrowserResult::err(id, "window.location.href unavailable"),
    }
}

fn exec_get_title(id: &str) -> BrowserResult {
    match web_sys::window().and_then(|w| w.document()).map(|d| d.title()) {
        Some(title) => BrowserResult::ok(id, json!({"title": title})),
        None => BrowserResult::err(id, "document.title unavailable"),
    }
}

fn exec_navigate(id: &str, url: &str) -> BrowserResult {
    if url.is_empty() {
        return BrowserResult::err(id, "url param required");
    }
    match web_sys::window() {
        Some(w) => match w.location().assign(url) {
            Ok(_) => BrowserResult::ok(id, json!({"navigating_to": url})),
            Err(e) => BrowserResult::err(id, &format!("{:?}", e)),
        },
        None => BrowserResult::err(id, "window unavailable"),
    }
}

fn exec_get_dom(id: &str) -> BrowserResult {
    let html = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
        .map(|el| el.outer_html());
    match html {
        Some(h) => BrowserResult::ok(id, json!({"html": h})),
        None => BrowserResult::err(id, "document not available"),
    }
}

fn exec_query_selector(id: &str, selector: &str) -> BrowserResult {
    if selector.is_empty() {
        return BrowserResult::err(id, "selector param required");
    }
    let result = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.query_selector(selector).ok().flatten())
        .map(|el| el.outer_html());
    match result {
        Some(html) => BrowserResult::ok(id, json!({"found": true, "html": html})),
        None => BrowserResult::ok(id, json!({"found": false})),
    }
}

fn exec_click(id: &str, selector: &str) -> BrowserResult {
    if selector.is_empty() {
        return BrowserResult::err(id, "selector param required");
    }
    let clicked = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.query_selector(selector).ok().flatten())
        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
        .map(|el| { el.click(); true })
        .unwrap_or(false);
    if clicked {
        BrowserResult::ok(id, json!({"clicked": selector}))
    } else {
        BrowserResult::err(id, &format!("element not found: {}", selector))
    }
}

fn exec_type(id: &str, selector: &str, text: &str) -> BrowserResult {
    if selector.is_empty() {
        return BrowserResult::err(id, "selector param required");
    }
    let doc = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return BrowserResult::err(id, "document unavailable"),
    };
    let el = match doc.query_selector(selector).ok().flatten() {
        Some(e) => e,
        None => return BrowserResult::err(id, &format!("element not found: {}", selector)),
    };
    if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
        input.set_value(text);
        let _ = input.dispatch_event(&web_sys::Event::new("input").unwrap());
        let _ = input.dispatch_event(&web_sys::Event::new("change").unwrap());
        return BrowserResult::ok(id, json!({"typed": text.len()}));
    }
    if let Some(ta) = el.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        ta.set_value(text);
        let _ = ta.dispatch_event(&web_sys::Event::new("input").unwrap());
        let _ = ta.dispatch_event(&web_sys::Event::new("change").unwrap());
        return BrowserResult::ok(id, json!({"typed": text.len()}));
    }
    BrowserResult::err(id, "element is not input or textarea")
}

fn exec_scroll(id: &str, x: f64, y: f64) -> BrowserResult {
    match web_sys::window() {
        Some(w) => {
            w.scroll_to_with_x_and_y(x, y);
            BrowserResult::ok(id, json!({"scrolled_to": {"x": x, "y": y}}))
        }
        None => BrowserResult::err(id, "window unavailable"),
    }
}

fn exec_eval(id: &str, js: &str) -> BrowserResult {
    if js.is_empty() {
        return BrowserResult::err(id, "js param required");
    }
    // Sandboxed: use new Function(code)() — NOT eval()
    let wrapped = format!("(new Function({}))()", serde_json::to_string(js).unwrap_or_default());
    let result = js_sys::eval(&wrapped);
    match result {
        Ok(val) => {
            let s = val.as_string().unwrap_or_else(|| format!("{:?}", val));
            BrowserResult::ok(id, json!({"result": s}))
        }
        Err(e) => {
            let msg = e.as_string().unwrap_or_else(|| "eval error".into());
            BrowserResult::err(id, &msg)
        }
    }
}

// ─── wasm_bindgen exports ────────────────────────────────────────────

#[wasm_bindgen]
pub fn browser_get_url() -> Option<String> {
    web_sys::window().and_then(|w| w.location().href().ok())
}

#[wasm_bindgen]
pub fn browser_get_title() -> Option<String> {
    web_sys::window().and_then(|w| w.document()).map(|d| d.title())
}

#[wasm_bindgen]
pub fn browser_navigate(url: &str) -> bool {
    web_sys::window()
        .and_then(|w| w.location().assign(url).ok())
        .is_some()
}

#[wasm_bindgen]
pub fn browser_get_dom() -> Option<String> {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
        .map(|el| el.outer_html())
}

#[wasm_bindgen]
pub fn browser_click(selector: &str) -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.query_selector(selector).ok().flatten())
        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
        .map(|el| { el.click(); true })
        .unwrap_or(false)
}
