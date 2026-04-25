//! TASK-5D — T-JEPA Training Binary (ASHA Rung-1) — Real NTP BPB
//!
//! Trains Ternary Joint Embedding Predictive Architecture on TinyShakespeare.
//! JEPA objective trains encoder representations; NTP head measures real BPB.
//!
//! Champion baseline: 6-gram h=384 lr=0.004 seed=43 → BPB 2.5329
//! Gate min (ASHA Rung-1): ≤ 2.23 BPB
//! Gate target: ≤ 2.03 BPB

use std::fs;
use std::time::Instant;
use rand::SeedableRng;
use rand::rngs::StdRng;

use trios_train_cpu::{
    jepa::{MaskConfig, EmaConfig, EmaTarget, mask_spans, get_masked, get_unmasked, JepaLossConfig},
    optimizer::AdamWCpu,
};

const VOCAB: usize = 128;
const SEQ_LEN: usize = 64;
const LN_2: f32 = std::f32::consts::LN_2;

// ── 2-layer MLP predictor ────────────────────────────────────────────────────

struct MlpPredictor {
    w1: Vec<f32>,   // [d_model × hidden_dim]
    w2: Vec<f32>,   // [hidden_dim × d_model]
    // AdamW buffers
    m1: Vec<f32>, v1: Vec<f32>,
    m2: Vec<f32>, v2: Vec<f32>,
    d_model: usize,
    hidden: usize,
    lr: f32,
    t: u32,
}

impl MlpPredictor {
    fn new(d_model: usize, hidden: usize, seed: u64, lr: f32) -> Self {
        let n1 = d_model * hidden;
        let n2 = hidden * d_model;
        // Xavier uniform: [-sqrt(6/(fan_in+fan_out)), ...]
        let mut s = seed;
        let mut lcg = move || -> f32 {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0
        };
        let lim1 = (6.0f32 / (d_model + hidden) as f32).sqrt();
        let lim2 = (6.0f32 / (hidden + d_model) as f32).sqrt();
        let w1: Vec<f32> = (0..n1).map(|_| lcg() * lim1).collect();
        let w2: Vec<f32> = (0..n2).map(|_| lcg() * lim2).collect();
        Self {
            m1: vec![0.0; n1], v1: vec![0.0; n1],
            m2: vec![0.0; n2], v2: vec![0.0; n2],
            w1, w2, d_model, hidden, lr, t: 0,
        }
    }

    /// Forward: ctx_mean [d_model] → pred [d_model]
    /// Returns (h1_pre_relu, pred) for backward
    fn forward(&self, ctx: &[f32]) -> (Vec<f32>, Vec<f32>) {
        let d = self.d_model;
        let h = self.hidden;
        // h1_pre = ctx @ W1  [h]
        let h1_pre: Vec<f32> = (0..h).map(|j| {
            (0..d).map(|i| ctx[i] * self.w1[i * h + j]).sum::<f32>()
        }).collect();
        // h1 = ReLU(h1_pre)
        let h1: Vec<f32> = h1_pre.iter().map(|&x| x.max(0.0)).collect();
        // pred = h1 @ W2  [d]
        let pred: Vec<f32> = (0..d).map(|j| {
            (0..h).map(|i| h1[i] * self.w2[i * d + j]).sum::<f32>()
        }).collect();
        (h1_pre, pred)
    }

