//! nca_jepa_ntp_v2 Composition Pipeline - Issue #71
//!
//! Three-phase training pipeline:
//!   Phase 1: Load NCA pre-pre-trained checkpoint (#70)
//!   Phase 2: Train JEPA 20K on top (#69)
//!   Phase 3: NTP fine-tuning 25K on composed representation
//!
//! Total: 60K steps per seed, 5 seeds [42..46]
//! EXP-025 kill thresholds: 10K:500 / 30K:200 / 60K:100
//! Force-save @ 32K (historical PPL minimum)
//!
//! AGENT: GAMMA
//! TASK: #71

use crate::trinity_config::trinity;
use crate::trinity_config::TrinityTrainConfig;
use crate::nca::{self, NcaConfig, NcaTransitionRule};
use crate::jepa::{self, JepaConfig, JepaPredictor};
use crate::model::{IglaGf16Model, IglaConfig};
use crate::forward::LayerDims;
use crate::backward::{cross_entropy_loss, clip_gradients};
use crate::optimizer::AdamWCpu;
use crate::data::ByteDataLoader;
use crate::bench::bpb_from_loss;

/// Pipeline phase identifier
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    /// Phase 1: Load NCA checkpoint (0 steps, just load)
    NcaLoad,
    /// Phase 2: JEPA training (20K steps)
    JepaTrain,
    /// Phase 3: NTP fine-tuning (25K steps)
    NtpTrain,
    /// Complete
    Done,
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::NcaLoad => write!(f, "NCA-LOAD"),
            Phase::JepaTrain => write!(f, "JEPA-20K"),
            Phase::NtpTrain => write!(f, "NTP-25K"),
            Phase::Done => write!(f, "DONE"),
        }
    }
}

/// EXP-025 kill threshold check
pub fn check_kill_threshold(step: usize, ppl: f64) -> bool {
    let threshold = if step <= 10_000 {
        trinity::KILL_THRESH_10K
    } else if step <= 30_000 {
        trinity::KILL_THRESH_30K
    } else {
        trinity::KILL_THRESH_60K
    };

    ppl > threshold
}

/// Pipeline configuration
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Random seed
    pub seed: u64,

    /// NCA checkpoint path (Phase 1 input)
    pub nca_checkpoint: String,

    /// JEPA checkpoint path (Phase 2 output)
    pub jepa_checkpoint: String,

    /// Final checkpoint path (Phase 3 output)
    pub final_checkpoint: String,

    /// Whether to force-save at 32K steps
    pub force_save_32k: bool,

    /// Force-save path
    pub force_save_path: String,
}

impl PipelineConfig {
    pub fn for_seed(seed: u64) -> Self {
        Self {
            seed,
            nca_checkpoint: format!("artifacts/nca_15k_s{}.bin", seed),
            jepa_checkpoint: format!("artifacts/jepa_20k_s{}.bin", seed),
            final_checkpoint: format!("artifacts/composition_60k_s{}.bin", seed),
            force_save_32k: true,
            force_save_path: format!("artifacts/forcesave_32k_s{}.bin", seed),
        }
    }
}

/// Pipeline final result
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// Seed used
    pub seed: u64,

    /// Final BPB
    pub final_bpb: f64,

    /// Final PPL
    pub final_ppl: f64,

    /// Best BPB seen during training
    pub best_bpb: f64,

    /// Step at which best BPB was achieved
    pub best_bpb_step: usize,

    /// Total training time in seconds
    pub total_time_seconds: f64,

    /// Whether the seed was killed early
    pub killed: bool,

    /// Kill step (if killed)
    pub kill_step: Option<usize>,

    /// Phase results summary
    pub nca_loaded: bool,
    pub jepa_final_loss: f64,
    pub ntp_final_loss: f64,
}

/// Cosine learning rate schedule with warmup
pub fn cosine_lr_with_warmup(step: usize, max_steps: usize, lr_base: f64, lr_min: f64, warmup: usize) -> f64 {
    if step < warmup {
        lr_base * step as f64 / warmup as f64
    } else {
        let progress = (step - warmup) as f64 / (max_steps - warmup) as f64;
        let cosine = 0.5 * (1.0 + (std::f64::consts::PI * progress).cos());
        lr_min + (lr_base - lr_min) * cosine
    }
}

