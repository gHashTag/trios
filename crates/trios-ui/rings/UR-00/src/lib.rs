//! UR-00 — WASM Entry Point for trios-ui

use dioxus::prelude::*;
use trios_ui_ring_ur07 as api;
use trios_ui_ring_ur08 as theme;

pub const BUILD_VERSION: &str = env!("TRIOS_BUILD_VERSION", "dev");

/// Provider config injected at build time from .env
pub const ANTHROPIC_KEY_HINT: &str = env!("TRIOS_ANTHROPIC_KEY_HINT", "");
pub const OPENAI_KEY_HINT: &str = env!("TRIOS_OPENAI_KEY_HINT", "");
pub const VENICE_KEY_HINT: &str = env!("TRIOS_VENICE_KEY_HINT", "");

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("[trios-ui] Launching Dioxus sidebar v{}", BUILD_VERSION);
    launch(app);
}

#[derive(Clone, PartialEq, Debug)]
struct ChatMsg {
    role: String,
    content: String,
}

fn inject_theme() {
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Some(head) = doc.query_selector("head").ok().flatten() {
                if let Ok(style) = doc.create_element("style") {
                    style.set_text_content(Some(theme::STYLESHEET));
                    let _ = head.append_child(&style);
                }
            }
        }
    }
}

fn app() -> Element {
    let mut messages: Signal<Vec<ChatMsg>> = use_signal(Vec::new);
    let mut status: Signal<String> = use_signal(|| "Connecting...".to_string());
    let mut agents: Signal<String> = use_signal(|| "Loading agents...".to_string());
    let mut tools: Signal<Vec<String>> = use_signal(Vec::new);
    let mut active_tab: Signal<String> = use_signal(|| "chat".to_string());
    let connected: Signal<bool> = use_signal(|| false);
    let mut input_text: Signal<String> = use_signal(String::new);
    let mut ws_client: Signal<Option<api::ApiClient>> = use_signal(|| None);

    // Settings state — pre-filled from .env hints
    let mut anthropic_key: Signal<String> = use_signal(|| ANTHROPIC_KEY_HINT.to_string());
    let mut openai_key: Signal<String> = use_signal(|| OPENAI_KEY_HINT.to_string());
    let mut venice_key: Signal<String> = use_signal(|| VENICE_KEY_HINT.to_string());

    use_hook(inject_theme);

    let mut connected_open = connected;
    let mut status_open = status;
    let mut connected_err = connected;
    let mut status_err = status;

    use_hook(move || {
        let mut client = api::ApiClient::new();
        let result = client.connect_with_callback(
            move |text| {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    let method = val.get("method").and_then(|m| m.as_str()).unwrap_or("");
                    match method {
                        "agents/chat" | "chat" => {
                            if let Some(r) = val.get("result") {
                                let (role, content) = if let Some(err) = r.get("error").and_then(|e| e.as_str()) {
                                    ("error".to_string(), format!("⚠ {}", err))
                                } else {
                                    let resp = r.get("response").and_then(|v| v.as_str())
                                        .or_else(|| r.get("content").and_then(|v| v.as_str()))
                                        .unwrap_or("{no content}");
                                    ("agent".to_string(), resp.to_string())
                                };
                                messages.write().push(ChatMsg { role, content });
                            }
                        }
                        "agents/list" => {
                            if let Some(r) = val.get("result") {
                                let count = r.as_array().map(|a| a.len()).unwrap_or(0);
                                *agents.write() = if count == 0 {
                                    "No agents registered".to_string()
                                } else {
                                    serde_json::to_string_pretty(r).unwrap_or_default()
                                };
                            }
                        }
                        "tools/list" => {
                            if let Some(r) = val.get("result") {
                                if let Some(arr) = r.as_array() {
                                    let names: Vec<String> = arr.iter()
                                        .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                                        .collect();
                                    *tools.write() = names;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            },
            move || {
                status_open.set("Connected".to_string());
                connected_open.set(true);
            },
            move || {
                connected_err.set(false);
                status_err.set("Disconnected".to_string());
            },
        );
        if result.is_ok() {
            ws_client.set(Some(client));
        } else {
            status.set("Connection failed".to_string());
        }
    });

    let active = active_tab.read().clone();
    let is_connected = *connected.read();
    let status_text = status.read().clone();
    let input_val = input_text.read().clone();
    let tools_list = tools.read().clone();
    let tool_count = tools_list.len();

    let status_cls = if is_connected { "status connected" } else { "status error" };

    macro_rules! tab_cls { ($name:expr) => { if active == $name { "tab active" } else { "tab" } }; }
    macro_rules! content_cls { ($name:expr) => { if active == $name { "tab-content active" } else { "tab-content" } }; }

    let anthropic_val = anthropic_key.read().clone();
    let openai_val = openai_key.read().clone();
    let venice_val = venice_key.read().clone();

    let has_anthropic = !anthropic_val.is_empty() && anthropic_val != "";
    let has_openai = !openai_val.is_empty() && openai_val != "";
    let has_venice = !venice_val.is_empty() && venice_val != "";

    rsx! {
        div { id: "main",
            header { class: "header",
                span { class: "phi-icon", "\u{03A6}" }
                h1 { "Trinity" }
                span { class: "{status_cls}", "{status_text}" }
            }

            nav { class: "tabs",
                button { class: tab_cls!("chat"),    onclick: move |_| active_tab.set("chat".to_string()),     "Chat" }
                button { class: tab_cls!("agents"),  onclick: move |_| active_tab.set("agents".to_string()),   "Agents" }
                button { class: tab_cls!("tools"),   onclick: move |_| active_tab.set("tools".to_string()),    "Tools ({tool_count})" }
                button { class: tab_cls!("settings"),onclick: move |_| active_tab.set("settings".to_string()), "\u{2699}" }
            }

            // CHAT
            section { class: content_cls!("chat"), id: "tab-chat",
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
                                messages.write().push(ChatMsg { role: "you".to_string(), content: text.clone() });
                                input_text.set(String::new());
                                let guard = ws_client.read();
                                if let Some(client) = guard.as_ref() {
                                    let _ = client.send_chat(&text);
                                }
                            }
                        }
                    }
                }
            }

            // AGENTS
            section { class: content_cls!("agents"), id: "tab-agents",
                div { id: "agent-list", "{agents}" }
            }

            // TOOLS
            section { class: content_cls!("tools"), id: "tab-tools",
                div { id: "tool-list",
                    if tools_list.is_empty() {
                        div { "Loading tools..." }
                    } else {
                        for tool in tools_list.iter() {
                            div { class: "tool-item", "{tool}" }
                        }
                    }
                }
            }

            // SETTINGS
            section { class: content_cls!("settings"), id: "tab-settings",
                div { class: "settings-section",
                    div { class: "settings-title", "AI Providers" }

                    // Anthropic
                    div { class: "provider-card",
                        div { class: "provider-header",
                            span { class: "provider-name", "Anthropic" }
                            span { class: if has_anthropic { "provider-badge active" } else { "provider-badge inactive" },
                                if has_anthropic { "active" } else { "no key" }
                            }
                        }
                        div { class: "token-row",
                            input {
                                class: "token-input",
                                r#type: "password",
                                value: "{anthropic_val}",
                                placeholder: "sk-ant-...",
                                oninput: move |ev| anthropic_key.set(ev.value()),
                            }
                        }
                        div { class: "env-hint", "auto-loaded from "
                            span { "ANTHROPIC_API_KEY" }
                            " in .env"
                        }
                    }

                    // OpenAI
                    div { class: "provider-card",
                        div { class: "provider-header",
                            span { class: "provider-name", "OpenAI" }
                            span { class: if has_openai { "provider-badge active" } else { "provider-badge inactive" },
                                if has_openai { "active" } else { "no key" }
                            }
                        }
                        div { class: "token-row",
                            input {
                                class: "token-input",
                                r#type: "password",
                                value: "{openai_val}",
                                placeholder: "sk-...",
                                oninput: move |ev| openai_key.set(ev.value()),
                            }
                        }
                        div { class: "env-hint", "auto-loaded from "
                            span { "OPENAI_API_KEY" }
                            " in .env"
                        }
                    }

                    // Venice.ai
                    div { class: "provider-card",
                        div { class: "provider-header",
                            span { class: "provider-name", "Venice.ai" }
                            span { class: if has_venice { "provider-badge active" } else { "provider-badge inactive" },
                                if has_venice { "active" } else { "no key" }
                            }
                        }
                        div { class: "token-row",
                            input {
                                class: "token-input",
                                r#type: "password",
                                value: "{venice_val}",
                                placeholder: "venice-...",
                                oninput: move |ev| venice_key.set(ev.value()),
                            }
                        }
                        div { class: "env-hint", "auto-loaded from "
                            span { "VENICE_API_KEY" }
                            " in .env"
                        }
                    }
                }

                div { class: "settings-section",
                    div { class: "settings-title", "Server" }
                    div { class: "provider-card",
                        div { class: "provider-header",
                            span { class: "provider-name", "trios-server" }
                            span { class: if is_connected { "provider-badge active" } else { "provider-badge inactive" },
                                if is_connected { "ws:9005 \u{2713}" } else { "offline" }
                            }
                        }
                        div { class: "env-hint",
                            span { "v{BUILD_VERSION}" }
                            " — ws://localhost:9005/ws"
                        }
                    }
                }
            }
        }
        div { id: "ver", "v{BUILD_VERSION}" }
    }
}
