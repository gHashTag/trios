# UR-04 Ring Spec

## Component: ThemeToggle

### Props
- `initial: Theme` (Light | Dark | System)

### Behavior
- Renders button with sun/moon icon
- On click: cycles Light -> Dark -> System -> Light
- Updates `GlobalSignal<Theme>` via `use_theme()`
- Persists selection to localStorage key `trios-theme`

### Dependencies
- UR-00: GlobalSignal API (Dioxus 0.6)
- UR-03: Theme signal wiring

### Tests
- Renders default theme
- Cycles through themes on click
- Persists to localStorage
