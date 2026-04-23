//! trios-a2a — Agent-to-Agent protocol
//!
//! Re-exports all rings for convenient access.

pub use trios_a2a_sr00::*;
pub use trios_a2a_sr01::*;
pub use trios_a2a_sr02::*;
pub use trios_a2a_sr03::*;

// Re-export A2ARouter from BR-OUTPUT (not a workspace dep, use extern crate)
pub use trios_a2a_br_output::A2ARouter;
