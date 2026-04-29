# UR-06 Ring Spec

## Component: ToastProvider

### Props
- `max_visible: u32` (default 3)
- `duration_ms: u32` (default 5000)

### Behavior
- Global toast provider wrapping app
- Shows success/error/warning/info toasts
- Stacks up to max_visible, auto-dismisses after duration
- Manual dismiss via close button
- Animates in from top-right, slides out on dismiss

### API
```rust
fn show_toast(level: ToastLevel, message: String)
```

### Dependencies
- UR-00: GlobalSignal API

### Tests
- Shows toast on trigger
- Auto-dismisses after duration
- Stacks multiple toasts
