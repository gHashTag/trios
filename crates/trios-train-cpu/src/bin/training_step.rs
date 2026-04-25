//! Training step utilities for T-JEPA optimization.
//!
//! Module: training_step
//! Size: ≤ 60 lines (NASA Rule 4)

use std::time::Instant;

use crate::train_state::OptWrapper;
use crate::model::NgramModel;
use trios_train_cpu::{
    jepa::{EmaTarget, JepaPredictor, EmaConfig, build_span_mask},
    objective::{ObjectiveConfig, compute_combined_loss, ComponentLosses},
};

const LN_2: f32 = std::f32::consts::LN_2;
const HEARTBEAT_INTERVAL_SECS: u64 = 60;
const HIDDEN: usize = 384;
const DIM: usize = 64;
const NGRAM: usize = 6;
const SEQ: usize = 64;
const VOCAB: usize = 128;
const NUM_CTX: usize = 4;

/// Result of a JEPA training step.
pub struct JepaStepResult {
    pub loss: f64,
}

/// Execute one JEPA training step with predictor.
///
/// # Arguments
/// - `predictor`: JEPA predictor for hidden state prediction
/// - `model`: Current target model
/// - `target_model`: Model to update
/// - `hidden_vecs`: Hidden states from forward pass
/// - `seq`: Token sequence
/// - `seed`: Random seed for mask construction
/// - `step`: Current step number
/// - `ema_target`: EMA target for weight updates
///
/// # Returns
/// Loss from JEPA prediction step.
pub fn jepa_training_step(
    predictor: &mut JepaPredictor,
    model: &NgramModel,
    target_model: &mut NgramModel,
    hidden_vecs: &[Vec<f32>],
    seq: &[usize],
    seed: u64,
    step: usize,
    ema_target: &mut EmaTarget,
) -> JepaStepResult {
    use trios_train_cpu::jepa::build_span_mask;

    let mask_result = build_span_mask(hidden_vecs.len().min(SEQ), seed, step);
    let (tgt_pos, ctx_pos) = mask_result;
    if tgt_pos.is_empty() || ctx_pos.is_empty() {
        return JepaStepResult { loss: 0.0 };
    }

    let zero_h = vec![0.0f32; HIDDEN];
    let ctx_flat: Vec<f32> = ctx_pos.iter()
        .flat_map(|&p| hidden_vecs.get(p).unwrap_or(&zero_h).iter().copied())
        .collect();

    let tgt_hidden: Vec<Vec<f32>> = tgt_pos.iter()
        .filter_map(|&p| {
            if p + NGRAM <= seq.len() {
                Some(target_model.compute_hidden(&seq[p..p + NGRAM]))
            } else {
                None
            }
        })
        .collect();

    let loss = if tgt_hidden.is_empty() {
        0.0f64
    } else {
        let tgt_flat: Vec<f32> = tgt_hidden.iter().flat_map(|v| v.iter().copied()).collect();
        predictor.forward_backward(&ctx_flat, &tgt_flat, tgt_hidden.len()) as f64
    };

    let decay = ema_target.decay() as f32;
    model.update_ema(target_model, decay);

    JepaStepResult { loss }
}

/// Execute one NCA entropy step.
///
/// # Arguments
/// - `nca`: NCA objective
/// - `seed`: Random seed for state initialization
/// - `step`: Current step number
///
/// # Returns
/// Entropy loss from NCA step.
pub fn nca_training_step(nca: &NcaObjective, seed: u64, step: usize) -> f64 {
    let nca_seed = seed.wrapping_add(step as u64).wrapping_mul(7919);
    let nca_state = nca.init_grid(nca_seed);
    let (loss, _) = nca.entropy_loss(
        &nca_state,
        nca.k_states,
        nca.entropy_min,
        nca.entropy_max,
        nca.weight,
    );
    assert!(loss.is_finite(), "NCA loss is not finite");
    loss
}

/// Cosine learning rate scheduler.
///
/// # Arguments
/// - `step`: Current training step
/// - `max_steps`: Maximum training steps
/// - `base_lr`: Base learning rate
/// - `warmup`: Number of warmup steps
///
/// # Returns
/// Scheduled learning rate for current step.
pub fn cosine_lr(step: usize, max_steps: usize, base_lr: f32, warmup: usize) -> f32 {
    assert!(max_steps > 0, "cosine_lr: max_steps=0");
    if step < warmup {
        return base_lr * step as f32 / warmup.max(1) as f32;
    }
    let p = (step - warmup) as f32 / (max_steps - warmup).max(1) as f32;
    1e-5 + (base_lr - 1e-5) * 0.5 * (1.0 + (std::f32::consts::PI * p).cos())
}

