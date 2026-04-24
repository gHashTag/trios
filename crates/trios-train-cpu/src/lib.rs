#![allow(clippy::field_reassign_with_default)]

//! Pure Rust CPU Training for IGLA-GF16

pub mod backward;
pub mod bench;
pub mod forward;
pub mod optimizer;
pub mod tokenizer;
pub mod phi_ortho_init;
pub mod ortho_init_baseline;
pub mod swa_phi;
pub mod residual_mix;
pub mod sliding_eval;
pub mod trinity_3k_model;
pub mod gf16;
pub mod real_igla_model;
pub mod real_igla_trainer;

// T-JEPA: Ternary Joint Embedding Predictive Architecture (TASK-5A)
// Spec: .trinity/specs/issue143-task5a-jepa-design.md
// Theory: https://github.com/gHashTag/trinity/tree/main/docs/research/models/JEPA-T/
pub mod jepa;

// Multi-objective loss + ASHA rung schedules (TASK-5A.6)
pub mod objective;

pub use backward::{clip_gradients, cross_entropy_loss, LinearGradients};
pub use bench::{
    bpb_from_loss, estimate_model_size, print_metrics, train_cpu_loop, train_cpu_trace,
    BenchmarkConfig, BenchmarkMetrics, BenchmarkRun, StepTrace, TrainConfig, TrainMetrics,
};
pub use forward::{gelu, layer_norm, matmul, softmax, LayerDims};
pub use optimizer::{phi_lr_schedule, AdamWCpu};
pub use tokenizer::BPETokenizer;
pub use phi_ortho_init::phi_ortho_init;
pub use ortho_init_baseline::ortho_init_baseline;
pub use swa_phi::{SwaState, swa_init};
pub use residual_mix::ResidualMixConfig;
pub use sliding_eval::SlidingEvalConfig;
pub use jepa::{JepaConfig, JepaResult};
pub use objective::{ObjectiveConfig, ComponentLosses, CombinedLoss, compute_combined_loss};
