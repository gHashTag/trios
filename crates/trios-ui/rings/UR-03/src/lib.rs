//! UR-03 — Layout (Sidebar, Tabs, Panel)
//!
//! Layout primitives: sidebar navigation, tabbed panels, and
//! resizable panel containers.

use dioxus::prelude::*;
use trios_ui_ur00::use_settings_atom;
use trios_ui_ur01::{use_palette, ColorPalette, radius, spacing, typography};

// ─── Sidebar ─────────────────────────────────────────────────

/// Sidebar navigation item.
#[derive(Debug, Clone, PartialEq)]
pub struct NavItem {
    /// Item label.
    pub label: String,
    /// Icon (emoji or text).
    pub icon: String,
    /// Active state.
    pub active: bool,
}

/// Sidebar component props.
#[derive(Props, Clone, PartialEq)]
pub struct SidebarProps {
    /// Navigation items.
    pub items: Vec<NavItem>,
    /// Active item changed handler.
    pub on_select: EventHandler<usize>,
}

/// Collapsible sidebar navigation.
pub fn Sidebar(props: SidebarProps) -> Element {
    let palette = use_palette();
    let settings = use_settings_atom();
    let collapsed = settings.read().sidebar_collapsed;
    let width = if collapsed { "48px" } else { "220px" };

    rsx! {
        nav {
            style: "
                width: {width};
                min-width: {width};
                background: {palette.surface};
                border-right: 1px solid {palette.border};
                display: flex;
                flex-direction: column;
                padding: {spacing::SM};
                transition: width 0.2s;
                overflow: hidden;
            ",
            for (idx, item) in props.items.iter().enumerate() {
                { render_nav_item(idx, item, &props, palette) }
            }
        }
    }
}

fn render_nav_item(idx: usize, item: &NavItem, props: &SidebarProps, palette: ColorPalette) -> Element {
    let bg = if item.active { palette.primary } else { "transparent" };
    let color = if item.active { palette.background } else { palette.text };
    let on_select = props.on_select.clone();

    rsx! {
        button {
            key: "{idx}",
            style: "
                display: flex;
                align-items: center;
                gap: {spacing::SM};
                background: {bg};
                color: {color};
                border: none;
                border-radius: {radius::MD};
                padding: {spacing::SM} {spacing::MD};
                font-family: {typography::FONT_FAMILY};
                font-size: {typography::SIZE_MD};
                cursor: pointer;
                text-align: left;
                white-space: nowrap;
            ",
            onclick: move |_| {
                on_select.call(idx);
            },
            span { style: "font-size: 18px;", "{item.icon}" }
            span { "{item.label}" }
        }
    }
}

// ─── Tabs ────────────────────────────────────────────────────

/// Tab definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Tab {
    /// Tab label.
    pub label: String,
    /// Tab ID.
    pub id: String,
}

/// Tabs component props.
#[derive(Props, Clone, PartialEq)]
pub struct TabsProps {
    /// Tab definitions.
    pub tabs: Vec<Tab>,
    /// Active tab ID.
    pub active_id: String,
    /// Tab changed handler.
    pub on_change: EventHandler<String>,
}

/// Horizontal tab bar.
pub fn Tabs(props: TabsProps) -> Element {
    let palette = use_palette();

    rsx! {
        div {
            style: "
                display: flex;
                border-bottom: 1px solid {palette.border};
                gap: 0;
            ",
            for tab in props.tabs.iter() {
                { render_tab(tab, &props, palette) }
            }
        }
    }
}

fn render_tab(tab: &Tab, props: &TabsProps, palette: ColorPalette) -> Element {
    let active = tab.id == props.active_id;
    let border_bottom = if active {
        format!("2px solid {}", palette.primary)
    } else {
        "2px solid transparent".to_string()
    };
    let color = if active { palette.primary } else { palette.text_muted };
    let on_change = props.on_change.clone();
    let tab_id = tab.id.clone();

    rsx! {
        button {
            key: "{tab.id}",
            style: "
                background: none;
                border: none;
                border-bottom: {border_bottom};
                color: {color};
                padding: {spacing::SM} {spacing::LG};
                font-family: {typography::FONT_FAMILY};
                font-size: {typography::SIZE_MD};
                font-weight: {typography::WEIGHT_MEDIUM};
                cursor: pointer;
            ",
            onclick: move |_| {
                on_change.call(tab_id.clone());
            },
            {tab.label.clone()}
        }
    }
}

// ─── Panel ───────────────────────────────────────────────────

/// Panel component props.
#[derive(Props, Clone, PartialEq)]
pub struct PanelProps {
    /// Panel title.
    pub title: String,
    /// Panel content.
    pub children: Element,
}

/// Container panel with title bar.
pub fn Panel(props: PanelProps) -> Element {
    let palette = use_palette();

    rsx! {
        div {
            style: "
                display: flex;
                flex-direction: column;
                background: {palette.surface};
                border: 1px solid {palette.border};
                border-radius: {radius::LG};
                overflow: hidden;
            ",
            // Title bar
            div {
                style: "
                    padding: {spacing::SM} {spacing::MD};
                    border-bottom: 1px solid {palette.border};
                    font-family: {typography::FONT_FAMILY};
                    font-size: {typography::SIZE_SM};
                    font-weight: {typography::WEIGHT_BOLD};
                    color: {palette.text_muted};
                    text-transform: uppercase;
                    letter-spacing: 0.5px;
                ",
                {props.title.clone()}
            }
            // Content
            div {
                style: "
                    flex: 1;
                    padding: {spacing::MD};
                    overflow-y: auto;
                ",
                {props.children}
            }
        }
    }
}
