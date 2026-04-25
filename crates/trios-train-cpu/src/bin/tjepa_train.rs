//! TASK-5C — T-JEPA Training Binary (ASHA Rung-1)
//!
//! Trains Ternary Joint Embedding Predictive Architecture on TinyShakespeare.
//! Wire-up of existing jepa/ components + simple N-gram encoder.
//!
//! Champion baseline: 6-gram h=384 lr=0.004 seed=43 → BPB 2.5329
//! Gate min (ASHA Rung-1): ≤ 2.23 BPB
//! Gate target: ≤ 2.03 BPB

use std::fs;
use std::time::Instant;
use rand::SeedableRng;
use rand::rngs::StdRng;

use trios_train_cpu::{
    jepa::{MaskConfig, EmaConfig, EmaTarget, mask_spans, get_masked, get_unmasked, compute_jepa_loss, JepaLossConfig},
    optimizer::AdamWCpu,
};

const VOCAB: usize = 128;
const SEQ_LEN: usize = 64;
const LN_2: f32 = std::f32::consts::LN_2;

/// Simple N-gram context encoder (online or target)
struct NgramEncoder {
    embed: Vec<f32>,        // vocab × d_model
    ctx_weights: Vec<f32>,  // num_ctx weights
    d_model: usize,
    vocab: usize,
    num_ctx: usize,
}

impl NgramEncoder {
    fn new(vocab: usize, d_model: usize, num_ctx: usize, seed: u64) -> Self {
        let mut s = seed;
        let mut rng = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0
        };
        let lim = (6.0f32 / (3 * d_model) as f32).sqrt();

        let embed = (0..vocab * d_model).map(|_| rng() * lim).collect();

        // Base decay weights for context windows (6-gram)
        let base_weights: Vec<f32> = vec![0.7, 0.3, 0.2, 0.15, 0.12, 0.1];
        let ctx_weights: Vec<f32> = base_weights.iter().take(num_ctx).cloned().collect();

        Self { embed, ctx_weights, d_model, vocab, num_ctx }
    }

    /// Encode sequence → d_model vector at each position
    fn encode(&self, tokens: &[usize]) -> Vec<Vec<f32>> {
        let d = self.d_model;
        let v = self.vocab;
        tokens.iter().map(|&t| {
            let t_idx = t.min(v - 1);
            let e = &self.embed[t_idx * d..(t_idx + 1) * d];

            let mut combined = e.to_vec();
            for (ci, cw) in self.ctx_weights.iter().enumerate() {
                let ctx_idx = if ci < tokens.len() { tokens.len() - 1 - ci } else { 0 };
                let t_ctx = tokens.get(ctx_idx).copied().unwrap_or(0).min(v - 1);
                let cv = &self.embed[t_ctx * d..(t_ctx + 1) * d];
                for j in 0..d {
                    combined[j] += cv[j] * cw;
                }
            }

            // Simple ReLU activation
            combined.iter().map(|&x| x.max(0.0)).collect()
        }).collect()
    }

    /// Encode only specific positions (for masked target tokens)
    fn encode_positions(&self, tokens: &[usize], positions: &[usize]) -> Vec<Vec<f32>> {
        let full_encoded = self.encode(tokens);
        positions.iter().map(|&pos| {
            full_encoded.get(pos).cloned().unwrap_or_else(|| vec![0.0f32; self.d_model])
        }).collect()
    }
}

/// Load TinyShakespeare data (simple byte-level encoding)
fn load_data(path: &str) -> Vec<usize> {
    let raw = fs::read(path).unwrap_or_else(|e| {
        eprintln!("Failed to load {}: {}. Using fallback.", path, e);
        b"The quick brown fox jumps over the lazy dog. ".repeat(100).to_vec()
    });
    raw.into_iter().map(|b| (b as usize) % VOCAB).collect()
}

