//! trios-trainer — Single source of truth for IGLA training

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
pub use model::MinimalTransformer;
pub use optimizer::{AdamWCpu, MuonOptimizer, SGDMomentum, OptimizerKind, phi_lr_schedule};
pub use forward::{matmul, gelu, layer_norm, softmax, LayerDims};
pub use backward::{linear_backward, gelu_backward, layer_norm_backward, softmax_cross_entropy_backward, cross_entropy_loss, clip_gradients};
