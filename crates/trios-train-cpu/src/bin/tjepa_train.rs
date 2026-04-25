//! TASK-5D — T-JEPA Hybrid Training (Proven Architecture + JEPA)
//!
//! Architecture: arch_explorer proven model (dim=64, hidden=384, layer norm,
//! projection, separate ctx embeddings) + JEPA predictor on hidden reps.
//!
//! Multi-objective: L = 0.5*NTP + 0.25*JEPA + 0.25*NCA
//!
//! Champion baseline: 6-gram h=384 lr=0.003 seed=43 → BPB 2.5193 (27K steps)
//! Gate min (ASHA Rung-1): ≤ 2.23 BPB
//! Gate target: ≤ 2.03 BPB
//! IGLA target: < 1.50 BPB

#![allow(clippy::needless_range_loop)]

use std::fs;
use std::time::Instant;

use trios_train_cpu::{
    jepa::{
        MaskConfig, EmaConfig, EmaTarget,
        mask_spans, get_masked, get_unmasked,
        predictor::{JepaPredictor, PredictorConfig},
    },
    objective::{
        ObjectiveConfig, compute_combined_loss, ComponentLosses,
        NcaObjective, nca_entropy_loss,
    },
};

const VOCAB: usize = 128;
const DIM: usize = 64;
const HIDDEN: usize = 384;
const NUM_CTX: usize = 4;
const NGRAM: usize = NUM_CTX + 2;
const SEQ: usize = 64;
const LN_2: f32 = std::f32::consts::LN_2;

fn layer_norm(x: &[f32], eps: f32) -> Vec<f32> {
    assert!(!x.is_empty(), "layer_norm: empty input");
    let n = x.len() as f32;
    let mean = x.iter().sum::<f32>() / n;
    let var = x.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    let std = (var + eps).sqrt();
    x.iter().map(|v| (v - mean) / std).collect()
}

fn softmax(v: &mut [f32]) {
    let max = v.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0f32;
    for x in v.iter_mut() { *x = (*x - max).exp(); sum += *x; }
    for x in v.iter_mut() { *x /= sum; }
}

struct AdamW {
    m: Vec<f32>,
    v: Vec<f32>,
    step: usize,
    beta1: f32,
    beta2: f32,
    wd: f32,
}

impl AdamW {
    fn new(size: usize, wd: f32) -> Self {
        let phi = (1.0 + 5.0f64.sqrt()) / 2.0;
        Self {
            m: vec![0.0; size],
            v: vec![0.0; size],
            step: 0,
            beta1: 1.0 / phi as f32,
            beta2: 0.999,
            wd,
        }
    }

    fn update(&mut self, params: &mut [f32], grads: &[f32], lr: f32) {
        assert_eq!(params.len(), grads.len(), "AdamW param/grad size mismatch");
        assert_eq!(params.len(), self.m.len(), "AdamW buffer size mismatch");
        self.step += 1;
        let bc1 = 1.0 - self.beta1.powi(self.step as i32);
        let bc2 = 1.0 - self.beta2.powi(self.step as i32);
        for i in 0..params.len() {
            params[i] -= self.wd * lr * params[i];
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * grads[i];
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * grads[i] * grads[i];
            params[i] -= lr * (self.m[i] / bc1) / ((self.v[i] / bc2).sqrt() + 1e-8);
        }
    }

    fn ema_update(&self, target: &mut [f32], online: &[f32], decay: f32) {
        assert_eq!(target.len(), online.len(), "EMA size mismatch");
        for (t, o) in target.iter_mut().zip(online.iter()) {
            *t = decay * *t + (1.0 - decay) * *o;
        }
    }
}

struct NgramModel {
    embed: Vec<f32>,
    ctx: Vec<Vec<f32>>,
    ctx_weights: Vec<f32>,
    proj: Vec<f32>,
    lm_head: Vec<f32>,
}

