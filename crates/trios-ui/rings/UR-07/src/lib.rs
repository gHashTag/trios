//! UR-07 — Settings
//!
//! Settings panel: theme toggle, API key configuration,
//! MCP server URL, and sidebar preferences.
//! Reads/writes the `SettingsAtom` from UR-00.

use dioxus::prelude::*;
use trios_ui_ur00::{use_settings_atom, Theme};
use trios_ui_ur01::{toggle_theme, use_palette, radius, spacing, typography};
use trios_ui_ur02::{Button, ButtonVariant, Input};

// ─── SettingsPanel ───────────────────────────────────────────

/// Full settings panel.
pub fn SettingsPanel() -> Element {
    let palette = use_palette();
    let settings = use_settings_atom();
    let theme_label = match settings.read().theme {
        Theme::Dark => "🌙 Dark",
        Theme::Light => "☀️ Light",
    };

    rsx! {
        div {
            style: "
                display: flex;
                flex-direction: column;
                gap: {spacing::LG};
                padding: {spacing::MD};
                background: {palette.background};
                height: 100%;
                overflow-y: auto;
            ",
            // Header
            div {
                style: "
                    font-family: {typography::FONT_FAMILY};
                    font-size: {typography::SIZE_LG};
                    font-weight: {typography::WEIGHT_BOLD};
                    color: {palette.text};
                ",
                "⚙ Settings"
            }
            // Theme section
            { SettingsSection {
                title: "Appearance".to_string(),
                children: rsx! {
                    div {
                        style: "display: flex; align-items: center; justify-content: space-between;",
                        span {
                            style: "
                                font-family: {typography::FONT_FAMILY};
                                font-size: {typography::SIZE_MD};
                                color: {palette.text};
                            ",
                            "Theme: {theme_label}"
                        }
                        Button {
                            children: "Toggle Theme".to_string(),
                            variant: ButtonVariant::Secondary,
                            onclick: move |_| { toggle_theme(); },
                        }
                    }
                },
            } }
            // API Key section
            { ApiKeySection {} }
            // MCP Server URL section (local + public endpoint switcher)
            { McpUrlSection {} }
        }
    }
}

// ─── SettingsSection ─────────────────────────────────────────

#[derive(Props, Clone, PartialEq)]
pub struct SettingsSectionProps {
    pub title: String,
    pub children: Element,
}

pub fn SettingsSection(props: SettingsSectionProps) -> Element {
    let palette = use_palette();

    rsx! {
        div {
            style: "
                display: flex;
                flex-direction: column;
                gap: {spacing::SM};
                background: {palette.surface};
                border: 1px solid {palette.border};
                border-radius: {radius::LG};
                padding: {spacing::MD};
            ",
            div {
                style: "
                    font-family: {typography::FONT_FAMILY};
                    font-size: {typography::SIZE_SM};
                    font-weight: {typography::WEIGHT_BOLD};
                    color: {palette.text_muted};
                    text-transform: uppercase;
                    letter-spacing: 0.5px;
                ",
                {props.title.clone()}
            }
            {props.children}
        }
    }
}

// ─── ApiKeySection ───────────────────────────────────────────

fn ApiKeySection() -> Element {
    let mut settings = use_settings_atom();
    let api_key = settings.read().api_key.clone();
    let masked = if api_key.is_empty() {
        String::new()
    } else {
        format!("{}••••••••", &api_key[..api_key.len().min(4)])
    };

    rsx! {
        SettingsSection {
            title: "API Key".to_string(),
            Input {
                placeholder: "Enter z.ai API key...".to_string(),
                value: masked,
                label: "z.ai Direct Chat Key".to_string(),
                mono: true,
                oninput: move |val: String| {
                    settings.write().api_key = val;
                },
            }
        }
    }
}

// ─── McpUrlSection ───────────────────────────────────────────

const URL_LOCAL: &str = "http://localhost:9005";
const URL_PUBLIC: &str = "https://playras-macbook-pro-1.tail01804b.ts.net";

/// MCP server URL section with Local / Public quick-select buttons.
fn McpUrlSection() -> Element {
    let mut settings = use_settings_atom();
    let palette = use_palette();
    let mcp_url = settings.read().mcp_url.clone();

    let is_local = mcp_url == URL_LOCAL || mcp_url.starts_with("http://localhost");
    let is_public = mcp_url.contains("tail01804b.ts.net");

    rsx! {
        SettingsSection {
            title: "MCP Server".to_string(),
            // Quick-select row
            div {
                style: "display: flex; gap: {spacing::SM}; margin-bottom: {spacing::XS};",
                // Local button
                button {
                    style: "
                        flex: 1;
                        padding: 6px 0;
                        border-radius: {radius::MD};
                        border: 1px solid {if is_local { palette.accent } else { palette.border }};
                        background: {if is_local { palette.accent } else { palette.surface }};
                        color: {if is_local { palette.background } else { palette.text }};
                        font-family: {typography::FONT_FAMILY};
                        font-size: {typography::SIZE_SM};
                        cursor: pointer;
                    ",
                    onclick: move |_| {
                        settings.write().mcp_url = URL_LOCAL.to_string();
                    },
                    "🖥 Local"
                }
                // Public (Funnel) button
                button {
                    style: "
                        flex: 1;
                        padding: 6px 0;
                        border-radius: {radius::MD};
                        border: 1px solid {if is_public { palette.accent } else { palette.border }};
                        background: {if is_public { palette.accent } else { palette.surface }};
                        color: {if is_public { palette.background } else { palette.text }};
                        font-family: {typography::FONT_FAMILY};
                        font-size: {typography::SIZE_SM};
                        cursor: pointer;
                    ",
                    onclick: move |_| {
                        settings.write().mcp_url = URL_PUBLIC.to_string();
                    },
                    "🌐 Public"
                }
            }
            // Manual URL input
            Input {
                placeholder: "http://localhost:9005".to_string(),
                value: mcp_url,
                label: "Server URL".to_string(),
                mono: true,
                oninput: move |val: String| {
                    settings.write().mcp_url = val;
                },
            }
        }
    }
}