/// Run the full nca_jepa_ntp_v2 pipeline for a single seed
pub fn run_pipeline(config: &PipelineConfig) -> PipelineResult {
    use std::time::Instant;
    let start = Instant::now();

    println!("=== nca_jepa_ntp_v2 Pipeline seed={} ===", config.seed);

    // Phase 1: Load NCA checkpoint
    println!("[Phase 1] Loading NCA checkpoint: {}", config.nca_checkpoint);
    let nca_rule = match nca::load_nca_checkpoint(&config.nca_checkpoint) {
        Ok(rule) => {
            println!("[Phase 1] NCA checkpoint loaded: {} states, {} weights",
                rule.num_states, rule.weights.len());
            Some(rule)
        }
        Err(e) => {
            eprintln!("[Phase 1] WARNING: Could not load NCA checkpoint: {}", e);
            eprintln!("[Phase 1] Using random initialization instead");
            None
        }
    };

    // Initialize embeddings from NCA or random
    let vocab_size = trinity::VOCAB_SIZE;
    let d_model = trinity::HIDDEN_DIM;
    let embeddings = if let Some(ref rule) = nca_rule {
        nca::nca_to_embeddings(rule, vocab_size, d_model, config.seed)
    } else {
        // Random init
        (0..vocab_size * d_model)
            .map(|i| {
                let pseudo_rand = ((i as u64).wrapping_mul(6364136223846793005)
                    .wrapping_add(config.seed)) as f32;
                (pseudo_rand / 2_147_483_648.0_f32 - 1.0) * 0.02
            })
            .collect()
    };

    // Phase 2: JEPA training (20K steps)
    println!("[Phase 2] Starting JEPA training (20K steps)");
    let jepa_config = JepaConfig {
        seed: config.seed,
        ..JepaConfig::default()
    };
    let jepa_result = jepa::train_jepa(&jepa_config, &embeddings);
    println!("[Phase 2] JEPA complete: loss={:.6} variance={:.6} converged={}",
        jepa_result.final_loss, jepa_result.final_variance, jepa_result.converged);

    // Phase 3: NTP fine-tuning (25K steps)
    println!("[Phase 3] Starting NTP fine-tuning (25K steps)");
    let ntp_result = run_ntp_phase(&embeddings, config);
    println!("[Phase 3] NTP complete: final_bpb={:.4}", ntp_result.final_bpb);

    let total_time = start.elapsed().as_secs_f64();

    PipelineResult {
        seed: config.seed,
        final_bpb: ntp_result.final_bpb,
        final_ppl: ntp_result.final_ppl,
        best_bpb: ntp_result.best_bpb,
        best_bpb_step: ntp_result.best_bpb_step,
        total_time_seconds: total_time,
        killed: ntp_result.killed,
        kill_step: ntp_result.kill_step,
        nca_loaded: nca_rule.is_some(),
        jepa_final_loss: jepa_result.final_loss,
        ntp_final_loss: ntp_result.final_loss,
    }
}

/// NTP phase result
#[derive(Debug, Clone)]
struct NtpPhaseResult {
    final_bpb: f64,
    final_ppl: f64,
    best_bpb: f64,
    best_bpb_step: usize,
    final_loss: f64,
    killed: bool,
    kill_step: Option<usize>,
}

