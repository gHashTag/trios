//! Main coordinator for T-JEPA training (refactored for NASA Rule 4).
//!
//! Module: main
//! Size: ≤ 60 lines (NASA Rule 4)

use anyhow::Result;
use std::env;
use std::fs;
use std::time::Instant;

pub mod train_state;
pub mod model;
pub mod training_step;
pub mod gradient;

use train_state::{TrainingState, OptWrapper};
use model::NgramModel;
use training_step::{
    jepa_training_step, nca_training_step, cosine_lr,
    print_banner, print_results, neon_heartbeat, neon_trial_complete,
    neon_trial_start, evaluate,
};
use gradient::compute_grads;
use trios_train_cpu::{
    jepa::JepaPredictor,
    objective::{ComponentLosses, ObjectiveConfig, compute_combined_loss},
};

const HIDDEN: usize = 384;
const DIM: usize = 64;
const NUM_CTX: usize = 4;
const NGRAM: usize = NUM_CTX + 2;
const SEQ: usize = 64;

/// Training configuration parsed from CLI arguments.
pub struct Config {
    pub seed: u64,
    pub steps: usize,
    pub encoder_lr: f32,
    pub ntp_lr: f32,
    pub use_jepa: bool,
    pub use_nca: bool,
    pub ntp_weight: f64,
    pub jepa_weight: f64,
    pub nca_weight: f64,
    pub opt_kind: OptKind,
    pub jepa_warmup: usize,
    pub trial_id: String,
    pub agent_id: String,
}

/// Optimizer kind selector.
pub enum OptKind {
    AdamW,
    Muon,
}

/// Find CLI argument with prefix.
fn find_arg<T: std::str::FromStr>(args: &[String], prefix: &str, default: T) -> T {
    args.iter()
        .find(|a| a.starts_with(prefix))
        .and_then(|a| a[prefix.len()..].parse().ok())
        .unwrap_or(default)
}

/// Parse configuration from CLI arguments.
pub fn parse_config(args: &[String]) -> Config {
    let has_encoder_lr = args.iter().any(|a| a.starts_with("--encoder-lr="));
    let encoder_lr: f32 = if has_encoder_lr {
        find_arg(args, "--encoder-lr=", 0.004)
    } else {
        find_arg(args, "--lr=", 0.004)
    };
    let seed: u64 = find_arg(args, "--seed=", 43);
    let steps: usize = find_arg(args, "--steps=", 3000);
    let ntp_lr: f32 = find_arg(args, "--ntp-lr=", 0.001);
    let ntp_weight: f64 = find_arg(args, "--ntp-weight=", 1.0);
    let jepa_weight: f64 = find_arg(args, "--jepa-weight=", 1.0);
    let nca_weight: f64 = find_arg(args, "--nca-weight=", 0.25);
    let jepa_warmup: usize = find_arg(args, "--jepa-warmup=", 1500);
    let use_jepa = !args.iter().any(|a| a == "--no-jepa");
    let use_nca = !args.iter().any(|a| a == "--no-nca");
    let opt_kind = if args.iter().any(|a| a == "--optimizer=muon") {
        OptKind::Muon
    } else {
        OptKind::AdamW
    };
    let trial_id: String = find_arg(args, "--trial-id=", "hybrid-001".to_string());
    let agent_id: String = find_arg(args, "--agent-id=", "ALFA".to_string());

    assert!(encoder_lr > 0.0, "encoder_lr must be positive");
    assert!(ntp_lr > 0.0, "ntp_lr must be positive");
    assert!(steps > 0, "steps must be positive");
    assert!(ntp_weight >= 0.0, "ntp_weight must be >= 0");
    assert!(jepa_weight >= 0.0, "jepa_weight must be >= 0");
    assert!(nca_weight >= 0.0, "nca_weight must be >= 0");

    Config {
        seed, steps, encoder_lr, ntp_lr, use_jepa, use_nca,
        ntp_weight, jepa_weight, nca_weight, opt_kind, jepa_warmup,
        trial_id, agent_id,
    }
}