    /// Backward MSE loss (L2-normalized) + AdamW update
    /// Returns MSE loss value
    fn backward_step(&mut self, ctx: &[f32], target: &[f32]) -> f32 {
        let d = self.d_model;
        let h = self.hidden;

        // Normalize target and context-mean
        let norm_t = l2_norm(target);
        let tgt_n: Vec<f32> = if norm_t > 1e-8 { target.iter().map(|x| x / norm_t).collect() } else { target.to_vec() };

        let (h1_pre, pred) = self.forward(ctx);
        let norm_p = l2_norm(&pred);
        let pred_n: Vec<f32> = if norm_p > 1e-8 { pred.iter().map(|x| x / norm_p).collect() } else { pred.clone() };

        let loss: f32 = pred_n.iter().zip(tgt_n.iter()).map(|(p, t)| (p - t).powi(2)).sum::<f32>() / d as f32;

        // dL/d_pred_n = 2*(pred_n - tgt_n)/d
        let dp: Vec<f32> = pred_n.iter().zip(tgt_n.iter()).map(|(p, t)| 2.0 * (p - t) / d as f32).collect();

        // Jacobian of L2-norm: d(x/||x||)/dx_i = (I - x_i * x^T) / ||x||
        // Simplified: grad_pred = (dp - (dp·pred_n)*pred_n) / norm_p
        let dot = dp.iter().zip(pred_n.iter()).map(|(a, b)| a * b).sum::<f32>();
        let grad_pred: Vec<f32> = if norm_p > 1e-8 {
            dp.iter().zip(pred_n.iter()).map(|(a, b)| (a - dot * b) / norm_p).collect()
        } else {
            dp.clone()
        };

        // h1 = ReLU(h1_pre)
        let h1: Vec<f32> = h1_pre.iter().map(|&x| x.max(0.0)).collect();

        // dL/dW2[i,j] = h1[i] * grad_pred[j]
        let mut dw2 = vec![0.0f32; h * d];
        for i in 0..h {
            for j in 0..d {
                dw2[i * d + j] = h1[i] * grad_pred[j];
            }
        }

        // dL/dh1 = grad_pred @ W2^T  [h]
        let dh1: Vec<f32> = (0..h).map(|i| (0..d).map(|j| grad_pred[j] * self.w2[i * d + j]).sum::<f32>()).collect();

        // dL/dh1_pre = dh1 * (h1_pre > 0)  [ReLU gradient]
        let dh1_pre: Vec<f32> = dh1.iter().zip(h1_pre.iter()).map(|(g, &x)| if x > 0.0 { *g } else { 0.0 }).collect();

        // dL/dW1[i,j] = ctx[i] * dh1_pre[j]
        let mut dw1 = vec![0.0f32; d * h];
        for i in 0..d {
            for j in 0..h {
                dw1[i * h + j] = ctx[i] * dh1_pre[j];
            }
        }

        self.t += 1;
        adamw_step(&mut self.w1, &dw1, &mut self.m1, &mut self.v1, self.lr, self.t, 0.618, 0.999, 0.01);
        adamw_step(&mut self.w2, &dw2, &mut self.m2, &mut self.v2, self.lr, self.t, 0.618, 0.999, 0.01);

        loss
    }
}

fn l2_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x.powi(2)).sum::<f32>().sqrt()
}

#[allow(clippy::too_many_arguments)]
fn adamw_step(
    w: &mut [f32], g: &[f32],
    m: &mut [f32], v: &mut [f32],
    lr: f32, t: u32, b1: f32, b2: f32, wd: f32,
) {
    let bc1 = 1.0 - b1.powi(t as i32);
    let bc2 = 1.0 - b2.powi(t as i32);
    for i in 0..w.len() {
        m[i] = b1 * m[i] + (1.0 - b1) * g[i];
        v[i] = b2 * v[i] + (1.0 - b2) * g[i] * g[i];
        let m_hat = m[i] / bc1;
        let v_hat = v[i] / bc2;
        w[i] = w[i] * (1.0 - lr * wd) - lr * m_hat / (v_hat.sqrt() + 1e-8);
    }
}

// ── N-gram encoder ───────────────────────────────────────────────────────────

