// GF16-based N-Gram Language Model Training
use std::fs;
use std::io::Write;
use std::os::raw::c_float;
use std::time::Instant;

const VOCAB: usize = 128;
const SEQ: usize = 64;
const LN_2: f32 = std::f32::consts::LN_2;

// FFI to GF16 library
#[link(name = "goldenfloat")]
extern "C" {
    fn gf16_from_f32(x: c_float) -> u16;
    fn gf16_to_f32(g: u16) -> c_float;
    fn gf16_add(a: u16, b: u16) -> u16;
    fn gf16_mul(a: u16, b: u16) -> u16;
    fn gf16_sub(a: u16, b: u16) -> u16;
    fn gf16_neg(g: u16) -> u16;
    fn gf16_abs(g: u16) -> u16;
}

// Helper functions
fn f32_to_gf16(x: f32) -> u16 {
    unsafe { gf16_from_f32(x) }
}

fn gf16_to_f32_helper(x: u16) -> f32 {
    unsafe { gf16_to_f32(x) }
}

fn gelu(x: f32) -> f32 {
    let x3 = x * x * x;
    let tanh_arg = 0.7978846 * (x + 0.044715 * x3);
    let tanh_val = tanh_arg.tanh();
    0.5 * x * (1.0 + tanh_val)
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

// GF16 parameter storage - all weights stored as GF16
struct Gf16Parameters {
    data: Vec<u16>,
}

impl Gf16Parameters {
    fn new(size: usize) -> Self {
        Self { data: vec![0u16; size] }
    }

    fn from_f32(values: &[f32]) -> Self {
        Self {
            data: values.iter().map(|&v| f32_to_gf16(v)).collect(),
        }
    }

    fn to_f32(&self) -> Vec<f32> {
        self.data.iter().map(|&v| gf16_to_f32_helper(v)).collect()
    }

    fn get(&self, idx: usize) -> f32 {
        gf16_to_f32_helper(self.data[idx])
    }

    fn set(&mut self, idx: usize, val: f32) {
        self.data[idx] = f32_to_gf16(val);
    }
}

struct Optimizers { e: AdamW, c1: AdamW, c2: AdamW, p: AdamW, h: AdamW }

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
    fn update(&mut self, params: &mut Gf16Parameters, grads: &[f32], lr: f32) {
        self.step += 1;
        let bc1 = 1.0 - self.beta1.powi(self.step as i32);
        let bc2 = 1.0 - self.beta2.powi(self.step as i32);
        for i in 0..params.data.len() {
            let p = params.get(i);
            let wd_update = self.wd * lr * p;
            params.data[i] = f32_to_gf16(gf16_to_f32_helper(params.data[i]) - wd_update); // weight decay
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * grads[i];
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * grads[i] * grads[i];
            let update = lr * (self.m[i] / bc1) / ((self.v[i] / bc2).sqrt() + self.eps);
            params.data[i] = f32_to_gf16(gf16_to_f32_helper(params.data[i]) - update);
        }
    }
}

struct NgramModelGF16 {
    embed: Gf16Parameters,
    ctx1: Gf16Parameters,
    ctx2: Gf16Parameters,
    proj: Gf16Parameters,
    lm_head: Gf16Parameters,
    vocab: usize,
    dim: usize,
    hidden: usize,
}

impl NgramModelGF16 {
    fn new(vocab: usize, dim: usize, hidden: usize, seed: u64) -> Self {
        let mut s = seed;
        let mut rng = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0
        };
        let lim = (6.0f32 / (3.0 * dim as f32)).sqrt();
        let lim_h = (6.0f32 / (dim + hidden) as f32).sqrt();
        let lim_o = (6.0f32 / (hidden + dim) as f32).sqrt();

        let mut embed_f32: Vec<f32> = (0..vocab * dim).map(|_| rng() * lim).collect();
        let mut ctx1_f32: Vec<f32> = (0..vocab * dim).map(|_| rng() * lim).collect();
        let mut ctx2_f32: Vec<f32> = (0..vocab * dim).map(|_| rng() * lim).collect();
        let mut proj_f32: Vec<f32> = (0..hidden * dim).map(|_| rng() * lim_h).collect();
        let mut lm_head_f32: Vec<f32> = (0..vocab * hidden).map(|_| rng() * lim_o).collect();

