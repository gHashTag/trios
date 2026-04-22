use std::fs;
use std::io::Write;
use std::time::Instant;

const VOCAB: usize = 128;
const DIM: usize = 96;
const SEQ: usize = 32;
const LN_2: f32 = std::f32::consts::LN_2;

fn load_data(path: &str) -> Vec<usize> {
    let raw = fs::read(path).unwrap_or_else(|e| {
        eprintln!("Failed to load {}: {}. Using fallback.", path, e);
        b"Hello world this is a tiny training dataset for IGLA".to_vec()
    });
    raw.into_iter().map(|b| (b as usize) % VOCAB).collect()
}

fn softmax(v: &mut [f32]) {
    let max_val = v.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0f32;
    for x in v.iter_mut() {
        *x = (*x - max_val).exp();
        sum += *x;
    }
    for x in v.iter_mut() {
        *x /= sum;
    }
}

fn rng_next(s: &mut u64) -> f32 {
    *s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let t = ((*s >> 33) as f32) / (u32::MAX as f32);
    t * 2.0 - 1.0
}

struct BigramHash {
    embed: Vec<f32>,
    vocab: usize,
    dim: usize,
}

impl BigramHash {
    fn new(vocab: usize, dim: usize, seed: &mut u64) -> Self {
        let embed: Vec<f32> = (0..vocab * dim).map(|_| rng_next(seed) * 0.02).collect();
        Self { embed, vocab, dim }
    }

    fn hash(&self, curr: usize, prev: usize) -> usize {
        ((36313u32.wrapping_mul(curr as u32)) ^ (27191u32.wrapping_mul(prev as u32))) as usize % (self.vocab - 1)
    }

    fn forward(&self, tokens: &[usize]) -> Vec<Vec<f32>> {
        let d = self.dim;
        let mut out = Vec::with_capacity(tokens.len());
        for (i, &t) in tokens.iter().enumerate() {
            let prev = if i > 0 { tokens[i - 1] } else { 0 };
            let h = self.hash(t, prev);
            out.push(self.embed[h * d..(h + 1) * d].to_vec());
        }
        out
    }

    fn grad_step(&mut self, tokens: &[usize], grad: &[Vec<f32>], lr: f32) {
        let d = self.dim;
        for (i, &t) in tokens.iter().enumerate() {
            let prev = if i > 0 { tokens[i - 1] } else { 0 };
            let h = self.hash(t, prev);
            for (j, g) in grad[i].iter().enumerate().take(d) {
                self.embed[h * d + j] -= lr * g;
            }
        }
    }
}

struct SmearGate {
    gate: Vec<f32>,
}

impl SmearGate {
    fn new(dim: usize) -> Self {
        Self { gate: vec![0.0f32; dim] }
    }

    fn forward(&self, xs: &[Vec<f32>]) -> Vec<Vec<f32>> {
        let mut out = Vec::with_capacity(xs.len());
        for (i, x) in xs.iter().enumerate() {
            let g: Vec<f32> = self.gate.iter().map(|&g| 1.0 / (1.0 + (-g).exp())).collect();
            if i == 0 {
                out.push(x.iter().zip(g.iter()).map(|(xi, gi)| xi * (1.0 - gi)).collect());
            } else {
                out.push(x.iter().zip(g.iter()).zip(xs[i - 1].iter())
                    .map(|((xi, gi), pi)| xi * (1.0 - gi) + pi * gi).collect());
            }
        }
        out
    }

    fn grad_step(&mut self, grad: &[Vec<f32>], lr: f32) {
        for (i, g) in self.gate.iter_mut().enumerate() {
            let mut total = 0.0f32;
            for g_vec in grad {
                total += g_vec[i];
            }
            *g -= lr * total;
        }
    }
}

struct AdamW {
    m: Vec<f32>,
    v: Vec<f32>,
    lr: f32,
    beta1: f32,
    beta2: f32,
    wd: f32,
    step: usize,
}

impl AdamW {
    fn new(size: usize, lr: f32) -> Self {
        Self { m: vec![0.0; size], v: vec![0.0; size], lr, beta1: 0.9, beta2: 0.999, wd: 0.01, step: 0 }
    }

    fn step(&mut self, params: &mut [f32], grads: &[f32]) {
        self.step += 1;
        let bc1 = 1.0 - self.beta1.powi(self.step as i32);
        let bc2 = 1.0 - self.beta2.powi(self.step as i32);
        for i in 0..params.len() {
            let g = grads[i];
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * g;
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * g * g;
            let m_hat = self.m[i] / bc1;
            let v_hat = self.v[i] / bc2;
            params[i] -= self.lr * (m_hat / (v_hat.sqrt() + 1e-8) + self.wd * params[i]);
        }
    }
}

struct CpuModel {
    embed: Vec<f32>,
    lm_head: Vec<f32>,
    bigram: BigramHash,
    smear: SmearGate,
    bigram_scale: f32,
    vocab: usize,
    dim: usize,
}