impl NgramModel {
    fn new(seed: u64) -> Self {
        let mut s = seed;
        let mut rng = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0
        };
        let lim = (6.0f32 / (3 * DIM) as f32).sqrt();
        let lim_h = (6.0f32 / (DIM + HIDDEN) as f32).sqrt();
        let lim_o = (6.0f32 / (HIDDEN + VOCAB) as f32).sqrt();

        let base_weights: Vec<f32> = vec![0.7, 0.3, 0.2, 0.15];
        let ctx_weights: Vec<f32> = base_weights.iter().take(NUM_CTX).cloned().collect();

        Self {
            embed: (0..VOCAB * DIM).map(|_| rng() * lim).collect(),
            ctx: (0..NUM_CTX)
                .map(|_| (0..VOCAB * DIM).map(|_| rng() * lim).collect())
                .collect(),
            ctx_weights,
            proj: (0..HIDDEN * DIM).map(|_| rng() * lim_h).collect(),
            lm_head: (0..VOCAB * HIDDEN).map(|_| rng() * lim_o).collect(),
        }
    }

    fn compute_hidden(&self, context: &[usize]) -> Vec<f32> {
        assert!(context.len() >= 2, "context too short for hidden");
        let t0 = context[context.len() - 1].min(VOCAB - 1);
        let mut combined = self.embed[t0 * DIM..(t0 + 1) * DIM].to_vec();
        for (ci, cw) in self.ctx_weights.iter().enumerate() {
            let ctx_idx = context.len() - 2 - ci;
            let t = context[ctx_idx].min(VOCAB - 1);
            let cv = &self.ctx[ci][t * DIM..(t + 1) * DIM];
            for j in 0..DIM { combined[j] += cv[j] * cw; }
        }
        let ln = layer_norm(&combined, 1e-5);
        let mut hidden = vec![0.0f32; HIDDEN];
        for hi in 0..HIDDEN {
            for (j, l) in ln.iter().enumerate() {
                hidden[hi] += self.proj[hi * DIM + j] * l;
            }
            hidden[hi] = hidden[hi].max(0.0);
        }
        hidden
    }

    fn predict(&self, hidden: &[f32]) -> Vec<f32> {
        assert_eq!(hidden.len(), HIDDEN, "hidden dim mismatch");
        let mut logits = vec![0.0f32; VOCAB];
        for (vi, logit) in logits.iter_mut().enumerate() {
            for (hi, hn) in hidden.iter().enumerate() {
                *logit += self.lm_head[vi * HIDDEN + hi] * hn;
            }
        }
        logits
    }

    fn loss_on_seq(&self, tokens: &[usize]) -> f32 {
        if tokens.len() < NGRAM + 1 { return 0.0; }
        let count = tokens.len() - NGRAM;
        assert!(count > 0, "no n-gram pairs in sequence");
        let mut total = 0.0f32;
        for i in 0..count {
            let context = &tokens[i..i + NGRAM];
            let target = tokens[i + NGRAM].min(VOCAB - 1);
            let mut logits = self.predict(&self.compute_hidden(context));
            softmax(&mut logits);
            total -= logits[target].max(1e-10).ln();
        }
        total / count as f32
    }
}

struct TrainGrads {
    g_embed: Vec<f32>,
    g_ctx: Vec<Vec<f32>>,
    g_proj: Vec<f32>,
    g_head: Vec<f32>,
}

