//! EXT-03 — Content Injectors
//!
//! GitHub and Claude.ai content injectors.
//! Pure Rust/WASM implementation replacing deleted TypeScript files.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// ---- GitHub Injector ----

/// Find issue/PR number from current URL.
#[wasm_bindgen]
pub fn github_parse_issue_number() -> Option<u32> {
    let window = web_sys::window()?;
    let href = window.location().href().ok()?;
    let parts: Vec<&str> = href.split('/').collect();
    for i in 0..parts.len().saturating_sub(1) {
        if (parts[i] == "issues" || parts[i] == "pull") && i + 1 < parts.len() {
            if let Ok(n) = parts[i + 1].parse::<u32>() {
                return Some(n);
            }
        }
    }
    None
}

/// Inject a Trinity button next to issue/PR title.
#[wasm_bindgen]
pub fn github_inject_button() -> Result<bool, JsValue> {
    let doc = trios_ext_00::document()?;

    // Check if already injected
    if doc.query_selector("[data-trinity-button]")?.is_some() {
        return Ok(false);
    }

    let title_container = doc
        .query_selector(".gh-header-title")?
        .or_else(|| doc.query_selector(".js-issue-title").ok().flatten())
        .ok_or_else(|| JsValue::from_str("GitHub issue/PR title container not found"))?;

    let issue_num = github_parse_issue_number()
        .ok_or_else(|| JsValue::from_str("Not on an issue/PR page"))?;

    let btn = doc.create_element("button")?;
    btn.set_attribute("data-trinity-button", "true")?;
    btn.set_attribute("title", "Open in Trinity Agent Bridge")?;
    btn.set_inner_html("⬡");

    if let Some(el) = btn.dyn_ref::<web_sys::HtmlElement>() {
        el.set_attribute("style", "\
            margin-left:8px;padding:4px 12px;border:1px solid #F5D3F2;\
            border-radius:6px;background:#000;color:#F5D3F2;\
            cursor:pointer;font-size:14px;font-weight:600;")?;
    }

    title_container.append_child(&btn)?;

    log::info!("[Trinity-GitHub] Button injected for issue #{issue_num}");
    Ok(true)
}

/// Entry point for GitHub content script bootstrap.
#[wasm_bindgen]
pub fn github_injector_start() {
    log::info!("[Trinity-GitHub] Content script loaded");
    let _ = (|| -> Result<(), JsValue> {
        if github_inject_button()? {
            log::info!("[Trinity-GitHub] Button injected successfully");
        }
        Ok(())
    })();
}

// ---- Claude.ai Injector ----

const PROSEMIRROR_ATTR: &str = "data-prosemirror-view";

/// Find ProseMirror textarea used by Claude.ai.
#[wasm_bindgen]
pub fn claude_find_textarea() -> Option<web_sys::HtmlTextAreaElement> {
    let doc = web_sys::window()?.document()?;
    let textareas = doc.query_selector_all("textarea").ok()?;

    for i in 0..textareas.length() {
        if let Some(el) = textareas.item(i) {
            if let Some(ta) = el.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                if ta.has_attribute(PROSEMIRROR_ATTR) {
                    return Some(ta.clone());
                }
            }
        }
    }
    None
}

/// Inject text into Claude.ai ProseMirror textarea.
#[wasm_bindgen]
pub fn claude_inject_text(text: &str) -> bool {
    let ta = match claude_find_textarea() {
        Some(t) => t,
        None => {
            log::error!("[Trinity-Claude] ProseMirror textarea not found");
            return false;
        }
    };

    let _ = ta.focus();
    ta.set_value(text);

    let input_event = web_sys::Event::new("input").unwrap();
    let _ = ta.dispatch_event(&input_event);
    let change_event = web_sys::Event::new("change").unwrap();
    let _ = ta.dispatch_event(&change_event);

    log::info!("[Trinity-Claude] Text injected, length: {}", text.len());
    true
}

/// Find and click submit button in Claude.ai.
#[wasm_bindgen]
pub fn claude_auto_submit() -> bool {
    let doc = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return false,
    };

    let buttons = match doc.query_selector_all("button, [role=\"button\"]") {
        Ok(b) => b,
        Err(_) => return false,
    };

    for i in 0..buttons.length() {
        if let Some(btn) = buttons.item(i) {
            if let Some(html_btn) = btn.dyn_ref::<web_sys::HtmlElement>() {
                let label = html_btn
                    .get_attribute("aria-label")
                    .unwrap_or_default()
                    .to_lowercase();
                let title = html_btn
                    .get_attribute("title")
                    .unwrap_or_default()
                    .to_lowercase();
                let text = html_btn.text_content().unwrap_or_default().to_lowercase();
                let is_disabled = html_btn.get_attribute("disabled").is_some();

                let is_send = label.contains("send")
                    || title.contains("send")
                    || (text.contains("send") && !text.contains("settings"));

                if is_send && !is_disabled {
                    html_btn.click();
                    log::info!("[Trinity-Claude] Auto-submitted");
                    return true;
                }
            }
        }
    }

    log::warn!("[Trinity-Claude] Submit button not found or disabled");
    false
}

/// Inject text and optionally auto-submit.
#[wasm_bindgen]
pub fn claude_dispatch(text: &str, auto_submit: bool) -> bool {
    if claude_inject_text(text) {
        if auto_submit {
            let win = web_sys::window().ok_or("no window").unwrap();
            let cb = Closure::once(|| {
                claude_auto_submit();
            });
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                100,
            );
            cb.forget();
        }
        true
    } else {
        false
    }
}

/// Entry point for Claude.ai content script bootstrap.
#[wasm_bindgen(start)]
pub fn claude_injector_start() {
    log::info!("[Trinity-Claude] Content script loaded");
}
