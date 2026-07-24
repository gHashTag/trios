//! trios-browser — browser control contracts.
//!
//! Re-export facade only (L-ARCH-001). Ported from the TS backend
//! `apps/server/src/browser/*` during Wave 2. The live CDP driver stays next
//! to the Chrome process; these rings are the data + action contract that
//! trios-server proxies over A2A.
//!
//! Rings:
//! - BW-00 `core`  — PageInfo, WindowInfo, WindowType/State (data + serde)
//! - BW-01 `proto` — BrowserCommand / BrowserResponse envelope

pub use trios_browser_bw00 as core;
pub use trios_browser_bw01 as proto;

pub use trios_browser_bw00::{
    PageInfo, SetWindowVisibilityResult, WindowBounds, WindowInfo, WindowState, WindowType,
};
pub use trios_browser_bw01::{
    BrowserCommand, BrowserResponse, ClickPoint, ScreenshotResult,
};
