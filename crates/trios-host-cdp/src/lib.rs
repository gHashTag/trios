//! Facade for the host CDP agent (rings HC-00..02).
//!
//! - HC-00 — raw-WebSocket CDP client + target discovery
//! - HC-01 — SR-03 `BrowserCommand` → CDP execution (all 12 host tools)
//! - HC-02 — poll loop against trios-server (`browser/poll` → `browser/result`)

pub use trios_host_cdp_hc00::{discover_page_ws, CdpClient};
pub use trios_host_cdp_hc01::{execute_command, CdpCall};
pub use trios_host_cdp_hc02::{poll_round, run, CdpExecutor, CommandExecutor, PollerConfig};