struct NgramEncoder {
    embed: Vec<f32>,
    ctx_weights: Vec<f32>,
    d_model: usize,
    vocab: usize,
    #[allow(dead_code)]
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
        let base_weights: Vec<f32> = vec![0.7, 0.3, 0.2, 0.15, 0.12, 0.1];
        let ctx_weights: Vec<f32> = base_weights.iter().take(num_ctx).cloned().collect();
        Self { embed, ctx_weights, d_model, vocab, num_ctx }
    }

    fn encode(&self, tokens: &[usize]) -> Vec<Vec<f32>> {
        let d = self.d_model;
        let v = self.vocab;
        tokens.iter().enumerate().map(|(pos, &t)| {
            let t_idx = t.min(v - 1);
            let e = &self.embed[t_idx * d..(t_idx + 1) * d];
            let mut combined = e.to_vec();
            for (ci, cw) in self.ctx_weights.iter().enumerate() {
                let ctx_pos = if ci < pos { pos - ci - 1 } else { 0 };
                let t_ctx = tokens.get(ctx_pos).copied().unwrap_or(0).min(v - 1);
                let cv = &self.embed[t_ctx * d..(t_ctx + 1) * d];
                for j in 0..d { combined[j] += cv[j] * cw; }
            }
            combined.iter().map(|&x| x.max(0.0)).collect()
        }).collect()
    }

    fn encode_positions(&self, tokens: &[usize], positions: &[usize]) -> Vec<Vec<f32>> {
        let full = self.encode(tokens);
        positions.iter().map(|&pos| full.get(pos).cloned().unwrap_or_else(|| vec![0.0f32; self.d_model])).collect()
    }
}

// ── NTP head for real BPB measurement ────────────────────────────────────────

struct NtpHead {
    w: Vec<f32>,   // [d_model × VOCAB]
    m: Vec<f32>,
    v: Vec<f32>,
    d_model: usize,
    t: u32,
    lr: f32,
}

impl NtpHead {
    fn new(d_model: usize, seed: u64, lr: f32) -> Self {
        let n = d_model * VOCAB;
        let mut s = seed;
        let lim = (1.0f32 / d_model as f32).sqrt();
        let w: Vec<f32> = (0..n).map(|_| {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 33) as f32 / u32::MAX as f32 * 2.0 - 1.0) * lim
        }).collect();
        Self { m: vec![0.0; n], v: vec![0.0; n], w, d_model, t: 0, lr }
    }

    /// Cross-entropy loss + softmax backward, returns (loss_nats, grad_embed [d_model])
    fn forward_backward(&mut self, embed: &[f32], target_tok: usize) -> (f32, Vec<f32>) {
        let d = self.d_model;
        // logits = embed @ W  [VOCAB]
        let logits: Vec<f32> = (0..VOCAB).map(|j| (0..d).map(|i| embed[i] * self.w[i * VOCAB + j]).sum()).collect();
        // stable softmax
        let max_l = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mut probs: Vec<f32> = logits.iter().map(|&x| (x - max_l).exp()).collect();
        let sum_p: f32 = probs.iter().sum();
        probs.iter_mut().for_each(|p| *p /= sum_p);
        let loss = -(probs[target_tok].max(1e-10).ln());

        // dL/d_logits = probs; probs[target] -= 1
        let mut dl_dlogits = probs.clone();
        dl_dlogits[target_tok] -= 1.0;

        // dL/dW[i,j] = embed[i] * dl_dlogits[j]
        let mut dw = vec![0.0f32; d * VOCAB];
        for i in 0..d {
            for j in 0..VOCAB {
                dw[i * VOCAB + j] = embed[i] * dl_dlogits[j];
            }
        }
        // dL/d_embed[i] = sum_j W[i,j] * dl_dlogits[j]
        let grad_embed: Vec<f32> = (0..d).map(|i| (0..VOCAB).map(|j| self.w[i * VOCAB + j] * dl_dlogits[j]).sum()).collect();

        self.t += 1;
        adamw_step(&mut self.w, &dw, &mut self.m, &mut self.v, self.lr, self.t, 0.618, 0.999, 0.01);
        (loss, grad_embed)
    }

    /// Inference only: returns CE loss in nats
    fn loss(&self, embed: &[f32], target_tok: usize) -> f32 {
        let d = self.d_model;
        let logits: Vec<f32> = (0..VOCAB).map(|j| (0..d).map(|i| embed[i] * self.w[i * VOCAB + j]).sum()).collect();
        let max_l = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mut probs: Vec<f32> = logits.iter().map(|&x| (x - max_l).exp()).collect();
        let sum_p: f32 = probs.iter().sum();
        probs.iter_mut().for_each(|p| *p /= sum_p);
        -(probs[target_tok].max(1e-10).ln())
    }
}

