//! UR-00 — WASM Entry Point for trios-ui
//!
//! Dioxus sidebar app that runs inside Chrome Extension sidepanel.
//! Connects to trios-server via WebSocket and provides chat, agents, and tools UI.

use dioxus::prelude::*;
use trios_ui_ring_ur07 as api;
use trios_ui_ring_ur08 as theme;

/// Build version — auto-incremented by xtask bump-version
pub const BUILD_VERSION: &str = env!("TRIOS_BUILD_VERSION", "dev");

/// WASM entry point — called when the module loads in the browser
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("[trios-ui] Launching Dioxus sidebar v{}", BUILD_VERSION);
    launch(app);
}

/// Message type for chat display
#[derive(Clone, PartialEq, Debug)]
struct ChatMsg {
    role: String,
    content: String,
}

/// Inject theme CSS into the document head
fn inject_theme() {
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Some(head) = doc.query_selector("head").ok().flatten() {
                if let Ok(style) = doc.create_element("style") {
                    style.set_text_content(Some(theme::STYLESHEET));
                    let _ = head.append_child(&style);
                    log::info!("[trios-ui] Theme CSS injected");
                }
            }
        }
    }
}

/// Root App component
fn app() -> Element {
    let mut messages: Signal<Vec<ChatMsg>> = use_signal(Vec::new);
    let mut status: Signal<String> = use_signal(|| "Connecting...".to_string());
    let mut agents: Signal<String> = use_signal(|| "Loading agents...".to_string());
    let mut tools: Signal<String> = use_signal(|| "Loading tools...".to_string());
    let mut active_tab: Signal<String> = use_signal(|| "chat".to_string());
    let connected: Signal<bool> = use_signal(|| false);
    let mut input_text: Signal<String> = use_signal(String::new);
    let mut ws_client: Signal<Option<api::ApiClient>> = use_signal(|| None);

    use_hook(inject_theme);

    let mut connected_open = connected;
    let mut status_open = status;
    let mut connected_err = connected;
    let mut status_err = status;

    use_hook(move || {
        let mut client = api::ApiClient::new();

        let result = client.connect_with_callback(
            move |text| {
                let preview = if text.len() > 120 {
                    format!("{}…", &text[..120])
                } else {
                    text.clone()
                };

                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    let method = val.get("method").and_then(|m| m.as_str()).unwrap_or("<no method>");
                    let has_result = val.get("result").is_some();
                    let has_error = val.get("error").is_some();
                    log::info!(
                        "[trios-ui] WS recv {} bytes | method={} result={} error={} | {}",
                        text.len(), method, has_result, has_error, preview
                    );

                    match method {
                        "agents/chat" | "chat" => {
                            if let Some(r) = val.get("result") {
                                // Check for error inside result
                                if let Some(err) = r.get("error").and_then(|v| v.as_str()) {
                                    log::warn!("[trios-ui] CHAT error: {}", err);
                                    messages.write().push(ChatMsg {
                                        role: "error".to_string(),
                                        content: format!("⚠ {}", err),
                                    });
                                } else {
                                    let response = r
                                        .get("response")
                                        .and_then(|v| v.as_str())
                                        .or_else(|| r.get("content").and_then(|v| v.as_str()))
                                        .unwrap_or("{no content}");
                                    log::info!("[trios-ui] CHAT response: {} chars", response.len());
                                    messages.write().push(ChatMsg {
                                        role: "agent".to_string(),
                                        content: response.to_string(),
                                    });
                                }
                            } else if has_error {
                                log::warn!("[trios-ui] CHAT error: {}", val.get("error").unwrap());
                            }
                        }
                        "agents/list" => {
                            if let Some(r) = val.get("result") {
                                let pretty = serde_json::to_string_pretty(r).unwrap_or_default();
                                let count = r.as_array().map(|a| a.len()).unwrap_or(0);
                                log::info!("[trios-ui] AGENTS list: {} agents", count);
                                *agents.write() = if count == 0 {
                                    "No agents registered".to_string()
                                } else {
                                    pretty
                                };
                            }
                        }
                        "tools/list" => {
                            if let Some(r) = val.get("result") {
                                let count = r.as_array().map(|a| a.len()).unwrap_or(0);
                                log::info!("[trios-ui] TOOLS list: {} tools", count);
                                if count == 0 {
                                    *tools.write() = "No tools available".to_string();
                                } else if let Some(arr) = r.as_array() {
                                    let names: Vec<String> = arr.iter()
                                        .filter_map(|t| {
                                            t.get("name").and_then(|n| n.as_str()).map(|s| s.to_string())
                                        })
                                        .collect();
                                    *tools.write() = format!("{} tools:\n{}", count, names.join("\n"));
                                }
                            }
                        }
                        other => {
                            log::warn!("[trios-ui] WS unhandled method='{}' payload={}", other, preview);
                        }
                    }
                } else {
                    log::warn!("[trios-ui] WS recv non-JSON {} bytes: {}", text.len(), preview);
                }
            },
            move || {
                log::info!("[trios-ui] WS on_open fired — sending agents/list + tools/list");
                status_open.set("Connected".to_string());
                connected_open.set(true);
            },
            move || {
                log::warn!("[trios-ui] WS on_error/close fired");
                connected_err.set(false);
                status_err.set("Disconnected".to_string());
            },
        );

        if result.is_ok() {
            status.set("Connecting...".to_string());
            ws_client.set(Some(client));
        } else {
            log::error!("[trios-ui] connect_with_callback failed");
            status.set("Connection failed".to_string());
        }
    });

    let active = active_tab.read().clone();
    let is_connected = *connected.read();
    let status_text = status.read().clone();
    let input_val = input_text.read().clone();

    let status_cls = if is_connected { "status connected" } else { "status error" };
    let chat_cls = if active == "chat" { "tab-content active" } else { "tab-content" };
    let agents_cls = if active == "agents" { "tab-content active" } else { "tab-content" };
    let tools_cls = if active == "tools" { "tab-content active" } else { "tab-content" };
    let chat_tab_cls = if active == "chat" { "tab active" } else { "tab" };
    let agents_tab_cls = if active == "agents" { "tab active" } else { "tab" };
    let tools_tab_cls = if active == "tools" { "tab active" } else { "tab" };

    rsx! {
        div { id: "main",
            header { class: "header",
                span { style: "font-size:24px;color:#D4AF37;font-weight:bold;", "Φ" }
                h1 { "Trinity" }
                span { class: "{status_cls}", "{status_text}" }
            }

            nav { class: "tabs",
                button {
                    class: "{chat_tab_cls}",
                    onclick: move |_| active_tab.set("chat".to_string()),
                    "Chat"
                }
                button {
                    class: "{agents_tab_cls}",
                    onclick: move |_| active_tab.set("agents".to_string()),
                    "Agents"
                }
                button {
                    class: "{tools_tab_cls}",
                    onclick: move |_| active_tab.set("tools".to_string()),
                    "Tools"
                }
            }

            section { class: "{chat_cls}", id: "tab-chat",
                div { id: "messages",
                    for msg in messages.read().iter() {
                        div { class: "message {msg.role}", "{msg.content}" }
                    }
                }
                input {
                    id: "chat-input",
                    r#type: "text",
                    value: "{input_val}",
                    placeholder: "Send a message...",
                    oninput: move |ev| input_text.set(ev.value()),
                    onkeydown: move |ev: KeyboardEvent| {
                        if ev.key() == Key::Enter {
                            let text = input_text.read().trim().to_string();
                            if !text.is_empty() {
                                messages.write().push(ChatMsg {
                                    role: "you".to_string(),
                                    content: text.clone(),
                                });
                                input_text.set(String::new());
                                let guard = ws_client.read();
                                if let Some(client) = guard.as_ref() {
                                    log::info!("[trios-ui] CHAT send: {}", &text);
                                    if let Err(e) = client.send_chat(&text) {
                                        log::error!("[trios-ui] send_chat failed: {:?}", e);
                                    }
                                } else {
                                    log::warn!("[trios-ui] No WS client — message dropped");
                                }
                            }
                        }
                    }
                }
            }

            section { class: "{agents_cls}", id: "tab-agents",
                div { id: "agent-list", "{agents}" }
            }

            section { class: "{tools_cls}", id: "tab-tools",
                div { id: "tool-list", "{tools}" }
            }
        }
    }
}
