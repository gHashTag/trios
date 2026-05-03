//! UR-02 — Primitives (Button, Input, Badge)
//!
//! Reusable UI primitives that consume design tokens from UR-01.
//! These are the building blocks for all higher-level components.

use dioxus::prelude::*;
use trios_ui_ur01::{use_palette, radius, spacing, typography};

// ─── Button ──────────────────────────────────────────────────

/// Button variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    /// Primary action button.
    Primary,
    /// Secondary action button.
    Secondary,
    /// Ghost/transparent button.
    Ghost,
    /// Danger/destructive button.
    Danger,
}

impl Default for ButtonVariant {
    fn default() -> Self {
        Self::Primary
    }
}

/// Button component props.
#[derive(Props, Clone, PartialEq)]
pub struct ButtonProps {
    /// Button label.
    pub children: Element,
    /// Button variant.
    #[props(default = ButtonVariant::Primary)]
    pub variant: ButtonVariant,
    /// Disabled state.
    #[props(default = false)]
    pub disabled: bool,
    /// Optional click handler.
    pub onclick: Option<EventHandler<()>>,
}

/// Primary button component.
pub fn Button(props: ButtonProps) -> Element {
    let palette = use_palette();
    let (bg, color, border) = match props.variant {
        ButtonVariant::Primary => (palette.primary, palette.background, "none"),
        ButtonVariant::Secondary => (palette.surface, palette.text, palette.border),
        ButtonVariant::Ghost => ("transparent", palette.text, "none"),
        ButtonVariant::Danger => (palette.accent_error, "#ffffff", "none"),
    };
    let opacity = if props.disabled { "0.5" } else { "1.0" };
    let cursor = if props.disabled { "not-allowed" } else { "pointer" };

    rsx! {
        button {
            style: "
                background: {bg};
                color: {color};
                border: 1px solid {border};
                border-radius: {radius::MD};
                padding: {spacing::SM} {spacing::LG};
                font-family: {typography::FONT_FAMILY};
                font-size: {typography::SIZE_MD};
                font-weight: {typography::WEIGHT_MEDIUM};
                opacity: {opacity};
                cursor: {cursor};
                transition: opacity 0.15s;
            ",
            disabled: props.disabled,
            onclick: move |_| {
                if let Some(handler) = &props.onclick {
                    handler.call(());
                }
            },
            {props.children.clone()}
        }
    }
}

// ─── Input ───────────────────────────────────────────────────

/// Input component props.
#[derive(Props, Clone, PartialEq)]
pub struct InputProps {
    /// Placeholder text.
    pub placeholder: String,
    /// Current value.
    pub value: String,
    /// Change handler.
    pub oninput: EventHandler<String>,
    /// Optional label.
    #[props(default = String::new())]
    pub label: String,
    /// Monospace font.
    #[props(default = false)]
    pub mono: bool,
}

/// Text input component.
pub fn Input(props: InputProps) -> Element {
    let palette = use_palette();
    let font = if props.mono {
        typography::FONT_MONO
    } else {
        typography::FONT_FAMILY
    };

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: {spacing::XS};",
            if !props.label.is_empty() {
                label {
                    style: "font-size: {typography::SIZE_SM}; color: {palette.text_muted}; font-family: {typography::FONT_FAMILY};",
                    {props.label.clone()}
                }
            }
            input {
                style: "
                    background: {palette.surface};
                    color: {palette.text};
                    border: 1px solid {palette.border};
                    border-radius: {radius::MD};
                    padding: {spacing::SM} {spacing::MD};
                    font-family: {font};
                    font-size: {typography::SIZE_MD};
                    outline: none;
                ",
                r#type: "text",
                placeholder: "{props.placeholder}",
                value: "{props.value}",
                oninput: move |e: Event<FormData>| {
                    let val = e.data.value();
                    props.oninput.call(val);
                },
            }
        }
    }
}

// ─── Badge ───────────────────────────────────────────────────

/// Badge variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BadgeVariant {
    /// Default/neutral badge.
    Default,
    /// Success badge.
    Success,
    /// Error badge.
    Error,
    /// Warning badge.
    Warning,
}

impl Default for BadgeVariant {
    fn default() -> Self {
        Self::Default
    }
}

/// Badge component props.
#[derive(Props, Clone, PartialEq)]
pub struct BadgeProps {
    /// Badge label.
    pub children: Element,
    /// Badge variant.
    #[props(default = BadgeVariant::Default)]
    pub variant: BadgeVariant,
}

/// Small badge/tag component.
pub fn Badge(props: BadgeProps) -> Element {
    let palette = use_palette();
    let (bg, color) = match props.variant {
        BadgeVariant::Default => (palette.surface, palette.text),
        BadgeVariant::Success => (palette.accent_success, "#ffffff"),
        BadgeVariant::Error => (palette.accent_error, "#ffffff"),
        BadgeVariant::Warning => (palette.accent_warning, "#ffffff"),
    };

    rsx! {
        span {
            style: "
                display: inline-block;
                background: {bg};
                color: {color};
                border-radius: {radius::FULL};
                padding: 2px {spacing::SM};
                font-size: {typography::SIZE_XS};
                font-family: {typography::FONT_FAMILY};
                font-weight: {typography::WEIGHT_MEDIUM};
            ",
            {props.children.clone()}
        }
    }
}
