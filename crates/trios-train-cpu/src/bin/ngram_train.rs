use std::fs;
use std::io::Write;
use std::time::Instant;

const VOCAB: usize = 128;
const SEQ: usize = 64;
const LN_2: f32 = std::f32::consts::LN_2;

fn gelu(x: f32) -> f32 {
    let x3 = x * x * x;
    let tanh_arg = 0.7978846 * (x + 0.044715 * x3); // sqrt(2/pi) ≈ 0.7978846
    let tanh_val = tanh_arg.tanh();
    0.5 * x * (1.0 + tanh_val)
}

fn load_data(path: &str) -> Vec<usize> {
    let raw = fs::read(path).unwrap_or_else(|e| {
        eprintln!("Failed to load {}: {}. Using fallback.", path, e);
        b"Hello world this is a tiny training dataset for IGLA".to_vec()
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

struct Optimizers { e: AdamW, c1: AdamW, c2: AdamW, c3: AdamW, c4: AdamW, p: AdamW, h: AdamW }

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
    ctx1: Vec<f32>,
    ctx2: Vec<f32>,
    ctx3: Vec<f32>,
    ctx4: Vec<f32>,
    proj: Vec<f32>,
    lm_head: Vec<f32>,
    #[allow(dead_code)]
    ln_g: Vec<f32>,
    #[allow(dead_code)]
    ln_b: Vec<f32>,
    residual_alpha: f32,
    vocab: usize,
    dim: usize,
    hidden: usize,
    activation: String,
    dropout: f32,
    use_ctx3: bool,
    use_ctx4: bool,
}

impl NgramModel {
    #[allow(clippy::too_many_arguments)]
    fn new(vocab: usize, dim: usize, hidden: usize, activation: String, seed: u64, dropout: f32, use_ctx3: bool, use_ctx4: bool) -> Self {
        let mut s = seed;
        let mut rng = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0
        };
        let n_ctx = if use_ctx4 { 5 } else if use_ctx3 { 4 } else { 3 };
        let lim = (6.0f32 / (n_ctx * dim) as f32).sqrt();
        let lim_h = (6.0f32 / (dim + hidden) as f32).sqrt();
        let lim_o = (6.0f32 / (hidden + dim) as f32).sqrt();
        Self {
            embed: (0..vocab * dim).map(|_| rng() * lim).collect(),
            ctx1: (0..vocab * dim).map(|_| rng() * lim).collect(),
            ctx2: (0..vocab * dim).map(|_| rng() * lim).collect(),
            ctx3: (0..vocab * dim).map(|_| rng() * lim).collect(),
            ctx4: (0..vocab * dim).map(|_| rng() * lim).collect(),
            proj: (0..hidden * dim).map(|_| rng() * lim_h).collect(),
            lm_head: (0..vocab * hidden).map(|_| rng() * lim_o).collect(),
            ln_g: vec![1.0; dim],
            ln_b: vec![0.0; dim],
            residual_alpha: 0.3,
            vocab, dim, hidden, activation,
            dropout, use_ctx3, use_ctx4,
        }
    }

    fn get_hidden(&self, t4: usize, t3: usize, t2: usize, t1: usize, t0: usize, residual: bool) -> Vec<f32> {
        let v = self.vocab;
        let d = self.dim;
        let h = self.hidden;
        let t4 = t4.min(v - 1);
        let t3 = t3.min(v - 1);
        let t2 = t2.min(v - 1);
        let t1 = t1.min(v - 1);
        let t0 = t0.min(v - 1);

        let e0 = &self.embed[t0 * d..(t0 + 1) * d];
        let c1 = &self.ctx1[t1 * d..(t1 + 1) * d];
        let c2 = &self.ctx2[t2 * d..(t2 + 1) * d];

        let mut combined = vec![0.0f32; d];
        if self.use_ctx4 {
            let c3 = &self.ctx3[t3 * d..(t3 + 1) * d];
            let c4 = &self.ctx4[t4 * d..(t4 + 1) * d];
            for j in 0..d { combined[j] = e0[j] + c1[j] * 0.4 + c2[j] * 0.25 + c3[j] * 0.2 + c4[j] * 0.15; }
        } else if self.use_ctx3 {
            let c3 = &self.ctx3[t3 * d..(t3 + 1) * d];
            for j in 0..d { combined[j] = e0[j] + c1[j] * 0.5 + c2[j] * 0.3 + c3[j] * 0.2; }
        } else {
            for j in 0..d { combined[j] = e0[j] + c1[j] * 0.7 + c2[j] * 0.3; }
        }

        let ln = layer_norm(&combined, 1e-5);

        let mut hidden = vec![0.0f32; h];
        for (hi, hn) in hidden.iter_mut().enumerate().take(h) {
            let w = &self.proj[hi * d..(hi + 1) * d];
            for (j, l) in ln.iter().enumerate() { *hn += w[j] * l; }
            *hn = if self.activation == "gelu" { gelu(*hn) } else { (*hn).max(0.0) };
            if self.dropout > 0.0 {
                let mask = (((hi as u64).wrapping_mul(6364136223846793005u64) >> 33) as f32) / (u32::MAX as f32);
                if mask < self.dropout { *hn = 0.0; }
                else { *hn /= 1.0 - self.dropout; }
            }
        }
        
        // Apply residual connection if enabled
        if residual {
            let mut output = vec![0.0f32; h];
            for i in 0..h {
                // Project combined to hidden dimension and apply residual
                let combined_component = if h >= d {
                    // If hidden >= dim, use combined directly
                    combined[i % d] * (1.0 - self.residual_alpha)
                } else {
                    // If hidden < dim, combine multiple elements
                    let sum = combined.iter().take(d).sum::<f32>() * (1.0 / d as f32);
                    sum * (1.0 - self.residual_alpha)
                };
                output[i] = combined_component + hidden[i] * self.residual_alpha;
            }
            output
        } else {
            hidden
        }
    }

    fn loss_on_seq(&self, tokens: &[usize], residual: bool) -> f32 {
        let start = if self.use_ctx4 { 4 } else if self.use_ctx3 { 3 } else { 2 };
        if tokens.len() < start + 2 { return 0.0; }
        let v = self.vocab;
        let mut total = 0.0f32;
        for i in start..tokens.len() - 1 {
            let t4 = if self.use_ctx4 { tokens[i - 4].min(v - 1) } else { 0 };
            let t3 = if self.use_ctx4 || self.use_ctx3 { tokens[i - 3].min(v - 1) } else { 0 };
            let t2 = tokens[i - 2].min(v - 1);
            let t1 = tokens[i - 1].min(v - 1);
            let t0 = tokens[i].min(v - 1);
            let h = self.get_hidden(t4, t3, t2, t1, t0, residual);
            let target = tokens[i + 1].min(v - 1);
            let mut logits = vec![0.0f32; v];
            for (vi, logit) in logits.iter_mut().enumerate() {
                let w = &self.lm_head[vi * self.hidden..(vi + 1) * self.hidden];
                for (hi, hn) in h.iter().enumerate() { *logit += w[hi] * hn; }
            }
            softmax(&mut logits);
            total -= logits[target].max(1e-10).ln();
        }
        total / (tokens.len() - start - 1) as f32
    }

    fn train_step(&mut self, tokens: &[usize], lr: f32,
        opts: &mut Optimizers, residual: bool) {
        let start = if self.use_ctx4 { 4 } else if self.use_ctx3 { 3 } else { 2 };
        if tokens.len() < start + 2 { return; }
        let v = self.vocab;
        let d = self.dim;
        let h = self.hidden;

        let mut g_embed = vec![0.0f32; v * d];
        let mut g_ctx1 = vec![0.0f32; v * d];
        let mut g_ctx2 = vec![0.0f32; v * d];
        let mut g_ctx3 = vec![0.0f32; v * d];
        let mut g_ctx4 = vec![0.0f32; v * d];
        let mut g_proj = vec![0.0f32; h * d];
        let mut g_head = vec![0.0f32; v * h];

        for i in start..tokens.len() - 1 {
            let t4 = if self.use_ctx4 { tokens[i - 4].min(v - 1) } else { 0 };
            let t3 = if self.use_ctx4 || self.use_ctx3 { tokens[i - 3].min(v - 1) } else { 0 };
            let t2 = tokens[i - 2].min(v - 1);
            let t1 = tokens[i - 1].min(v - 1);
            let t0 = tokens[i].min(v - 1);
            let tgt = tokens[i + 1].min(v - 1);

            let hidden = self.get_hidden(t4, t3, t2, t1, t0, residual);

            let mut logits = vec![0.0f32; v];
            for (vi, logit) in logits.iter_mut().enumerate() {
                let w = &self.lm_head[vi * h..(vi + 1) * h];
                for (hi, hn) in hidden.iter().enumerate() { *logit += w[hi] * hn; }
            }
            softmax(&mut logits);

            let mut d_hidden = vec![0.0f32; h];
            for (vi, prob) in logits.iter().enumerate() {
                let grad = prob - if vi == tgt { 1.0 } else { 0.0 };
                let w = &self.lm_head[vi * h..(vi + 1) * h];
                for hi in 0..h {
                    g_head[vi * h + hi] += grad * hidden[hi];
                    d_hidden[hi] += grad * w[hi];
                }
            }

        // Note: LayerNorm backward pass simplified for now
        // Focus on forward correctness first

            let e0 = &self.embed[t0 * d..(t0 + 1) * d];
            let c1v = &self.ctx1[t1 * d..(t1 + 1) * d];
            let c2v = &self.ctx2[t2 * d..(t2 + 1) * d];
            let mut combined = vec![0.0f32; d];
            if self.use_ctx4 {
                let c3v = &self.ctx3[t3 * d..(t3 + 1) * d];
                let c4v = &self.ctx4[t4 * d..(t4 + 1) * d];
                for j in 0..d { combined[j] = e0[j] + c1v[j] * 0.4 + c2v[j] * 0.25 + c3v[j] * 0.2 + c4v[j] * 0.15; }
            } else if self.use_ctx3 {
                let c3v = &self.ctx3[t3 * d..(t3 + 1) * d];
                for j in 0..d { combined[j] = e0[j] + c1v[j] * 0.5 + c2v[j] * 0.3 + c3v[j] * 0.2; }
            } else {
                for j in 0..d { combined[j] = e0[j] + c1v[j] * 0.7 + c2v[j] * 0.3; }
            }
            let ln = layer_norm(&combined, 1e-5);

            for hi in 0..h {
                let relu_mask = if hidden[hi] > 0.0 { 1.0f32 } else { 0.0f32 };
                for j in 0..d {
                    g_proj[hi * d + j] += d_hidden[hi] * relu_mask * ln[j];
                }
            }
            for j in 0..d {
                let grad_j = d_hidden.iter().enumerate()
                    .map(|(hi, dh)| self.proj[hi * d + j] * if hidden[hi] > 0.0 { 1.0 } else { 0.0 } * dh)
                    .sum::<f32>();
                g_embed[t0 * d + j] += grad_j;
                if self.use_ctx4 {
                    g_ctx1[t1 * d + j] += 0.4 * grad_j;
                    g_ctx2[t2 * d + j] += 0.25 * grad_j;
                    g_ctx3[t3 * d + j] += 0.2 * grad_j;
                    g_ctx4[t4 * d + j] += 0.15 * grad_j;
                } else if self.use_ctx3 {
                    g_ctx1[t1 * d + j] += 0.5 * grad_j;
                    g_ctx2[t2 * d + j] += 0.3 * grad_j;
                    g_ctx3[t3 * d + j] += 0.2 * grad_j;
                } else {
                    g_ctx1[t1 * d + j] += 0.7 * grad_j;
                    g_ctx2[t2 * d + j] += 0.3 * grad_j;
                }
            }
        }

        let n = (tokens.len() - start - 1) as f32;
        for g in [&mut g_embed, &mut g_ctx1, &mut g_ctx2, &mut g_ctx3, &mut g_ctx4, &mut g_proj, &mut g_head] {
            for x in g.iter_mut() { *x /= n; }
        }

        opts.e.update(&mut self.embed, &g_embed, lr);
        opts.c1.update(&mut self.ctx1, &g_ctx1, lr);
        opts.c2.update(&mut self.ctx2, &g_ctx2, lr);
        if self.use_ctx3 || self.use_ctx4 {
            opts.c3.update(&mut self.ctx3, &g_ctx3, lr);
        }
        if self.use_ctx4 {
            opts.c4.update(&mut self.ctx4, &g_ctx4, lr);
        }
        opts.p.update(&mut self.proj, &g_proj, lr);
        opts.h.update(&mut self.lm_head, &g_head, lr);
    }
}

fn evaluate(model: &NgramModel, tokens: &[usize], seq_len: usize, residual: bool) -> (f32, f32) {
    let mut total = 0.0f32;
    let mut n = 0usize;
    for c in (0..tokens.len()).step_by(seq_len + 1) {
        let end = (c + seq_len + 1).min(tokens.len());
        if end - c < 5 { continue; }
        let loss = model.loss_on_seq(&tokens[c..end], residual);
        if loss.is_finite() { total += loss / LN_2; n += 1; }
    }
    if n == 0 { return (f32::MAX, f32::MAX); }
    let bpb = total / n as f32;
    (bpb * LN_2, bpb)
}

fn cosine_lr(step: usize, max_steps: usize, base_lr: f32, warmup: usize) -> f32 {
    if step < warmup { return base_lr * step as f32 / warmup as f32; }
    let p = (step - warmup) as f32 / (max_steps - warmup).max(1) as f32;
    1e-5 + (base_lr - 1e-5) * 0.5 * (1.0 + (std::f32::consts::PI * p).cos())
}

fn main() {
    let seed = std::env::args().find(|a| a.starts_with("--seed="))
        .map(|a| a[7..].parse::<u64>().unwrap_or(42)).unwrap_or(42);
    let steps = std::env::args().find(|a| a.starts_with("--steps="))
        .map(|a| a[8..].parse::<usize>().unwrap_or(10000)).unwrap_or(10000);
    let base_lr = std::env::args().find(|a| a.starts_with("--lr="))
        .map(|a| a[5..].parse::<f32>().unwrap_or(0.003)).unwrap_or(0.003);
    let hidden = std::env::args().find(|a| a.starts_with("--hidden="))
        .map(|a| a[9..].parse::<usize>().unwrap_or(128)).unwrap_or(128);
    let dim = std::env::args().find(|a| a.starts_with("--dim="))
        .map(|a| a[6..].parse::<usize>().unwrap_or(64)).unwrap_or(64);
    let activation = std::env::args().find(|a| a.starts_with("--activation="))
        .map(|a| a[13..].to_string()).unwrap_or_else(|| "relu".to_string());
    let residual = std::env::args().any(|a| a == "--residual");
    let wd = std::env::args().find(|a| a.starts_with("--wd="))
        .map(|a| a[5..].parse::<f32>().unwrap_or(0.04)).unwrap_or(0.04);
    let warmup = std::env::args().find(|a| a.starts_with("--warmup="))
        .map(|a| a[9..].parse::<usize>().unwrap_or(0)).unwrap_or(0);
    let dropout = std::env::args().find(|a| a.starts_with("--dropout="))
        .map(|a| a[10..].parse::<f32>().unwrap_or(0.0)).unwrap_or(0.0);
    let use_ctx3 = std::env::args().any(|a| a == "--ctx3");
    let use_ctx4 = std::env::args().any(|a| a == "--ctx4");

    let ngram = if use_ctx4 { "6-Gram" } else if use_ctx3 { "5-Gram" } else { "4-Gram" };
    let activation_name = if activation == "gelu" { "GELU" } else { "ReLU" };
    let res_suffix = if residual { " + Residual" } else { "" };
    println!("=== {} Context Model + {} Hidden{} ===", ngram, activation_name, res_suffix);
    println!("vocab={} dim={} hidden={} seq={} steps={} seed={} lr={} activation={} residual={} wd={} warmup={} dropout={}", VOCAB, dim, hidden, SEQ, steps, seed, base_lr, activation, residual, wd, warmup, dropout);

    let tokens = load_data("data/tinyshakespeare.txt");
    println!("Dataset: {} tokens", tokens.len());

    let train_end = (tokens.len() as f64 * 0.9) as usize;
    let train = &tokens[..train_end];
    let val = &tokens[train_end..];
    println!("Split: {} train / {} val", train.len(), val.len());

    let mut model = NgramModel::new(VOCAB, dim, hidden, activation.clone(), seed, dropout, use_ctx3, use_ctx4);
    let ps = VOCAB * dim;
    let mut opts = Optimizers {
        e: AdamW::new(ps, wd), c1: AdamW::new(ps, wd), c2: AdamW::new(ps, wd),
        c3: AdamW::new(ps, wd), c4: AdamW::new(ps, wd),
        p: AdamW::new(hidden * dim, wd), h: AdamW::new(VOCAB * hidden, wd),
    };

    let (init_loss, init_bpb) = evaluate(&model, val, SEQ, residual);
    println!("Initial val: loss={:.4} bpb={:.4}", init_loss, init_bpb);
    println!();
    println!("{:>6} | {:>10} | {:>10} | {:>10} | {:>8}", "step", "val_loss", "val_bpb", "best_bpb", "ms");
    println!("{}", "-".repeat(60));

    let t0 = Instant::now();
    let mut best_bpb = init_bpb;
    let mut results: Vec<(usize, f32, f32)> = Vec::new();
    let dl = train.len();

    for step in 1..=steps {
        let lr = cosine_lr(step, steps, base_lr, if warmup > 0 { warmup } else { steps / 10 });
        let off = (step * 97 + seed as usize) % (dl.saturating_sub(SEQ + 1));
        model.train_step(&train[off..off + SEQ + 1], lr, &mut opts, residual);

        if step % 500 == 0 || step == steps {
            let ms = t0.elapsed().as_millis();
            let (vl, vb) = evaluate(&model, val, SEQ, residual);
            if vb < best_bpb && vb.is_finite() { best_bpb = vb; }
            println!("{:>6} | {:>10.4} | {:>10.4} | {:>10.4} | {:>6}ms", step, vl, vb, best_bpb, ms);
            results.push((step, vl, vb));
        }
    }

    let total = t0.elapsed();
    println!("\n=== Done ===");
    println!("Time: {:.1}s | BPB: {:.4} → {:.4} | Delta: {:.4}", total.as_secs_f64(), init_bpb, best_bpb, best_bpb - init_bpb);

    let _ = fs::create_dir_all(".trinity/results");
    let exp_name = format!("4gram-{}{}", 
        activation,
        if residual { "-res" } else { "" });
    let model_desc = format!("3-context + embed + {} hidden{} + LM head", 
        if activation == "gelu" { "GELU" } else { "ReLU" },
        if residual { " + Residual" } else { "" });
    
    let rj = serde_json::json!({
        "experiment": exp_name,
        "model": model_desc,
        "seed": seed, "steps": steps, "base_lr": base_lr,
        "hidden_size": hidden, "activation": activation, "residual": residual,
        "train_tokens": train.len(), "val_tokens": val.len(),
        "initial_val_bpb": init_bpb, "final_val_bpb": best_bpb,
        "delta_bpb": best_bpb - init_bpb,
        "duration_seconds": total.as_secs_f64(),
        "results": results.iter().map(|(s, l, b)| serde_json::json!({"step":*s,"loss":*l,"bpb":*b})).collect::<Vec<_>>(),
    });
    let rp = format!(".trinity/results/4gram_{}_seed{}.json", 
        if residual { "res" } else { activation.as_str() }, seed);
    fs::File::create(&rp).unwrap().write_all(serde_json::to_string_pretty(&rj).unwrap().as_bytes()).unwrap();
    println!("Results: {}", rp);

    let ts = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let ep = format!(".trinity/experience/trios_{}.trinity", chrono::Utc::now().format("%Y%m%d"));
    let _ = fs::create_dir_all(".trinity/experience");
    let _ = fs::OpenOptions::new().create(true).append(true).open(&ep).unwrap()
        .write_all(format!("[{}] TASK: 4-gram training | seed={} | steps={} | val_bpb={:.4}->{:.4} | {:.1}s\n",
            ts, seed, steps, init_bpb, best_bpb, total.as_secs_f64()).as_bytes());
    println!("Experience: {}", ep);
}
