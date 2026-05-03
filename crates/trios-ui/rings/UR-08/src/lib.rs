//! UR-08 — App Shell + Router
//!
//! Top-level application shell with sidebar navigation and
//! tab-based routing. Wires all atoms via the `AtomProvider`.
//! This is the main entry point for the Dioxus app.

use dioxus::prelude::*;
use trios_ui_ur00::use_settings_atom;
use trios_ui_ur01::{use_palette, spacing, typography};
use trios_ui_ur03::{NavItem, Sidebar};

// ─── Route state ─────────────────────────────────────────────

/// Available app routes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Route {
    /// Social feed (A2A agent chat).
    Social,
    /// Chat panel.
    Chat,
    /// Agent list.
    Agents,
    /// MCP tools panel.
    Mcp,
    /// Settings panel.
    Settings,
}

impl Route {
    /// Get the navigation label.
    pub fn label(&self) -> &'static str {
        match self {
            Route::Social => "Social",
            Route::Chat => "Chat",
            Route::Agents => "Agents",
            Route::Mcp => "MCP",
            Route::Settings => "Settings",
        }
    }

    /// Get the navigation icon.
    pub fn icon(&self) -> &'static str {
        match self {
            Route::Social => "🕸️",
            Route::Chat => "💬",
            Route::Agents => "🤖",
            Route::Mcp => "🔌",
            Route::Settings => "⚙️",
        }
    }

    /// All routes in sidebar order.
    pub fn all() -> Vec<Route> {
        vec![Route::Social, Route::Chat, Route::Agents, Route::Mcp, Route::Settings]
    }
}

// ─── App Shell ───────────────────────────────────────────────

/// Main application shell component.
/// Renders sidebar + content area based on active route.
pub fn AppShell() -> Element {
    let palette = use_palette();
    let mut active_route = use_signal(|| Route::Social);
    let _settings = use_settings_atom();

    let nav_items: Vec<NavItem> = Route::all()
        .iter()
        .map(|r| NavItem {
            label: r.label().to_string(),
            icon: r.icon().to_string(),
            active: *active_route.read() == *r,
        })
        .collect();

    let current = *active_route.read();

    rsx! {
        div {
            style: "
                display: flex;
                height: 100vh;
                width: 100vw;
                background: {palette.background};
                color: {palette.text};
                font-family: {typography::FONT_FAMILY};
                overflow: hidden;
            ",
            // Sidebar
            Sidebar {
                items: nav_items,
                on_select: move |idx: usize| {
                    let routes = Route::all();
                    if idx < routes.len() {
                        active_route.set(routes[idx]);
                    }
                },
            }
            // Main content area
            div {
                style: "
                    flex: 1;
                    display: flex;
                    flex-direction: column;
                    overflow: hidden;
                ",
                // Header
                div {
                    style: "
                        padding: {spacing::MD} {spacing::LG};
                        border-bottom: 1px solid {palette.border};
                        font-family: {typography::FONT_FAMILY};
                        font-size: {typography::SIZE_XL};
                        font-weight: {typography::WEIGHT_BOLD};
                        color: {palette.text};
                    ",
                    "Trinity {current.label()}"
                }
                // Content
                div {
                    style: "flex: 1; overflow: hidden;",
                    { render_route(current) }
                }
            }
        }
    }
}

/// Render the content for a given route.
fn render_route(route: Route) -> Element {
    match route {
        Route::Social => rsx! { trios_ui_ur09::SocialPanel {} },
        Route::Chat => rsx! { trios_ui_ur04::ChatPanel {} },
        Route::Agents => rsx! { trios_ui_ur05::AgentList {} },
        Route::Mcp => rsx! { trios_ui_ur06::McpPanel {} },
        Route::Settings => rsx! { trios_ui_ur07::SettingsPanel {} },
    }
}

// ─── mount_app ───────────────────────────────────────────────

/// Mount the full TRIOS application.
///
/// This is the primary entry point called by the root `trios-ui` crate
/// and by `trios-ext` via `trios_ui::mount_app()`.
pub fn mount_app() {
    let _dom = VirtualDom::new(AppShell);
    // In a real WASM build, this would use dioxus::web::launch_cfg
    // For now, we just ensure the VirtualDom is created successfully.
}