        Self {
            embed: Gf16Parameters::from_f32(&embed_f32),
            ctx1: Gf16Parameters::from_f32(&ctx1_f32),
            ctx2: Gf16Parameters::from_f32(&ctx2_f32),
            proj: Gf16Parameters::from_f32(&proj_f32),
            lm_head: Gf16Parameters::from_f32(&lm_head_f32),
            vocab, dim, hidden,
        }
    }

    fn get_hidden(&self, t2: usize, t1: usize, t0: usize) -> Vec<f32> {
        let v = self.vocab;
        let d = self.dim;
        let h = self.hidden;

        let e0: Vec<f32> = (0..d).map(|j| self.embed.get(t0 * d + j)).collect();
        let c1: Vec<f32> = (0..d).map(|j| self.ctx1.get(t1 * d + j)).collect();
        let c2: Vec<f32> = (0..d).map(|j| self.ctx2.get(t2 * d + j)).collect();

        let mut combined = vec![0.0f32; d];
        for j in 0..d {
            combined[j] = e0[j] + c1[j] * 0.7 + c2[j] * 0.3;
        }

        let ln = layer_norm(&combined, 1e-5);

        let mut hidden = vec![0.0f32; h];
        for hi in 0..h {
            for j in 0..d {
                hidden[hi] += self.proj.get(hi * d + j) * ln[j];
            }
            hidden[hi] = hidden[hi].max(0.0); // ReLU
        }
        hidden
    }

    fn loss_on_seq(&self, tokens: &[usize]) -> f32 {
        if tokens.len() < 4 { return 0.0; }
        let v = self.vocab;
        let mut total = 0.0f32;
        for i in 2..tokens.len() - 1 {
            let t2 = tokens[i - 2].min(v - 1);
            let t1 = tokens[i - 1].min(v - 1);
            let t0 = tokens[i].min(v - 1);
            let hidden_vec = self.get_hidden(t2, t1, t0);
            let target = tokens[i + 1].min(v - 1);
            let mut logits = vec![0.0f32; v];
            for vi in 0..v {
                for hi in 0..self.hidden {
                    logits[vi] += self.lm_head.get(vi * self.hidden + hi) * hidden_vec[hi];
                }
            }
            softmax(&mut logits);
            total -= logits[target].max(1e-10).ln();
        }
        total / (tokens.len() - 3) as f32
    }

    fn train_step(&mut self, tokens: &[usize], lr: f32, opts: &mut Optimizers) {
        if tokens.len() < 4 { return; }
        let v = self.vocab;
        let d = self.dim;
        let h = self.hidden;

        let mut g_embed = vec![0.0f32; v * d];
        let mut g_ctx1 = vec![0.0f32; v * d];
        let mut g_ctx2 = vec![0.0f32; v * d];
        let mut g_proj = vec![0.0f32; h * d];
        let mut g_head = vec![0.0f32; v * h];

        for i in 2..tokens.len() - 1 {
            let t2 = tokens[i - 2].min(v - 1);
            let t1 = tokens[i - 1].min(v - 1);
            let t0 = tokens[i].min(v - 1);
            let tgt = tokens[i + 1].min(v - 1);

            let hidden = self.get_hidden(t2, t1, t0);

            let mut logits = vec![0.0f32; v];
            for vi in 0..v {
                for hi in 0..h {
                    logits[vi] += self.lm_head.get(vi * h + hi) * hidden[hi];
                }
            }
            softmax(&mut logits);

            let mut d_hidden = vec![0.0f32; h];
            for (vi, &prob) in logits.iter().enumerate() {
                let target_val = if vi == tgt { 1.0 } else { 0.0 };
                let grad = prob - target_val;
                for hi in 0..h {
                    g_head[vi * h + hi] += grad * hidden[hi];
                    d_hidden[hi] += grad * self.lm_head.get(vi * h + hi);
                }
            }

            // Backprop through proj (simplified, no full LN backward)
            for hi in 0..h {
                let relu_mask = if hidden[hi] > 0.0 { 1.0 } else { 0.0 };
                for j in 0..d {
                    let e0 = self.embed.get(t0 * d + j);
                    let c1 = self.ctx1.get(t1 * d + j);
                    let c2 = self.ctx2.get(t2 * d + j);
                    let combined = e0 + c1 * 0.7 + c2 * 0.3;
                    g_proj[hi * d + j] += d_hidden[hi] * relu_mask * combined;
                }
            }
        }

        let n = (tokens.len() - 3) as f32;
        for g in [&mut g_embed, &mut g_ctx1, &mut g_ctx2, &mut g_proj, &mut g_head] {
            for x in g.iter_mut() { *x /= n; }
        }

        opts.e.update(&mut self.embed, &g_embed, lr);
        opts.c1.update(&mut self.ctx1, &g_ctx1, lr);
        opts.c2.update(&mut self.ctx2, &g_ctx2, lr);
        opts.p.update(&mut self.proj, &g_proj, lr);
        opts.h.update(&mut self.lm_head, &g_head, lr);
    }
}

