//! EXT-00/dom — Sidepanel UI builder

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
:root { --bg:#000000; --surface:#0a0a0a; --border:#1a1a1a; --text:#ffffff; --accent:#F5D3F2; --green:#28A745; --orange:#FF4500; --red:#E74C3C; --purple:#F5D3F2; }
* { margin:0; padding:0; box-sizing:border-box; }
body { font-family:-apple-system,sans-serif; background:var(--bg); color:var(--text); height:100vh; display:flex; flex-direction:column; }
#status { padding:6px 12px; background:var(--surface); border-bottom:1px solid var(--border); font-size:12px; color:#888; }
#tab-bar { display:flex; background:var(--surface); border-bottom:1px solid var(--border); }
.tab { flex:1; padding:8px; text-align:center; cursor:pointer; font-size:13px; border-bottom:2px solid transparent; color:#888; }
.tab.active { border-bottom-color:var(--accent); color:var(--accent); }
#content { flex:1; overflow:hidden; position:relative; }
.panel { display:none; position:absolute; inset:0; overflow-y:auto; padding:12px; }
.panel.active { display:block; }
#messages { flex:1; overflow-y:auto; }
.msg { margin:4px 0; padding:6px 10px; border-radius:8px; max-width:85%; font-size:13px; }
.msg.user { background:#F5D3F222; }
.msg.agent { background:var(--surface); border:1px solid var(--border); }
.msg.error { background:#E74C3C22; color:var(--red); }
#input-area { display:flex; gap:8px; padding:8px 0; border-top:1px solid var(--border); }
#chat-input { flex:1; background:var(--surface); border:1px solid var(--border); border-radius:6px; padding:8px; color:var(--text); outline:none; }
#chat-input:focus { border-color:var(--accent); }
#send-btn { background:var(--accent); color:#000; border:none; border-radius:6px; padding:8px 16px; cursor:pointer; font-weight:600; }
.issue-item { display:flex; align-items:flex-start; gap:8px; padding:10px 12px; border:1px solid var(--border); border-radius:8px; margin-bottom:8px; background:var(--surface); cursor:pointer; }
.issue-item:hover { border-color:var(--accent); }
.issue-state { font-size:12px; padding:2px 8px; border-radius:12px; font-weight:600; white-space:nowrap; }
.issue-state.open { background:#28A74522; color:var(--green); }
.issue-state.closed { background:#E74C3C22; color:var(--red); }
.issue-state.in-progress { background:#FF450022; color:var(--orange); }
.issue-number { color:var(--accent); font-size:12px; font-weight:600; }
.issue-title { font-size:13px; line-height:1.4; }
.issue-meta { font-size:11px; color:#888; margin-top:4px; }
.issue-label { display:inline-block; padding:1px 8px; border-radius:12px; font-size:10px; margin-right:4px; border:1px solid var(--border); }
.issue-empty { text-align:center; padding:40px 20px; color:#888; font-size:14px; }
.tool-item { display:flex; flex-direction:column; gap:4px; padding:12px; border:1px solid var(--border); border-radius:8px; margin-bottom:8px; background:var(--surface); }
.tool-item:hover { border-color:var(--accent); }
.tool-name { font-size:14px; font-weight:600; color:var(--accent); }
.tool-description { font-size:12px; color:#aaa; line-height:1.4; }
.tool-empty { text-align:center; padding:40px 20px; color:#888; font-size:14px; }
.settings-section { margin-bottom:20px; }
.settings-label { font-size:12px; color:#888; margin-bottom:6px; display:block; }
.settings-input { width:100%; background:var(--surface); border:1px solid var(--border); border-radius:6px; padding:8px; color:var(--text); font-size:13px; outline:none; font-family:monospace; }
.settings-input:focus { border-color:var(--accent); }
.settings-btn { background:var(--accent); color:#000; border:none; border-radius:6px; padding:8px 16px; cursor:pointer; font-weight:600; margin-top:8px; }
.settings-btn:hover { opacity:0.9; }
.settings-status { font-size:12px; margin-top:8px; }
.settings-status.ok { color:var(--green); }
.settings-status.err { color:var(--red); }
"#;

pub fn set_status(t: &str) -> Result<(), JsValue> { el("status")?.set_text_content(Some(t)); Ok(()) }
pub fn append_message(role: &str, t: &str) {
    let _ = (|| -> Result<(), JsValue> { let m = el("messages")?; let d = document()?.create_element("div")?; d.set_class_name(&format!("msg {role}")); d.set_text_content(Some(t)); m.append_child(&d)?; Ok(()) })();
}
pub fn set_agent_list(t: &str) { let _ = el("agents-panel").map(|p| p.set_text_content(Some(t))); }

/// Render artifacts from BR-OUTPUT JSON into the artifacts panel.
pub fn set_artifacts(json: &str) {
    let _ = (|| -> Result<(), JsValue> {
        let panel = el("artifacts-panel")?;
        match trios_ext_01::render_artifacts(json) {
            Ok(html) => { panel.set_inner_html(&html); }
            Err(e) => {
                panel.set_inner_html(&format!(
                    "<div style=\"text-align:center;padding:40px 20px;color:#E74C3C;font-size:13px;\">\
                     Failed to render artifacts: {:?}</div>", e
                ));
            }
        }
        Ok(())
    })();
}

/// Append a single artifact HTML to the artifacts panel.
pub fn append_artifact(artifact_json: &str) {
    let _ = (|| -> Result<(), JsValue> {
        let panel = el("artifacts-panel")?;
        if let Some(empty) = panel.query_selector(".artifact-empty")? {
            empty.remove();
        }
        let html = trios_ext_01::render_artifacts(artifact_json)?;
        panel.insert_adjacent_html("beforeend", &html)?;
        Ok(())
    })();
}

/// Render tools list from JSON: `{"tools":[{"name":"...","description":"..."},...]}`
pub fn set_tool_list(tools_json: &str) {
    let _ = (|| -> Result<(), JsValue> {
        let panel = el("tools-panel")?;
        let doc = document()?;
        panel.set_inner_html("");

        let tools_val: serde_json::Value = serde_json::from_str(tools_json)
            .unwrap_or(serde_json::json!([]));

        let tools: Vec<serde_json::Value> = if let Some(obj) = tools_val.as_object() {
            obj.get("tools")
                .and_then(|t| t.as_array())
                .cloned()
                .unwrap_or_default()
        } else {
            tools_val.as_array()
                .cloned()
                .unwrap_or_default()
        };

        if tools.is_empty() {
            let empty = doc.create_element("div")?;
            empty.set_class_name("tool-empty");
            empty.set_text_content(Some("No tools available"));
            panel.append_child(&empty)?;
            return Ok(());
        }

        for tool in tools {
            let name = tool.get("name").and_then(|v| v.as_str()).unwrap_or("(unnamed)");
            let description = tool.get("description").and_then(|v| v.as_str()).unwrap_or("");

            let item = doc.create_element("div")?;
            item.set_class_name("tool-item");

            let name_el = doc.create_element("div")?;
            name_el.set_class_name("tool-name");
            name_el.set_text_content(Some(name));
            item.append_child(&name_el)?;

            if !description.is_empty() {
                let desc_el = doc.create_element("div")?;
                desc_el.set_class_name("tool-description");
                desc_el.set_text_content(Some(description));
                item.append_child(&desc_el)?;
            }

            panel.append_child(&item)?;
        }
        Ok(())
    })();
}

/// Render a full issue list from a JSON array.
pub fn set_issue_list(issues_json: &str) {
    let _ = (|| -> Result<(), JsValue> {
        let panel = el("issues-panel")?;
        let doc = document()?;
        panel.set_inner_html("");
        let issues: Vec<serde_json::Value> = serde_json::from_str(issues_json)
            .map_err(|e| JsValue::from_str(&format!("parse error: {e}")))?;
        if issues.is_empty() {
            let empty = doc.create_element("div")?;
            empty.set_class_name("issue-empty");
            empty.set_text_content(Some("No issues found"));
            panel.append_child(&empty)?;
            return Ok(());
        }
        for issue in &issues {
            let number = issue.get("number").and_then(|v| v.as_u64()).unwrap_or(0);
            let title = issue.get("title").and_then(|v| v.as_str()).unwrap_or("(untitled)");
            let state = issue.get("state").and_then(|v| v.as_str()).unwrap_or("open");
            let labels: Vec<String> = issue.get("labels")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|l| {
                    l.as_str().map(String::from)
                        .or_else(|| l.get("name").and_then(|n| n.as_str()).map(String::from))
                }).collect())
                .unwrap_or_default();
            append_issue_el(&panel, &doc, number, title, state, &labels)?;
        }
        Ok(())
    })();
}

/// Append a single issue item to the issues panel.
pub fn append_issue(number: u64, title: &str, state: &str, labels_json: &str) {
    let _ = (|| -> Result<(), JsValue> {
        let panel = el("issues-panel")?;
        let doc = document()?;
        let labels: Vec<String> = serde_json::from_str(labels_json).unwrap_or_default();
        append_issue_el(&panel, &doc, number, title, state, &labels)
    })();
}

fn append_issue_el(
    panel: &web_sys::HtmlElement,
    doc: &web_sys::Document,
    number: u64,
    title: &str,
    state: &str,
    labels: &[String],
) -> Result<(), JsValue> {
    let item = doc.create_element("div")?;
    item.set_class_name("issue-item");

    let badge = doc.create_element("span")?;
    badge.set_class_name(&format!("issue-state {state}"));
    badge.set_text_content(Some(state));
    item.append_child(&badge)?;

    let num_el = doc.create_element("span")?;
    num_el.set_class_name("issue-number");
    num_el.set_text_content(Some(&format!("#{}", number)));
    item.append_child(&num_el)?;

    let body_el = doc.create_element("div")?;
    body_el.set_class_name("issue-body");
    let title_el = doc.create_element("div")?;
    title_el.set_class_name("issue-title");
    title_el.set_text_content(Some(title));
    body_el.append_child(&title_el)?;

    if !labels.is_empty() {
        let meta = doc.create_element("div")?;
        meta.set_class_name("issue-meta");
        for lbl in labels {
            let l_el = doc.create_element("span")?;
            l_el.set_class_name("issue-label");
            l_el.set_text_content(Some(lbl));
            meta.append_child(&l_el)?;
        }
        body_el.append_child(&meta)?;
    }
    item.append_child(&body_el)?;
    panel.append_child(&item)?;
    Ok(())
}

pub fn build_ui() -> Result<(), JsValue> {
    let doc = document()?; let b = body()?;
    let style = doc.create_element("style")?;
    style.set_text_content(Some(&format!("{STYLE}{}", trios_ext_01::ARTIFACT_CSS)));
    doc.query_selector("head")?.unwrap().append_child(&style)?;
    let sb = doc.create_element("div")?; sb.set_id("status"); sb.set_text_content(Some("Initializing...")); b.append_child(&sb)?;
    let tb = doc.create_element("div")?; tb.set_id("tab-bar");
    tb.set_inner_html(r#"<div class="tab active" data-tab="chat">Chat</div><div class="tab" data-tab="agents">Agents</div><div class="tab" data-tab="tools">Tools</div><div class="tab" data-tab="issues">Issues</div><div class="tab" data-tab="artifacts">Artifacts</div><div class="tab" data-tab="settings">⚙</div>"#);
    b.append_child(&tb)?;
    let ct = doc.create_element("div")?; ct.set_id("content");
    let cp = doc.create_element("div")?; cp.set_class_name("panel active"); cp.set_id("chat-panel");
    cp.set_inner_html(r#"<div id="messages"></div><div id="input-area"><input id="chat-input" type="text" placeholder="Type..."/><button id="send-btn">Send</button></div>"#);
    ct.append_child(&cp)?;
    let ap = doc.create_element("div")?; ap.set_class_name("panel"); ap.set_id("agents-panel"); ap.set_text_content(Some("Loading...")); ct.append_child(&ap)?;
    let tp = doc.create_element("div")?; tp.set_class_name("panel"); tp.set_id("tools-panel"); tp.set_text_content(Some("Loading...")); ct.append_child(&tp)?;
    let ip = doc.create_element("div")?; ip.set_class_name("panel"); ip.set_id("issues-panel"); ip.set_text_content(Some("Loading issues...")); ct.append_child(&ip)?;
    // Artifacts panel — renders BR-OUTPUT artifacts
    let artp = doc.create_element("div")?; artp.set_class_name("panel"); artp.set_id("artifacts-panel");
    artp.set_inner_html(r#"<div class="artifact-empty" style="text-align:center;padding:40px 20px;color:#888;font-size:14px;">No artifacts loaded</div>"#);
    ct.append_child(&artp)?;
    let sp = doc.create_element("div")?; sp.set_class_name("panel"); sp.set_id("settings-panel");
    sp.set_inner_html(r#"<div class="settings-section"><label class="settings-label">z.ai API Key</label><input id="zai-key-input" class="settings-input" type="password" placeholder="Enter z.ai API key..."/><button id="save-key-btn" class="settings-btn">Save Key</button><div id="key-status" class="settings-status"></div></div>"#);
    ct.append_child(&sp)?;
    b.append_child(&ct)?;
    setup_tabs(&doc)?; setup_chat(&doc)?; setup_settings(&doc)?;
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
        let v = i.value(); if !v.is_empty() { append_message("user", &v); let _ = super::mcp::mcp_send_chat(&v); i.set_value(""); }
    }
}

fn setup_settings(doc: &web_sys::Document) -> Result<(), JsValue> {
    let btn = doc.get_element_by_id("save-key-btn").ok_or("no save-key-btn")?;
    let cl: Closure<dyn Fn()> = Closure::new(|| {
        let doc = match web_sys::window().and_then(|w| w.document()) {
            Some(d) => d,
            None => return,
        };
        let input = match doc.get_element_by_id("zai-key-input") {
            Some(i) => i,
            None => return,
        };
        let status_el = doc.get_element_by_id("key-status");
        if let Some(inp) = input.dyn_ref::<web_sys::HtmlInputElement>() {
            let key = inp.value();
            if key.is_empty() {
                if let Some(s) = &status_el {
                    s.set_text_content(Some("Key cannot be empty"));
                    if let Ok(el) = s.clone().dyn_into::<web_sys::HtmlElement>() {
                        let _ = el.class_list().remove_1("ok");
                        let _ = el.class_list().add_1("err");
                    }
                }
                return;
            }
            match trios_ext_02::save_api_key(&key) {
                Ok(()) => {
                    if let Some(s) = &status_el {
                        s.set_text_content(Some("✓ Key saved"));
                        if let Ok(el) = s.clone().dyn_into::<web_sys::HtmlElement>() {
                            let _ = el.class_list().remove_1("err");
                            let _ = el.class_list().add_1("ok");
                        }
                    }
                }
                Err(e) => {
                    if let Some(s) = &status_el {
                        s.set_text_content(Some(&format!("Save failed: {:?}", e)));
                        if let Ok(el) = s.clone().dyn_into::<web_sys::HtmlElement>() {
                            let _ = el.class_list().remove_1("ok");
                            let _ = el.class_list().add_1("err");
                        }
                    }
                }
            }
        }
    });
    btn.add_event_listener_with_callback("click", cl.as_ref().unchecked_ref())?;
    cl.forget();
    Ok(())
}
