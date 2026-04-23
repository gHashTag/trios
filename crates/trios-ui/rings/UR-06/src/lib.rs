//! UR-06 — MCP Panel
//!
//! MCP tools panel: lists available tools, shows connection status,
//! and provides a tool execution interface.
//! Reads the `McpAtom` from UR-00.

use dioxus::prelude::*;
use trios_ui_ur00::{use_mcp_atom, McpTool};
use trios_ui_ur01::{use_palette, radius, spacing, typography};
use trios_ui_ur02::{Badge, BadgeVariant, Button, ButtonVariant};

// ─── McpPanel ────────────────────────────────────────────────

/// Full MCP tools panel.
pub fn McpPanel() -> Element {
    let palette = use_palette();
    let mcp = use_mcp_atom();
    let connected = mcp.read().connected;
    let tools_count = mcp.read().tools.len();

    rsx! {
        div {
            style: "
                display: flex;
                flex-direction: column;
                gap: {spacing::MD};
                padding: {spacing::MD};
                background: {palette.background};
                height: 100%;
                overflow-y: auto;
            ",
            // Header
            div {
                style: "
                    display: flex;
                    align-items: center;
                    justify-content: space-between;
                ",
                div {
                    style: "
                        font-family: {typography::FONT_FAMILY};
                        font-size: {typography::SIZE_LG};
                        font-weight: {typography::WEIGHT_BOLD};
                        color: {palette.text};
                    ",
                    "MCP Tools ({tools_count})"
                }
                Badge {
                    children: if connected { "connected".to_string() } else { "disconnected".to_string() },
                    variant: if connected { BadgeVariant::Success } else { BadgeVariant::Error },
                }
            }
            // Server URL
            div {
                style: "
                    font-family: {typography::FONT_MONO};
                    font-size: {typography::SIZE_SM};
                    color: {palette.text_muted};
                ",
                "{mcp.read().server_url}"
            }
            // Tool list
            for tool in mcp.read().tools.iter() {
                { McpToolCard { key: "{tool.name}", tool: tool.clone() } }
            }
            if !connected {
                div {
                    style: "
                        color: {palette.text_muted};
                        font-family: {typography::FONT_FAMILY};
                        font-size: {typography::SIZE_MD};
                        text-align: center;
                        padding: {spacing::XXL};
                    ",
                    "MCP server offline — configure in ⚙ Settings"
                }
            }
        }
    }
}

// ─── McpToolCard ─────────────────────────────────────────────

/// Props for a single MCP tool card.
#[derive(Props, Clone, PartialEq)]
pub struct McpToolCardProps {
    /// The MCP tool to display.
    pub tool: McpTool,
}

/// Render a single MCP tool with name, description, and execute button.
pub fn McpToolCard(props: McpToolCardProps) -> Element {
    let palette = use_palette();
    let tool = &props.tool;

    rsx! {
        div {
            style: "
                background: {palette.surface};
                border: 1px solid {palette.border};
                border-radius: {radius::LG};
                padding: {spacing::MD};
                display: flex;
                flex-direction: column;
                gap: {spacing::XS};
            ",
            // Tool name
            div {
                style: "
                    font-family: {typography::FONT_MONO};
                    font-size: {typography::SIZE_MD};
                    font-weight: {typography::WEIGHT_MEDIUM};
                    color: {palette.primary};
                ",
                "{tool.name}"
            }
            // Description
            div {
                style: "
                    font-family: {typography::FONT_FAMILY};
                    font-size: {typography::SIZE_SM};
                    color: {palette.text_muted};
                ",
                "{tool.description}"
            }
            // Parameters (if any)
            if let Some(params) = &tool.parameters {
                div {
                    style: "
                        font-family: {typography::FONT_MONO};
                        font-size: {typography::SIZE_XS};
                        color: {palette.text_muted};
                        background: {palette.background};
                        border-radius: {radius::SM};
                        padding: {spacing::XS} {spacing::SM};
                        overflow-x: auto;
                    ",
                    "{params}"
                }
            }
        }
    }
}