/// Print banner with configuration details.
pub fn print_banner(cfg: &Config) {
    let opt_name = match cfg.opt_kind {
        OptKind::AdamW => "AdamW",
        OptKind::Muon => "Muon",
    };
    eprintln!("=== T-JEPA Hybrid Training ===");
    eprintln!("dim={} hidden={} enc_lr={} ntp_lr={} seed={} steps={}",
              DIM, HIDDEN, cfg.encoder_lr, cfg.ntp_lr, cfg.seed, cfg.steps);
    eprintln!("optimizer={} jepa={} nca={} jepa_warmup={}",
              opt_name, cfg.use_jepa, cfg.use_nca, cfg.jepa_warmup);
    eprintln!("L = {}*NTP + {}*JEPA + {}*NCA",
              cfg.ntp_weight, cfg.jepa_weight, cfg.nca_weight);
    eprintln!("trial_id={} agent_id={}", cfg.trial_id, cfg.agent_id);
    eprintln!("Champion: BPB 2.5193 | Gate-1: ≤2.22 | Gate-2: ≤2.03");
}

/// Print training results summary.
pub fn print_results(cfg: &Config, best_bpb: f32, elapsed: f64) {
    eprintln!("\n=== Training Complete ===");
    eprintln!("Steps={} Time={:.1}s best_val_bpb={:.4} vs_champion={:+.4}",
              cfg.steps, elapsed, best_bpb, best_bpb - 2.5193);
    println!("BPB={:.4}", best_bpb);
    if best_bpb <= 2.22 { eprintln!("Gate-1 PASSED (≤2.22)"); }
    else { eprintln!("Gate-1 FAILED: {:.4} > 2.22", best_bpb); }
    if best_bpb <= 2.03 { eprintln!("Gate-2 PASSED (≤2.03)"); }
    else { eprintln!("Gate-2 FAILED: {:.4} > 2.03", best_bpb); }
}

/// Update Neon database with trial heartbeat.
pub fn neon_heartbeat(cfg: &Config, step: usize, bpb: f32, last: &mut Instant) {
    if last.elapsed().as_secs() >= HEARTBEAT_INTERVAL_SECS {
        eprintln!(
            "NEON_SQL: INSERT INTO igla_agents_heartbeat (agent_id, machine_id, branch, task, status, last_heartbeat) VALUES ('{}', 'local', 'main', '{}', 'active', NOW()) ON CONFLICT (agent_id) DO UPDATE SET status=EXCLUDED.status, last_heartbeat=EXCLUDED.last_heartbeat;",
            cfg.agent_id, cfg.trial_id,
        );
        eprintln!("NEON_SQL: UPDATE igla_race_trials SET bpb_latest={:.4}, steps_done={} WHERE trial_id='{}';",
                  bpb, step, cfg.trial_id);
        *last = Instant::now();
    }
}

/// Mark trial as complete in Neon database.
pub fn neon_trial_complete(cfg: &Config, bpb: f32) {
    eprintln!(
        "NEON_SQL: UPDATE igla_race_trials SET bpb_final={:.4}, status='complete' WHERE trial_id='{}';",
        bpb, cfg.trial_id,
    );
}

/// Insert trial start into Neon database.
pub fn neon_trial_start(cfg: &Config) {
    let config_json = format!(
        "{{\"arch\":\"tjepa\",\"d_model\":{},\"lr\":{},\"seed\":{},\"optimizer\":\"{}\",\"ntp_w\":{},\"jepa_w\":{},\"nca_w\":{}}}",
        HIDDEN, cfg.encoder_lr, cfg.seed,
        match cfg.opt_kind { OptKind::AdamW => "adamw", OptKind::Muon => "muon" },
        cfg.ntp_weight, cfg.jepa_weight, cfg.nca_weight,
    );
    eprintln!(
        "NEON_SQL: INSERT INTO igla_race_trials (trial_id, config, status, agent_id, branch) VALUES ('{}', '{}', 'running', '{}', 'main');",
        cfg.trial_id, config_json, cfg.agent_id,
    );
}

/// Compute validation BPB on validation data.
///
/// # Returns
/// Validation BPB or `f32::MAX` if insufficient data.
pub fn evaluate(model: &NgramModel, tokens: &[usize]) -> f32 {
    assert!(!tokens.is_empty(), "evaluate: empty tokens");
    let mut total = 0.0f32;
    let mut n = 0usize;
    for c in (0..tokens.len()).step_by(SEQ + 1) {
        let end = (c + SEQ + 1).min(tokens.len());
        if end - c < NGRAM + 1 {
            continue;
        }
        let loss = model.loss_on_seq(&tokens[c..end]);
        if loss.is_finite() {
            total += loss / LN_2;
            n += 1;
        }
    }
    if n == 0 {
        return f32::MAX;
    }
    total / n as f32
}