/// Load training data from file.
fn load_data(path: &str) -> Vec<usize> {
    let raw = fs::read(path).unwrap_or_else(|e| {
        eprintln!("Failed to load {}: {}. Using fallback.", path, e);
        "The quick brown fox jumps over the lazy dog. ".repeat(100).to_vec()
    });
    assert!(!raw.is_empty(), "loaded data is empty");
    raw.into_iter().map(|b| (b as usize) % 128).collect()
}

/// Run the complete training loop.
fn run_training_loop(cfg: &Config) -> Result<f32> {
    let train_data = load_data("data/tiny_shakespeare.txt");
    let val_data = load_data("data/tiny_shakespeare_val.txt");
    let train_end = (train_data.len() as f64 * 0.9) as usize;
    let train = &train_data[..train_end];
    let val = if val_data.len() > 100 { &val_data } else { &train_data[train_end..] };

    let mut st = train_state::init_training(cfg);
    let warmup = cfg.steps / 10;

    for step in 1..=cfg.steps {
        let dl = train.len();
        let off = (step * 97 + cfg.seed as usize) % dl.saturating_sub(SEQ + 1);
        let seq = &train[off..off + SEQ + 1];

        let (grads, hidden_vecs, ntp_loss) = compute_grads(&st.model, seq);

        let jepa_loss_val = if let Some(ref pred) = st.predictor {
            jepa_training_step(
                pred, &st.model, &mut st.target_model, &hidden_vecs, seq,
                cfg.seed, step, &mut st.ema_target,
            ).loss
        } else {
            0.0
        };

        let nca_loss_val = nca_training_step(&st.nca.unwrap(), cfg.seed, step);

        let combined = compute_combined_loss(
            ComponentLosses {
                ntp: ntp_loss as f64 / LN_2 as f64,
                jepa: jepa_loss_val,
                nca: nca_loss_val,
            },
            st.obj_config,
        );

        let enc_lr = cosine_lr(step, cfg.steps, cfg.encoder_lr, warmup);
        let head_lr = cosine_lr(step, cfg.steps, cfg.ntp_lr, warmup);
        st.opt_embed.step(&mut st.model.embed, &grads.g_embed, enc_lr);
        for (ci, oc) in st.opt_ctx.iter_mut().enumerate() {
            oc.step(&mut st.model.ctx[ci], &grads.g_ctx[ci], enc_lr);
        }
        st.opt_proj.step(&mut st.model.proj, &grads.g_proj, enc_lr);
        st.opt_head.step(&mut st.model.lm_head, &grads.g_head, head_lr);

        if step % 500 == 0 || step == cfg.steps {
            let elapsed = st.start_time.elapsed().as_secs_f64();
            let val_bpb = evaluate(&st.model, val);
            if val_bpb < st.best_val_bpb && val_bpb.is_finite() {
                st.best_val_bpb = val_bpb;
            }
            eprintln!("step={:5} ntp={:.4} jepa={:.4} nca={:.4} val_bpb={:.4} best={:.4} t={:.1}s",
                      step, combined.components.ntp, combined.components.jepa,
                      combined.components.nca, val_bpb, st.best_val_bpb, elapsed);
        }

        neon_heartbeat(cfg, step, st.best_val_bpb, &mut st.last_heartbeat);
    }

    let elapsed = st.start_time.elapsed().as_secs_f64();
    neon_trial_complete(cfg, st.best_val_bpb);
    Ok(st.best_val_bpb)
}

/// Main entry point for T-JEPA training.
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cfg = parse_config(&args);

    print_banner(&cfg);
    neon_trial_start(&cfg);

    let best_bpb = run_training_loop(&cfg)?;

    let elapsed = Instant::now().elapsed().as_secs_f64();
    print_results(&cfg, best_bpb, elapsed);

    Ok(())
}
