//! UR-01 — Design tokens / Theme
//!
//! Provides design tokens (colors, spacing, typography) and theme
//! switching. Reads the active theme from the `SettingsAtom` (UR-00).

use dioxus::prelude::*;
use trios_ui_ur00::{use_settings_atom, Theme};

// ─── Design tokens ───────────────────────────────────────────

/// Color palette for the active theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPalette {
    /// Primary brand color.
    pub primary: &'static str,
    /// Secondary color.
    pub secondary: &'static str,
    /// Background color.
    pub background: &'static str,
    /// Surface color (cards, panels).
    pub surface: &'static str,
    /// Text color.
    pub text: &'static str,
    /// Text muted color.
    pub text_muted: &'static str,
    /// Border color.
    pub border: &'static str,
    /// Accent color (success).
    pub accent_success: &'static str,
    /// Accent color (error).
    pub accent_error: &'static str,
    /// Accent color (warning).
    pub accent_warning: &'static str,
}

/// Dark theme palette.
pub const DARK_PALETTE: ColorPalette = ColorPalette {
    primary: "#c8a23c",       // Trinity gold
    secondary: "#8b7355",     // Warm bronze
    background: "#1a1a2e",    // Deep navy
    surface: "#16213e",       // Dark blue surface
    text: "#e0e0e0",          // Light gray text
    text_muted: "#888888",    // Muted gray
    border: "#2a2a4a",        // Subtle border
    accent_success: "#4caf50", // Green
    accent_error: "#ef5350",  // Red
    accent_warning: "#ff9800", // Orange
};

/// Light theme palette.
pub const LIGHT_PALETTE: ColorPalette = ColorPalette {
    primary: "#8b6914",       // Darker gold
    secondary: "#a0845c",     // Bronze
    background: "#f5f5f5",    // Light gray
    surface: "#ffffff",       // White
    text: "#1a1a2e",          // Dark navy text
    text_muted: "#666666",    // Muted gray
    border: "#dddddd",        // Light border
    accent_success: "#2e7d32", // Dark green
    accent_error: "#c62828",  // Dark red
    accent_warning: "#e65100", // Dark orange
};

/// Spacing tokens (in px).
pub mod spacing {
    /// 4px — tightest spacing.
    pub const XS: &str = "4px";
    /// 8px — small spacing.
    pub const SM: &str = "8px";
    /// 12px — medium spacing.
    pub const MD: &str = "12px";
    /// 16px — standard spacing.
    pub const LG: &str = "16px";
    /// 24px — large spacing.
    pub const XL: &str = "24px";
    /// 32px — extra large spacing.
    pub const XXL: &str = "32px";
}

/// Typography tokens.
pub mod typography {
    /// Font family.
    pub const FONT_FAMILY: &str = "'Inter', 'SF Pro', -apple-system, sans-serif";
    /// Monospace font family.
    pub const FONT_MONO: &str = "'JetBrains Mono', 'Fira Code', monospace";
    /// Font size: caption.
    pub const SIZE_XS: &str = "11px";
    /// Font size: small.
    pub const SIZE_SM: &str = "13px";
    /// Font size: body.
    pub const SIZE_MD: &str = "14px";
    /// Font size: large.
    pub const SIZE_LG: &str = "16px";
    /// Font size: heading.
    pub const SIZE_XL: &str = "20px";
    /// Font size: title.
    pub const SIZE_XXL: &str = "28px";
    /// Font weight: normal.
    pub const WEIGHT_NORMAL: &str = "400";
    /// Font weight: medium.
    pub const WEIGHT_MEDIUM: &str = "500";
    /// Font weight: bold.
    pub const WEIGHT_BOLD: &str = "600";
}

/// Border radius tokens.
pub mod radius {
    /// Small radius (2px).
    pub const SM: &str = "2px";
    /// Medium radius (4px).
    pub const MD: &str = "4px";
    /// Large radius (8px).
    pub const LG: &str = "8px";
    /// Full radius (pill shape).
    pub const FULL: &str = "9999px";
}

// ─── Theme hook ──────────────────────────────────────────────

/// Get the active color palette based on current settings.
///
/// # Example
/// ```rust,ignore
/// fn MyComponent() -> Element {
///     let palette = use_palette();
///     rsx! {
///         div { style: "background: {palette.background}; color: {palette.text};" }
///     }
/// }
/// ```
pub fn use_palette() -> &'static ColorPalette {
    let settings = use_settings_atom();
    match settings.read().theme {
        Theme::Dark => &DARK_PALETTE,
        Theme::Light => &LIGHT_PALETTE,
    }
}

/// Toggle between dark and light theme.
pub fn toggle_theme() {
    let mut settings = use_settings_atom();
    let current = settings.read().theme.clone();
    settings.write().theme = match current {
        Theme::Dark => Theme::Light,
        Theme::Light => Theme::Dark,
    };
}