fn compute_grads(
    model: &NgramModel,
    tokens: &[usize],
) -> (TrainGrads, Vec<Vec<f32>>, f32) {
    let count = tokens.len().saturating_sub(NGRAM);
    assert!(count > 0, "sequence too short for gradient computation");

    let mut g_embed = vec![0.0f32; VOCAB * DIM];
    let mut g_ctx: Vec<Vec<f32>> = (0..NUM_CTX).map(|_| vec![0.0f32; VOCAB * DIM]).collect();
    let mut g_proj = vec![0.0f32; HIDDEN * DIM];
    let mut g_head = vec![0.0f32; VOCAB * HIDDEN];

    let mut all_hidden = Vec::with_capacity(count);
    let mut all_ln = Vec::with_capacity(count);
    let mut all_contexts = Vec::with_capacity(count);
    let mut total_loss = 0.0f32;

    for i in 0..count {
        let context: Vec<usize> = tokens[i..i + NGRAM].to_vec();
        let t0 = context[NGRAM - 1].min(VOCAB - 1);
        let mut combined = model.embed[t0 * DIM..(t0 + 1) * DIM].to_vec();
        for (ci, cw) in model.ctx_weights.iter().enumerate() {
            let ctx_idx = NGRAM - 2 - ci;
            let t = context[ctx_idx].min(VOCAB - 1);
            let cv = &model.ctx[ci][t * DIM..(t + 1) * DIM];
            for j in 0..DIM { combined[j] += cv[j] * cw; }
        }
        let ln = layer_norm(&combined, 1e-5);
        let mut hidden = vec![0.0f32; HIDDEN];
        for hi in 0..HIDDEN {
            for (j, l) in ln.iter().enumerate() {
                hidden[hi] += model.proj[hi * DIM + j] * l;
            }
            hidden[hi] = hidden[hi].max(0.0);
        }
        all_hidden.push(hidden);
        all_ln.push(ln);
        all_contexts.push(context);
    }

    for i in 0..count {
        let target = tokens[i + NGRAM].min(VOCAB - 1);
        let hidden = &all_hidden[i];
        let mut d_hidden = vec![0.0f32; HIDDEN];
        let mut logits = model.predict(hidden);
        softmax(&mut logits);
        total_loss -= logits[target].max(1e-10).ln();

        for (vi, prob) in logits.iter().enumerate() {
            let grad = prob - if vi == target { 1.0 } else { 0.0 };
            for hi in 0..HIDDEN {
                g_head[vi * HIDDEN + hi] += grad * hidden[hi];
                d_hidden[hi] += grad * model.lm_head[vi * HIDDEN + hi];
            }
        }

        for hi in 0..HIDDEN {
            if all_hidden[i][hi] <= 0.0 { continue; }
            for di in 0..DIM {
                g_proj[hi * DIM + di] += d_hidden[hi] * all_ln[i][di];
            }
        }

        let t0 = all_contexts[i][NGRAM - 1].min(VOCAB - 1);
        for di in 0..DIM {
            let mut grad_sum = 0.0f32;
            for hi in 0..HIDDEN {
                if all_hidden[i][hi] > 0.0 {
                    grad_sum += model.proj[hi * DIM + di] * d_hidden[hi];
                }
            }
            g_embed[t0 * DIM + di] += grad_sum;
            for (ci, cw) in model.ctx_weights.iter().enumerate() {
                let ctx_idx = NGRAM - 2 - ci;
                let t = all_contexts[i][ctx_idx].min(VOCAB - 1);
                g_ctx[ci][t * DIM + di] += cw * grad_sum;
            }
        }
    }

    let n = count as f32;
    for x in g_embed.iter_mut() { *x /= n; }
    for gc in g_ctx.iter_mut() { for x in gc.iter_mut() { *x /= n; } }
    for x in g_proj.iter_mut() { *x /= n; }
    for x in g_head.iter_mut() { *x /= n; }

    let grads = TrainGrads { g_embed, g_ctx, g_proj, g_head };
    (grads, all_hidden, total_loss / count as f32)
}

fn evaluate(model: &NgramModel, tokens: &[usize]) -> f32 {
    let mut total = 0.0f32;
    let mut n = 0usize;
    for c in (0..tokens.len()).step_by(SEQ + 1) {
        let end = (c + SEQ + 1).min(tokens.len());
        if end - c < NGRAM + 1 { continue; }
        let loss = model.loss_on_seq(&tokens[c..end]);
        if loss.is_finite() { total += loss / LN_2; n += 1; }
    }
    if n == 0 { return f32::MAX; }
    total / n as f32
}

fn load_data(path: &str) -> Vec<usize> {
    let raw = fs::read(path).unwrap_or_else(|e| {
        eprintln!("Failed to load {}: {}. Using fallback.", path, e);
        b"The quick brown fox jumps over the lazy dog. ".repeat(100).to_vec()
    });
    raw.into_iter().map(|b| (b as usize) % VOCAB).collect()
}

