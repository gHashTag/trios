//! trios-ui — Dioxus + Jotai-style state (Ring Isolation architecture)
//!
//! This is the workspace root package. Actual code lives in rings/.
//! This file re-exports all ring modules for backward compatibility.
//!
//! # Ring Architecture
//!
//! | Ring | Purpose |
//! |------|---------|
//! | UR-00 | State atoms (Jotai-style) |
//! | UR-01 | Design tokens / Theme |
//! | UR-02 | Primitives (Button, Input, Badge) |
//! | UR-03 | Layout (Sidebar, Tabs, Panel) |
//! | UR-04 | Chat UI |
//! | UR-05 | Agent UI |
//! | UR-06 | MCP Panel |
//! | UR-07 | Settings |
//! | UR-08 | App Shell + Router |
//! | BR-APP | WASM entry → mount_app() |

pub use trios_ui_ur00::*;
pub use trios_ui_ur01::*;
pub use trios_ui_ur02::*;
pub use trios_ui_ur03::*;
pub use trios_ui_ur04::*;
pub use trios_ui_ur05::*;
pub use trios_ui_ur06::*;
pub use trios_ui_ur07::*;
pub use trios_ui_ur08::*;

/// Mount the full TRIOS application.
///
/// This is the main entry point called by the Chrome Extension via `trios_ui::mount_app()`.
/// It delegates to UR-08 (App Shell) which wires all atoms and components.
pub fn mount_app() {
    trios_ui_ur08::mount_app();
}
