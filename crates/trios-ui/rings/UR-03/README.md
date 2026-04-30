# UR-03 — Layout (Sidebar, Tabs, Panel)

## Purpose
Layout primitives: collapsible sidebar navigation, horizontal tab bar, and
container panels with title bars. Reads sidebar collapsed state from UR-00
`SettingsAtom`.

## Public API
- `NavItem` — struct: `label`, `icon`, `active`
- `SidebarProps` — props: `items: Vec<NavItem>`, `on_select: EventHandler<usize>`
- `Sidebar(props)` — collapsible sidebar with nav items (48px collapsed, 220px expanded)
- `Tab` — struct: `label`, `id`
- `TabsProps` — props: `tabs: Vec<Tab>`, `active_id: String`, `on_change: EventHandler<String>`
- `Tabs(props)` — horizontal tab bar with active indicator
- `PanelProps` — props: `title: String`, `children: Element`
- `Panel(props)` — container with title bar and scrollable content

## Dependencies
- `trios-ui-ur00` — `use_settings_atom()` for sidebar collapsed state
- `trios-ui-ur01` — `use_palette()`, `radius`, `spacing`, `typography`, `ColorPalette`
- `trios-ui-ur02` — listed in Cargo.toml but not directly used in code

## Ring Rules
- R1: Layout components are pure — state comes from UR-00 atoms
- R2: All dimensions use UR-01 spacing tokens
- R3: `render_nav_item()` and `render_tab()` are helper functions (not components)

## ⚠ Compilation Status
**DOES NOT COMPILE** — imports `use_settings_atom` from UR-00 (not exported).
Also blocked on UR-01 compilation.
