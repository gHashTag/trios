use trios_train_cpu::real_igla_model::RealIglaModel;
use trios_train_cpu::phi_ortho_init;

fn load_data() -> Vec<usize> {
    let paths = ["data/input.txt", "data/tiny_shakespeare.txt"];
    for p in &paths {
        if let Ok(c) = std::fs::read_to_string(p) {
            return c.bytes().map(|b| b as usize).collect();
        }
    }
    (0..50000).map(|i| i % 256).collect()
}

fn main() {
    let data = load_data();
    println!("Data: {} bytes", data.len());

    let vocab: usize = 256;
    let d_model: usize = 64;
    let n_layers: usize = 1;
    let seq_len: usize = 64;
    let max_steps: usize = 5000;
    let lr: f32 = 5e-4;
    let warmup_steps: usize = 200;

    let mut model = RealIglaModel::new(vocab, d_model, n_layers);
    phi_ortho_init(&mut model.embed, d_model, vocab);

    println!(
        "Model: vocab={}, d_model={}, layers={}, params={}",
        vocab, d_model, n_layers, model.param_count()
    );
    println!("Config: steps={}, seq_len={}, lr={}", max_steps, seq_len, lr);

    let start = std::time::Instant::now();
    let mut best_bpb = f64::MAX;
    let eval_tokens: Vec<usize> = data[..256.min(data.len())].to_vec();
    let (_, initial_bpb) = model.loss_bpb(&eval_tokens);
    println!("Initial eval BPB: {:.4}\n", initial_bpb);

    for step in 0..max_steps {
        let idx = (step * seq_len + step * 7) % (data.len().saturating_sub(seq_len + 1));
        let tokens: Vec<usize> = data[idx..idx + seq_len + 1].to_vec();

        let current_lr = if step < warmup_steps {
            lr * (step as f32 / warmup_steps as f32)
        } else {
            let p = (step - warmup_steps) as f32 / (max_steps - warmup_steps) as f32;
            lr * 0.5 * (1.0 + (std::f32::consts::PI * p).cos())
        };

        let loss = model.train_step(&tokens, current_lr);
        let bpb = loss as f64 / std::f64::consts::LN_2;
        if bpb < best_bpb { best_bpb = bpb; }

        if step % 500 == 0 || step == max_steps - 1 {
            let (_, eval_bpb) = model.loss_bpb(&eval_tokens);
            let elapsed = start.elapsed().as_secs_f64();
            println!(
                "step {:>5}: train_bpb={:.4} eval_bpb={:.4} best={:.4} lr={:.6} {:.0}s",
                step, bpb, eval_bpb, best_bpb, current_lr, elapsed
            );
        }
    }

    let total = start.elapsed().as_secs_f64();
    let (_, final_bpb) = model.loss_bpb(&eval_tokens);
    println!("\nDone {:.0}s | Best: {:.4} | Final eval: {:.4}", total, best_bpb, final_bpb);
}
