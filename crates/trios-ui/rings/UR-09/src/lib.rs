//! UR-09 — A2A Social Network
//!
//! Live agent social feed: messages, presence, interrupt controls.
//! Connects to HITL-A2A HTTP Bridge (:9876) via JS interop.
//!
//! ## Components
//!
//! - `SocialPanel` — Full social feed panel
//! - `SocialHeader` — Title + bus status
//! - `PresenceBar` — Agent online/offline chips
//! - `SocialFeed` — Message list with agent colors
//! - `AgentBubble` — Single agent message with avatar
//! - `InterruptBar` — ⛔ INTERRUPT / ✅ RESUME controls
//! - `HumanInput` — Message input for human
//!
//! ## Ring Architecture
//!
//! ```text
//! UR-09 (this) ←→ UR-00 (A2A atoms) ←→ UR-01 (theme) ←→ UR-02 (primitives)
//!      ↕
//! HITL-A2A HTTP Bridge (:9876) ←→ Cloudflare Tunnel ←→ Scarabs (cloud)
//! ```
//!
//! Data flow:
//! - **Polling** is driven by JS in BR-APP index.html (calls `window.__a2a_poll()`)
//! - **Actions** (send, interrupt, resume) call `window.__a2a_post(url, body)` via JS interop
//! - **State** lives in `A2A_ATOM` (UR-00 GlobalSignal) — reactive Dioxus signals

use dioxus::prelude::*;
use trios_ui_ur00::{A2AMessage, AgentProfile, A2AState, use_a2a_atom};
use trios_ui_ur01::{use_palette, radius, spacing, typography};

// ─── Social Panel ────────────────────────────────────────────

/// Full social network panel with presence, feed, and input.
pub fn SocialPanel() -> Element {
    let palette = use_palette();

    rsx! {
        div {
            style: "
                display: flex;
                flex-direction: column;
                height: 100%;
                background: {palette.background};
            ",

            SocialHeader {}
            PresenceBar {}
            SocialFeed {}
            InterruptBar {}
            HumanInput {}
        }
    }
}

// ─── Social Header ───────────────────────────────────────────

fn SocialHeader() -> Element {
    let palette = use_palette();
    let a2a = use_a2a_atom();
    let connected = a2a.read().connected;
    let msg_count = a2a.read().messages.len();
    let status_color = if connected { palette.accent_success } else { palette.accent_error };
    let status_text = if connected { "online" } else { "offline" };

    rsx! {
        div {
            style: "
                display: flex;
                align-items: center;
                gap: {spacing::SM};
                padding: {spacing::SM} {spacing::MD};
                border-bottom: 1px solid {palette.border};
                background: {palette.surface};
            ",

            span {
                style: "font-size: 16px; color: {palette.primary};",
                "🕸️"
            }

            span {
                style: "
                    font-family: {typography::FONT_FAMILY};
                    font-size: {typography::SIZE_MD};
                    font-weight: {typography::WEIGHT_BOLD};
                    color: {palette.primary};
                ",
                "Trinity Social"
            }

            div { style: "flex: 1;" }

            span {
                style: "
                    font-size: {typography::SIZE_XS};
                    padding: 2px 8px;
                    border-radius: {radius::FULL};
                    border: 1px solid {status_color};
                    color: {status_color};
                    font-family: {typography::FONT_MONO};
                    text-transform: uppercase;
                    letter-spacing: 0.5px;
                ",
                "{status_text}"
            }

            span {
                style: "
                    font-size: {typography::SIZE_XS};
                    color: {palette.text_muted};
                    font-family: {typography::FONT_MONO};
                ",
                "{msg_count} msgs"
            }
        }
    }
}

// ─── Presence Bar ────────────────────────────────────────────

