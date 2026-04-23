# TASK.md — UR-01

## Current
- [x] `ColorPalette` struct with 10 color fields
- [x] `DARK_PALETTE` — Trinity gold primary, deep navy background
- [x] `LIGHT_PALETTE` — darker gold primary, white surface
- [x] `spacing` module — XS(4px) through XXL(32px)
- [x] `typography` module — font families, sizes, weights
- [x] `radius` module — SM(2px) through FULL(9999px)
- [x] `use_palette()` hook
- [x] `toggle_theme()` function

## Blockers
- [ ] **Cannot compile**: imports `use_settings_atom` and `Theme` from `trios_ui_ur00`,
  but UR-00 doesn't export these. Needs UR-00 refactoring or self-contained types.

## Next
- [ ] Fix compilation (see Blockers)
- [ ] Add shadow tokens (SM, MD, LG)
- [ ] Add transition tokens (fast, normal, slow)
- [ ] Add z-index tokens (dropdown, modal, tooltip)
- [ ] Add high-contrast palette variant