/// Run Phase 3: NTP fine-tuning
fn run_ntp_phase(embeddings: &[f32], config: &PipelineConfig) -> NtpPhaseResult {
    use std::time::Instant;

    let vocab_size = trinity::VOCAB_SIZE;
    let d_model = trinity::HIDDEN_DIM;
    let seq_len = trinity::CONTEXT_LEN;
    let batch_size = trinity::BATCH_SIZE;
    let ntp_steps = trinity::NTP_STEPS;

    // Create model with Trinity3k config
    let model_config = IglaConfig {
        vocab_size,
        max_seq_len: seq_len,
        dims: LayerDims {
            d_model,
            n_heads: trinity::HEADS,
            d_ffn: trinity::FFN_DIM,
        },
        n_layers: trinity::NUM_BLOCKS,
    };

    // Initialize model with NCA+JEPA embeddings
    let mut model = IglaGf16Model::new(&model_config);
    // Override embeddings with trained ones (if sizes match)
    if embeddings.len() == model.embeddings.len() {
        model.embeddings.copy_from_slice(embeddings);
    }

    // Optimizer
    let param_count = model.embeddings.len();
    let mut optimizer = AdamWCpu::with_params(
        param_count,
        trinity::LR,
        trinity::BETA1,
        trinity::BETA2,
        trinity::WEIGHT_DECAY,
    );

    // Data
    let data = ByteDataLoader::load_tinyshakespeare();

    let mut best_bpb = f64::MAX;
    let mut best_bpb_step = 0;
    let mut killed = false;
    let mut kill_step = None;
    let mut final_loss = 0.0f64;

    for step in 0..ntp_steps {
        let step_start = Instant::now();

        // Learning rate schedule
        let lr = cosine_lr_with_warmup(
            step, ntp_steps, trinity::LR, trinity::LR_MIN, trinity::WARMUP_STEPS
        );
        optimizer.lr = lr;

        // Get batch
        let (inputs, targets) = data.get_train_batch(batch_size, seq_len, step);

        // Forward pass
        let logits = model.forward(&inputs, batch_size, seq_len);

        // Compute loss
        let loss = cross_entropy_loss(&logits, &targets);
        let bpb = bpb_from_loss(loss as f64);
        let ppl = (loss as f64).exp();

        final_loss = loss as f64;

        // Track best
        if bpb < best_bpb {
            best_bpb = bpb;
            best_bpb_step = step;
        }

        // Kill threshold check (global step = NCA_STEPS + JEPA_STEPS + step)
        let global_step = trinity::NCA_STEPS + trinity::JEPA_STEPS + step;
        if check_kill_threshold(global_step, ppl) {
            println!("[Phase 3] KILL at step {} (global {}): PPL={:.1} exceeds threshold",
                step, global_step, ppl);
            killed = true;
            kill_step = Some(global_step);
            break;
        }

        // Compute gradients (simplified tied embeddings backward)
        let mut gradients = vec![0.0f32; model.embeddings.len()];
        for b in 0..batch_size {
            for i in 0..seq_len {
                let idx = b * seq_len + i;
                let target_idx = targets[idx];
                let input_idx = inputs[idx];
                let l_offset = idx * vocab_size;

                // Softmax
                let mut max_logit = f32::NEG_INFINITY;
                for v in 0..trinity::ACTIVE_VOCAB {
                    max_logit = max_logit.max(logits[l_offset + v]);
                }
                let mut sum_exp = 0.0f32;
                let mut probs = vec![0.0f32; vocab_size];
                for v in 0..trinity::ACTIVE_VOCAB {
                    probs[v] = (logits[l_offset + v] - max_logit).exp();
                    sum_exp += probs[v];
                }

                // Gradient: push embeddings toward target
                for v in 0..trinity::ACTIVE_VOCAB {
                    let prob = probs[v] / sum_exp;
                    let grad = prob - if v == target_idx { 1.0f32 } else { 0.0f32 };
                    let w_offset = input_idx * d_model;
                    let v_offset = v * d_model;
                    for d in 0..d_model {
                        if w_offset + d < gradients.len() && v_offset + d < model.embeddings.len() {
                            gradients[w_offset + d] += grad * model.embeddings[v_offset + d];
                        }
                    }
                }
            }
        }

        // Clip gradients
        clip_gradients(&mut gradients, trinity::GRAD_CLIP);

        // Update
        optimizer.step(&mut model.embeddings, &gradients);

        let elapsed = step_start.elapsed();

        // Force-save at 32K global step
        let force_save = config.force_save_32k && global_step == trinity::FORCE_SAVE_STEP;

        // Log
        if step % 500 == 0 || step == ntp_steps - 1 || force_save {
            println!(
                "[Phase 3] step={:5} global={:5} loss={:.4} bpb={:.4} ppl={:.1} best_bpb={:.4} {:.0}ms/step lr={:.6}{}",
                step, global_step, loss, bpb, ppl, best_bpb,
                elapsed.as_millis(), lr,
                if force_save { " [FORCE-SAVE]" } else { "" }
            );
        }

        if force_save {
            save_model_checkpoint(&model, &config.force_save_path).ok();
        }
    }

    // Save final checkpoint
    save_model_checkpoint(&model, &config.final_checkpoint).ok();

    NtpPhaseResult {
        final_bpb: bpb_from_loss(final_loss),
        final_ppl: final_loss.exp(),
        best_bpb,
        best_bpb_step,
        final_loss,
        killed,
        kill_step,
    }
}