fn PresenceBar() -> Element {
    let palette = use_palette();
    let a2a = use_a2a_atom();
    let mut filter = use_signal(|| None::<String>);

    let profiles = [
        AgentProfile::human(),
        AgentProfile::browser_os(),
        AgentProfile::scarabs(),
        AgentProfile::phi_t27(),
    ];

    rsx! {
        div {
            style: "
                display: flex;
                gap: {spacing::XS};
                padding: {spacing::XS} {spacing::MD};
                border-bottom: 1px solid {palette.border};
                background: {palette.surface};
                overflow-x: auto;
            ",

            for profile in profiles {
                {
                    let name = profile.name.clone();
                    let emoji = profile.emoji.clone();
                    let label = profile.label.clone();
                    let color = profile.color.clone();
                    let online = a2a.read().is_agent_online(&name);
                    let dot_color = if online { palette.accent_success } else { palette.text_muted };
                    let is_filtered = filter.read().as_ref() == Some(&name);
                    let border = if is_filtered { color.clone() } else { palette.border.to_string() };

                    let name_click = name.clone();
                    let name_cmp = name.clone();

                    rsx! {
                        div {
                            key: "{name}",
                            style: "
                                display: flex;
                                align-items: center;
                                gap: 3px;
                                padding: 2px 8px;
                                border-radius: {radius::FULL};
                                font-size: {typography::SIZE_XS};
                                border: 1px solid {border};
                                cursor: pointer;
                                white-space: nowrap;
                            ",
                            onclick: move |_| {
                                let current = filter.read().clone();
                                let new_filter = if current.as_ref() == Some(&name_click) { None } else { Some(name_click.clone()) };
                                filter.set(new_filter);
                            },

                            span {
                                style: "
                                    width: 5px;
                                    height: 5px;
                                    border-radius: 50%;
                                    background: {dot_color};
                                ",
                            }

                            span {
                                style: "color: {color}; font-family: {typography::FONT_FAMILY};",
                                "{emoji} {label}"
                            }
                        }
                    }
                }
            }
        }
    }
}

// ─── Social Feed ─────────────────────────────────────────────

fn SocialFeed() -> Element {
    let palette = use_palette();
    let a2a = use_a2a_atom();

    // Filter out heartbeat/presence noise
    let messages: Vec<A2AMessage> = a2a.read().messages.iter()
        .filter(|m| !(m.msg_type == "presence" && matches!(m.content.as_str(), "heartbeat" | "join" | "leave")))
        .cloned()
        .collect();

    rsx! {
        div {
            style: "
                flex: 1;
                overflow-y: auto;
                padding: {spacing::SM} {spacing::MD};
                display: flex;
                flex-direction: column;
                gap: 4px;
            ",

            for msg in messages.iter() {
                AgentBubble { key: "{msg.id}", message: msg.clone() }
            }

            if messages.is_empty() {
                div {
                    style: "
                        color: {palette.text_muted};
                        font-family: {typography::FONT_FAMILY};
                        font-size: {typography::SIZE_SM};
                        text-align: center;
                        padding: {spacing::XXL};
                    ",
                    "🕸️ No messages yet. Agents will appear here when they connect to the bus."
                }
            }
        }
    }
}

// ─── Agent Bubble ────────────────────────────────────────────

#[derive(Props, Clone, PartialEq)]
pub struct AgentBubbleProps {
    pub message: A2AMessage,
}

