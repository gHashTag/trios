# UR-01 — Design Tokens / Theme

## Purpose
Provides design tokens (colors, spacing, typography, border radius) and theme
switching. Reads the active theme from the `SettingsAtom` (UR-00).

## Public API
- `ColorPalette` — struct with 10 color fields (primary, secondary, background, surface, text, text_muted, border, accent_success, accent_error, accent_warning)
- `DARK_PALETTE` — dark theme palette (Trinity gold primary, deep navy background)
- `LIGHT_PALETTE` — light theme palette (darker gold primary, white surface)
- `spacing` module — `XS`(4px), `SM`(8px), `MD`(12px), `LG`(16px), `XL`(24px), `XXL`(32px)
- `typography` module — `FONT_FAMILY`, `FONT_MONO`, `SIZE_XS`..`SIZE_XXL`, `WEIGHT_NORMAL`..`WEIGHT_BOLD`
- `radius` module — `SM`(2px), `MD`(4px), `LG`(8px), `FULL`(9999px)
- `use_palette()` — Dioxus hook returning `&'static ColorPalette` based on current theme
- `toggle_theme()` — switches between Dark and Light theme

## Dependencies
- `trios-ui-ur00` — imports `use_settings_atom()`, `Theme` enum

## Ring Rules
- R1: All color values reference Trinity Brand Kit
- R2: Spacing/typography/radius are `&'static str` constants (CSS-compatible)
- R3: `use_palette()` reads theme from UR-00 `SettingsAtom`

## ⚠ Compilation Status
**DOES NOT COMPILE** — `trios_ui_ur00` does not export `use_settings_atom` or `Theme`.
UR-00 needs to be refactored to export these, or UR-01 needs self-contained theme state.
