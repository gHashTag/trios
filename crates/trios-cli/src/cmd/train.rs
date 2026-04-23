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

pub fn train_cpu(seeds: Vec<u64>, steps: usize, hidden: usize, lr: f64, activation: String, parallel: bool) -> Result<Vec<TrainResult>> {
    let binary = find_ngram_train()?;

    println!("=== tri train: CPU N-Gram ===");
    println!("seeds={:?} steps={} hidden={} lr={} activation={} parallel={}", seeds, steps, hidden, lr, activation, parallel);

    if parallel && seeds.len() > 1 {
        run_parallel(&binary, &seeds, steps, hidden, lr, &activation)
    } else {
        run_sequential(&binary, &seeds, steps, hidden, lr, &activation)
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

fn run_sequential(binary: &Path, seeds: &[u64], steps: usize, hidden: usize, lr: f64, activation: &str) -> Result<Vec<TrainResult>> {
    let mut results = Vec::new();
    for &seed in seeds {
        let r = run_single(binary, seed, steps, hidden, lr, activation)?;
        results.push(r);
    }
    Ok(results)
}

fn run_parallel(binary: &Path, seeds: &[u64], steps: usize, hidden: usize, lr: f64, activation: &str) -> Result<Vec<TrainResult>> {
    let mut handles = Vec::new();

    for &seed in seeds {
        let binary = binary.to_path_buf();
        let activation = activation.to_string(); // Clone for thread safety
        let handle = std::thread::spawn(move || {
            let start = Instant::now();
            let output = Command::new(&binary)
                .arg(format!("--seed={}", seed))
                .arg(format!("--steps={}", steps))
                .arg(format!("--hidden={}", hidden))
                .arg(format!("--lr={}", lr))
                .arg(format!("--activation={}", activation))
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

fn run_single(binary: &Path, seed: u64, steps: usize, hidden: usize, lr: f64, activation: &str) -> Result<TrainResult> {
    let start = Instant::now();
    let output = Command::new(binary)
        .arg(format!("--seed={}", seed))
        .arg(format!("--steps={}", steps))
        .arg(format!("--hidden={}", hidden))
        .arg(format!("--lr={}", lr))
        .arg(format!("--activation={}", activation))
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
    let mut best = None;
    for line in stdout.lines() {
        if line.contains("best_bpb") || line.contains("BPB:") {
            for part in line.split('|') {
                let trimmed = part.trim();
                if trimmed.contains("best_bpb") {
                    if let Some(val_str) = trimmed.split('|').next_back().or_else(|| trimmed.split(':').next_back()) {
                        if let Ok(v) = val_str.trim().parse::<f64>() {
                            best = Some(v);
                        }
                    }
                }
            }
        }
        if line.contains("Delta:") {
            if let Some(pos) = line.find("BPB:") {
                let after = &line[pos + 4..];
                let end_pos = after.find('→').unwrap_or(after.len());
                if let Ok(v) = after[..end_pos].trim().parse::<f64>() {
                    return Ok(v);
                }
            }
            if let Some(pos) = line.find("→") {
                let after = &line[pos + "→".len()..];
                let val_str = after.split('|').next().unwrap_or(after).trim();
                let num: String = val_str.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
                if let Ok(v) = num.parse::<f64>() {
                    return Ok(v);
                }
            }
        }
    }
    best.context("Could not parse BPB from training output")
}