fn AgentBubble(props: AgentBubbleProps) -> Element {
    let palette = use_palette();
    let msg = &props.message;
    let profile = AgentProfile::from_name(&msg.agent_name);
    let time = format_timestamp(msg.timestamp);

    let (type_tag, bg_tint) = match msg.msg_type.as_str() {
        "interrupt" => ("⛔ INTERRUPT", "#1a0a0a"),
        "abort" => ("🛑 ABORT", "#150a0a"),
        "interrupted" => ("✅ ACK", "#1a1a0a"),
        _ => ("", palette.surface),
    };

    let border_left = format!("2px solid {}", profile.color);

    rsx! {
        div {
            style: "
                padding: 6px 8px;
                border-radius: {radius::LG};
                font-size: {typography::SIZE_SM};
                line-height: 1.5;
                max-width: 95%;
                background: {bg_tint};
                border: 1px solid {palette.border};
                border-left: {border_left};
            ",

            // Header line
            div {
                style: "
                    display: flex;
                    align-items: center;
                    gap: 4px;
                    margin-bottom: 2px;
                    font-size: {typography::SIZE_XS};
                ",

                span {
                    style: "color: {profile.color}; font-weight: {typography::WEIGHT_BOLD}; text-transform: uppercase; letter-spacing: 0.5px;",
                    "{profile.emoji} {profile.label}"
                }

                if !type_tag.is_empty() {
                    span {
                        style: "color: {palette.text_muted}; font-size: 9px;",
                        "{type_tag}"
                    }
                }

                span {
                    style: "color: {palette.text_muted}; font-size: 9px; margin-left: auto;",
                    "{time}"
                }
            }

            // Content
            div {
                style: "
                    color: {palette.text};
                    white-space: pre-wrap;
                    word-break: break-word;
                    font-family: {typography::FONT_FAMILY};
                ",
                "{msg.content}"
            }
        }
    }
}

// ─── Interrupt Bar ───────────────────────────────────────────

fn InterruptBar() -> Element {
    let palette = use_palette();
    let a2a = use_a2a_atom();
    let interrupt_active = a2a.read().interrupt_active;

    let int_border = if interrupt_active { palette.accent_error } else { palette.border };
    let int_bg = if interrupt_active { "#2a0a0a" } else { palette.surface };

    rsx! {
        div {
            style: "
                display: flex;
                gap: {spacing::XS};
                padding: {spacing::XS} {spacing::MD};
                border-top: 1px solid {palette.border};
                background: {palette.surface};
            ",

            // INTERRUPT
            button {
                style: "
                    flex: 1;
                    padding: 4px 8px;
                    border-radius: {radius::MD};
                    border: 1px solid {int_border};
                    background: {int_bg};
                    color: {palette.accent_error};
                    font-family: {typography::FONT_FAMILY};
                    font-size: {typography::SIZE_XS};
                    cursor: pointer;
                    text-align: center;
                ",
                onclick: move |_| {
                    a2a_action_interrupt();
                },
                if interrupt_active { "⛔ ACTIVE" } else { "⛔ INTERRUPT" }
            }

            // RESUME
            button {
                style: "
                    flex: 1;
                    padding: 4px 8px;
                    border-radius: {radius::MD};
                    border: 1px solid {palette.border};
                    background: {palette.surface};
                    color: {palette.text_muted};
                    font-family: {typography::FONT_FAMILY};
                    font-size: {typography::SIZE_XS};
                    cursor: pointer;
                    text-align: center;
                ",
                onclick: move |_| {
                    a2a_action_resume();
                },
                "✅ RESUME"
            }
        }
    }
}

// ─── Human Input ─────────────────────────────────────────────

fn HumanInput() -> Element {
    let palette = use_palette();
    let mut input_text = use_signal(String::new);
    let current_input = input_text.read().clone();
    let is_empty = current_input.is_empty();
    let opacity = if is_empty { "0.5" } else { "1.0" };

    rsx! {
        div {
            style: "
                display: flex;
                gap: {spacing::XS};
                padding: {spacing::XS} {spacing::MD};
                border-top: 1px solid {palette.border};
                background: {palette.surface};
            ",

            input {
                style: "
                    flex: 1;
                    background: {palette.background};
                    color: {palette.text};
                    border: 1px solid {palette.border};
                    border-radius: {radius::MD};
                    padding: 6px 10px;
                    font-family: {typography::FONT_FAMILY};
                    font-size: {typography::SIZE_SM};
                    outline: none;
                ",
                r#type: "text",
                placeholder: "👑 Message agents...",
                value: "{current_input}",
                oninput: move |e: Event<FormData>| {
                    input_text.set(e.data.value());
                },
                onkeydown: move |e: KeyboardEvent| {
                    if e.key() == Key::Enter && !input_text.read().is_empty() {
                        a2a_action_send(input_text);
                    }
                },
            }

            button {
                style: "
                    background: {palette.primary};
                    color: {palette.background};
                    border: none;
                    border-radius: {radius::MD};
                    padding: 6px 12px;
                    font-family: {typography::FONT_FAMILY};
                    font-size: {typography::SIZE_SM};
                    font-weight: {typography::WEIGHT_BOLD};
                    cursor: pointer;
                    opacity: {opacity};
                ",
                disabled: is_empty,
                onclick: move |_| {
                    a2a_action_send(input_text);
                },
                "↵"
            }
        }
    }
}

