# AGENTS.md — UR-02

## Agent: ALPHA
- Add new primitives (Select, Toggle, Tooltip, Modal)
- Add new Button variants (Outline, Link, Icon)
- Add size variants (SM, MD, LG) for all components

## Agent: BETA
- Verify component rendering in Chrome Extension sidebar
- Test keyboard navigation and focus styles
- Test disabled states

## Agent: DELTA
- Fix compilation (blocked on UR-01 fix)
- Consider removing `trios-ui-ur00` from Cargo.toml if not needed

## Rules
- R1: All styling via inline `style` attributes using UR-01 tokens
- R2: Components accept `oninput`/`onclick` as `EventHandler` props
- R3: No internal state — all state passed via props
