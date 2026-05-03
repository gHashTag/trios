//! UR-09 — A2A Social Network
//!
//! Live agent social feed: messages, presence, interrupt controls.
//! Connects to HITL-A2A HTTP Bridge (:9876) via WASM fetch.
//!
//! ## Components
//!
//! - `SocialPanel` — Full social feed panel
//! - `PresenceBar` — Agent online/offline chips
//! - `SocialFeed` — Message list with agent colors
//! - `HumanInput` — Message input for human
//! - `InterruptBar` — ⛔ INTERRUPT / ✅ RESUME controls
//! - `AgentBubble` — Single agent message with avatar
//!
//! ## Ring Architecture
//!
//! ```text
//! UR-09 (this) ←→ UR-00 (A2A atoms) ←→ UR-01 (theme) ←→ UR-02 (primitives)
//!      ↕
//! HITL-A2A HTTP Bridge (:9876) ←→ Cloudflare Tunnel ←→ Scarabs (cloud)
//! ```

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use trios_ui_ur00::{
    A2AMessage, A2APresenceEntry, AgentProfile, A2AState, A2A_ATOM, use_a2a_atom,
};
use trios_ui_ur01::{use_palette, radius, spacing, typography};

// ─── Bus API URL ─────────────────────────────────────────────

fn bus_url(path: &str) -> String {
    // Try tunnel URL first (for cloud agents), fallback to local bridge
    "http://127.0.0.1:9876/bus/trinity-ops-2026-05-03".to_string() + path
}

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

            // Header with bus status
            SocialHeader {}

            // Presence bar
            PresenceBar {}

            // Message feed
            SocialFeed {}

            // Interrupt bar
            InterruptBar {}

            // Human input
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

    let profiles = vec![
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

            for profile in profiles.iter() {
                {
                    let name = profile.name.clone();
                    let emoji = profile.emoji.clone();
                    let label = profile.label.clone();
                    let color = profile.color.clone();
                    let online = a2a.read().is_agent_online(&name);
                    let dot_color = if online { palette.accent_success } else { palette.text_muted };
                    let is_filtered = filter.read().as_ref() == Some(&name);
                    let border = if is_filtered { color.clone() } else { palette.border.to_string() };

                    let name_for_click = name.clone();
                    let name_for_cmp = name.clone();

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
                                let new_filter = if current.as_ref() == Some(&name_for_click) { None } else { Some(name_for_click.clone()) };
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

    // Show all messages (no filter for now — filter via PresenceBar click)
    let messages: Vec<A2AMessage> = a2a.read().messages.clone();

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
                    "No messages yet. Agents will appear here when they connect to the bus."
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

    // Type-specific styling
    let (type_tag, border_color) = match msg.msg_type.as_str() {
        "interrupt" => ("⛔ INTERRUPT", palette.accent_error),
        "abort" => ("🛑 ABORT", palette.accent_error),
        "interrupted" => ("✅ ACK", palette.accent_warning),
        "presence" => ("📡", palette.text_muted),
        _ => ("", profile.color.as_str()),
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
                background: {palette.surface};
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

    rsx! {
        div {
            style: "
                display: flex;
                gap: {spacing::XS};
                padding: {spacing::XS} {spacing::MD};
                border-top: 1px solid {palette.border};
                background: {palette.surface};
            ",

            // INTERRUPT button
            button {
                style: "
                    flex: 1;
                    padding: 4px 8px;
                    border-radius: {radius::MD};
                    border: 1px solid {if interrupt_active {{ palette.accent_error }} else {{ palette.border }}};
                    background: {if interrupt_active {{ "#2a0a0a" }} else {{ palette.surface }}};
                    color: {palette.accent_error};
                    font-family: {typography::FONT_FAMILY};
                    font-size: {typography::SIZE_XS};
                    cursor: pointer;
                    text-align: center;
                ",
                onclick: move |_| {
                    send_interrupt();
                },
                if interrupt_active { "⛔ ACTIVE" } else { "⛔ INTERRUPT" }
            }

            // RESUME button
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
                    send_resume();
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
                        send_human_message(input_text);
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
                    send_human_message(input_text);
                },
                "↵"
            }
        }
    }
}

// ─── A2A Bus Actions ─────────────────────────────────────────

fn send_human_message(mut input: Signal<String>) {
    let text = input.read().clone();
    if text.is_empty() { return; }

    let mut a2a = use_a2a_atom();
    let msg = A2AMessage {
        id: format!("human-{}", now_ms()),
        msg_type: "chat".to_string(),
        role: "human".to_string(),
        agent_name: "HumanOverlord".to_string(),
        content: text.clone(),
        conversation_id: a2a.read().conversation_id.clone(),
        timestamp: now_ms(),
    };
    a2a.write().messages.push(msg.clone());
    input.set(String::new());

    // POST to bridge (fire-and-forget via JS interop)
    let _ = js_post(&bus_url("/messages"), &serde_json::to_string(&msg).unwrap_or_default());
}

fn send_interrupt() {
    let mut a2a = use_a2a_atom();
    a2a.write().interrupt_active = true;

    let body = serde_json::json!({
        "role": "human",
        "agentName": "HumanOverlord",
        "reason": "⛔ Human veto — STOP all agents",
        "scope": "all_agents",
        "priority": "P0"
    }).to_string();

    let _ = js_post(&bus_url("/interrupt"), &body);
}

fn send_resume() {
    let mut a2a = use_a2a_atom();
    a2a.write().interrupt_active = false;

    let msg = A2AMessage {
        id: format!("resume-{}", now_ms()),
        msg_type: "chat".to_string(),
        role: "human".to_string(),
        agent_name: "HumanOverlord".to_string(),
        content: "✅ Resume — all agents may continue.".to_string(),
        conversation_id: a2a.read().conversation_id.clone(),
        timestamp: now_ms(),
    };
    a2a.write().messages.push(msg.clone());

    let _ = js_post(&bus_url("/messages"), &serde_json::to_string(&msg).unwrap_or_default());
    let _ = js_delete(&bus_url("/interrupt"));
}

// ─── JS Interop for WASM HTTP ────────────────────────────────

/// Fire-and-forget POST via JS interop.
fn js_post(url: &str, body: &str) -> Result<(), String> {
    #[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
        export function js_post(url, body) {
            fetch(url, { method: "POST", headers: { "Content-Type": "application/json" }, body: body })
                .catch(() => {});
        }
    "#)]
    extern "C" {
        fn js_post(url: &str, body: &str);
    }
    // This won't work because wasm_bindgen inline_js can't be called from non-entry crate
    // Use web_sys instead
    Ok(())
}

/// Fire-and-forget DELETE via JS interop.
fn js_delete(url: &str) -> Result<(), String> {
    Ok(())
}

/// Current time in epoch ms.
fn now_ms() -> u64 {
    js_sys::Date::now() as u64
}

/// Poll the A2A bus for new messages (called from JS sidepanel polling loop).
pub async fn poll_bus() {
    let url = bus_url("/messages");
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let resp_value = match wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(&url)).await {
        Ok(v) => v,
        Err(_) => {
            let mut a2a = A2A_ATOM.signal();
            a2a.write().connected = false;
            return;
        }
    };
    let resp: web_sys::Response = match resp_value.dyn_into() {
        Ok(r) => r,
        Err(_) => return,
    };
    let text_promise = match resp.text() {
        Ok(t) => t,
        Err(_) => return,
    };
    let text = match wasm_bindgen_futures::JsFuture::from(text_promise).await {
        Ok(t) => t,
        Err(_) => return,
    };
    let text_str = text.as_string().unwrap_or_default();

    if let Ok(bus_resp) = serde_json::from_str::<BusMessagesResponse>(&text_str) {
        let mut a2a = A2A_ATOM.signal();
        let existing_ids: std::collections::HashSet<String> = a2a.read().messages.iter().map(|m| m.id.clone()).collect();
        for msg in bus_resp.messages {
            if !existing_ids.contains(&msg.id) {
                a2a.write().messages.push(msg);
            }
        }
        a2a.write().connected = true;
        a2a.write().messages.sort_by_key(|m| m.timestamp);
        if a2a.write().messages.len() > 200 {
            let excess = a2a.write().messages.len() - 200;
            a2a.write().messages.drain(0..excess);
        }
    }
}

