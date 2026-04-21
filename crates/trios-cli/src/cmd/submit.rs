//! `tri submit` — Submit parameters to experiment
//!
//! Usage:
//!   tri submit --bpb 6.5609 --artifact model.safetensors

use anyhow::{Context, Result};
use std::process::Command;

use crate::{config::Config, metrics::validate_bpb};

/// Submit experiment result with BPB and artifact
pub fn submit(bpb: f64, artifact: &str) -> Result<()> {
    println!("Submitting result: bpb={:.4} artifact={}", bpb, artifact);

    validate_bpb(bpb).context("Invalid BPB value")?;

    if !std::path::Path::new(artifact).exists() {
        anyhow::bail!("Artifact not found: {}", artifact);
    }

    let _config = Config::load();

    let branch = current_branch()?;
    println!("  BPB: {:.4}", bpb);
    println!("  Artifact: {}", artifact);
    println!("  Branch: {}", branch);

    let title = format!("feat(golf): submit bpb={:.4} artifact={}", bpb, artifact);
    let body = format!(
        "## Parameter Golf Submission\n\n- BPB: {:.4}\n- Artifact: `{}`\n- Branch: `{}`\n\nCloses #110",
        bpb, artifact, branch
    );

    let status = Command::new("gh")
        .args([
            "pr", "create", "--title", &title, "--body", &body, "--base", "dev",
        ])
        .status()?;

    if status.success() {
        println!("PR created successfully");
    } else {
        println!("PR creation failed (gh may need auth or branch may be ahead)");
        println!(
            "Manual: gh pr create --title '{}' --body '...' --base dev",
            title
        );
    }

    Ok(())
}

fn current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
