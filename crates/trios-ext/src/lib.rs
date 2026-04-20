use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlElement, Window};

fn window() -> Result<Window, JsValue> {
    web_sys::window().ok_or_else(|| JsValue::from_str("no global window"))
}

fn document() -> Result<Document, JsValue> {
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

#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
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
    header.set_inner_html("<h1>Trios</h1>");
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
    chat.set_inner_html(r#"<div id="messages"></div><input id="chat-input" type="text" placeholder="Send a message..." />"#);
    app.append_child(&chat)?;

    let agents = doc.create_element("section")?;
    agents.set_id("tab-agents");
    agents.set_class_name("tab-content");
    agents.set_inner_html(r#"<div id="agent-list">No agents connected.</div>"#);
    app.append_child(&agents)?;

    let tools = doc.create_element("section")?;
    tools.set_id("tab-tools");
    tools.set_class_name("tab-content");
    tools.set_inner_html(r#"<div id="tool-list">No tools loaded.</div>"#);
    app.append_child(&tools)?;

    body.append_child(&app)?;

    setup_tab_listeners(&doc)?;

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

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Trios says: Hello, {}!", name)
}

#[wasm_bindgen]
pub fn get_agent_list() -> String {
    r#"[]"#.to_string()
}

const STYLE: &str = r#"
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #1a1a2e; color: #e0e0e0; }
.header { display: flex; align-items: center; gap: 8px; padding: 12px 16px; background: #16213e; border-bottom: 1px solid #0f3460; }
.header h1 { font-size: 16px; color: #e94560; }
.tabs { display: flex; background: #16213e; border-bottom: 1px solid #0f3460; }
.tab { flex: 1; padding: 10px; border: none; background: transparent; color: #a0a0a0; cursor: pointer; font-size: 13px; }
.tab.active { color: #e94560; border-bottom: 2px solid #e94560; }
.tab-content { display: none; padding: 16px; }
.tab-content.active { display: block; }
#chat-input { width: 100%; padding: 10px; border: 1px solid #0f3460; border-radius: 6px; background: #16213e; color: #e0e0e0; font-size: 14px; margin-top: 12px; }
#messages { max-height: 400px; overflow-y: auto; }
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greet_returns_message() {
        let result = greet("Agent");
        assert!(result.contains("Agent"));
    }

    #[test]
    fn greet_format() {
        let result = greet("Echo");
        assert_eq!(result, "Trios says: Hello, Echo!");
    }

    #[test]
    fn get_agent_list_returns_empty_array() {
        let result = get_agent_list();
        assert_eq!(result, "[]");
    }
}
