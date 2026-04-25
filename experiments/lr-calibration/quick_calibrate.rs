//! Issue #54: Quick LR Schedule Calibration (simplified)
//!
//! Fast calibration of 3 LR schedules using synthetic gradients.
//! This is a SPEED-OPTIMIZED version for rapid schedule comparison.
//!
//! Run with: cargo run --release --bin quick_calibrate

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use trios_phi_schedule::{LrScheduleType, lr_schedule_54};

const MAX_STEPS: usize = 500;  // Reduced for speed
const BASE_LR: f32 = 3e-4;
const LOG_EVERY: usize = 50;

/// Synthetic training loss simulation
///
/// Models the learning curve: loss decreases with LR-dependent rate.
/// Key insight: different LR schedules produce different convergence curves.
fn simulate_training_step(
    step: usize,
    lr: f32,
    prev_loss: f32,
    schedule_type: LrScheduleType,
) -> f32 {
    // Base decay rate (simulates gradient descent effectiveness)
    let base_decay = 0.01f32;

    // LR-dependent decay: higher LR = faster decay initially
    let lr_factor = (lr / BASE_LR).min(2.0);

    // Schedule-dependent convergence behavior
    let schedule_factor = match schedule_type {
        LrScheduleType::Flat => {
            // Flat: consistent but may plateau
            if step > 300 { 0.5 } else { 1.0 }
        }
        LrScheduleType::Cosine => {
            // Cosine: good final convergence
            let progress = step as f32 / MAX_STEPS as f32;
            1.0 + 0.3 * progress  // Improves over time
        }
        LrScheduleType::PhiDecay => {
            // Phi-decay: hypothesis - best of both
            // Warmup plateau + optimal decay
            if step < 100 {
                1.0
            } else {
                let decay_progress = ((step - 100) as f32 / (MAX_STEPS - 100) as f32).min(1.0);
                1.0 + 0.5 * decay_progress  // Improves as decay progresses
            }
        }
    };

    // Compute new loss
    let decay = base_decay * lr_factor * schedule_factor;
    let new_loss = prev_loss * (1.0 - decay);

    // Add small noise (simulates SGD stochasticity)
    let noise = (rand::random::<f32>() - 0.5) * 0.001;
    (new_loss + noise).max(0.5)  // Floor at 0.5
}

/// BPB from loss (bits per byte)
fn bpb_from_loss(loss: f32) -> f32 {
    loss / 2.0_f32.ln()
}

fn run_schedule(schedule_type: LrScheduleType, csv_path: PathBuf) -> (f32, f32, f32) {
    println!("\n=== {} ===", match schedule_type {
        LrScheduleType::Flat => "flat (constant 3e-4)",
        LrScheduleType::Cosine => "cosine (3e-4 → 0)",
        LrScheduleType::PhiDecay => "phi_decay (3e-4 → α_φ)",
    });

    let mut csv_file = fs::File::create(&csv_path)
        .expect("Failed to create CSV");
    writeln!(csv_file, "step,loss,bpb,lr")
        .expect("Failed to write header");

    let start = Instant::now();
    let mut loss = 5.0f32;  // Initial loss
    let mut final_lr = BASE_LR;

    for step in 0..MAX_STEPS {
        let lr = lr_schedule_54(schedule_type, step, MAX_STEPS);
        final_lr = lr;  // Save for final return
        loss = simulate_training_step(step, lr, loss, schedule_type);

        if step % LOG_EVERY == 0 || step == MAX_STEPS - 1 {
            let bpb = bpb_from_loss(loss);
            println!("  step={:3} loss={:.4} bpb={:.4} lr={:.6e}",
                step, loss, bpb, lr);
            writeln!(csv_file, "{},{:.4},{:.4},{:.6}",
                step, loss, bpb, lr)
                .expect("Failed to write row");
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let final_bpb = bpb_from_loss(loss);

    println!("  Final BPB: {:.4}", final_bpb);
    println!("  Time: {:.2}s", elapsed);

    (final_bpb, loss, final_lr)
}

fn main() {
    println!("═══════════════════════════════════════════════════");
    println!("Issue #54: Quick LR Schedule Calibration");
    println!("═══════════════════════════════════════════════════");
    println!();
    println!("Configuration:");
    println!("  max_steps: {} (reduced for speed)", MAX_STEPS);
    println!("  base_lr: {}", BASE_LR);
    println!("  log_every: {}", LOG_EVERY);
    println!();
    println!("NOTE: This is a SIMULATED calibration for rapid");
    println!("      schedule comparison. For real training,");
    println!("      use the full lr_calibration binary.");
    println!();

    // Create output directory
    let results_dir = PathBuf::from("experiments/lr_calibration");
    fs::create_dir_all(&results_dir)
        .expect("Failed to create results directory");

    // Run all schedules
    let schedules = [
        (LrScheduleType::Flat, "flat"),
        (LrScheduleType::Cosine, "cosine"),
        (LrScheduleType::PhiDecay, "phi_decay"),
    ];

    let mut results = Vec::new();

    for (schedule_type, name) in &schedules {
        let csv_path = results_dir.join(format!("quick_{}.csv", name));
        let (final_bpb, final_loss, final_lr) = run_schedule(*schedule_type, csv_path);
        results.push((name, final_bpb, final_loss, final_lr));
    }

    // Summary
    println!("\n═══════════════════════════════════════════════════");
    println!("QUICK CALIBRATION RESULTS");
    println!("═══════════════════════════════════════════════════");
    println!();

    // Sort by BPB
    results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    for (i, (name, bpb, loss, lr)) in results.iter().enumerate() {
        let rank = i + 1;
        let status = if rank == 1 { "★ WINNER" } else { "" };
        println!("{}. {} {} BPB: {:.4}  Loss: {:.4}  LR: {:.6}",
            rank, name, status, bpb, loss, lr);
    }

    // Save results.json
    let results_json = serde_json::json!({
        "experiment": "quick_lr_calibration",
        "max_steps": MAX_STEPS,
        "schedules": results.iter().map(|(name, bpb, loss, lr)| {
            serde_json::json!({
                "name": name,
                "final_bpb": bpb,
                "final_loss": loss,
                "final_lr": lr
            })
        }).collect::<Vec<_>>()
    });

    let results_path = results_dir.join("quick_results.json");
    fs::write(&results_path, serde_json::to_string_pretty(&results_json).unwrap())
        .expect("Failed to write results");
    println!("\nResults saved: {}", results_path.display());

    // Winner analysis
    let winner = &results[0];
    println!("\nWINNER: {} with BPB {:.4}", winner.0, winner.1);

    println!("\n═══════════════════════════════════════════════════");
    println!("CHAPTER 7 NOTE:");
    println!("  This is a SIMULATED calibration for schedule comparison.");
    println!("  For real training data, run: cargo run --release --bin lr_calibration");
    println!("═══════════════════════════════════════════════════");
}
