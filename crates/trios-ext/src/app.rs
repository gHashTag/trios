//! Dioxus App for Trios Chrome Extension

use dioxus::prelude::*;
use std::rc::Rc;

// MCP WebSocket Client
#[derive(Clone, Debug)]
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

    use_effect(move || {
        // In real app, this would connect to ws://localhost:9005/mcp
        // For now, simulate connection after 1 second
        spawn(async move {
            gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await;
            mcp.set(McpState {
                connected: true,
                agents: vec!["ALFA".to_string(), "BRAVO".to_string(), "CHARLIE".to_string()],
                tools: vec!["git_status".to_string(), "git_commit".to_string(), "gb_list_branches".to_string()],
            });
        });
    });

    rsx! {
        style { {include_str!("../src/style.css")} }
        div { class: "trios-container",
            Header {}
            MainPanel { mcp: mcp.read().clone() }
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

#[derive(Props, Clone)]
struct MainPanelProps {
    mcp: McpState,
}

fn MainPanel(props: MainPanelProps) -> Element {
    let messages = use_signal(|| vec![
        ("agent", "Welcome to Trios! Connected to trios-server".to_string()),
    ]);
    let input = use_signal(|| String::new());

    let send_message = move |_: Event<FormData>| {
        let msg = input.read().clone();
        if msg.is_empty() {
            return;
        }

        // Add user message
        messages.with_mut(|m| m.push(("user", msg.clone())));

        // Simulate agent response
        let response = format!("Echo: {}", msg);
        messages.with_mut(|m| m.push(("agent", response)));

        // Clear input
        input.set(String::new());
    };

    rsx! {
        div { class: "chat-container",
            div { class: "messages",
                for (role, text) in messages.read().iter() {
                    div { class: "message {role}",
                        "{text}"
                    }
                }
            }
            div { class: "input-area",
                input {
                    r#type: "text",
                    placeholder: "Type a message...",
                    value: "{input}",
                    oninput: move |e: Event<FormData>| {
                        input.set(e.value());
                    },
                    onkeypress: move |e: Event<KeyboardData>| {
                        if e.key() == "Enter" {
                            send_message(e);
                        }
                    }
                }
                button { onclick: send_message, "Send" }
            }
        }
    }
}
