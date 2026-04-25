//! Training state management for T-JEPA training.
//!
//! Module: train_state
//! Size: ≤ 60 lines (NASA Rule 4)

use std::time::Instant;

use trios_train_cpu::{
    jepa::{EmaConfig, EmaTarget},
    objective::{ObjectiveConfig, compute_combined_loss},
    MuonOptimizer,
};

const VOCAB: usize = 128;
const DIM: usize = 64;
const HIDDEN: usize = 384;
const NUM_CTX: usize = 4;
const NGRAM: usize = NUM_CTX + 2;

/// Training state tracking model, optimizers, and metrics.
pub struct TrainingState {
    pub model: NgramModel,
    pub target_model: NgramModel,
    pub opt_embed: OptWrapper,
    pub opt_ctx: Vec<OptWrapper>,
    pub opt_proj: OptWrapper,
    pub opt_head: OptWrapper,
    pub predictor: Option<JepaPredictor>,
    pub ema_target: EmaTarget,
    pub nca: Option<NcaObjective>,
    pub obj_config: ObjectiveConfig,
    pub best_val_bpb: f32,
    pub start_time: Instant,
    pub last_heartbeat: Instant,
}

/// Optimizer wrapper for different optimizer types.
pub enum OptWrapper {
    LocalAdamW(AdamW),
    CrateMuon(MuonOptimizer),
}

impl OptWrapper {
    /// Create AdamW optimizer.
    pub fn adamw(size: usize, wd: f32) -> Self {
        OptWrapper::LocalAdamW(AdamW::new(size, wd))
    }

    /// Create Muon optimizer.
    pub fn muon(size: usize, lr: f64, wd: f32) -> Self {
        OptWrapper::CrateMuon(MuonOptimizer::new(size, lr, 0.95, wd as f64))
    }

    /// Execute one optimization step.
    pub fn step(&mut self, params: &mut [f32], grads: &[f32], lr: f32) {
        match self {
            OptWrapper::LocalAdamW(opt) => opt.update(params, grads, lr),
            OptWrapper::CrateMuon(opt) => {
                opt.lr = lr as f64;
                opt.step(params, grads);
            }
        }
    }
}

/// Initialize training state from configuration.
///
/// # Returns
/// Fully initialized `TrainingState` ready for training loop.
pub fn init_training(cfg: &Config) -> TrainingState {
    let make_opt = |size: usize, wd: f32| -> OptWrapper {
        match cfg.opt_kind {
            OptKind::AdamW => OptWrapper::adamw(size, wd),
            OptKind::Muon => OptWrapper::muon(size, cfg.encoder_lr as f64, wd),
        }
    };

    let wd = 0.04f32;
    TrainingState {
        model: NgramModel::new(cfg.seed),
        target_model: NgramModel::new(cfg.seed),
        opt_embed: make_opt(VOCAB * DIM, wd),
        opt_ctx: (0..NUM_CTX).map(|_| make_opt(VOCAB * DIM, wd)).collect(),
        opt_proj: make_opt(HIDDEN * DIM, wd),
        opt_head: make_opt(VOCAB * HIDDEN, wd),
        predictor: if cfg.use_jepa {
            Some(JepaPredictor::new(PredictorConfig::with_d_model(HIDDEN)))
        } else {
            None
        },
        ema_target: EmaTarget::new(EmaConfig { start: 0.996, end: 1.0, ramp_steps: cfg.steps }),
        nca: if cfg.use_nca { Some(NcaObjective::default()) } else { None },
        obj_config: ObjectiveConfig {
            ntp_weight: cfg.ntp_weight,
            jepa_weight: cfg.jepa_weight,
            nca_weight: cfg.nca_weight,
        },
        best_val_bpb: f32::MAX,
        start_time: Instant::now(),
        last_heartbeat: Instant::now(),
    }
}
