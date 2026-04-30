# UR-05 Ring Spec

## Component: SidebarLayout

### Props
- `collapsed: bool` (default false)
- `width: u32` (default 280px)

### Behavior
- Renders collapsible sidebar with main content area
- Sidebar can be toggled via button or keyboard shortcut (Cmd+B)
- Responsive: auto-collapses below 768px viewport
- Animation: slide transition (200ms ease)

### Dependencies
- UR-00: GlobalSignal API

### Tests
- Renders with sidebar open by default
- Toggles collapsed state
- Auto-collapses on narrow viewport
