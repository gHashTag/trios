//! `tri submit` — Submit parameters to experiment
//!
//! Usage:
//!   tri submit --bpb 6.5609 --artifact model.safetensors

use anyhow::{Context, Result};

use crate::{
    config::Config,
    metrics::{validate_bpb, validate_param_count},
};

/// Submit experiment result with BPB and artifact
pub fn submit(bpb: f64, artifact: &str) -> Result<()> {
    println!("📤 Submitting result: bpb={:.4} artifact={}", bpb, artifact);

    // Validate BPB
    validate_bpb(bpb)
        .context("Invalid BPB value")?;

    // Check artifact exists
    if !std::path::Path::new(artifact).exists() {
        anyhow::bail!("Artifact not found: {}", artifact);
    }

    let config = Config::load();

    println!("  Agent: {}", config.agent_nato);
    println!("  BPB: {:.4}", bpb);
    println!("  Artifact: {}", artifact);

    // TODO: Create PR with artifact or upload to release
    println!("✓ Submitted (PR creation not yet implemented)");

    Ok(())
}
