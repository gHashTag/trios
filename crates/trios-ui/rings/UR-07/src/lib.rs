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
#[component]
pub fn SettingsPanel() -> Element {
    let palette = use_palette();
    let settings = use_settings_atom();
    let theme_label = match settings.read().theme {
        Theme::Dark => "🌙 Dark",
        Theme::Light => "☀️ Light",
    };
    let theme_label_owned = theme_label.to_string();

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
            SettingsSection {
                title: "Appearance".to_string(),
                div {
                    style: "display: flex; align-items: center; justify-content: space-between;",
                    span {
                        style: "
                            font-family: {typography::FONT_FAMILY};
                            font-size: {typography::SIZE_MD};
                            color: {palette.text};
                        ",
                        "Theme: {theme_label_owned}"
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| { toggle_theme(); },
                        "Toggle Theme"
                    }
                }
            }
            // API Key section
            ApiKeySection {}
            // MCP Server URL section (local + public endpoint switcher)
            McpUrlSection {}
        }
    }
}

// ─── SettingsSection ─────────────────────────────────────────

#[derive(Props, Clone, PartialEq)]
pub struct SettingsSectionProps {
    pub title: String,
    pub children: Element,
}

#[component]
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

#[component]
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
#[component]
fn McpUrlSection() -> Element {
    let mut settings = use_settings_atom();
    let palette = use_palette();
    let mcp_url = settings.read().mcp_url.clone();

    let is_local = mcp_url == URL_LOCAL || mcp_url.starts_with("http://localhost");
    let is_public = mcp_url.contains("tail01804b.ts.net");

    let local_border = if is_local { palette.primary } else { palette.border };
    let local_bg = if is_local { palette.primary } else { palette.surface };
    let local_color = if is_local { palette.background } else { palette.text };
    let public_border = if is_public { palette.primary } else { palette.border };
    let public_bg = if is_public { palette.primary } else { palette.surface };
    let public_color = if is_public { palette.background } else { palette.text };

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
                        border: 1px solid {local_border};
                        background: {local_bg};
                        color: {local_color};
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
                        border: 1px solid {public_border};
                        background: {public_bg};
                        color: {public_color};
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
