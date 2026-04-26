//! trios-trainer — Single source of truth for IGLA training
//!
//! Run on any machine:
//! ```bash
//! cargo run --release -p trios-trainer -- \
//!     --config crates/trios-trainer/configs/champion.toml --seed 43
//! ```

pub mod config;
pub mod data;
pub mod ledger;
pub mod train_loop;

// Re-exports for convenience
pub use config::{Config, LoadConfigError};
pub use data::FineWebDataset;
pub use ledger::{emit_row, EmbargoBlock, Triplet};
pub use train_loop::run;
