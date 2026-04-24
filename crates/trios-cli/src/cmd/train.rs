use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrainResult {
    pub seed: u64,
    pub best_bpb: f64,
    pub steps: usize,
    pub hidden: usize,
    pub lr: f64,
    pub time_sec: f64,
}

pub fn train_cpu(seeds: Vec<u64>, steps: usize, hidden: usize, lr: f64, activation: String, parallel: bool, residual: bool, dropout: String, warmup: String, wd: String) -> Result<Vec<TrainResult>> {
    let binary = find_ngram_train()?;

    println!("=== tri train: CPU N-Gram ===");
    println!("seeds={:?} steps={} hidden={} lr={} activation={} parallel={}", seeds, steps, hidden, lr, activation, parallel);

    if parallel && seeds.len() > 1 {
        run_parallel(&binary, &seeds, steps, hidden, lr, &activation, residual, &dropout, &warmup, &wd)
    } else {
        run_sequential(&binary, &seeds, steps, hidden, lr, &activation, residual, &dropout, &warmup, &wd)
    }
}

fn find_ngram_train() -> Result<PathBuf> {
    let paths = [
        "target/release/ngram_train",
        "target/debug/ngram_train",
    ];
    for p in &paths {
        if PathBuf::from(p).exists() {
            return Ok(PathBuf::from(p));
        }
    }
    anyhow::bail!("ngram_train binary not found. Run: cargo build --release -p trios-train-cpu --bin ngram_train");
}

fn run_sequential(binary: &Path, seeds: &[u64], steps: usize, hidden: usize, lr: f64, activation: &str, residual: bool, dropout: &str, warmup: &str, wd: &str) -> Result<Vec<TrainResult>> {
    let mut results = Vec::new();
    for &seed in seeds {
        let r = run_single(binary, seed, steps, hidden, lr, activation, residual, dropout, warmup, wd)?;
        results.push(r);
    }
    Ok(results)
}

fn run_parallel(binary: &Path, seeds: &[u64], steps: usize, hidden: usize, lr: f64, activation: &str, residual: bool, dropout: &str, warmup: &str, wd: &str) -> Result<Vec<TrainResult>> {
    let mut handles = Vec::new();

    for &seed in seeds {
        let binary = binary.to_path_buf();
        let activation = activation.to_string(); // Clone for thread safety
        let dropout = dropout.to_string(); // Clone for thread safety
        let warmup = warmup.to_string(); // Clone for thread safety
        let wd = wd.to_string(); // Clone for thread safety
        let handle = std::thread::spawn(move || {
            let start = Instant::now();
            let output = Command::new(&binary)
                .arg(format!("--seed={}", seed))
                .arg(format!("--steps={}", steps))
                .arg(format!("--hidden={}", hidden))
                .arg(format!("--lr={}", lr))
                .arg(format!("--activation={}", activation))
                .arg(if residual { "--residual" } else { "" })
                .arg(format!("--dropout={}", dropout))
                .arg(format!("--warmup={}", warmup))
                .arg(format!("--wd={}", wd))
                .output()
                .context("Failed to execute ngram_train")?;

            let time_sec = start.elapsed().as_secs_f64();
            let stdout = String::from_utf8_lossy(&output.stdout);

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Seed {} failed: {}", seed, stderr);
            }

            let best_bpb = parse_best_bpb(&stdout)?;
            Ok::<TrainResult, anyhow::Error>(TrainResult {
                seed,
                best_bpb,
                steps,
                hidden,
                lr,
                time_sec,
            })
        });
        handles.push(handle);
    }

    let mut results = Vec::new();
    for h in handles {
        let r = h.join().map_err(|_| anyhow::anyhow!("Thread panicked"))??;
        println!("  seed={} bpb={:.4} time={:.0}s", r.seed, r.best_bpb, r.time_sec);
        results.push(r);
    }

    let avg = results.iter().map(|r| r.best_bpb).sum::<f64>() / results.len() as f64;
    let std = {
        let mean = avg;
        let variance = results.iter().map(|r| (r.best_bpb - mean).powi(2)).sum::<f64>() / results.len() as f64;
        variance.sqrt()
    };
    println!("\n=== RESULT: BPB {:.3} ± {:.3} ({})", avg, std, results.len());

    Ok(results)
}

fn run_single(binary: &Path, seed: u64, steps: usize, hidden: usize, lr: f64, activation: &str, residual: bool, dropout: &str, warmup: &str, wd: &str) -> Result<TrainResult> {
    let start = Instant::now();
    let output = Command::new(binary)
        .arg(format!("--seed={}", seed))
        .arg(format!("--steps={}", steps))
        .arg(format!("--hidden={}", hidden))
        .arg(format!("--lr={}", lr))
        .arg(format!("--activation={}", activation))
        .arg(if residual { "--residual" } else { "" })
        .arg(format!("--dropout={}", dropout))
        .arg(format!("--warmup={}", warmup))
        .arg(format!("--wd={}", wd))
        .output()
        .context("Failed to execute ngram_train")?;

    let time_sec = start.elapsed().as_secs_f64();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Training failed (seed={}): {}", seed, stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let best_bpb = parse_best_bpb(&stdout)?;

    println!("  seed={} bpb={:.4} time={:.0}s", seed, best_bpb, time_sec);

    Ok(TrainResult {
        seed,
        best_bpb,
        steps,
        hidden,
        lr,
        time_sec,
    })
}

fn parse_best_bpb(stdout: &str) -> Result<f64> {
    for line in stdout.lines() {
        // Parse from backup binary format: "Time: X.Xs | BPB: X.XXXX → Y.YYYY | Delta: Z.ZZZZ"
        // We want Y.YYYY (the final BPB, not the delta)
        if line.contains("Done ===") || line.contains("Delta:") {
            if let Some(arrow_pos) = line.find("→") {
                let after_arrow = &line[arrow_pos + "→".len()..];
                let val_str = after_arrow.split('|').next().unwrap_or(after_arrow).trim();
                if let Ok(v) = val_str.parse::<f64>() {
                    return Ok(v);
                }
            }
        }
        // Parse from progressive table format: "step | val_bpb | best_bpb | ms"
        if line.contains("val_bpb") && !line.contains("step") {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                if let Ok(v) = parts[2].trim().parse::<f64>() {
                    return Ok(v);
                }
            }
        }
    }
    anyhow::bail!("Could not parse BPB from training output")
}