impl CpuModel {
    fn new(vocab: usize, dim: usize, seed: u64) -> Self {
        let mut s = seed;
        let embed: Vec<f32> = (0..vocab * dim).map(|_| rng_next(&mut s) * 0.02).collect();
        let lm_head: Vec<f32> = (0..vocab * dim).map(|_| rng_next(&mut s) * 0.02).collect();
        let bigram = BigramHash::new(vocab, dim, &mut s);
        let smear = SmearGate::new(dim);
        Self { embed, lm_head, bigram, smear, bigram_scale: 0.1, vocab, dim }
    }

    #[allow(dead_code)]
    fn forward_logits(&self, tokens: &[usize]) -> Vec<Vec<f32>> {
        let d = self.dim;
        let v = self.vocab;

        let tok_emb: Vec<Vec<f32>> = tokens.iter()
            .map(|&id| self.embed[(id % v) * d..((id % v) + 1) * d].to_vec())
            .collect();

        let bigram_emb = self.bigram.forward(tokens);
        let mut xs: Vec<Vec<f32>> = tok_emb.iter().zip(bigram_emb.iter())
            .map(|(t, b)| t.iter().zip(b.iter()).map(|(ti, bi)| ti + bi * self.bigram_scale).collect())
            .collect();

        xs = self.smear.forward(&xs);

        let mut logits = Vec::with_capacity(tokens.len());
        for x in &xs {
            let mut row = vec![0.0f32; v];
            for (vi, r) in row.iter_mut().enumerate() {
                for (j, xj) in x.iter().enumerate() {
                    *r += self.lm_head[vi * d + j] * xj;
                }
            }
            logits.push(row);
        }
        logits
    }

    fn loss_and_grad(&self, tokens: &[usize]) -> (f32, Vec<Vec<f32>>, Vec<Vec<f32>>) {
        let d = self.dim;
        let v = self.vocab;
        let n = tokens.len();

        let tok_emb: Vec<Vec<f32>> = tokens.iter()
            .map(|&id| self.embed[(id % v) * d..((id % v) + 1) * d].to_vec())
            .collect();
        let bigram_emb = self.bigram.forward(tokens);
        let xs: Vec<Vec<f32>> = tok_emb.iter().zip(bigram_emb.iter())
            .map(|(t, b)| t.iter().zip(b.iter()).map(|(ti, bi)| ti + bi * self.bigram_scale).collect())
            .collect();
        let xs_smeared = self.smear.forward(&xs);

        let mut total_loss = 0.0f32;
        let mut d_logits = vec![vec![0.0f32; v]; n - 1];

        for i in 0..n - 1 {
            let x = &xs_smeared[i];
            let target = tokens[i + 1] % v;
            let mut logits = vec![0.0f32; v];
            for (vi, l) in logits.iter_mut().enumerate() {
                for (j, xj) in x.iter().enumerate() {
                    *l += self.lm_head[vi * d + j] * xj;
                }
            }
            softmax(&mut logits);
            let p_target = logits[target].max(1e-10);
            total_loss -= p_target.ln();
            for (vi, dl) in d_logits[i].iter_mut().enumerate() {
                *dl = logits[vi] - if vi == target { 1.0 } else { 0.0 };
            }
        }

        let loss = total_loss / (n - 1) as f32;

        let mut d_hidden = vec![vec![0.0f32; d]; n];
        for i in 0..n - 1 {
            for (vi, dl) in d_logits[i].iter().enumerate() {
                for (j, dh) in d_hidden[i].iter_mut().enumerate() {
                    *dh += dl * self.lm_head[vi * d + j];
                }
            }
        }

        (loss, d_logits, d_hidden)
    }

    fn train_step(&mut self, tokens: &[usize], opt_embed: &mut AdamW, opt_head: &mut AdamW, lr: f32) -> f32 {
        let d = self.dim;
        let v = self.vocab;
        let n = tokens.len();

        let (loss, d_logits, d_hidden) = self.loss_and_grad(tokens);

        let mut d_lm_head = vec![0.0f32; v * d];

        let tok_emb: Vec<Vec<f32>> = tokens.iter()
            .map(|&id| self.embed[(id % v) * d..((id % v) + 1) * d].to_vec())
            .collect();
        let bigram_emb = self.bigram.forward(tokens);
        let xs: Vec<Vec<f32>> = tok_emb.iter().zip(bigram_emb.iter())
            .map(|(t, b)| t.iter().zip(b.iter()).map(|(ti, bi)| ti + bi * self.bigram_scale).collect())
            .collect();
        let xs_smeared = self.smear.forward(&xs);

        for i in 0..n - 1 {
            for (vi, dl) in d_logits[i].iter().enumerate() {
                for j in 0..d {
                    d_lm_head[vi * d + j] += dl * xs_smeared[i][j];
                }
            }
        }

        let mut d_embed = vec![0.0f32; v * d];
        for i in 0..n {
            let id = tokens[i] % v;
            for (j, dh) in d_hidden[i.min(n - 2)].iter().enumerate().take(d) {
                d_embed[id * d + j] += dh;
            }
        }

        opt_head.step(&mut self.lm_head, &d_lm_head);
        opt_embed.step(&mut self.embed, &d_embed);

        self.bigram.grad_step(tokens, &d_hidden, lr);
        self.smear.grad_step(&d_hidden, lr);

        loss
    }

