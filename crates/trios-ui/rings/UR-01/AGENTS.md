# AGENTS.md — UR-01

## Agent: ALPHA
- Add new design tokens (shadows, transitions, z-index)
- Add new palette variants (high-contrast, custom accent)
- Modify spacing/typography/radius constants

## Agent: BETA
- Verify token values render correctly in Chrome Extension sidebar
- Test dark/light palette contrast ratios

## Agent: DELTA
- Fix compilation: `use_settings_atom()` and `Theme` are not exported by UR-00
- Either: add exports to UR-00, or make UR-01 self-contained with its own theme Signal

## Rules
- R1: All color values must match Trinity Brand Kit (`BRAND_KIT.md`)
- R2: Token values are `&'static str` for direct CSS interpolation
- R3: No external dependencies beyond Dioxus and UR-00