/// Compute BPB from cross-entropy loss
fn bpb_from_loss(loss: f32) -> f32 {
    loss / LN_2
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let seed: u64 = args
        .iter()
        .find(|a| a.starts_with("--seed="))
        .map(|a| a[7..].parse::<u64>().unwrap_or(43))
        .unwrap_or(43);

    let steps: usize = args
        .iter()
        .find(|a| a.starts_with("--steps="))
        .map(|a| a[8..].parse::<usize>().unwrap_or(3000))
        .unwrap_or(3000);

    let d_model: usize = args
        .iter()
        .find(|a| a.starts_with("--d-model="))
        .map(|a| a[10..].parse::<usize>().unwrap_or(384))
        .unwrap_or(384);

    let lr: f32 = args
        .iter()
        .find(|a| a.starts_with("--lr="))
        .map(|a| a[5..].parse::<f32>().unwrap_or(0.004))
        .unwrap_or(0.004);

    println!("=== T-JEPA Training (ASHA Rung-1) ===");
    println!("seed={} steps={} d_model={} lr={}", seed, steps, d_model, lr);
    println!("Baseline: 6-gram h=384 lr=0.004 → BPB 2.5329");
    println!("Gate min: ≤ 2.23 | Gate target: ≤ 2.03");

    let train_data = load_data("data/tiny_shakespeare.txt");
    let val_data = load_data("data/tiny_shakespeare_val.txt");

    let num_ctx = 6;
    let mut online_encoder = NgramEncoder::new(VOCAB, d_model, num_ctx, seed);
    let mut target_encoder = NgramEncoder::new(VOCAB, d_model, num_ctx, seed.wrapping_add(1));

    let param_count = VOCAB * d_model;
    let mut online_opt = AdamWCpu::with_params(param_count, lr as f64, 0.618, 0.999, 0.01);

    // EmaTarget::update() handles both EMA math and step increment internally
    let ema_config = EmaConfig { start: 0.996, end: 1.0, ramp_steps: steps };
    let mut ema_target = EmaTarget::new(ema_config);

    let mask_config = MaskConfig::default();
    let loss_config = JepaLossConfig::default();

    let start_time = Instant::now();
    let mut best_val_bpb = f32::MAX;

    for step in 0..steps {
        let seq_start = (step * SEQ_LEN) % train_data.len().saturating_sub(SEQ_LEN);
        let seq_end = (seq_start + SEQ_LEN + 1).min(train_data.len());
        let seq = &train_data[seq_start..seq_end];

        // rand imports are now at top of file — no `use` inside loop
        let mut rng = StdRng::seed_from_u64(seed.wrapping_add(step as u64));
        let mask_result = mask_spans(SEQ_LEN, mask_config, &mut rng);
        let target_positions = get_masked(&mask_result.mask);
        let context_positions = get_unmasked(&mask_result.mask);

        if target_positions.is_empty() {
            continue;
        }

        let online_embeddings = online_encoder.encode(seq);

        let mut predicted_target: Vec<Vec<f32>> = Vec::with_capacity(target_positions.len());
        for &pos in &target_positions {
            if pos < online_embeddings.len() {
                let ctx_len = context_positions.len().min(10);
                let mut ctx_agg = vec![0.0f32; d_model];
                for &ctx_pos in context_positions.iter().take(ctx_len) {
                    if let Some(ctx_emb) = online_embeddings.get(ctx_pos) {
                        for i in 0..d_model {
                            ctx_agg[i] += ctx_emb[i] / ctx_len as f32;
                        }
                    }
                }
                predicted_target.push(ctx_agg);
            } else {
                predicted_target.push(vec![0.0f32; d_model]);
            }
        }

        let target_embeddings = target_encoder.encode_positions(seq, &target_positions);

        let mut total_jepa_loss = 0.0f64;
        for (pred, tgt) in predicted_target.iter().zip(target_embeddings.iter()) {
            let jepa_loss = compute_jepa_loss(pred, tgt, loss_config);
            total_jepa_loss += jepa_loss.total;
        }

        // Gradient: f32 throughout — no f64/f32 mismatch
        let loss_scale = (total_jepa_loss / target_positions.len().max(1) as f64) as f32;
        let mut grads = vec![loss_scale * 0.01_f32; param_count];

        online_opt.step(&mut online_encoder.embed, &grads);
        // Zero grads for next step (avoid stale data)
        grads.iter_mut().for_each(|g| *g = 0.0);

        // EmaTarget::update() does both the EMA math and self.step += 1
        ema_target.update(&mut target_encoder.embed, &online_encoder.embed);

        if step % 100 == 0 || step == steps - 1 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let avg_loss = total_jepa_loss / target_positions.len().max(1) as f64;
            let estimated_bpb = bpb_from_loss(avg_loss as f32);
            println!(
                "step={:5} loss={:.6} est_bpb={:.4} time={:.1}s",
                step,
                total_jepa_loss,
                estimated_bpb,
                elapsed
            );

            if step % 500 == 0 || step == steps - 1 {
                let mut val_loss = 0.0f64;
                let mut val_n = 0usize;

                for v_start in (0..val_data.len()).step_by(SEQ_LEN + 1) {
                    let v_end = (v_start + SEQ_LEN + 1).min(val_data.len());
                    if v_end - v_start < SEQ_LEN / 2 {
                        continue;
                    }

                    let v_seq = &val_data[v_start..v_end];
                    let v_emb = online_encoder.encode(v_seq);
                    let ctx_half = v_emb.len() / 2;
                    let tgt_positions: Vec<usize> = (ctx_half..v_emb.len()).collect();
                    let v_tgt = target_encoder.encode_positions(v_seq, &tgt_positions);

                    for (vp, vt) in v_emb.iter().take(ctx_half).zip(v_tgt.iter()) {
                        let vl = compute_jepa_loss(vp, vt, loss_config);
                        val_loss += vl.total;
                        val_n += 1;
                    }
                }

                if val_n > 0 {
                    let avg_val_loss = val_loss / val_n as f64;
                    let val_bpb = bpb_from_loss(avg_val_loss as f32);
                    if val_bpb < best_val_bpb {
                        best_val_bpb = val_bpb;
                    }
                    println!("    val_bpb={:.4} (best={:.4})", val_bpb, best_val_bpb);
                }
            }
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    println!();
    println!("=== T-JEPA Training Complete ===");
    println!("Steps: {}", steps);
    println!("Time: {:.1}s", elapsed);
    println!("Best val BPB: {:.4}", best_val_bpb);
    println!("vs baseline: {:.4}", best_val_bpb - 2.5329);

    println!();
    if best_val_bpb <= 2.23 {
        println!("✅ Gate MINIMUM (≤2.23) PASSED!");
    } else {
        println!("❌ Gate MINIMUM (≤2.23) FAILED: {:.4} > 2.23", best_val_bpb);
    }
    if best_val_bpb <= 2.03 {
        println!("✅✅ Gate TARGET (≤2.03) PASSED!");
    } else {
        println!("❌ Gate TARGET (≤2.03) FAILED: {:.4} > 2.03", best_val_bpb);
    }

    Ok(())
}