    fn eval_bpb(&self, tokens: &[usize], seq_len: usize) -> f32 {
        let max_eval = 5000.min(tokens.len());
        let eval_tokens = &tokens[..max_eval];
        let mut total_bpb = 0.0f32;
        let mut n = 0usize;
        for c in (0..eval_tokens.len()).step_by(seq_len + 1) {
            let end = (c + seq_len + 1).min(eval_tokens.len());
            if end - c < 3 { continue; }
            let seq = &eval_tokens[c..end];
            let (loss, _, _) = self.loss_and_grad(seq);
            if loss.is_finite() {
                total_bpb += loss / LN_2;
                n += 1;
            }
        }
        if n == 0 { return f32::MAX; }
        total_bpb / n as f32
    }
}

fn main() {
    let seed = std::env::args()
        .find(|a| a.starts_with("--seed="))
        .map(|a| a[7..].parse::<u64>().unwrap_or(42))
        .unwrap_or(42);
    let steps = std::env::args()
        .find(|a| a.starts_with("--steps="))
        .map(|a| a[8..].parse::<usize>().unwrap_or(3000))
        .unwrap_or(3000);
    let lr = std::env::args()
        .find(|a| a.starts_with("--lr="))
        .map(|a| a[5..].parse::<f32>().unwrap_or(0.003))
        .unwrap_or(0.003);

    println!("=== trios CPU Training (Analytical Backprop) ===");
    println!("vocab={} dim={} seq={} steps={} seed={} lr={}", VOCAB, DIM, SEQ, steps, seed, lr);

    let tokens = load_data("data/tinyshakespeare.txt");
    let train_end = (tokens.len() as f64 * 0.9) as usize;
    let train_tokens = &tokens[..train_end];
    let val_tokens = &tokens[train_end..];
    println!("Dataset: {} train / {} val tokens", train_tokens.len(), val_tokens.len());

    let mut model = CpuModel::new(VOCAB, DIM, seed);
    let mut opt_embed = AdamW::new(VOCAB * DIM, lr);
    let mut opt_head = AdamW::new(VOCAB * DIM, lr);

    let init_bpb = model.eval_bpb(val_tokens, SEQ);
    println!("Initial val BPB: {:.4}", init_bpb);
    println!();
    println!("{:>6} | {:>10} | {:>10} | {:>10} | {:>8}", "step", "train_loss", "val_bpb", "best_bpb", "ms");
    println!("{}", "-".repeat(60));

    let t0 = Instant::now();
    let mut best_bpb = init_bpb;
    let data_len = train_tokens.len();
    let mut rng_state = seed;

    for step in 1..=steps {
        let offset = {
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (rng_state as usize) % (data_len.saturating_sub(SEQ + 1))
        };
        let seq = &train_tokens[offset..offset + SEQ + 1];
        let train_loss = model.train_step(seq, &mut opt_embed, &mut opt_head, lr);

        if step % 500 == 0 || step == steps {
            let ms = t0.elapsed().as_millis();
            let val_bpb = model.eval_bpb(val_tokens, SEQ);
            if val_bpb < best_bpb && val_bpb.is_finite() {
                best_bpb = val_bpb;
            }
            println!("{:>6} | {:>10.4} | {:>10.4} | {:>10.4} | {:>6}ms",
                step, train_loss, val_bpb, best_bpb, ms);
        }
    }

    let total = t0.elapsed();
    println!();
    println!("=== Training Complete ===");
    println!("Time: {:.1}s | Init BPB: {:.4} | Best BPB: {:.4} | Delta: {:.4}",
        total.as_secs_f64(), init_bpb, best_bpb, init_bpb - best_bpb);

    let _ = fs::create_dir_all(".trinity/results");
    let result_json = serde_json::json!({
        "experiment": "cpu-backprop-v1",
        "model": "embed+bigram+smear+lm_head",
        "seed": seed,
        "vocab_size": VOCAB,
        "dim": DIM,
        "seq_len": SEQ,
        "steps": steps,
        "lr": lr,
        "initial_bpb": init_bpb,
        "final_bpb": best_bpb,
        "delta_bpb": init_bpb - best_bpb,
        "duration_seconds": total.as_secs_f64(),
    });

    let rpath = format!(".trinity/results/cpu_train_seed{}.json", seed);
    fs::File::create(&rpath).unwrap()
        .write_all(serde_json::to_string_pretty(&result_json).unwrap().as_bytes()).unwrap();
    println!("Results: {}", rpath);
}
