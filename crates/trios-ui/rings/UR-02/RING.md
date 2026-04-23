# UR-02 — UI Primitives (Button, Input, Badge)

## Purpose
Reusable UI primitives that consume design tokens from UR-01.
These are the building blocks for all higher-level components.

## Public API
- `ButtonVariant` — enum: `Primary`, `Secondary`, `Ghost`, `Danger`
- `ButtonProps` — props: `children: String`, `variant`, `disabled`, `onclick`
- `Button(props)` — styled button component with variant-based theming
- `InputProps` — props: `placeholder`, `value`, `oninput`, `label`, `mono`
- `Input(props)` — text input with optional label and monospace font
- `BadgeVariant` — enum: `Default`, `Success`, `Error`, `Warning`
- `BadgeProps` — props: `children: String`, `variant`
- `Badge(props)` — small badge/tag component with color variants

## Dependencies
- `trios-ui-ur00` — listed in Cargo.toml (transitive, not directly used in code)
- `trios-ui-ur01` — `use_palette()`, `radius`, `spacing`, `typography` tokens

## Ring Rules
- R1: All styling via inline `style` attributes using UR-01 tokens
- R2: Components are pure functions — no internal state
- R3: No external CSS frameworks

## ⚠ Compilation Status
**DOES NOT COMPILE** — depends on UR-01 which doesn't compile (UR-00 missing exports).
If UR-01 is fixed, UR-02 should compile (it only uses UR-01's public API).