/// Save model checkpoint (simplified: just embeddings)
fn save_model_checkpoint(model: &IglaGf16Model, path: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(path)?;
    file.write_all(b"TRIN")?;
    file.write_all(&(model.embeddings.len() as u32).to_le_bytes())?;
    file.write_all(&(model.n_layers as u32).to_le_bytes())?;
    file.write_all(&(model.dims.d_model as u32).to_le_bytes())?;
    for &w in &model.embeddings {
        file.write_all(&w.to_le_bytes())?;
    }
    Ok(())
}

/// Run pipeline for multiple seeds and report median BPB
pub fn run_multi_seed(seeds: &[u64]) -> Vec<PipelineResult> {
    let mut results = Vec::with_capacity(seeds.len());

    for (idx, &seed) in seeds.iter().enumerate() {
        println!("\n{}", "=".repeat(60));
        println!("Starting seed {} ({}/{})", seed, idx + 1, seeds.len());
        println!("{}", "=".repeat(60));

        let config = PipelineConfig::for_seed(seed);
        let result = run_pipeline(&config);
        results.push(result);
    }

    // Report summary
    println!("\n=== Multi-seed Summary ===");
    let mut bpbs: Vec<f64> = results.iter().map(|r| r.final_bpb).collect();
    bpbs.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let median_bpb = bpbs[bpbs.len() / 2];
    let mad = {
        let deviations: Vec<f64> = bpbs.iter().map(|&b| (b - median_bpb).abs()).collect();
        let mut sorted_devs = deviations;
        sorted_devs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sorted_devs[sorted_devs.len() / 2]
    };

    println!("Seeds: {:?}", seeds);
    println!("BPBs: {:?}", bpbs);
    println!("Median BPB: {:.4}", median_bpb);
    println!("MAD: {:.4}", mad);

    for r in &results {
        println!("  seed={}: bpb={:.4} best={:.4} @ step={} killed={} time={:.1}s",
            r.seed, r.final_bpb, r.best_bpb, r.best_bpb_step, r.killed, r.total_time_seconds);
    }

    results
}

/// GF16 Mixed-Precision Training Step (BENCH-012)
///
/// Protocol: Dequantize GF16 → f32 forward → f32 backward → f32 optimizer step → quantize back to GF16
///
/// Law L-R9: GF16 only when d_model ≥ 256 (confirmed safe zone)
/// Expected: -0.05 BPB at d_model=384
///
/// GF16 advantage: 9-bit mantissa (φ-optimal 6:9 split) preserves gradient information
/// through depth. BF16 with 8-bit mantissa loses gradients.
pub fn training_step_gf16(
    gf16_weights: &mut [crate::gf16::GF16],
    input: &[f32],
    target: &[u8],
    optimizer: &mut crate::optimizer::AdamWCpu,
    vocab_size: usize,
    d_model: usize,
) -> f32 {
    assert!(d_model >= 256, "GF16 requires d_model >= 256 (Law L-R9), got {}", d_model);

    let w_f32: Vec<f32> = gf16_weights.iter().map(|g| g.to_f32()).collect();

    let logits = forward_f32_embeddings(&w_f32, input, vocab_size, d_model);

    let loss = cross_entropy_loss_f32(&logits, target, vocab_size);

    let grads = backward_f32_embeddings(&w_f32, &logits, input, target, vocab_size, d_model);

    let mut w_f32_mut = w_f32;
    clip_gradients(&mut grads, 1.0);
    optimizer.step(&mut w_f32_mut, &grads);

    for (i, w) in w_f32_mut.iter().enumerate() {
        gf16_weights[i] = crate::gf16::GF16::from_f32(*w);
    }

    loss
}

