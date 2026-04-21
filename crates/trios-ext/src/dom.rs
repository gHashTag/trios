use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub fn document() -> Result<web_sys::Document, JsValue> {
    web_sys::window().ok_or("no window")?.document().ok_or("no doc".into())
}
fn body() -> Result<web_sys::HtmlElement, JsValue> { document()?.body().ok_or("no body".into()) }
fn el(id: &str) -> Result<web_sys::HtmlElement, JsValue> {
    document()?.get_element_by_id(id).ok_or_else(|| JsValue::from_str(&format!("#{id} not found")))?
        .dyn_into::<web_sys::HtmlElement>().map_err(|_| "not HtmlElement".into())
}

pub const STYLE: &str = r#"
:root { --bg:#0d1117; --surface:#161b22; --border:#30363d; --text:#c9d1d9; --accent:#58a6ff; }
* { margin:0; padding:0; box-sizing:border-box; }
body { font-family:-apple-system,sans-serif; background:var(--bg); color:var(--text); height:100vh; display:flex; flex-direction:column; }
#status { padding:6px 12px; background:var(--surface); border-bottom:1px solid var(--border); font-size:12px; }
#tab-bar { display:flex; background:var(--surface); border-bottom:1px solid var(--border); }
.tab { flex:1; padding:8px; text-align:center; cursor:pointer; font-size:13px; border-bottom:2px solid transparent; }
.tab.active { border-bottom-color:var(--accent); color:var(--accent); }
#content { flex:1; overflow:hidden; position:relative; }
.panel { display:none; position:absolute; inset:0; overflow-y:auto; padding:12px; }
.panel.active { display:block; }
#messages { flex:1; overflow-y:auto; }
.msg { margin:4px 0; padding:6px 10px; border-radius:8px; max-width:85%; font-size:13px; }
.msg.user { background:#1f6feb33; }
.msg.agent { background:var(--surface); border:1px solid var(--border); }
.msg.error { background:#f8514922; color:#f85149; }
#input-area { display:flex; gap:8px; padding:8px 0; border-top:1px solid var(--border); }
#chat-input { flex:1; background:var(--surface); border:1px solid var(--border); border-radius:6px; padding:8px; color:var(--text); outline:none; }
#chat-input:focus { border-color:var(--accent); }
#send-btn { background:var(--accent); color:#fff; border:none; border-radius:6px; padding:8px 16px; cursor:pointer; }
"#;

pub fn set_status(t: &str) -> Result<(), JsValue> { el("status")?.set_text_content(Some(t)); Ok(()) }
pub fn append_message(role: &str, t: &str) {
    let _ = (|| -> Result<(), JsValue> { let m = el("messages")?; let d = document()?.create_element("div")?; d.set_class_name(&format!("msg {role}")); d.set_text_content(Some(t)); m.append_child(&d)?; Ok(()) })();
}
pub fn set_agent_list(t: &str) { let _ = el("agents-panel").map(|p| p.set_text_content(Some(t))); }
pub fn set_tool_list(t: &str) { let _ = el("tools-panel").map(|p| p.set_text_content(Some(t))); }

pub fn build_ui() -> Result<(), JsValue> {
    let doc = document()?; let b = body()?;
    let style = doc.create_element("style")?; style.set_text_content(Some(STYLE)); doc.query_selector("head")?.unwrap().append_child(&style)?;
    let sb = doc.create_element("div")?; sb.set_id("status"); sb.set_text_content(Some("Initializing...")); b.append_child(&sb)?;
    let tb = doc.create_element("div")?; tb.set_id("tab-bar");
    tb.set_inner_html(r#"<div class="tab active" data-tab="chat">Chat</div><div class="tab" data-tab="agents">Agents</div><div class="tab" data-tab="tools">Tools</div>"#);
    b.append_child(&tb)?;
    let ct = doc.create_element("div")?; ct.set_id("content");
    let cp = doc.create_element("div")?; cp.set_class_name("panel active"); cp.set_id("chat-panel");
    cp.set_inner_html(r#"<div id="messages"></div><div id="input-area"><input id="chat-input" type="text" placeholder="Type..."/><button id="send-btn">Send</button></div>"#);
    ct.append_child(&cp)?;
    let ap = doc.create_element("div")?; ap.set_class_name("panel"); ap.set_id("agents-panel"); ap.set_text_content(Some("Loading...")); ct.append_child(&ap)?;
    let tp = doc.create_element("div")?; tp.set_class_name("panel"); tp.set_id("tools-panel"); tp.set_text_content(Some("Loading...")); ct.append_child(&tp)?;
    b.append_child(&ct)?;
    setup_tabs(&doc)?; setup_chat(&doc)?;
    Ok(())
}

fn setup_tabs(doc: &web_sys::Document) -> Result<(), JsValue> {
    let tabs = doc.query_selector_all("#tab-bar .tab")?;
    for i in 0..tabs.length() {
        if let Some(tab) = tabs.item(i) {
            let te = tab.clone();
            let cl: Closure<dyn Fn()> = Closure::new(move || { let _ = (|| -> Result<(), JsValue> {
                let d = web_sys::window().unwrap().document().unwrap();
                for j in 0..d.query_selector_all("#tab-bar .tab")?.length() { if let Some(t) = d.query_selector_all("#tab-bar .tab")?.item(j) { t.dyn_into::<web_sys::HtmlElement>()?.class_list().remove_1("active")?; } }
                for j in 0..d.query_selector_all(".panel")?.length() { if let Some(p) = d.query_selector_all(".panel")?.item(j) { p.dyn_into::<web_sys::HtmlElement>()?.class_list().remove_1("active")?; } }
                let te_el: web_sys::Element = te.clone().dyn_into()?;
                te_el.class_list().add_1("active")?;
                if let Some(n) = te_el.get_attribute("data-tab") { if let Some(p) = d.get_element_by_id(&format!("{n}-panel")) { p.class_list().add_1("active")?; } }
                Ok(())
            })(); });
            tab.add_event_listener_with_callback("click", cl.as_ref().unchecked_ref())?; cl.forget();
        }
    }
    Ok(())
}

fn setup_chat(doc: &web_sys::Document) -> Result<(), JsValue> {
    let inp = doc.get_element_by_id("chat-input").ok_or("no input")?;
    let ic = inp.clone();
    let cl: Closure<dyn Fn()> = Closure::new(move || { send_msg(&ic); });
    doc.get_element_by_id("send-btn").ok_or("no btn")?.add_event_listener_with_callback("click", cl.as_ref().unchecked_ref())?; cl.forget();
    let ic2 = inp.clone();
    let kl: Closure<dyn Fn(web_sys::KeyboardEvent)> = Closure::new(move |e: web_sys::KeyboardEvent| { if e.key() == "Enter" { send_msg(&ic2); } });
    inp.add_event_listener_with_callback("keydown", kl.as_ref().unchecked_ref())?; kl.forget();
    Ok(())
}

fn send_msg(inp: &web_sys::Element) {
    if let Some(i) = inp.dyn_ref::<web_sys::HtmlInputElement>() {
        let v = i.value(); if !v.is_empty() { append_message("user", &v); let _ = crate::mcp::mcp_send_chat(&v); i.set_value(""); }
    }
}
