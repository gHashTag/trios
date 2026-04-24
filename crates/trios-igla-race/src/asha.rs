//! ASHA (Asynchronous Successive Halving Algorithm) worker

use anyhow::Result;
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::Serialize;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{info, error};
use uuid::Uuid;

use crate::NeonDb;

#[derive(Debug, Serialize)]
struct TrialConfig {
    arch: String,
    d_model: usize,
    context: usize,
    lr: f64,
    seed: u64,
}

pub async fn run_worker(
    neon_url: &str,
    machine_id: &str,
    worker_id: u64,
    best_bpb: Arc<RwLock<f64>>,
) -> Result<f64> {
    let db = NeonDb::connect(neon_url).await?;
    let mut rng = StdRng::from_entropy();

    loop {
        // Sample random hyperparameters
        let config = sample_config(worker_id, &mut rng);
        let config_json = serde_json::to_string(&config)?;
        let trial_id = Uuid::new_v4();

        // Register trial in database
        db.register_trial(trial_id, machine_id, worker_id as i32, &config_json).await?;

        info!(
            "worker={} trial={} arch={} d={} ctx={} lr={:.4}",
            worker_id, trial_id, config.arch, config.d_model, config.context, config.lr
        );

        // ASHA rungs: 1000, 3000, 9000, 27000
        let rungs = [1000i32, 3000, 9000, 27000];
        let mut final_bpb = f64::MAX;

        for rung in &rungs {
            match run_training_step(&config, rung).await {
                Ok(bpb) => {
                    // Update database with current rung BPB
                    db.record_checkpoint(&trial_id, *rung, bpb).await?;

                    info!("worker={} rung={} BPB={:.4}", worker_id, rung, bpb);

                    // ASHA prune: check if worse than median * 1.33
                    let median_bpb = {
                        let best = best_bpb.read().unwrap();
                        *best * 1.33
                    };
                    if bpb > median_bpb && *rung == 1000 {
                        db.mark_pruned(&trial_id, *rung, bpb).await?;
                        info!("worker={} trial={} pruned at rung {} (bpb={:.4} > median={:.4})",
                              worker_id, trial_id, rung, bpb, median_bpb);
                        break;
                    }

                    if bpb < 1.50 {
                        // Success! Complete trial
                        db.mark_completed(&trial_id, *rung, bpb).await?;

                        // Update global best
                        {
                            let mut best = best_bpb.write().unwrap();
                            if bpb < *best {
                                *best = bpb;
                            }
                        }

                        return Ok(bpb);
                    }

                    final_bpb = bpb;
                }
                Err(e) => {
                    // Trainer failed - mark trial as failed and continue
                    error!("worker={} rung={} trainer failed: {}", worker_id, rung, e);
                    // Small backoff before retry
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    break;
                }
            }
        }

        // Trial pruned after all rungs
        if final_bpb != f64::MAX {
            db.mark_pruned(&trial_id, 27000, final_bpb).await?;
        }
    }
}

fn sample_config(worker_id: u64, rng: &mut StdRng) -> TrialConfig {
    let archs = ["ngram", "attn", "hybrid"];
    let dims = [192usize, 256, 384];
    let ctxs = [4usize, 6, 8];

    let arch = archs[rng.gen_range(0..archs.len())].to_string();
    let d_model = dims[rng.gen_range(0..dims.len())];
    let context = ctxs[rng.gen_range(0..ctxs.len())];
    let lr: f64 = rng.gen_range(1e-4_f64..1e-2_f64);
    let seed: u64 = worker_id * 1000 + rng.gen_range(0..1000);

    TrialConfig {
        arch,
        d_model,
        context,
        lr,
        seed,
    }
}

async fn run_training_step(config: &TrialConfig, rung: &i32) -> Result<f64> {
    use tokio::process::Command as TokioCommand;

    // Calculate timeout based on rung size (30 seconds per 1000 steps)
    let timeout = Duration::from_secs(*rung as u64 / 100 * 3);

    info!("Starting trainer: arch={} steps={} timeout={:?}", config.arch, rung, timeout);

    let output = tokio::time::timeout(
        timeout,
        TokioCommand::new("./target/release/trios-igla-trainer")
            .args([
                "--seed", &config.seed.to_string(),
                "--steps", &rung.to_string(),
                "--hidden", &config.d_model.to_string(),
                "--context", &config.context.to_string(),
                "--lr", &config.lr.to_string(),
                "--arch", &config.arch,
                "--exp-id", "igla-race",
            ])
            .output()
    ).await
    .map_err(|_| anyhow::anyhow!("Trainer timeout after {:?}", timeout))??;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Trainer exited with {}: {}", output.status, stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse BPB from stdout (format: BPB=X.XXXX)
    stdout
        .lines()
        .rev()
        .find(|l| l.starts_with("BPB="))
        .and_then(|l| l.trim_start_matches("BPB=").trim().parse().ok())
        .ok_or_else(|| anyhow::anyhow!("Failed to parse BPB from stdout"))
}