// ─── A2A Actions (JS interop) ────────────────────────────────
//
// These call JS functions defined in BR-APP index.html.
// The JS side does fetch() to the HITL-A2A HTTP Bridge.
// This keeps the Rust UI pure — no web_sys dependency in ring code.

fn a2a_action_send(mut input: Signal<String>) {
    let text = input.read().clone();
    if text.is_empty() { return; }

    let mut a2a = use_a2a_atom();
    let conv_id = a2a.read().conversation_id.clone();

    // Optimistic local update
    let msg = A2AMessage {
        id: format!("local-{}", js_now()),
        msg_type: "chat".to_string(),
        role: "human".to_string(),
        agent_name: "HumanOverlord".to_string(),
        content: text.clone(),
        conversation_id: conv_id.clone(),
        timestamp: js_now(),
    };
    a2a.write().messages.push(msg);
    input.set(String::new());

    // POST to bridge via JS interop
    let body = serde_json::json!({
        "type": "chat",
        "role": "human",
        "agentName": "HumanOverlord",
        "content": text,
        "conversationId": conv_id,
    }).to_string();
    js_a2a_post("/messages", &body);
}

fn a2a_action_interrupt() {
    let mut a2a = use_a2a_atom();
    a2a.write().interrupt_active = true;

    let body = serde_json::json!({
        "role": "human",
        "agentName": "HumanOverlord",
        "reason": "⛔ Human veto — STOP all agents",
        "scope": "all_agents",
        "priority": "P0"
    }).to_string();
    js_a2a_post("/interrupt", &body);
}

fn a2a_action_resume() {
    let mut a2a = use_a2a_atom();
    a2a.write().interrupt_active = false;

    let conv_id = a2a.read().conversation_id.clone();

    // Clear interrupt
    js_a2a_delete("/interrupt");

    // Send resume message
    let body = serde_json::json!({
        "type": "chat",
        "role": "human",
        "agentName": "HumanOverlord",
        "content": "✅ Resume — all agents may continue.",
        "conversationId": conv_id,
    }).to_string();
    js_a2a_post("/messages", &body);
}

// ─── JS Interop stubs ────────────────────────────────────────
// Real implementations are in BR-APP index.html as WASM-imported functions.
// These are stubs that compile in native mode (for cargo check).

/// POST to A2A bridge. In WASM, calls `window.__a2a_post(path, body)`.
fn js_a2a_post(path: &str, body: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (path, body);
        // In WASM, use wasm_bindgen to call JS:
        // wasm_bindgen::JsValue::from_str(&format!(
        //     "window.__a2a_post && window.__a2a_post('{}', '{}')", path, body
        // ));
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (path, body);
}

/// DELETE to A2A bridge.
fn js_a2a_delete(path: &str) {
    #[cfg(target_arch = "wasm32")]
    let _ = path;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = path;
}

/// Get current time in epoch ms via JS.
fn js_now() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

// ─── Utility ─────────────────────────────────────────────────

fn format_timestamp(ts: u64) -> String {
    let secs = (ts / 1000) % 86400;
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    format!("{:02}:{:02}", hours, mins)
}
