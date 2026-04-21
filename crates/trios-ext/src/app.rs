//! Dioxus App for Trios Chrome Extension

use dioxus::prelude::*;

// MCP WebSocket Client
#[derive(Clone, Debug, PartialEq)]
pub struct McpState {
    pub connected: bool,
    pub agents: Vec<String>,
    pub tools: Vec<String>,
}

impl Default for McpState {
    fn default() -> Self {
        Self {
            connected: false,
            agents: vec!["Loading...".to_string()],
            tools: vec!["Loading...".to_string()],
        }
    }
}

pub fn App() -> Element {
    let mcp = use_signal(McpState::default);
    let messages = use_signal(|| vec![
        ("agent", "Welcome to Trios! Connected to trios-server".to_string()),
    ]);
    let input = use_signal(|| String::new());

    rsx! {
        div { class: "trios-container",
            Header {}
            MainPanel { mcp: mcp.read().clone(), messages: messages.clone(), input: input.clone() }
        }
    }
}

fn Header() -> Element {
    rsx! {
        div { class: "trinity-logo",
            div { class: "trinity-logo-symbol", "Φ" }
            div { class: "trinity-logo-text", "Trios" }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct MainPanelProps {
    mcp: McpState,
    messages: Signal<Vec<(String, String)>>,
    input: Signal<String>,
}

fn MainPanel(props: MainPanelProps) -> Element {
    rsx! {
        div { class: "chat-container",
            div { class: "messages",
                for (role, text) in props.messages.read().iter() {
                    div { class: "message {role}",
                        "{text}"
                    }
                }
            }
            div { class: "input-area",
                input {
                    r#type: "text",
                    placeholder: "Type a message...",
                    value: "{props.input}",
                    oninput: move |e: Event<FormData>| {
                        props.input.set(e.value());
                    },
                }
                button { onclick: move |_| {
                    let msg = props.input.read().clone();
                    if !msg.is_empty() {
                        props.messages.with_mut(|m| m.push(("user".to_string(), msg.clone())));
                        let response = format!("Echo: {}", msg);
                        props.messages.with_mut(|m| m.push(("agent".to_string(), response)));
                        props.input.set(String::new());
                    }
                }, "Send" }
            }
        }
    }
}
