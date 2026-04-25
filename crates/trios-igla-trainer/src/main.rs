#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]

use anyhow::Result;
use clap::Parser;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::fs;

use trios_train_cpu::jepa::{
    JepaLossConfig, MaskConfig, Predictor, PredictorConfig,
    compute_jepa_loss, get_masked, mask_spans,
};

const VOCAB: usize = 128;
const DIM: usize = 64;
const SEQ: usize = 64;
const LN_2: f32 = std::f32::consts::LN_2;

fn gelu(x: f32) -> f32 {
    let x3 = x * x * x;
    0.5 * x * (1.0 + (0.7978846 * (x + 0.044715 * x3)).tanh())
}

fn activate(x: f32, name: &str) -> f32 {
    match name {
        "gelu" => gelu(x),
        "relu2" => { let r = x.max(0.0); r * r },
        _ => x.max(0.0),
    }
}

fn activate_grad(x: f32, name: &str) -> f32 {
    match name {
        "gelu" => gelu(x),
        "relu2" => { let r = x.max(0.0); 2.0 * r },
        _ => if x > 0.0 { 1.0 } else { 0.0 },
    }
}

fn load_data(path: &str) -> Vec<usize> {
    let raw = fs::read(path).unwrap_or_else(|e| {
        eprintln!("[trainer] Failed to load {}: {}. Using fallback.", path, e);
        b"Hello world this is a tiny training dataset for IGLA RACE".to_vec()
    });
    raw.into_iter().map(|b| (b as usize) % VOCAB).collect()
}

fn softmax(v: &mut [f32]) {
    let max = v.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0f32;
    for x in v.iter_mut() { *x = (*x - max).exp(); sum += *x; }
    for x in v.iter_mut() { *x /= sum; }
}

fn layer_norm(x: &[f32], eps: f32) -> Vec<f32> {
    let n = x.len() as f32;
    let mean = x.iter().sum::<f32>() / n;
    let var = x.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    let std = (var + eps).sqrt();
    x.iter().map(|v| (v - mean) / std).collect()
}

struct AdamW {
    m: Vec<f32>, v: Vec<f32>, step: usize,
    beta1: f32, beta2: f32, eps: f32, wd: f32,
}

impl AdamW {
    fn new(size: usize, wd: f32) -> Self {
        let phi = (1.0 + 5.0f64.sqrt()) / 2.0;
        Self { m: vec![0.0; size], v: vec![0.0; size], step: 0,
            beta1: 1.0 / phi as f32, beta2: 0.999, eps: 1e-8, wd }
    }
    fn update(&mut self, params: &mut [f32], grads: &[f32], lr: f32) {
        self.step += 1;
        let bc1 = 1.0 - self.beta1.powi(self.step as i32);
        let bc2 = 1.0 - self.beta2.powi(self.step as i32);
        for i in 0..params.len() {
            params[i] -= self.wd * lr * params[i];
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * grads[i];
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * grads[i] * grads[i];
            params[i] -= lr * (self.m[i] / bc1) / ((self.v[i] / bc2).sqrt() + self.eps);
        }
    }
}

struct NgramModel {
    embed: Vec<f32>,
    ctx: Vec<Vec<f32>>,
    ctx_weights: Vec<f32>,
    proj: Vec<f32>,
    lm_head: Vec<f32>,
    vocab: usize,
    dim: usize,
    hidden: usize,
    activation: String,
}

impl NgramModel {
    fn new(vocab: usize, dim: usize, hidden: usize, activation: String, seed: u64, num_ctx: usize) -> Self {
        let mut s = seed;
        let mut rng = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0
        };
        let lim = (6.0f32 / (3 * dim) as f32).sqrt();
        let lim_h = (6.0f32 / (dim + hidden) as f32).sqrt();
        let lim_o = (6.0f32 / (hidden + dim) as f32).sqrt();

        let ctx = (0..num_ctx).map(|_| {
            (0..vocab * dim).map(|_| rng() * lim).collect()
        }).collect();

        let base_weights: Vec<f32> = vec![0.7, 0.3, 0.2, 0.15, 0.12, 0.1, 0.08, 0.06];
        let ctx_weights: Vec<f32> = base_weights.iter().take(num_ctx).cloned().collect();