fn evaluate(model: &NgramModelGF16, tokens: &[usize], seq_len: usize) -> (f32, f32) {
    let mut total = 0.0f32;
    let mut n = 0usize;
    for c in (0..tokens.len()).step_by(seq_len + 1) {
        let end = (c + seq_len + 1).min(tokens.len());
        if end - c < 5 { continue; }
        let loss = model.loss_on_seq(&tokens[c..end]);
        if loss.is_finite() { total += loss / LN_2; n += 1; }
    }
    if n == 0 { return (f32::MAX, f32::MAX); }
    let bpb = total / n as f32;
    (bpb * LN_2, bpb)
}

fn cosine_lr(step: usize, max_steps: usize, base_lr: f32) -> f32 {
    let p = step as f32 / max_steps.max(1) as f32;
    1e-5 + (base_lr - 1e-5) * 0.5 * (1.0 + (std::f32::consts::PI * p).cos())
}

fn load_data(path: &str) -> Vec<usize> {
    let raw = fs::read(path).unwrap_or_else(|e| {
        eprintln!("Failed to load {}: {}. Using fallback.", path, e);
        b"Hello world this is a tiny training dataset for IGLA".to_vec()
    });
    raw.into_iter().map(|b| (b as usize) % VOCAB).collect()
}

fn main() {
    let seed = std::env::args().find(|a| a.starts_with("--seed="))
        .map(|a| a[7..].parse::<u64>().unwrap_or(42)).unwrap_or(42);
    let steps = std::env::args().find(|a| a.starts_with("--steps="))
        .map(|a| a[8..].parse::<usize>().unwrap_or(1000)).unwrap_or(1000);
    let base_lr = std::env::args().find(|a| a.starts_with("--lr="))
        .map(|a| a[5..].parse::<f32>().unwrap_or(0.004)).unwrap_or(0.004);
    let hidden = std::env::args().find(|a| a.starts_with("--hidden="))
        .map(|a| a[9..].parse::<usize>().unwrap_or(64)).unwrap_or(64);
    let dim = std::env::args().find(|a| a.starts_with("--dim="))
        .map(|a| a[6..].parse::<usize>().unwrap_or(64)).unwrap_or(64);

    println!("=== GF16 N-Gram Context Model ===");
    println!("vocab={} dim={} hidden={} steps={} seed={} lr={}", VOCAB, dim, hidden, steps, seed, base_lr);

    let tokens = load_data("data/tinyshakespeare.txt");
    println!("Dataset: {} tokens", tokens.len());

    let train_end = (tokens.len() as f64 * 0.9) as usize;
    let train = &tokens[..train_end];
    let val = &tokens[train_end..];
    println!("Split: {} train / {} val", train.len(), val.len());

    let mut model = NgramModelGF16::new(VOCAB, dim, hidden, seed);
    let ps = VOCAB * dim;
    let mut opts = Optimizers {
        e: AdamW::new(ps, 0.04), c1: AdamW::new(ps, 0.04), c2: AdamW::new(ps, 0.04),
        p: AdamW::new(hidden * dim, 0.04), h: AdamW::new(VOCAB * hidden, 0.04),
    };

    let (init_loss, init_bpb) = evaluate(&model, val, SEQ);
    println!("Initial val: loss={:.4} bpb={:.4}", init_loss, init_bpb);
    println!();
    println!("{:>6} | {:>10} | {:>10} | {:>10} | {:>8}", "step", "val_loss", "val_bpb", "best_bpb", "ms");
    println!("{}", "-".repeat(60));

    let t0 = Instant::now();
    let mut best_bpb = init_bpb;
    let dl = train.len();

    for step in 1..=steps {
        let lr = cosine_lr(step, steps, base_lr);
        let off = (step * 97 + seed as usize) % (dl.saturating_sub(SEQ + 1));
        model.train_step(&train[off..off + SEQ + 1], lr, &mut opts);

        if step % 100 == 0 || step == steps {
            let ms = t0.elapsed().as_millis();
            let (vl, vb) = evaluate(&model, val, SEQ);
            if vb < best_bpb && vb.is_finite() { best_bpb = vb; }
            println!("{:>6} | {:>10.4} | {:>10.4} | {:>10.4} | {:>6}ms", step, vl, vb, best_bpb, ms);
        }
    }

    let total = t0.elapsed();
    println!("\n=== GF16 Training Complete ===");
    println!("Time: {:.1}s | BPB: {:.4} → {:.4} | Delta: {:.4}", total.as_secs_f64(), init_bpb, best_bpb, best_bpb - init_bpb);
}