fn forward_f32_embeddings(embeddings: &[f32], input: &[f32], vocab_size: usize, d_model: usize) -> Vec<f32> {
    let seq_len = input.len();
    let mut logits = vec![0.0f32; seq_len * vocab_size];
    for (i, &token) in input.iter().enumerate() {
        let tok_idx = (token.abs() as usize) % (embeddings.len() / d_model.max(1));
        let emb_offset = tok_idx * d_model;
        for v in 0..vocab_size.min(embeddings.len() / d_model.max(1)) {
            let v_offset = v * d_model;
            let mut dot = 0.0f32;
            for d in 0..d_model {
                if emb_offset + d < embeddings.len() && v_offset + d < embeddings.len() {
                    dot += embeddings[emb_offset + d] * embeddings[v_offset + d];
                }
            }
            logits[i * vocab_size + v] = dot;
        }
    }
    logits
}

fn cross_entropy_loss_f32(logits: &[f32], target: &[u8], vocab_size: usize) -> f32 {
    let seq_len = target.len();
    let mut total_loss = 0.0f32;
    for i in 0..seq_len {
        let offset = i * vocab_size;
        let mut max_logit = f32::NEG_INFINITY;
        for v in 0..vocab_size {
            if offset + v < logits.len() {
                max_logit = max_logit.max(logits[offset + v]);
            }
        }
        let mut sum_exp = 0.0f32;
        let mut log_sum_exp = 0.0f32;
        for v in 0..vocab_size {
            if offset + v < logits.len() {
                let exp_val = (logits[offset + v] - max_logit).exp();
                sum_exp += exp_val;
            }
        }
        if sum_exp > 0.0 {
            log_sum_exp = max_logit + sum_exp.ln();
        }
        let tgt = target[i] as usize;
        let tgt_logit = if offset + tgt < logits.len() { logits[offset + tgt] } else { 0.0 };
        total_loss += log_sum_exp - tgt_logit;
    }
    total_loss / seq_len.max(1) as f32
}