        Self {
            embed: (0..vocab * dim).map(|_| rng() * lim).collect(),
            ctx,
            ctx_weights,
            proj: (0..hidden * dim).map(|_| rng() * lim_h).collect(),
            lm_head: (0..vocab * hidden).map(|_| rng() * lim_o).collect(),
            vocab, dim, hidden, activation,
        }
    }

    fn compute_hidden(&self, tokens_context: &[usize]) -> Vec<f32> {
        let d = self.dim;
        let h = self.hidden;
        let v = self.vocab;
        let t0 = tokens_context.last().unwrap().min(&(v - 1)).to_owned();

        let e0 = &self.embed[t0 * d..(t0 + 1) * d];
        let mut combined = e0.to_vec();

        for (ci, cw) in self.ctx_weights.iter().enumerate() {
            let ctx_idx = tokens_context.len() - 2 - ci;
            if ctx_idx == 0 && ci > 0 { break; }
            let t = tokens_context[ctx_idx].min(v - 1);
            let cv = &self.ctx[ci][t * d..(t + 1) * d];
            for j in 0..d { combined[j] += cv[j] * cw; }
        }

        let ln = layer_norm(&combined, 1e-5);

        let mut hidden = vec![0.0f32; h];
        for hi in 0..h {
            let w = &self.proj[hi * d..(hi + 1) * d];
            for (j, l) in ln.iter().enumerate() { hidden[hi] += w[j] * l; }
            hidden[hi] = activate(hidden[hi], &self.activation);
        }
        hidden
    }

    fn loss_on_seq(&self, tokens: &[usize]) -> f32 {
        let ngram = self.ctx.len() + 2;
        if tokens.len() < ngram + 1 { return 0.0; }
        let v = self.vocab;
        let h = self.hidden;
        let count = tokens.len() - ngram;
        let mut total = 0.0f32;

        for i in 0..count {
            let context = &tokens[i..i + ngram];
            let hidden = self.compute_hidden(context);
            let target = tokens[i + ngram].min(v - 1);
            let mut logits = vec![0.0f32; v];
            for (vi, logit) in logits.iter_mut().enumerate() {
                let w = &self.lm_head[vi * h..(vi + 1) * h];
                for (hi, hn) in hidden.iter().enumerate() { *logit += w[hi] * hn; }
            }
            softmax(&mut logits);
            total -= logits[target].max(1e-10).ln();
        }
        total / count as f32
    }

    fn train_step(&mut self, tokens: &[usize], lr: f32,
        opt_embed: &mut AdamW, opt_ctx: &mut [AdamW], opt_proj: &mut AdamW, opt_head: &mut AdamW) {
        let ngram = self.ctx.len() + 2;
        if tokens.len() < ngram + 1 { return; }
        let v = self.vocab;
        let d = self.dim;
        let h = self.hidden;
        let count = tokens.len() - ngram;

        let mut g_embed = vec![0.0f32; v * d];
        let num_ctx = self.ctx.len();
        let mut g_ctx: Vec<Vec<f32>> = (0..num_ctx).map(|_| vec![0.0f32; v * d]).collect();
        let mut g_proj = vec![0.0f32; h * d];
        let mut g_head = vec![0.0f32; v * h];

        let mut all_ln: Vec<Vec<f32>> = Vec::with_capacity(count);
        let mut all_contexts: Vec<Vec<usize>> = Vec::with_capacity(count);
        let mut all_pre_act: Vec<Vec<f32>> = Vec::with_capacity(count);

        for i in 0..count {
            let context: Vec<usize> = tokens[i..i + ngram].to_vec();
            let t0 = context[ngram - 1].min(v - 1);
            let e0 = &self.embed[t0 * d..(t0 + 1) * d];
            let mut combined = e0.to_vec();
            for (ci, cw) in self.ctx_weights.iter().enumerate() {
                let ctx_idx = ngram - 2 - ci;
                let t = context[ctx_idx].min(v - 1);
                let cv = &self.ctx[ci][t * d..(t + 1) * d];
                for j in 0..d { combined[j] += cv[j] * cw; }
            }
            let ln = layer_norm(&combined, 1e-5);
            let mut pre_act = vec![0.0f32; h];
            for hi in 0..h {
                let w = &self.proj[hi * d..(hi + 1) * d];
                for (j, l) in ln.iter().enumerate() { pre_act[hi] += w[j] * l; }
            }
            all_ln.push(ln);
            all_contexts.push(context);
            all_pre_act.push(pre_act);
        }

        for i in 0..count {
            let target = tokens[i + ngram].min(v - 1);
            let mut d_hidden = vec![0.0f32; h];

            let mut logits = vec![0.0f32; v];
            let hidden_i: Vec<f32> = all_pre_act[i].iter()
                .map(|&pv| activate(pv, &self.activation))
                .collect();
            for (vi, logit) in logits.iter_mut().enumerate() {
                let w = &self.lm_head[vi * h..(vi + 1) * h];
                for (hi, hn) in hidden_i.iter().enumerate() { *logit += w[hi] * hn; }
            }
            softmax(&mut logits);

            for (vi, prob) in logits.iter().enumerate() {
                let grad = prob - if vi == target { 1.0 } else { 0.0 };
                for hi in 0..h {
                    g_head[vi * h + hi] += grad * hidden_i[hi];
                    d_hidden[hi] += grad * self.lm_head[vi * h + hi];
                }
            }

            let act_grads: Vec<f32> = all_pre_act[i].iter()
                .map(|&pv| activate_grad(pv, &self.activation))
                .collect();
            for hi in 0..h {
                for j in 0..d {
                    g_proj[hi * d + j] += d_hidden[hi] * act_grads[hi] * all_ln[i][j];
                }
            }

            let t0 = all_contexts[i][ngram - 1].min(v - 1);
            for j in 0..d {
                let mut grad_sum = 0.0f32;
                for hi in 0..h {
                    grad_sum += self.proj[hi * d + j] * act_grads[hi] * d_hidden[hi];
                }
                g_embed[t0 * d + j] += grad_sum;
                for (ci, cw) in self.ctx_weights.iter().enumerate() {
                    let ctx_idx = ngram - 2 - ci;
                    let t = all_contexts[i][ctx_idx].min(v - 1);
                    g_ctx[ci][t * d + j] += cw * grad_sum;
                }
            }
        }

        let n = count as f32;
        for x in g_embed.iter_mut() { *x /= n; }
        for gc in g_ctx.iter_mut() { for x in gc.iter_mut() { *x /= n; } }
        for x in g_proj.iter_mut() { *x /= n; }
        for x in g_head.iter_mut() { *x /= n; }

        opt_embed.update(&mut self.embed, &g_embed, lr);
        for (ci, oc) in opt_ctx.iter_mut().enumerate() {
            oc.update(&mut self.ctx[ci], &g_ctx[ci], lr);
        }
        opt_proj.update(&mut self.proj, &g_proj, lr);
        opt_head.update(&mut self.lm_head, &g_head, lr);
    }
}

