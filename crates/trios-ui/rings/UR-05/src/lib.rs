//! UR-05 — Agent UI
//!
//! Agent list, agent cards, and agent status display.
//! Reads the `AgentsAtom` from UR-00.

use dioxus::prelude::*;
use trios_ui_ur00::{use_agents_atom, use_chat_atom, Agent, AgentStatus};
use trios_ui_ur01::{use_palette, radius, spacing, typography};
use trios_ui_ur02::{badge, BadgeVariant};

// ─── AgentList ───────────────────────────────────────────────

/// Full agent list panel.
pub fn AgentList() -> Element {
    let palette = use_palette();
    let agents = use_agents_atom();

    rsx! {
        div {
            style: "
                display: flex;
                flex-direction: column;
                gap: {spacing::SM};
                padding: {spacing::MD};
                background: {palette.background};
                height: 100%;
                overflow-y: auto;
            ",
            div {
                style: "
                    font-family: {typography::FONT_FAMILY};
                    font-size: {typography::SIZE_LG};
                    font-weight: {typography::WEIGHT_BOLD};
                    color: {palette.text};
                    margin-bottom: {spacing::SM};
                ",
                "Agents ({agents.read().len()})"
            },
            for agent in agents.read().iter() {
                AgentCard { key: "{agent.id}", agent: agent.clone() }
            }
            if agents.read().is_empty() {
                div {
                    style: "
                        color: {palette.text_muted};
                        font-family: {typography::FONT_FAMILY};
                        font-size: {typography::SIZE_MD};
                        text-align: center;
                        padding: {spacing::XXL};
                    ",
                    "No agents connected"
                }
            }
        }
    }
}

// ─── AgentCard ───────────────────────────────────────────────

/// Props for a single agent card.
#[derive(Props, Clone, PartialEq)]
pub struct AgentCardProps {
    /// The agent to display.
    pub agent: Agent,
}

/// Render a single agent card with status badge.

pub fn AgentCard(props: AgentCardProps) -> Element {
    let palette = use_palette();
    let agent = &props.agent;
    let (badge_variant, badge_text) = match &agent.status {
        AgentStatus::Idle => (BadgeVariant::Success, "idle".to_string()),
        AgentStatus::Busy => (BadgeVariant::Warning, "busy".to_string()),
        AgentStatus::Error(e) => (BadgeVariant::Error, format!("error: {e}")),
        AgentStatus::Offline => (BadgeVariant::Default, "offline".to_string()),
    };
    let mut chat = use_chat_atom();
    let agent_id = agent.id.clone();

    rsx! {
        div {
            style: "
                display: flex;
                align-items: center;
                justify-content: space-between;
                background: {palette.surface};
                border: 1px solid {palette.border};
                border-radius: {radius::LG};
                padding: {spacing::MD};
                cursor: pointer;
                transition: border-color 0.15s;
            ",
            onclick: move |_| {
                chat.write().active_agent_id = Some(agent_id.clone());
            },
            // Left: agent info
            div {
                style: "display: flex; flex-direction: column; gap: 2px;",
                div {
                    style: "
                        font-family: {typography::FONT_FAMILY};
                        font-size: {typography::SIZE_MD};
                        font-weight: {typography::WEIGHT_MEDIUM};
                        color: {palette.text};
                    ",
                    "{agent.name}"
                }
                div {
                    style: "
                        font-family: {typography::FONT_FAMILY};
                        font-size: {typography::SIZE_SM};
                        color: {palette.text_muted};
                    ",
                    "{agent.description}"
                }
            }
            // Right: status badge
            Badge {
                variant: badge_variant,
                {badge_text}
            }
        }
    }
}
