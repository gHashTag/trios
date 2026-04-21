//! Phase B Fine v2: Upward search after 0.0262 edge win
//!
//! Previous winner: LR=0.0262 → val_bpb=6.5609
//! New grid: {0.0162, 0.0262, 0.0424} (φ-triad around 0.0262)
//! 300 steps each for stable signal
//! 1 seed (42)

use std::time::Instant;
use trios_train_cpu::{
    bpb_from_loss,
    backward::{cross_entropy_loss, clip_gradients},
    optimizer::AdamWCpu,
};

const STEPS: usize = 300;
const BATCH_SIZE: usize = 32;
const SEQ_LEN: usize = 81;
const VOCAB_SIZE: usize = 256;
const D_MODEL: usize = 144;
const SEED: u64 = 42;

fn run_lr(lr: f64, train_data: &[u8], val_data: &[u8]) -> (f64, f64) {
    let mut embeddings = vec![0.0f32; VOCAB_SIZE * D_MODEL];
    for emb in embeddings.iter_mut() {
        *emb = (rand::random::<f32>() - 0.5) * 0.1;
    }

    let mut optimizer = AdamWCpu::new(embeddings.len(), lr);
    let mut rng: u64 = SEED;

    // Validation data (fixed)
    let val_len = val_data.len().min(BATCH_SIZE * SEQ_LEN);
    let val_inputs: Vec<usize> = val_data[..val_len].iter().map(|&b| b as usize).collect();
    let val_targets: Vec<usize> = val_inputs.iter().skip(1).chain(std::iter::once(&val_inputs[0])).copied().collect();

    let start = Instant::now();
    let mut final_train_bpb = 0.0f64;

    for step in 0..STEPS {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        let batch_offset = rng as usize % (train_data.len() - BATCH_SIZE * SEQ_LEN);
        let mut inputs = Vec::with_capacity(BATCH_SIZE * SEQ_LEN);

        for b in 0..BATCH_SIZE {
            let offset = (batch_offset + b * SEQ_LEN) % (train_data.len() - SEQ_LEN);
            for i in 0..SEQ_LEN {
                inputs.push(train_data[offset + i] as usize);
            }
        }

        let targets: Vec<usize> = inputs.iter().skip(1).chain(std::iter::once(&inputs[0])).copied().collect();

        // Forward (embedding projection)
        let mut logits = vec![0.0f32; BATCH_SIZE * SEQ_LEN * VOCAB_SIZE];
        for b in 0..BATCH_SIZE {
            for i in 0..SEQ_LEN {
                let idx = b * SEQ_LEN + i;
                let input_idx = inputs[idx];
                let input_offset = input_idx * D_MODEL;
                let l_offset = idx * VOCAB_SIZE;

                for v in 0..VOCAB_SIZE {
                    let emb_offset = v * D_MODEL;
                    let mut logit = 0.0f32;
                    for d in 0..D_MODEL {
                        logit += embeddings[input_offset + d] * embeddings[emb_offset + d];
                    }
                    logits[l_offset + v] = logit;
                }
            }
        }

        let loss = cross_entropy_loss(&logits, &targets);

        if step == STEPS - 1 {
            final_train_bpb = bpb_from_loss(loss as f64);
        }

        // Backward (simplified)
        let mut gradients = vec![0.0f32; embeddings.len()];
        for b in 0..BATCH_SIZE {
            for i in 0..SEQ_LEN {
                let idx = b * SEQ_LEN + i;
                let input_idx = inputs[idx];
                let target_idx = targets[idx];
                let l_offset = idx * VOCAB_SIZE;

                // Softmax
                let mut max_logit = f32::NEG_INFINITY;
                for v in 0..VOCAB_SIZE {
                    max_logit = max_logit.max(logits[l_offset + v]);
                }

                let mut sum_exp = 0.0f32;
                for v in 0..VOCAB_SIZE {
                    sum_exp += (logits[l_offset + v] - max_logit).exp();
                }

                let _input_offset = input_idx * D_MODEL;
                for v in 0..VOCAB_SIZE {
                    let prob = (logits[l_offset + v] - max_logit).exp() / sum_exp;
                    let dlogits = prob - if v == target_idx { 1.0 } else { 0.0 };
                    let emb_offset = v * D_MODEL;
                    for d in 0..D_MODEL {
                        gradients[_input_offset + d] += dlogits * embeddings[emb_offset + d];
                        gradients[emb_offset + d] += dlogits * embeddings[_input_offset + d];
                    }
                }
            }
        }

        let scale = 1.0 / (BATCH_SIZE * SEQ_LEN) as f32;
        for g in gradients.iter_mut() { *g *= scale; }
        clip_gradients(&mut gradients, 1.0);
        optimizer.step(&mut embeddings, &gradients);
    }

    // Validation
    let mut val_logits = vec![0.0f32; BATCH_SIZE * SEQ_LEN * VOCAB_SIZE];
    for b in 0..BATCH_SIZE {
        for i in 0..SEQ_LEN {
            let idx = b * SEQ_LEN + i;
            let input_idx = val_inputs[idx];
            let input_offset = input_idx * D_MODEL;
            let l_offset = idx * VOCAB_SIZE;

            for v in 0..VOCAB_SIZE {
                let emb_offset = v * D_MODEL;
                let mut logit = 0.0f32;
                for d in 0..D_MODEL {
                    logit += embeddings[input_offset + d] * embeddings[emb_offset + d];
                }
                val_logits[l_offset + v] = logit;
            }
        }
    }

    let val_loss = cross_entropy_loss(&val_logits, &val_targets);
    let val_bpb = bpb_from_loss(val_loss as f64);

    let _elapsed = start.elapsed().as_secs_f64();

    (final_train_bpb, val_bpb)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════");
    println!("Phase B Fine v2: Upward Search");
    println!("═══════════════════════════════════════");
    println!();
    println!("Previous winner: LR=0.0262 → val_bpb=6.5609");
    println!("New grid (φ-triad around 0.0262):");
    println!("  LR=0.0262/φ (0.0162): 0.016212");
    println!("  LR=0.0262 (winner): 0.026212");
    println!("  LR=0.0262·φ (0.0424): 0.042402");
    println!();
    println!("Steps: {}, Seed: {}", STEPS, SEED);
    println!();

    // Generate synthetic data
    let mut train_data = vec![0u8; VOCAB_SIZE * 1000];
    let mut val_data = vec![0u8; VOCAB_SIZE * 100];
    for (i, b) in train_data.iter_mut().enumerate() {
        *b = (i % VOCAB_SIZE) as u8;
    }
    for (i, b) in val_data.iter_mut().enumerate() {
        *b = ((i * 7 + 13) % VOCAB_SIZE) as u8; // Different pattern
    }

    let phi = 1.618033988749895;
    let base_lr = 0.0262;

    let mut results = vec![];

    // Grid search
    for (i, factor) in [1.0 / phi, 1.0, phi].iter().enumerate() {
        let lr = base_lr * factor;
        let label = match i {
            0 => "0.0262/φ (0.0162)",
            1 => "0.0262 (winner)",
            _ => "0.0262·φ (0.0424 - upward probe)",
        };

        print!("LR={} ({:.6})... ", label, lr);
        let (train_bpb, val_bpb) = run_lr(lr, &train_data, &val_data);
        println!("train_bpb={:.4} val_bpb={:.4}", train_bpb, val_bpb);

        results.push((lr, label, train_bpb, val_bpb));
    }

    // Sort and display results
    results.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap());

    println!();
    println!("═══════════════════════════════════════");
    println!("RESULTS (Sorted by val_bpb)");
    println!("═══════════════════════════════════════");
    println!();

    for (i, (lr, label, train_bpb, val_bpb)) in results.iter().enumerate() {
        println!(
            "  {}. LR={} ({:.6}) → train_bpb={:.4} val_bpb={:.4}{}",
            i + 1,
            label,
            lr,
            train_bpb,
            val_bpb,
            if i == 0 { " ← WINNER" } else { "" }
        );
    }

    println!();
    println!("=== DECISION MATRIX ===");

    let winner = &results[0];
    let prev_best = 6.5609; // Previous winner

    if winner.3 < prev_best {
        println!("✅ New best LR found: {:.6} → val_bpb={:.4}", winner.0, winner.3);
        println!("   Previous best: 0.026212 → val_bpb={:.4}", prev_best);
        println!("   Improvement: {:.4} BPB", prev_best - winner.3);

        let (winner_idx, _) = [1.0 / phi, 1.0, phi]
            .iter()
            .enumerate()
            .find(|(_, &f)| (base_lr * f - winner.0).abs() < 1e-6)
            .unwrap();

        match winner_idx {
            0 => println!("   ⚠️  Control wins → move back to 0.0162"),
            1 => println!("   ✅ Winner confirmed → optimal LR={:.6}", winner.0),
            2 => println!("   ⚠️  Edge probe WINS → continue upward to 0.068+"),
            _ => unreachable!(),
        }
    } else if (winner.3 - prev_best).abs() < 0.01 {
        println!("⚠️  Within noise → optimal LR≈{:.6}", winner.0);
        println!("   Consider cross-validation or more steps");
    } else {
        println!("❌ Regression → previous LR=0.026212 was better");
        println!("   Consider: more training steps, different seed");
    }

    println!();
    println!("Winner: LR={} ({:.6}) → val_bpb={:.4}", winner.1, winner.0, winner.3);

    Ok(())
}