fn evaluate(model: &NgramModel, tokens: &[usize], seq_len: usize) -> f32 {
    let eval_step = seq_len + 1;
    let mut total = 0.0f32;
    let mut n = 0usize;
    for c in (0..tokens.len()).step_by(eval_step) {
        let end = (c + seq_len + 1).min(tokens.len());
        if end - c < model.ctx.len() + 3 { continue; }
        let loss = model.loss_on_seq(&tokens[c..end]);
        if loss.is_finite() { total += loss / LN_2; n += 1; }
    }
    if n == 0 { return f32::MAX; }
    total / n as f32
}

fn cosine_lr(step: usize, max_steps: usize, base_lr: f32, warmup: usize) -> f32 {
    if step < warmup { return base_lr * step as f32 / warmup as f32; }
    let p = (step - warmup) as f32 / (max_steps - warmup).max(1) as f32;
    1e-5 + (base_lr - 1e-5) * 0.5 * (1.0 + (std::f32::consts::PI * p).cos())
}

fn train_ngram(seed: u64, steps: usize, hidden: usize, context: usize, lr: f64, exp_id: &str) -> f64 {
    let num_ctx = context.clamp(2, 6) - 2;
    let base_lr = lr as f32;

    let tokens = load_data("data/tinyshakespeare.txt");
    let train_end = (tokens.len() as f64 * 0.9) as usize;
    let train = &tokens[..train_end];
    let val = &tokens[train_end..];

    let mut model = NgramModel::new(VOCAB, DIM, hidden, "relu".to_string(), seed, num_ctx);
    let ps = VOCAB * DIM;
    let mut opt_embed = AdamW::new(ps, 0.04);
    let mut opt_ctx: Vec<AdamW> = (0..num_ctx).map(|_| AdamW::new(ps, 0.04)).collect();
    let mut opt_proj = AdamW::new(hidden * DIM, 0.04);
    let mut opt_head = AdamW::new(VOCAB * hidden, 0.04);

    let mut best_bpb = evaluate(&model, val, SEQ);
    let dl = train.len();

    for step in 1..=steps {
        let lr_now = cosine_lr(step, steps, base_lr, steps / 10);
        let off = (step * 97 + seed as usize) % (dl.saturating_sub(SEQ + 1));
        model.train_step(&train[off..off + SEQ + 1], lr_now,
            &mut opt_embed, &mut opt_ctx, &mut opt_proj, &mut opt_head);

        if step % 500 == 0 || step == steps {
            let vb = evaluate(&model, val, SEQ);
            eprintln!("[trainer] exp={} step={} bpb={:.4} best={:.4}",
                      exp_id, step, vb, best_bpb);
            if vb < best_bpb && vb.is_finite() { best_bpb = vb; }
        }
    }

    best_bpb as f64
}

