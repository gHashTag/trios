//! trios-trainer — Single source of truth for IGLA training
//!
//! Run on any machine:
//! ```bash
//! cargo run --release -p trios-trainer -- \
//!     --config crates/trios-trainer/configs/champion.toml --seed 43
//! ```
//!
//! ## Architecture
//!
//! - **config**: TOML loading with INV-8 validation
//! - **data**: FineWeb binary dataset loader
//! - **ledger**: Triplet-validated row emission
//! - **train_loop**: Main training orchestration
//! - **model**: MinimalTransformer (MHA + FFN)
//! - **forward**: CPU matmul, GELU, LayerNorm
//! - **backward**: Gradients, cross-entropy, clipping
//! - **optimizer**: AdamW, Muon, φ-schedule

pub mod config;
pub mod data;
pub mod ledger;
pub mod train_loop;
pub mod model;
pub mod optimizer;
pub mod forward;
pub mod backward;

// Re-exports for convenience
pub use config::{Config, LoadConfigError, validate_lr_phi_band};
pub use data::FineWebDataset;
pub use ledger::{emit_row, EmbargoBlock, Triplet, get_commit_sha};
pub use train_loop::{run, RunResult};
pub use model::{MinimalTransformer, ModelGradients, ModelParameters};
pub use optimizer::{AdamWCpu, MuonOptimizer, SGDMomentum, OptimizerKind, phi_lr_schedule};
pub use forward::{matmul, gelu, layer_norm, softmax, LayerDims};
pub use backward::{linear_backward, gelu_backward, layer_norm_backward, cross_entropy_loss, clip_gradients};
