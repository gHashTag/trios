use anyhow::{Result, Context};
use std::process::Command;
use std::fs;
use std::path::PathBuf;

/// Run an experiment and parse BPB from stdout
pub fn run(exp_id: &str, seeds: u32) -> Result<()> {
    println!("🚀 Running experiment: {exp_id} (seeds: {seeds})");

    // Find the trainer binary
    let trainer_path = find_trainer()?;

    // Run the trainer
    let output = Command::new(&trainer_path)
        .arg("--exp-id")
        .arg(exp_id)
        .arg("--seeds")
        .arg(seeds.to_string())
        .output()
        .context("Failed to execute trainer")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Trainer failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bpb = parse_bpb(&stdout)?;

    println!("✅ BPB: {bpb}");

    // Auto-push to table #143
    report_result(exp_id, bpb)?;

    Ok(())
}

fn find_trainer() -> Result<PathBuf> {
    let paths = [
        "target/debug/igla-trainer",
        "target/release/igla-trainer",
        "crates/igla-trainer/target/debug/igla-trainer",
        "crates/igla-trainer/target/release/igla-trainer",
    ];

    for path in paths {
        if PathBuf::from(path).exists() {
            return Ok(PathBuf::from(path));
        }
    }

    anyhow::bail!("Trainer binary not found. Run: cargo build --release -p igla-trainer")
}

fn parse_bpb(stdout: &str) -> Result<f64> {
    stdout
        .lines()
        .find_map(|line| {
            if line.contains("BPB") || line.contains("bpb") {
                line.split(':')
                    .nth(1)
                    .and_then(|s| s.trim().parse::<f64>().ok())
            } else {
                None
            }
        })
        .context("BPB not found in trainer output")
}

fn report_result(exp_id: &str, bpb: f64) -> Result<()> {
    println!("📊 Reporting to #143: {exp_id} → BPB {bpb}");
    // TODO: Implement auto-sync via gh CLI
    Ok(())
}