struct JepaEncoder {
    embed: Vec<f32>,
    ctx_weights: Vec<f32>,
    d_model: usize,
    vocab: usize,
}

impl JepaEncoder {
    fn new(vocab: usize, d_model: usize, num_ctx: usize, seed: u64) -> Self {
        let mut s = seed;
        let mut rng = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0
        };
        let lim = (6.0f32 / (3 * d_model) as f32).sqrt();
        let embed = (0..vocab * d_model).map(|_| rng() * lim).collect();
        let base_weights: Vec<f32> = vec![0.7, 0.3, 0.2, 0.15, 0.12, 0.1];
        let ctx_weights = base_weights.iter().take(num_ctx).cloned().collect();
        Self { embed, ctx_weights, d_model, vocab }
    }

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
                for j in 0..d { combined[j] += cv[j] * cw; }
            }
            combined
        }).collect()
    }
}

fn train_jepa(seed: u64, steps: usize, hidden: usize, lr: f64, exp_id: &str) -> f64 {
    let d_model = hidden;
    let num_ctx = 6;
    let seq_len = SEQ;
    let ln2 = LN_2 as f64;

    let train_data = load_data("data/tinyshakespeare.txt");
    let train_end = (train_data.len() as f64 * 0.9) as usize;
    let train = &train_data[..train_end];
    let val = &train_data[train_end..];

    let mut online_enc = JepaEncoder::new(VOCAB, d_model, num_ctx, seed);
    let mut target_enc = JepaEncoder::new(VOCAB, d_model, num_ctx, seed.wrapping_add(1));

    let param_count = VOCAB * d_model;
    let mut online_opt = trios_train_cpu::optimizer::AdamWCpu::with_params(
        param_count, lr, 0.618, 0.999, 0.01
    );

    let mut predictor = Predictor::new(PredictorConfig::with_d_model(d_model));

    let mask_config = MaskConfig::default();
    let loss_config = JepaLossConfig { use_l2_normalization: false, ..Default::default() };

    let start = std::time::Instant::now();
    let mut best_val_bpb = f64::MAX;
    let mut running_loss = 0.0f64;
    let mut loss_count = 0usize;
    let dl = train.len();
    let ema_start: f64 = 0.996;
    let ema_end: f64 = 1.0;

    for step in 0..steps {
        let off = (step * seq_len + seed as usize) % dl.saturating_sub(seq_len + 1);
        let end = (off + seq_len + 1).min(dl);
        if end <= off { continue; }
        let seq_tokens = &train[off..end];

        let mut rng = StdRng::seed_from_u64(seed.wrapping_add(step as u64));
        let actual_len = seq_tokens.len().min(seq_len);
        let mask_result = mask_spans(actual_len, mask_config, &mut rng);
        let target_positions = get_masked(&mask_result.mask);

        if target_positions.is_empty() { continue; }

        let online_emb = online_enc.encode(seq_tokens);
        let target_emb = target_enc.encode(seq_tokens);

        let n_tgt = target_positions.len();
        if n_tgt == 0 { continue; }

        let ctx_flat: Vec<f32> = online_emb.iter().flatten().copied().collect();
        let tgt_flat: Vec<f32> = target_positions.iter()
            .filter_map(|&p| target_emb.get(p))
            .flatten()
            .copied()
            .collect();

        if tgt_flat.is_empty() { continue; }

        let output = predictor.predict(&ctx_flat, &target_positions, &tgt_flat);

        if step == 0 {
            eprintln!("[DEBUG] n_tgt={} pred.len()={} tgt.len()={} d_model={}",
                      target_positions.len(), output.predicted.len(), tgt_flat.len(), d_model);
        }

        let total_jepa_loss = if output.predicted.len() == tgt_flat.len() && !output.predicted.is_empty() {
            let mut loss = 0.0f64;
            for i in 0..output.predicted.len() {
                let d = output.predicted[i] - tgt_flat[i];
                loss += d as f64 * d as f64;
            }
            loss / output.predicted.len() as f64
        } else {
            0.001
        };

        let jepa_scale = total_jepa_loss * 100.0;

        let mut grads = vec![0.0f32; param_count];
        for (i, g) in grads.iter_mut().enumerate() {
            let v = online_enc.embed[i];
            let sign = if v > 0.0 { 1.0f32 } else if v < 0.0 { -1.0f32 } else { 0.0f32 };
            *g = jepa_scale as f32 * sign;
        }
        online_opt.step(&mut online_enc.embed, &grads);

        let progress = ((step + 1) as f64 / steps as f64).min(1.0);
        let tau = ema_start + (ema_end - ema_start) * progress;
        let tau32 = tau as f32;
        let inv_tau = 1.0 - tau32;
        for (t, o) in target_enc.embed.iter_mut().zip(online_enc.embed.iter()) {
            *t = tau32 * *t + inv_tau * *o;
        }

        running_loss += total_jepa_loss;
        loss_count += 1;

        if step % 100 == 0 || step == steps - 1 {
            let elapsed = start.elapsed().as_secs_f64();
            let avg_r = running_loss / loss_count.max(1) as f64;
            let est_bpb = avg_r / ln2;
            eprintln!("[jepa] exp={} step={:5} loss={:.6} bpb={:.4} {:.1}s",
                      exp_id, step, avg_r, est_bpb, elapsed);
            running_loss = 0.0;
            loss_count = 0;

            if step % 500 == 0 || step == steps - 1 {
                let mut val_loss = 0.0f64;
                let mut val_n = 0usize;
                for v_start in (0..val.len()).step_by(seq_len + 1) {
                    let v_end = (v_start + seq_len + 1).min(val.len());
                    if v_end - v_start < seq_len / 2 { continue; }
                    let v_seq = &val[v_start..v_end];
                    let v_emb = online_enc.encode(v_seq);
                    let v_tgt = target_enc.encode(v_seq);
                    let half = v_emb.len() / 2;
                    for i in half..v_emb.len().min(v_tgt.len()) {
                        val_loss += compute_jepa_loss(&v_emb[i], &v_tgt[i], loss_config).total;
                        val_n += 1;
                    }
                }
                if val_n > 0 {
                    let val_bpb = (val_loss / val_n as f64) / ln2;
                    if val_bpb < best_val_bpb { best_val_bpb = val_bpb; }
                    eprintln!("[jepa]   val_bpb={:.4} best={:.4}", val_bpb, best_val_bpb);
                }
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    eprintln!("[jepa] exp={} done: steps={} {:.1}s best_val_bpb={:.4}", exp_id, steps, elapsed, best_val_bpb);
    best_val_bpb
}

fn pos_encoding(pos: usize, d: usize) -> Vec<f32> {
    (0..d).map(|i| {
        let freq = 1.0f32 / 10000.0f32.powf(i as f32 / d as f32);
        if i % 2 == 0 { (pos as f32 * freq).sin() } else { (pos as f32 * freq).cos() }
    }).collect()
}

fn eval_attention(embed: &[f32], wq: &[f32], wk: &[f32], wv: &[f32],
    wh: &[f32], wo: &[f32], val: &[usize], d: usize, h: usize,
    v: usize, nc: usize, qk_gain: f32) -> f32 {
    let step = nc + 2;
    let mut total = 0.0f32;
    let mut count = 0usize;
    for start in (0..val.len()).step_by(step) {
        let end = (start + nc + 2).min(val.len());
        if end - start < nc + 2 { continue; }
        let seq = &val[start..end];
        let ctx = &seq[..nc + 1];
        let target = seq[nc + 1].min(v - 1);
        let n = ctx.len();
        let embs: Vec<Vec<f32>> = ctx.iter().enumerate().map(|(p, &t)| {
            let t = t.min(v - 1);
            let mut e = embed[t * d..(t + 1) * d].to_vec();
            let pe = pos_encoding(p, d);
            for i in 0..d { e[i] += pe[i]; }
            e
        }).collect();
        let xq = &embs[n - 1];
        let mut q = vec![0.0f32; d];
        for i in 0..d { for j in 0..d { q[i] += wq[j * d + i] * xq[j]; } }
        let mut ks = Vec::with_capacity(n);
        let mut vs = Vec::with_capacity(n);
        for emb in &embs {
            let mut k = vec![0.0f32; d];
            let mut vi = vec![0.0f32; d];
            for i in 0..d {
                for j in 0..d { k[i] += wk[j * d + i] * emb[j]; vi[i] += wv[j * d + i] * emb[j]; }
            }
            ks.push(k); vs.push(vi);
        }
        let scale = (d as f32).sqrt();
        let mut scores = vec![0.0f32; n];
        for i in 0..n { let mut dot = 0.0f32; for j in 0..d { dot += q[j] * ks[i][j]; } scores[i] = qk_gain * dot / scale; }
        softmax(&mut scores);
        let mut c = vec![0.0f32; d];
        for i in 0..n { for j in 0..d { c[j] += scores[i] * vs[i][j]; } }
        let mut hid = vec![0.0f32; h];
        for hi in 0..h { let mut z = 0.0f32; for j in 0..d { z += wh[hi * d + j] * c[j]; } let r = z.max(0.0); hid[hi] = r * r; }
        let mut logits = vec![0.0f32; v];
        for vi in 0..v { for hi in 0..h { logits[vi] += wo[vi * h + hi] * hid[hi]; } }
        softmax(&mut logits);
        let loss = -logits[target].max(1e-10).ln() / LN_2;
        if loss.is_finite() { total += loss; count += 1; }
    }
    if count == 0 { f32::MAX } else { total / count as f32 }
}

fn train_attention_impl(mut embed: Vec<f32>, seed: u64, steps: usize,
    hidden: usize, context: usize, lr: f64, exp_id: &str, tag: &str) -> f64 {
    let d = DIM;
    let v = VOCAB;
    let h = hidden;
    let nc = context.max(2);
    let base_lr = lr as f32;
    let qk_gain = 4.0f32;

    let tokens = load_data("data/tinyshakespeare.txt");
    let train_end = (tokens.len() as f64 * 0.9) as usize;
    let train = &tokens[..train_end];
    let val = &tokens[train_end..];
    let dl = train.len();

    let mut s = seed;
    let mut rng = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((s >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0 };
    let lim_a = (6.0f32 / (2 * d) as f32).sqrt();
    let lim_h = (6.0f32 / (d + h) as f32).sqrt();
    let lim_o = (6.0f32 / (h + v) as f32).sqrt();
    let mut wq: Vec<f32> = (0..d * d).map(|_| rng() * lim_a).collect();
    let mut wk: Vec<f32> = (0..d * d).map(|_| rng() * lim_a).collect();
    let mut wv: Vec<f32> = (0..d * d).map(|_| rng() * lim_a).collect();
    let mut wh: Vec<f32> = (0..h * d).map(|_| rng() * lim_h).collect();
    let mut wo: Vec<f32> = (0..v * h).map(|_| rng() * lim_o).collect();

    let mut opt_e = AdamW::new(v * d, 0.04);
    let mut opt_q = AdamW::new(d * d, 0.04);
    let mut opt_k = AdamW::new(d * d, 0.04);
    let mut opt_v = AdamW::new(d * d, 0.04);
    let mut opt_h = AdamW::new(h * d, 0.04);
    let mut opt_o = AdamW::new(v * h, 0.04);

    let start = std::time::Instant::now();
    let mut best_bpb = f32::MAX;

    for step in 1..=steps {
        let lr_now = cosine_lr(step, steps, base_lr, steps / 10);
        let off = (step * 97 + seed as usize) % dl.saturating_sub(nc + 2);
        let end = (off + nc + 2).min(dl);
        if end - off < nc + 2 { continue; }
        let seq = &train[off..end];
        let ctx_tokens: Vec<usize> = seq[..nc + 1].to_vec();
        let target = seq[nc + 1].min(v - 1);
        let n = ctx_tokens.len();

        let embs: Vec<Vec<f32>> = ctx_tokens.iter().enumerate().map(|(p, &t)| {
            let t = t.min(v - 1);
            let mut e = embed[t * d..(t + 1) * d].to_vec();
            let pe = pos_encoding(p, d);
            for i in 0..d { e[i] += pe[i]; }
            e
        }).collect();

        let xq = &embs[n - 1];
        let mut q = vec![0.0f32; d];
        for i in 0..d { for j in 0..d { q[i] += wq[j * d + i] * xq[j]; } }

        let mut ks: Vec<Vec<f32>> = Vec::with_capacity(n);
        let mut vs: Vec<Vec<f32>> = Vec::with_capacity(n);
        for emb in &embs {
            let mut k = vec![0.0f32; d];
            let mut vi = vec![0.0f32; d];
            for i in 0..d { for j in 0..d { k[i] += wk[j * d + i] * emb[j]; vi[i] += wv[j * d + i] * emb[j]; } }
            ks.push(k); vs.push(vi);
        }

        let scale = (d as f32).sqrt();
        let mut scores = vec![0.0f32; n];
        for i in 0..n { let mut dot = 0.0f32; for j in 0..d { dot += q[j] * ks[i][j]; } scores[i] = qk_gain * dot / scale; }
        softmax(&mut scores);

        let mut c = vec![0.0f32; d];
        for i in 0..n { for j in 0..d { c[j] += scores[i] * vs[i][j]; } }

        let mut pre_act = vec![0.0f32; h];
        for hi in 0..h { for j in 0..d { pre_act[hi] += wh[hi * d + j] * c[j]; } }
        let mut hid = vec![0.0f32; h];
        for hi in 0..h { let r = pre_act[hi].max(0.0); hid[hi] = r * r; }

        let mut logits = vec![0.0f32; v];
        for vi in 0..v { for hi in 0..h { logits[vi] += wo[vi * h + hi] * hid[hi]; } }
        softmax(&mut logits);
        let _loss = -logits[target].max(1e-10).ln();

        // ── Backward ──
        let mut dl = logits;
        dl[target] -= 1.0;

        let mut dh = vec![0.0f32; h];
        for vi in 0..v { for hi in 0..h { dh[hi] += wo[vi * h + hi] * dl[vi]; } }

        let mut dwo = vec![0.0f32; v * h];
        for vi in 0..v { for hi in 0..h { dwo[vi * h + hi] = dl[vi] * hid[hi]; } }

        let mut dpa = vec![0.0f32; h];
        for hi in 0..h { dpa[hi] = dh[hi] * 2.0 * pre_act[hi].max(0.0); }

        let mut dc = vec![0.0f32; d];
        for hi in 0..h { for j in 0..d { dc[j] += wh[hi * d + j] * dpa[hi]; } }

        let mut dwh = vec![0.0f32; h * d];
        for hi in 0..h { for j in 0..d { dwh[hi * d + j] = dpa[hi] * c[j]; } }

        let mut da = vec![0.0f32; n];
        for i in 0..n { for j in 0..d { da[i] += dc[j] * vs[i][j]; } }

        let ada: f32 = scores.iter().zip(da.iter()).map(|(a, g)| a * g).sum();
        let mut ds = vec![0.0f32; n];
        for i in 0..n { ds[i] = scores[i] * (da[i] - ada) * qk_gain / scale; }

        let mut dvs: Vec<Vec<f32>> = (0..n).map(|_| vec![0.0f32; d]).collect();
        let mut dks: Vec<Vec<f32>> = (0..n).map(|_| vec![0.0f32; d]).collect();
        let mut dq = vec![0.0f32; d];
        for i in 0..n {
            for j in 0..d {
                dvs[i][j] = scores[i] * dc[j];
                dks[i][j] = ds[i] * q[j];
                dq[j] += ds[i] * ks[i][j];
            }
        }

        let mut dwq = vec![0.0f32; d * d];
        for j in 0..d { for i in 0..d { dwq[j * d + i] = dq[i] * xq[j]; } }

        let mut dwk = vec![0.0f32; d * d];
        let mut dwv = vec![0.0f32; d * d];
        for pos in 0..n {
            for j in 0..d {
                for i in 0..d {
                    dwk[j * d + i] += dks[pos][i] * embs[pos][j];
                    dwv[j * d + i] += dvs[pos][i] * embs[pos][j];
                }
            }
        }

        let mut de = vec![0.0f32; v * d];
        let tok_q = ctx_tokens[n - 1].min(v - 1);
        for i in 0..d { for j in 0..d { de[tok_q * d + i] += wq[j * d + i] * dq[j]; } }
        for pos in 0..n {
            let tok = ctx_tokens[pos].min(v - 1);
            for i in 0..d {
                for j in 0..d {
                    de[tok * d + i] += wk[j * d + i] * dks[pos][j];
                    de[tok * d + i] += wv[j * d + i] * dvs[pos][j];
                }
            }
        }

        opt_e.update(&mut embed, &de, lr_now);
        opt_q.update(&mut wq, &dwq, lr_now);
        opt_k.update(&mut wk, &dwk, lr_now);
        opt_v.update(&mut wv, &dwv, lr_now);
        opt_h.update(&mut wh, &dwh, lr_now);
        opt_o.update(&mut wo, &dwo, lr_now);

        if step % 500 == 0 || step == steps {
            let vb = eval_attention(&embed, &wq, &wk, &wv, &wh, &wo, val, d, h, v, nc, qk_gain);
            eprintln!("[{}] exp={} step={} bpb={:.4} best={:.4} {:.1}s",
                      tag, exp_id, step, vb, best_bpb, start.elapsed().as_secs_f64());
            if vb < best_bpb && vb.is_finite() { best_bpb = vb; }
        }
    }

    eprintln!("[{}] exp={} done: best_bpb={:.4} {:.1}s", tag, exp_id, best_bpb, start.elapsed().as_secs_f64());
    best_bpb as f64
}

fn train_attention(seed: u64, steps: usize, hidden: usize, context: usize, lr: f64, exp_id: &str) -> f64 {
    let d = DIM;
    let v = VOCAB;
    let mut s = seed;
    let mut rng = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((s >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0 };
    let lim = (6.0f32 / (3 * d) as f32).sqrt();
    let embed: Vec<f32> = (0..v * d).map(|_| rng() * lim).collect();
    train_attention_impl(embed, seed, steps, hidden, context, lr, exp_id, "attn")
}

fn train_hybrid(seed: u64, steps: usize, hidden: usize, context: usize, lr: f64, exp_id: &str) -> f64 {
    let d = DIM;
    let v = VOCAB;
    let jepa_steps = (steps as f64 * 0.3) as usize;
    let ntp_steps = steps.saturating_sub(jepa_steps);

    let tokens = load_data("data/tinyshakespeare.txt");
    let train_end = (tokens.len() as f64 * 0.9) as usize;
    let train = &tokens[..train_end];
    let dl = train.len();

    let embed = if jepa_steps > 0 {
        let num_ctx = context.clamp(2, 6) - 2;
        let mut enc = JepaEncoder::new(v, d, num_ctx, seed);
        let mut tgt_enc = JepaEncoder::new(v, d, num_ctx, seed.wrapping_add(1));
        let mut opt = trios_train_cpu::optimizer::AdamWCpu::with_params(v * d, lr, 0.618, 0.999, 0.01);
        let mask_config = MaskConfig { ratio: 0.3, min_span: 1, max_span: d / 8, num_spans: 2 };

        for step in 0..jepa_steps {
            let off = (step * SEQ + seed as usize) % dl.saturating_sub(SEQ + 1);
            let end = (off + SEQ + 1).min(dl);
            if end <= off { continue; }
            let seq = &train[off..end];
            let mut rng = StdRng::seed_from_u64(seed.wrapping_add(step as u64));
            let mr = mask_spans(seq.len().min(SEQ), mask_config, &mut rng);
            let tgt_pos = get_masked(&mr.mask);
            if tgt_pos.is_empty() { continue; }

            let o_emb = enc.encode(seq);
            let t_emb = tgt_enc.encode(seq);
            let mut loss = 0.0f64;
            for &p in &tgt_pos {
                if let (Some(o), Some(t)) = (o_emb.get(p), t_emb.get(p)) {
                    for i in 0..d.min(o.len()).min(t.len()) { let diff = o[i] - t[i]; loss += diff as f64 * diff as f64; }
                }
            }
            let sc = loss * 50.0;
            let mut g = vec![0.0f32; v * d];
            for (i, gi) in g.iter_mut().enumerate() {
                let val = enc.embed[i];
                *gi = sc as f32 * if val > 0.0 { 1.0 } else if val < 0.0 { -1.0 } else { 0.0 };
            }
            opt.step(&mut enc.embed, &g);

            let prog = ((step + 1) as f64 / jepa_steps as f64).min(1.0);
            let tau = (0.996 + 0.004 * prog) as f32;
            for (t, o) in tgt_enc.embed.iter_mut().zip(enc.embed.iter()) {
                *t = tau * *t + (1.0 - tau) * *o;
            }
        }
        eprintln!("[hybrid] JEPA pre-train: {} steps done", jepa_steps);
        enc.embed
    } else {
        let mut s = seed;
        let mut rng = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((s >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0 };
        let lim = (6.0f32 / (3 * d) as f32).sqrt();
        (0..v * d).map(|_| rng() * lim).collect()
    };

    train_attention_impl(embed, seed, ntp_steps, hidden, context, lr, exp_id, "hybrid")
}

#[derive(Parser)]
#[command(name = "trios-igla-trainer")]
struct Args {
    #[arg(long, default_value_t = 42)]
    seed: u64,

    #[arg(long, default_value_t = 1000)]
    steps: usize,

    #[arg(long, default_value_t = 384)]
    hidden: usize,

    #[arg(long, default_value_t = 6)]
    context: usize,

    #[arg(long, default_value_t = 0.004)]
    lr: f64,

    #[arg(long, default_value = "ngram")]
    arch: String,

    #[arg(long, default_value = "")]
    exp_id: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    eprintln!("[trainer] arch={} hidden={} ctx={} lr={} seed={} steps={} exp={}",
              args.arch, args.hidden, args.context, args.lr, args.seed, args.steps, args.exp_id);

    let bpb = match args.arch.as_str() {
        "ngram" => train_ngram(args.seed, args.steps, args.hidden, args.context, args.lr, &args.exp_id),
        "jepa" => train_jepa(args.seed, args.steps, args.hidden, args.lr, &args.exp_id),
        "attn" | "attention" => train_attention(args.seed, args.steps, args.hidden, args.context, args.lr, &args.exp_id),
        "hybrid" => train_hybrid(args.seed, args.steps, args.hidden, args.context, args.lr, &args.exp_id),
        other => {
            eprintln!("[trainer] unknown arch: {other}");
            std::process::exit(1);
        }
    };

    println!("BPB={:.4}", bpb);
    Ok(())
}
