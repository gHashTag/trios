//! trios-a2a — Agent-to-Agent protocol
//!
//! Re-exports all rings for convenient access.

pub use trios_a2a_sr00::*;
pub use trios_a2a_sr01::*;
pub use trios_a2a_sr02::*;

// SR-03 — host-runtime browser command queue (server queues, host agent polls
// & reports back over A2A). Ported browser-adapter transport.
pub use trios_a2a_sr03::{BrowserCommand, BrowserCommandQueue, BrowserResult};

// SR-04 — durable persistence (A2AStore trait + SqliteA2AStore).
pub use trios_a2a_sr04::{A2AStore, SqliteA2AStore};

// Re-export A2ARouter from BR-OUTPUT (not a workspace dep, use extern crate)
pub use trios_a2a_br_output::A2ARouter;