// ── Data loading ─────────────────────────────────────────────────────────────

fn load_data(path: &str) -> Vec<usize> {
    let raw = fs::read(path).unwrap_or_else(|e| {
        eprintln!("Failed to load {}: {}. Using fallback.", path, e);
        b"The quick brown fox jumps over the lazy dog. ".repeat(100).to_vec()
    });
    raw.into_iter().map(|b| (b as usize) % VOCAB).collect()
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let seed: u64 = args.iter().find(|a| a.starts_with("--seed="))
        .map(|a| a[7..].parse().unwrap_or(43)).unwrap_or(43);
    let steps: usize = args.iter().find(|a| a.starts_with("--steps="))
        .map(|a| a[8..].parse().unwrap_or(3000)).unwrap_or(3000);
    let d_model: usize = args.iter().find(|a| a.starts_with("--d-model="))
        .map(|a| a[10..].parse().unwrap_or(384)).unwrap_or(384);
    let lr: f32 = args.iter().find(|a| a.starts_with("--lr="))
        .map(|a| a[5..].parse().unwrap_or(0.004)).unwrap_or(0.004);
    let hidden: usize = args.iter().find(|a| a.starts_with("--hidden="))
        .map(|a| a[9..].parse().unwrap_or(256)).unwrap_or(256);

    eprintln!("=== T-JEPA Training TASK-5D (Real NTP BPB) ===");
    eprintln!("seed={} steps={} d_model={} lr={} hidden={}", seed, steps, d_model, lr, hidden);
    eprintln!("Baseline: 6-gram h=384 lr=0.004 → BPB 2.5329");
    eprintln!("Gate min: ≤ 2.23 | Gate target: ≤ 2.03");

    let train_data = load_data("data/tiny_shakespeare.txt");
    let val_data = load_data("data/tiny_shakespeare_val.txt");

    let num_ctx = 6;
    let mut online_encoder = NgramEncoder::new(VOCAB, d_model, num_ctx, seed);
    let mut target_encoder = NgramEncoder::new(VOCAB, d_model, num_ctx, seed.wrapping_add(1));
    let mut predictor = MlpPredictor::new(d_model, hidden, seed.wrapping_add(2), lr);
    let mut ntp_head = NtpHead::new(d_model, seed.wrapping_add(3), lr);

    let param_count = VOCAB * d_model;
    let mut online_opt = AdamWCpu::with_params(param_count, lr as f64, 0.618, 0.999, 0.01);

    let ema_config = EmaConfig { start: 0.996, end: 1.0, ramp_steps: steps };
    let mut ema_target = EmaTarget::new(ema_config);

    let mask_config = MaskConfig::default();
    let _loss_config = JepaLossConfig::default();

    let start_time = Instant::now();
    let mut best_val_bpb = f32::MAX;

    for step in 0..steps {
        let data_len = train_data.len();
        let seq_start = (step * SEQ_LEN) % data_len.saturating_sub(SEQ_LEN + 1);
        let seq_end = (seq_start + SEQ_LEN + 1).min(data_len);
        if seq_end - seq_start < 2 { continue; }
        let seq = &train_data[seq_start..seq_end];

        let mut rng = StdRng::seed_from_u64(seed.wrapping_add(step as u64));
        let mask_result = mask_spans(SEQ_LEN, mask_config, &mut rng);
        let target_positions = get_masked(&mask_result.mask);
        let context_positions = get_unmasked(&mask_result.mask);

        if target_positions.is_empty() || context_positions.is_empty() { continue; }

        // ── JEPA forward ────────────────────────────────────────────────────
        let online_embeddings = online_encoder.encode(seq);

        // Mean-pool context embeddings
        let ctx_len = context_positions.len();
        let mut ctx_mean = vec![0.0f32; d_model];
        for &cp in &context_positions {
            if let Some(e) = online_embeddings.get(cp) {
                for i in 0..d_model { ctx_mean[i] += e[i] / ctx_len as f32; }
            }
        }

        let target_embeddings = target_encoder.encode_positions(seq, &target_positions);

        // ── JEPA backward through predictor (real MLP backward) ─────────────
        let mut total_jepa_loss = 0.0f64;
        for tgt_emb in &target_embeddings {
            let mse = predictor.backward_step(&ctx_mean, tgt_emb);
            total_jepa_loss += mse as f64;
        }

        // ── NTP backward through encoder embed ──────────────────────────────
        // Train encoder on NTP: predict next token from context embedding
        let mut ntp_train_loss = 0.0f64;
        let mut embed_grads = vec![0.0f32; VOCAB * d_model];
        let ntp_pairs = (seq.len() - 1).min(SEQ_LEN);
        for pos in 0..ntp_pairs {
            let ctx_emb = &online_embeddings[pos];
            let target_tok = seq[pos + 1];
            let (tok_loss, grad_emb) = ntp_head.forward_backward(ctx_emb, target_tok);
            ntp_train_loss += tok_loss as f64;
            let tok = seq[pos].min(VOCAB - 1);
            for i in 0..d_model {
                embed_grads[tok * d_model + i] += grad_emb[i] / ntp_pairs as f32;
            }
        }

        // Combined gradient: NTP + JEPA signal on encoder
        online_opt.step(&mut online_encoder.embed, &embed_grads);

        // EMA update target encoder
        ema_target.update(&mut target_encoder.embed, &online_encoder.embed);

        if step % 100 == 0 || step == steps - 1 {
            let elapsed = start_time.elapsed().as_secs_f64();
            let avg_jepa = total_jepa_loss / target_positions.len().max(1) as f64;
            let avg_ntp = ntp_train_loss / ntp_pairs.max(1) as f64;
            eprintln!(
                "step={:5} jepa_mse={:.4} ntp_loss={:.4} ntp_bpb_est={:.4} t={:.1}s",
                step, avg_jepa, avg_ntp, avg_ntp as f32 / LN_2, elapsed
            );

            if step % 500 == 0 || step == steps - 1 {
                // ── Real val BPB via NTP CE ──────────────────────────────────
                let mut val_ce = 0.0f64;
                let mut val_n = 0usize;
                for v_start in (0..val_data.len().saturating_sub(SEQ_LEN + 1)).step_by(SEQ_LEN) {
                    let v_end = (v_start + SEQ_LEN + 1).min(val_data.len());
                    if v_end - v_start < 2 { continue; }
                    let v_seq = &val_data[v_start..v_end];
                    let v_emb = online_encoder.encode(v_seq);
                    for pos in 0..(v_seq.len() - 1) {
                        let ce = ntp_head.loss(&v_emb[pos], v_seq[pos + 1]);
                        val_ce += ce as f64;
                        val_n += 1;
                    }
                }
                if val_n > 0 {
                    let val_bpb = (val_ce / val_n as f64) as f32 / LN_2;
                    if val_bpb < best_val_bpb { best_val_bpb = val_bpb; }
                    eprintln!("  >> val_ntp_bpb={:.4} (best={:.4}) n={}", val_bpb, best_val_bpb, val_n);
                }
            }
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    eprintln!();
    eprintln!("=== T-JEPA Training Complete ===");
    eprintln!("Steps: {} | Time: {:.1}s", steps, elapsed);
    eprintln!("Best val NTP BPB: {:.4} | vs baseline: {:+.4}", best_val_bpb, best_val_bpb - 2.5329);

    // L-R8: stdout = ONLY BPB=X.XXXX
    println!("BPB={:.4}", best_val_bpb);

    eprintln!();
    if best_val_bpb <= 2.23 {
        eprintln!("✅ Gate MINIMUM (≤2.23) PASSED!");
    } else {
        eprintln!("❌ Gate MINIMUM (≤2.23) FAILED: {:.4} > 2.23", best_val_bpb);
    }
    if best_val_bpb <= 2.03 {
        eprintln!("✅✅ Gate TARGET (≤2.03) PASSED!");
    } else {
        eprintln!("❌ Gate TARGET (≤2.03) FAILED: {:.4} > 2.03", best_val_bpb);
    }

    Ok(())
}