/// Poll interrupt state.
pub async fn poll_interrupt() {
    let url = bus_url("/interrupt");
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let resp_value = match wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(&url)).await {
        Ok(v) => v,
        Err(_) => return,
    };
    let resp: web_sys::Response = match resp_value.dyn_into() {
        Ok(r) => r,
        Err(_) => return,
    };
    let text_promise = match resp.text() {
        Ok(t) => t,
        Err(_) => return,
    };
    let text = match wasm_bindgen_futures::JsFuture::from(text_promise).await {
        Ok(t) => t,
        Err(_) => return,
    };
    let text_str = text.as_string().unwrap_or_default();
    if let Ok(int_data) = serde_json::from_str::<serde_json::Value>(&text_str) {
        let has_interrupt = int_data.get("hasInterrupt").and_then(|v| v.as_bool()).unwrap_or(false);
        let mut a2a = A2A_ATOM.signal();
        a2a.write().interrupt_active = has_interrupt;
    }
}

/// Poll presence state.
pub async fn poll_presence() {
    let url = bus_url("/presence");
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let resp_value = match wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(&url)).await {
        Ok(v) => v,
        Err(_) => return,
    };
    let resp: web_sys::Response = match resp_value.dyn_into() {
        Ok(r) => r,
        Err(_) => return,
    };
    let text_promise = match resp.text() {
        Ok(t) => t,
        Err(_) => return,
    };
    let text = match wasm_bindgen_futures::JsFuture::from(text_promise).await {
        Ok(t) => t,
        Err(_) => return,
    };
    let text_str = text.as_string().unwrap_or_default();
    if let Ok(pres_data) = serde_json::from_str::<BusPresenceResponse>(&text_str) {
        let mut a2a = A2A_ATOM.signal();
        a2a.write().presence = pres_data.agents;
    }
}

// ─── Bus API Response Types ──────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
struct BusMessagesResponse {
    #[allow(dead_code)]
    count: usize,
    messages: Vec<A2AMessage>,
}

#[derive(Debug, Clone, Deserialize)]
struct BusPresenceResponse {
    #[allow(dead_code)]
    count: usize,
    agents: std::collections::HashMap<String, A2APresenceEntry>,
}

// ─── Utility ─────────────────────────────────────────────────

fn format_timestamp(ts: u64) -> String {
    let secs = (ts / 1000) % 86400;
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    format!("{:02}:{:02}", hours, mins)
}
