use crate::real_igla_model::RealIglaModel;
use crate::phi_ortho_init::phi_ortho_init;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseAConfig {
    pub lr: f64,
    pub warmup_steps: usize,
    pub max_steps: usize,
    pub batch_size: usize,
    pub seq_len: usize,
    pub vocab_size: usize,
    pub d_model: usize,
    pub n_layers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    pub phase: String,
    pub config: PhaseAConfig,
    pub final_bpb: f64,
    pub best_bpb: f64,
    pub steps: usize,
    pub duration_seconds: f64,
    pub param_count: usize,
}

impl Default for PhaseAConfig {
    fn default() -> Self {
        Self {
            lr: 3e-4,
            warmup_steps: 100,
            max_steps: 1000,
            batch_size: 4,
            seq_len: 64,
            vocab_size: 256,
            d_model: 64,
            n_layers: 1,
        }
    }
}

fn load_tiny_shakespeare() -> Vec<usize> {
    let paths = [
        "data/tiny_shakespeare.txt",
        "crates/trios-train-cpu/data/tiny_shakespeare.txt",
        "data/input.txt",
    ];

    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            return content.bytes().map(|b| b as usize).collect::<Vec<_>>();
        }
    }

    (0..10000).map(|i| i % 256).collect()
}

impl PhaseAConfig {
    pub fn run(&self, _seed: u64) -> ExperimentResult {
        let mut model = RealIglaModel::new(self.vocab_size, self.d_model, self.n_layers);

        phi_ortho_init(&mut model.embed, self.d_model, self.vocab_size);

        let data = load_tiny_shakespeare();
        let start = Instant::now();

        println!(
            "Phase A: LR={:.6}, warmup={}, steps={}, d_model={}, layers={}",
            self.lr, self.warmup_steps, self.max_steps, self.d_model, self.n_layers
        );
        println!("Data: {} tokens, params: {}", data.len(), model.param_count());

        let mut best_bpb = f64::MAX;
        let mut final_bpb = 0.0f64;
        let warmup_lr = self.lr * 0.1;

        for step in 0..self.max_steps {
            let idx = (step * self.seq_len) % (data.len().saturating_sub(self.seq_len + 1));
            let tokens: Vec<usize> = data[idx..idx + self.seq_len + 1].to_vec();

            let lr = if step < self.warmup_steps {
                warmup_lr + (self.lr - warmup_lr) * (step as f64 / self.warmup_steps.max(1) as f64)
            } else {
                self.lr * (1.0 - (step as f64 - self.warmup_steps as f64) / (self.max_steps as f64 - self.warmup_steps as f64)).max(0.01)
            };

            let loss = model.train_step(&tokens, lr as f32);
            let bpb = loss as f64 / std::f64::consts::LN_2;

            if bpb < best_bpb {
                best_bpb = bpb;
            }
            final_bpb = bpb;

            if step % 100 == 0 || step == self.max_steps - 1 {
                let (_, eval_bpb) = model.loss_bpb(&tokens);
                println!(
                    "  step {:>5}: loss={:.4} bpb={:.4} best={:.4} lr={:.6}",
                    step, loss, eval_bpb, best_bpb, lr
                );
            }
        }

        let elapsed = start.elapsed().as_secs_f64();

        ExperimentResult {
            phase: "A".to_string(),
            config: self.clone(),
            final_bpb,
            best_bpb,
            steps: self.max_steps,
            duration_seconds: elapsed,
            param_count: model.param_count(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PhaseBConfig {
    pub base_lr: f64,
    pub mix_ratio: f32,
    pub max_steps: usize,
    pub batch_size: usize,
    pub seq_len: usize,
}

impl Default for PhaseBConfig {
    fn default() -> Self {
        Self {
            base_lr: 1.62e-4,
            mix_ratio: 0.618,
            max_steps: 500,
            batch_size: 8,
            seq_len: 128,
        }
    }
}

impl PhaseBConfig {
    pub fn run(&self, _seed: u64) -> ExperimentResult {
        let mut model = RealIglaModel::new(256, 128, 2);
        phi_ortho_init(&mut model.embed, 128, 256);

        let data = load_tiny_shakespeare();
        let start = Instant::now();

        println!("Phase B: LR={:.6}, steps={}", self.base_lr, self.max_steps);

        let mut best_bpb = f64::MAX;
        let mut final_bpb = 0.0f64;

        for step in 0..self.max_steps {
            let idx = (step * self.seq_len) % (data.len().saturating_sub(self.seq_len + 1));
            let tokens: Vec<usize> = data[idx..idx + self.seq_len + 1].to_vec();

            let loss = model.train_step(&tokens, self.base_lr as f32);
            let bpb = loss as f64 / std::f64::consts::LN_2;

            if bpb < best_bpb {
                best_bpb = bpb;
            }
            final_bpb = bpb;

            if step % 50 == 0 || step == self.max_steps - 1 {
                println!("  step {:>5}: loss={:.4} bpb={:.4}", step, loss, bpb);
            }
        }

        let elapsed = start.elapsed().as_secs_f64();

        ExperimentResult {
            phase: "B".to_string(),
            config: PhaseAConfig {
                lr: self.base_lr,
                warmup_steps: 0,
                max_steps: self.max_steps,
                batch_size: self.batch_size,
                seq_len: self.seq_len,
                vocab_size: 256,
                d_model: 128,
                n_layers: 2,
            },
            final_bpb,
            best_bpb,
            steps: self.max_steps,
            duration_seconds: elapsed,
            param_count: model.param_count(),
        }
    }
}
