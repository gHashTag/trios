use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement, HtmlInputElement, Window};

fn window() -> Result<Window, JsValue> {
    web_sys::window().ok_or_else(|| JsValue::from_str("no global window"))
}

pub fn document() -> Result<Document, JsValue> {
    window()?
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))
}

fn body() -> Result<HtmlElement, JsValue> {
    document()?
        .body()
        .ok_or_else(|| JsValue::from_str("no body element"))
}

fn el(id: &str) -> Result<HtmlElement, JsValue> {
    document()?
        .get_element_by_id(id)
        .ok_or_else(|| JsValue::from_str(&format!("element #{} not found", id)))
        .and_then(|e| {
            e.dyn_into::<HtmlElement>()
                .map_err(|_| JsValue::from_str("cast failed"))
        })
}

pub const STYLE: &str = r#"
* { margin: 0; padding: 0; box-sizing: border-box; }
body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace;
    background: #000000;
    color: #FFFFFF;
    min-height: 100vh;
}
.header {
    display: flex; align-items: center; gap: 10px;
    padding: 14px 16px;
    background: #0A0A0A;
    border-bottom: 1px solid #D4AF37;
}
.header h1 { font-size: 16px; color: #D4AF37; letter-spacing: 2px; text-transform: uppercase; }
.header .status { font-size: 11px; color: #666; margin-left: auto; }
.tabs {
    display: flex;
    background: #0A0A0A;
    border-bottom: 1px solid #1A1A1A;
}
.tab {
    flex: 1; padding: 10px; border: none;
    background: transparent; color: #666;
    cursor: pointer; font-size: 12px; text-transform: uppercase; letter-spacing: 1px;
    transition: color 0.2s, border-bottom 0.2s;
}
.tab:hover { color: #999; }
.tab.active { color: #D4AF37; border-bottom: 2px solid #D4AF37; }
.tab-content { display: none; padding: 16px; }
.tab-content.active { display: block; }
#chat-input {
    width: 100%; padding: 10px 14px;
    border: 1px solid #1A1A1A; border-radius: 6px;
    background: #0A0A0A; color: #FFFFFF;
    font-size: 13px; margin-top: 12px;
}
#chat-input:focus { outline: none; border-color: #D4AF37; }
#chat-input::placeholder { color: #555; }
#messages { max-height: 400px; overflow-y: auto; display: flex; flex-direction: column; gap: 8px; }
.message { padding: 8px 12px; margin: 4px 0; border-radius: 6px; font-size: 13px; line-height: 1.5; }
.message.you { background: #1A1A1A; color: #FFFFFF; align-self: flex-end; border: 1px solid #333; }
.message.agent { background: #0D1117; color: #D4AF37; align-self: flex-start; border: 1px solid #1A3A5C; }
.message.error { background: #2A0A0A; color: #FF6B6B; }
#agent-list, #tool-list { font-size: 13px; font-family: monospace; white-space: pre-wrap; }
"#;

pub fn set_status(text: &str) -> Result<(), JsValue> {
    if let Ok(status_el) = el("connection-status") {
        status_el.set_text_content(Some(text));
    }
    Ok(())
}

pub fn append_message(role: &str, text: &str) {
    if let Ok(doc) = document() {
        if let Ok(messages) = el("messages") {
            let div = doc.create_element("div").unwrap();
            div.set_class_name(&format!("message {}", role));
            div.set_text_content(Some(text));
            let _ = messages.append_child(&div);
            messages.set_scroll_top(messages.scroll_height());
        }
    }
}

pub fn set_agent_list(text: &str) {
    if let Ok(el) = el("agent-list") {
        el.set_text_content(Some(text));
    }
}

pub fn set_tool_list(text: &str) {
    if let Ok(el) = el("tool-list") {
        el.set_text_content(Some(text));
    }
}

pub fn get_style() -> &'static str {
    STYLE
}

pub fn build_ui() -> Result<(), JsValue> {
    let doc = document()?;

    let head = doc
        .query_selector("head")?
        .ok_or_else(|| JsValue::from_str("no head"))?;
    let style = doc.create_element("style")?;
    style.set_text_content(Some(STYLE));
    head.append_child(&style)?;

    let body = body()?;

    let app = doc.create_element("div")?;
    app.set_id("app");
    let app: HtmlElement = app.dyn_into()?;

    let header = doc.create_element("header")?;
    header.set_class_name("header");
    header.set_inner_html(
        r#"<span style="font-size:24px;color:#D4AF37;font-weight:bold;">&#934;</span>
           <h1>Trinity</h1>
           <span id="connection-status" class="status">Connecting...</span>"#,
    );
    app.append_child(&header)?;

    let nav = doc.create_element("nav")?;
    nav.set_class_name("tabs");
    nav.set_inner_html(
        r#"<button class="tab active" data-tab="chat">Chat</button>
           <button class="tab" data-tab="agents">Agents</button>
           <button class="tab" data-tab="tools">MCP Tools</button>"#,
    );
    app.append_child(&nav)?;

    let chat = doc.create_element("section")?;
    chat.set_id("tab-chat");
    chat.set_class_name("tab-content active");
    chat.set_inner_html(
        r#"<div id="messages"></div>
           <input id="chat-input" type="text" placeholder="Send a message..." />"#,
    );
    app.append_child(&chat)?;

    let agents = doc.create_element("section")?;
    agents.set_id("tab-agents");
    agents.set_class_name("tab-content");
    agents.set_inner_html(r#"<div id="agent-list">Loading agents...</div>"#);
    app.append_child(&agents)?;

    let tools = doc.create_element("section")?;
    tools.set_id("tab-tools");
    tools.set_class_name("tab-content");
    tools.set_inner_html(r#"<div id="tool-list">Loading tools...</div>"#);
    app.append_child(&tools)?;

    body.append_child(&app)?;

    setup_tab_listeners(&doc)?;
    setup_chat_listener(&doc)?;

    Ok(())
}

fn setup_tab_listeners(doc: &Document) -> Result<(), JsValue> {
    let nav = doc.query_selector(".tabs")?.ok_or("no .tabs")?;
    let nav_el: &HtmlElement = nav.dyn_ref().ok_or("tabs cast")?;

    let closure = Closure::<dyn Fn(web_sys::Event)>::new(move |ev: web_sys::Event| {
        let target = ev.target();
        if let Some(btn) = target.and_then(|t| t.dyn_into::<HtmlElement>().ok()) {
            if btn.class_list().contains("tab") {
                if let Some(tab_name) = btn.get_attribute("data-tab") {
                    if let Ok(doc) = document() {
                        let _ = deactivate_all_tabs(&doc);
                        let _ = btn.class_list().add_1("active");
                        if let Ok(content) = el(&format!("tab-{}", tab_name)) {
                            let _ = content.class_list().add_1("active");
                        }
                    }
                }
            }
        }
    });

    nav_el.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
    closure.forget();

    Ok(())
}

fn deactivate_all_tabs(doc: &Document) -> Result<(), JsValue> {
    let tabs = doc.query_selector_all(".tab")?;
    for i in 0..tabs.length() {
        if let Some(t) = tabs.item(i) {
            let el: HtmlElement = t.dyn_into()?;
            el.class_list().remove_1("active")?;
        }
    }
    let contents = doc.query_selector_all(".tab-content")?;
    for i in 0..contents.length() {
        if let Some(c) = contents.item(i) {
            let el: HtmlElement = c.dyn_into()?;
            el.class_list().remove_1("active")?;
        }
    }
    Ok(())
}

fn setup_chat_listener(doc: &Document) -> Result<(), JsValue> {
    let input = doc.get_element_by_id("chat-input").ok_or("no chat-input")?;
    let input: HtmlInputElement = input.dyn_into()?;

    let closure =
        Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
            if ev.key() == "Enter" {
                if let Some(val) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("chat-input"))
                    .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                {
                    let text = val.value().trim().to_string();
                    if !text.is_empty() {
                        append_message("you", &text);
                        val.set_value("");
                        let msg = text.clone();
                        let _ = crate::mcp::mcp_send_chat(&msg);
                    }
                }
            }
        });

    input.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
    closure.forget();

    Ok(())
}
