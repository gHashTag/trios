use trios_train_cpu::real_igla_model::RealIglaModel;
use trios_train_cpu::optimizer::AdamWCpu;
use trios_train_cpu::phi_ortho_init;

fn load_data() -> Vec<usize> {
    for p in ["data/input.txt", "data/tiny_shakespeare.txt"] {
        if let Ok(c) = std::fs::read_to_string(p) {
            return c.bytes().map(|b| b as usize).collect();
        }
    }
    (0..50000).map(|i| i % 256).collect()
}

fn main() {
    let data = load_data();
    println!("Data: {} bytes", data.len());

    let d_model = 64;
    let n_layers = 1;
    let mut model = RealIglaModel::new(256, d_model, n_layers);
    phi_ortho_init(&mut model.embed, d_model, 256);
    let n_params = model.param_count();
    println!("Model: d={}, L={}, params={}", d_model, n_layers, n_params);

    let lr = 3e-3;
    let mut optimizer = AdamWCpu::new(n_params, lr);

    let seq_len = 32;
    let max_steps = 1000;

    let eval_end = (1024).min(data.len());
    let eval_tokens: Vec<usize> = data[..eval_end].to_vec();
    let (_, init_bpb) = model.loss_bpb(&eval_tokens);
    println!("Initial BPB: {:.4}\n", init_bpb);

    let start = std::time::Instant::now();
    let mut best = f64::MAX;

    for step in 0..max_steps {
        let idx = step * seq_len % (data.len().saturating_sub(seq_len + 1));
        let tokens: Vec<usize> = data[idx..idx + seq_len + 1].to_vec();

        let loss = model.train_step_adamw(&tokens, &mut optimizer);
        let bpb = loss as f64 / std::f64::consts::LN_2;
        if bpb < best { best = bpb; }

        if step % 200 == 0 || step == max_steps - 1 {
            let (_, eb) = model.loss_bpb(&eval_tokens);
            let t = start.elapsed().as_secs_f64();
            let sps = (step + 1) as f64 / t;
            println!("step {:>4}: bpb={:.3} eval={:.3} best={:.3} lr={:.5} {:.1}s ({:.0} sps)", step, bpb, eb, best, optimizer.lr, t, sps);
        }
    }

    let (_, final_bpb) = model.loss_bpb(&eval_tokens);
    let total = start.elapsed().as_secs_f64();
    println!("\n=== AdamW Benchmark ===");
    println!("Model: d={}, L={}, {} params", d_model, n_layers, n_params);
    println!("Steps: {} in {:.0}s ({:.1} sps)", max_steps, total, max_steps as f64 / total);
    println!("BPB: {:.4} -> {:.4} (best={:.4}, delta={:.4})", init_bpb, final_bpb, best, init_bpb - final_bpb);
}