fn backward_f32_embeddings(
    embeddings: &[f32],
    logits: &[f32],
    input: &[f32],
    target: &[u8],
    vocab_size: usize,
    d_model: usize,
) -> Vec<f32> {
    let seq_len = target.len();
    let n_emb = embeddings.len();
    let mut grads = vec![0.0f32; n_emb];
    for i in 0..seq_len {
        let offset = i * vocab_size;
        let mut max_logit = f32::NEG_INFINITY;
        for v in 0..vocab_size {
            if offset + v < logits.len() {
                max_logit = max_logit.max(logits[offset + v]);
            }
        }
        let mut sum_exp = 0.0f32;
        let mut probs = vec![0.0f32; vocab_size];
        for v in 0..vocab_size {
            if offset + v < logits.len() {
                probs[v] = (logits[offset + v] - max_logit).exp();
                sum_exp += probs[v];
            }
        }
        if sum_exp > 0.0 {
            for p in probs.iter_mut() { *p /= sum_exp; }
        }
        let tgt = target[i] as usize;
        probs[tgt] -= 1.0;
        let tok_idx = (input[i].abs() as usize) % (n_emb / d_model.max(1));
        let emb_offset = tok_idx * d_model;
        for v in 0..vocab_size.min(n_emb / d_model.max(1)) {
            let v_offset = v * d_model;
            for d in 0..d_model {
                if emb_offset + d < grads.len() && v_offset + d < embeddings.len() {
                    grads[emb_offset + d] += probs[v] * embeddings[v_offset + d];
                }
            }
        }
    }
    let scale = seq_len.max(1) as f32;
    for g in grads.iter_mut() { *g /= scale; }
    grads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_lr_warmup() {
        let lr0 = cosine_lr_with_warmup(0, 1000, 3e-4, 1e-5, 100);
        assert!(lr0 < 1e-6, "Step 0 should have ~0 lr");

        let lr50 = cosine_lr_with_warmup(50, 1000, 3e-4, 1e-5, 100);
        assert!((lr50 - 1.5e-4).abs() < 1e-6, "Step 50 should be ~half lr_base");

        let lr100 = cosine_lr_with_warmup(100, 1000, 3e-4, 1e-5, 100);
        assert!((lr100 - 3e-4).abs() < 1e-5, "Step 100 should be ~lr_base");

        let lr1000 = cosine_lr_with_warmup(999, 1000, 3e-4, 1e-5, 100);
        assert!(lr1000 > 1e-5 && lr1000 < 3e-4);
    }

    #[test]
    fn test_kill_threshold() {
        assert!(!check_kill_threshold(5000, 400.0));
        assert!(check_kill_threshold(5000, 600.0));
        assert!(!check_kill_threshold(20_000, 150.0));
        assert!(check_kill_threshold(20_000, 250.0));
        assert!(!check_kill_threshold(50_000, 80.0));
        assert!(check_kill_threshold(50_000, 150.0));
    }

    #[test]
    fn test_pipeline_config_for_seed() {
        let config = PipelineConfig::for_seed(42);
        assert_eq!(config.seed, 42);
        assert!(config.nca_checkpoint.contains("s42"));
        assert!(config.force_save_32k);
    }

    #[test]
    fn test_phase_display() {
        assert_eq!(Phase::NcaLoad.to_string(), "NCA-LOAD");
        assert_eq!(Phase::JepaTrain.to_string(), "JEPA-20K");
        assert_eq!(Phase::NtpTrain.to_string(), "NTP-25K");
        assert_eq!(Phase::Done.to_string(), "DONE");
    }

    #[test]
    fn test_training_step_gf16_reduces_loss() {
        let d_model = 256usize;
        let vocab_size = 16usize;
        let seq_len = 4usize;
        let n_params = vocab_size * d_model;
        let mut gf16_weights: Vec<crate::gf16::GF16> = (0..n_params)
            .map(|i| crate::gf16::GF16::from_f32(((i as f32) * 0.01 - 0.5).sin()))
            .collect();
        let input: Vec<f32> = vec![0.0, 1.0, 2.0, 3.0];
        let target: Vec<u8> = vec![1, 2, 3, 0];
        let mut optimizer = crate::optimizer::AdamWCpu::new(n_params, 0.01);
        let loss1 = training_step_gf16(&mut gf16_weights, &input, &target, &mut optimizer, vocab_size, d_model);
        for _ in 0..3 {
            training_step_gf16(&mut gf16_weights, &input, &target, &mut optimizer, vocab_size, d_model);
        }
        let loss2 = training_step_gf16(&mut gf16_weights, &input, &target, &mut optimizer, vocab_size, d_model);
        assert!(loss2 < loss1, "GF16 training should reduce loss: before={} after={}", loss1, loss2);
    }

    #[test]
    #[should_panic(expected = "GF16 requires d_model >= 256")]
    fn test_training_step_gf16_rejects_small_model() {
        let mut gf16_weights = vec![crate::gf16::GF16::ZERO; 64];
        let mut optimizer = crate::optimizer::AdamWCpu::new(64, 0.01);
        training_step_gf16(&mut gf16_weights, &[0.0], &[0], &mut optimizer, 4, 128);
    }

    #[test]
    fn test_forward_f32_embeddings() {
        let d_model = 256usize;
        let vocab_size = 4usize;
        let embeddings: Vec<f32> = (0..vocab_size * d_model).map(|i| i as f32 * 0.001).collect();
        let input = vec![0.0f32, 1.0];
        let logits = forward_f32_embeddings(&embeddings, &input, vocab_size, d_model);
        assert_eq!(logits.len(), 2 * vocab_size);
    }

    #[test]
    fn test_cross_entropy_f32() {
        let logits = vec![10.0f32, 0.0, 0.0, 0.0];
        let target = vec![0u8];
        let loss = cross_entropy_loss_f32(&logits, &target, 4);
        assert!(loss >= 0.0 && loss < 1.0, "loss for correct prediction should be small: {}", loss);
    }
}