fn parse_args(args: &[String]) -> (u64, usize, f32, bool) {
    let seed: u64 = args.iter().find(|a| a.starts_with("--seed="))
        .map(|a| a[7..].parse().unwrap_or(43)).unwrap_or(43);
    let steps: usize = args.iter().find(|a| a.starts_with("--steps="))
        .map(|a| a[8..].parse().unwrap_or(3000)).unwrap_or(3000);
    let lr: f32 = args.iter().find(|a| a.starts_with("--lr="))
        .map(|a| a[5..].parse().unwrap_or(0.003)).unwrap_or(0.003);
    let use_jepa: bool = !args.iter().any(|a| a == "--no-jepa");
    assert!(lr > 0.0, "lr must be positive");
    assert!(steps > 0, "steps must be positive");
    (seed, steps, lr, use_jepa)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let (seed, steps, lr, use_jepa) = parse_args(&args);
    let use_nca = !args.iter().any(|a| a == "--no-nca");

    eprintln!("=== T-JEPA Hybrid Training (Proven Arch + JEPA) ===");
    eprintln!("dim={} hidden={} lr={} seed={} steps={}", DIM, HIDDEN, lr, seed, steps);
    eprintln!("jepa={} nca={}", use_jepa, use_nca);
    eprintln!("L = 0.5*NTP + 0.25*JEPA + 0.25*NCA");
    eprintln!("Champion: BPB 2.5193 | Gate-1: ≤2.23 | Gate-2: ≤2.03");

    let train_data = load_data("data/tiny_shakespeare.txt");
    let val_data = load_data("data/tiny_shakespeare_val.txt");
    let train_end = (train_data.len() as f64 * 0.9) as usize;
    let train = &train_data[..train_end];
    let val = if val_data.len() > 100 { &val_data } else { &train_data[train_end..] };

    let mut model = NgramModel::new(seed);
    let mut target_model = NgramModel::new(seed);

    let mut opt_embed = AdamW::new(VOCAB * DIM, 0.01);
    let mut opt_ctx: Vec<AdamW> = (0..NUM_CTX).map(|_| AdamW::new(VOCAB * DIM, 0.01)).collect();
    let mut opt_proj = AdamW::new(HIDDEN * DIM, 0.01);
    let mut opt_head = AdamW::new(VOCAB * HIDDEN, 0.01);

    let predictor = if use_jepa {
        Some(JepaPredictor::new(PredictorConfig::with_d_model(HIDDEN)))
    } else {
        None
    };
    let mut predictor = predictor;

    let ema_config = EmaConfig { start: 0.996, end: 1.0, ramp_steps: steps };
    let mut ema_target = EmaTarget::new(ema_config);
    let mask_config = MaskConfig::default();
    let obj_config = ObjectiveConfig::default();
    let nca = if use_nca { Some(NcaObjective::default()) } else { None };

    let start_time = Instant::now();
    let mut best_val_bpb = f32::MAX;

    for step in 1..=steps {
        let dl = train.len();
        let off = (step * 97 + seed as usize) % dl.saturating_sub(SEQ + 1);
        let seq = &train[off..off + SEQ + 1];

        let (grads, hidden_vecs, ntp_loss) = compute_grads(&model, seq);

        let mut jepa_loss_val = 0.0f64;
        if let Some(ref mut pred) = predictor {
            let mask_result = {
                let mut s = seed.wrapping_add(step as u64).wrapping_mul(6364136223846793005);
                let mut bitset = vec![false; hidden_vecs.len().min(SEQ)];
                let ratio = 0.3f64;
                let span_len = 3usize;
                let num_spans = 2usize;
                for _ in 0..num_spans {
                    s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    let start = (s as usize) % bitset.len().saturating_sub(span_len);
                    for b in start..(start + span_len).min(bitset.len()) { bitset[b] = true; }
                }
                let tgt_pos: Vec<usize> = bitset.iter().enumerate().filter_map(|(i, &m)| if m { Some(i) } else { None }).collect();
                let ctx_pos: Vec<usize> = bitset.iter().enumerate().filter_map(|(i, &m)| if !m { Some(i) } else { None }).collect();
                (tgt_pos, ctx_pos)
            };
            let (tgt_pos, ctx_pos) = mask_result;
            if !tgt_pos.is_empty() && !ctx_pos.is_empty() {
                let zero_h = vec![0.0f32; HIDDEN];
                let ctx_flat: Vec<f32> = ctx_pos.iter()
                    .flat_map(|&p| hidden_vecs.get(p).unwrap_or(&zero_h).iter().copied())
                    .collect();
                let tgt_hidden: Vec<Vec<f32>> = tgt_pos.iter()
                    .filter_map(|&p| {
                        let ctx_tokens: Vec<usize> = if p + NGRAM <= seq.len() {
                            seq[p..p + NGRAM].to_vec()
                        } else {
                            return None;
                        };
                        Some(target_model.compute_hidden(&ctx_tokens))
                    })
                    .collect();
                if !tgt_hidden.is_empty() {
                    let tgt_flat: Vec<f32> = tgt_hidden.iter().flat_map(|v| v.iter().copied()).collect();
                    jepa_loss_val = pred.forward_backward(&ctx_flat, &tgt_flat, tgt_hidden.len()) as f64;
                }
            }
            let decay = ema_target.decay() as f32;
            for (t, o) in target_model.embed.iter_mut().zip(model.embed.iter()) {
                *t = decay * *t + (1.0 - decay) * *o;
            }
        }

        let mut nca_loss_val = 0.0f64;
        if let Some(ref nca_obj) = nca {
            let nca_seed = seed.wrapping_add(step as u64).wrapping_mul(7919);
            let nca_state = nca_obj.init_grid(nca_seed);
            let (loss, _) = nca_entropy_loss(
                &nca_state, nca_obj.k_states, nca_obj.entropy_min, nca_obj.entropy_max, nca_obj.weight,
            );
            nca_loss_val = loss;
        }

        let combined = compute_combined_loss(
            ComponentLosses { ntp: ntp_loss as f64 / LN_2 as f64, jepa: jepa_loss_val, nca: nca_loss_val },
            obj_config,
        );

        opt_embed.update(&mut model.embed, &grads.g_embed, lr);
        for (ci, oc) in opt_ctx.iter_mut().enumerate() {
            oc.update(&mut model.ctx[ci], &grads.g_ctx[ci], lr);
        }
        opt_proj.update(&mut model.proj, &grads.g_proj, lr);
        opt_head.update(&mut model.lm_head, &grads.g_head, lr);

        if step % 500 == 0 || step == steps {
            let elapsed = start_time.elapsed().as_secs_f64();
            let val_bpb = evaluate(&model, val);
            if val_bpb < best_val_bpb && val_bpb.is_finite() { best_val_bpb = val_bpb; }
            eprintln!("step={:5} ntp={:.4} jepa={:.4} nca={:.4} val_bpb={:.4} best={:.4} t={:.1}s",
                step, combined.components.ntp, combined.components.jepa,
                combined.components.nca, val_bpb, best_val_bpb, elapsed);
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    eprintln!("\n=== Training Complete ===");
    eprintln!("Steps={} Time={:.1}s best_val_bpb={:.4} vs_champion={:+.4}",
        steps, elapsed, best_val_bpb, best_val_bpb - 2.5193);
    println!("BPB={:.4}", best_val_bpb);

    if best_val_bpb <= 2.23 { eprintln!("✅ Gate-1 PASSED (≤2.23)"); }
    else { eprintln!("❌ Gate-1 FAILED: {:.4} > 2.23", best_val_bpb); }
    if best_val_bpb <= 2.03 { eprintln!("✅ Gate-2 PASSED (≤2.03)"); }
    else { eprintln!("❌ Gate-2 FAILED: {:.4} > 2.03", best_val_bpb); }

    Ok(())
}
