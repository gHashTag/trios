pub use tokio_postgres::{NoTls, types::Type};

use trios_igla_race::{
    neon::NeonDb,
};

/// ASHA (Asynchronous Successive Halving Algorithm) worker
///
/// Spawns ASHA worker process that continuously samples configurations,
/// trains them using trainer subprocess, updates results in Neon database,
/// and implements pruning based on results from all workers.
///
/// # Arguments
/// * `neon_url` - Neon database connection string
/// * `machine_id` - Machine identifier (for tracking purposes)
/// * `worker_id` - Unique worker identifier (for this worker)
/// * `best_bpb` - Shared RwLock tracking the best BPB found across all workers
///
/// # Returns
/// * `Result<f64>` - Final BPB value if found
///
/// # Algorithm
/// 1. Sample random configuration (d_model, context, lr, optimizer, etc.)
/// 2. Register trial in Neon database
/// 3. Spawn trainer subprocess with config
/// 4. Parse BPB from stdout: `BPB=X.XXXX`
/// 5. Update rung status in Neon database
/// 6. For each rung (1000, 3000, 9000, 27000):
///    a. If BPB > 2.7 at rung 1000, mark as 'pruned'
///    b. If BPB < 1.5, mark as 'completed' and return (found winner)
/// 7. Update shared best BPB if better than current
/// 8. Check pruning: kill trial if current BPB > median of all rung-N results × 1.33
///
/// # ASHA Rungs
/// Four rungs with increasing steps:
/// - Rung 1: 1000 steps (early evaluation, quick prune)
/// - Rung 2: 3000 steps (intermediate evaluation)
/// - Rung 3: 9000 steps (convergence)
/// - Rung 4: 27000 steps (final evaluation)
pub async fn run_worker(
    neon_url: &str,
    machine_id: &str,
    worker_id: u64,
    best_bpb: std::sync::Arc<std::sync::RwLock<f64>>,
) -> Result<f64> {
    let db = NeonDb::connect(neon_url).await?;
    let mut rng = rand::thread_rng();

    info!("Worker {} started", worker_id);

    loop {
        // Sample random configuration
        let d_model_val = [128, 192, 256, 384]
            .choose(&mut rng)
            .ok_or_else(|| anyhow::anyhow!("No d_model"))?;
        let context_val = [4, 5, 6, 7, 8]
            .choose(&mut rng)
            .ok_or_else(|| anyhow::anyhow!("No context"))?;

        let config = trios_igla_race::lessons::TrialConfig {
            d_model: Some(d_model_val),
            context: Some(context_val),
            lr: Some(rng.gen_range(0.0001..0.01)),
            optimizer: Some(if rng.gen_bool(0.5) {
                "adamw".to_string()
            } else {
                "muon".to_string()
            }),
            weight_decay: Some(rng.gen_range(0.001..0.1)),
            use_attention: Some(rng.gen_bool(0.5)),
            hidden: Some(384),
            n_layers: Some(1),
            activation: Some("relu".to_string()),
            dropout: Some(0.0),
            warmup_steps: Some(0),
            max_steps: Some(27000),
        };

        let config_value = serde_json::to_value(&config)?;

        // Register trial in Neon database
        let trial_id = db.register_trial(
            uuid::Uuid::new_v4(),
            machine_id,
            worker_id as i32,
            &serde_json::to_string(&config_value)?,
        ).await?;

        let mut prev_bpb = f64::MAX;
        let mut pruned = false;

        // ASHA rungs: 1000, 3000, 9000, 27000
        for rung in [1000, 3000, 9000, 27000] {
            let step = rung;

            // Spawn trainer subprocess
            let mut cmd = std::process::Command::new(&db.client().path().to_str_lossy());
            cmd.arg("--seed").arg(&worker_id.to_string());
            cmd.arg("--steps").arg(&step.to_string());
            cmd.arg("--hidden").arg(&config.hidden.unwrap_or(384).to_string());
            cmd.arg("--context").arg(&config.context.unwrap_or(6).to_string());
            cmd.arg("--lr").arg(&config.lr.unwrap_or(0.004).to_string());
            cmd.arg("--arch").arg(&"ngram".to_string());
            cmd.arg("--exp-id").arg(&worker_id.to_string());
            cmd.arg("--repo").arg(&"/Users/playra/trios".to_string());
            cmd.arg("--branch").arg(&"main".to_string());

            let stdout = cmd.output();
            let bpb = trios_igla_race::asha::parse_bpb(&stdout)?;

            // Update rung status in Neon database
            let query = format!(
                "UPDATE igla_race_trials SET rung_{}_step = $1, rung_{}_bpb = $2, final_step = $1, final_bpb = $2 WHERE trial_id = $3",
                step, step
            );

            let step_i32 = step as i32;
            let bpb_param = bpb;
            db.client().execute(&query, &[&step_i32, bpb_param, trial_id]).await?;

            // ASHA prune: check at rung 1000
            if bpb > 2.7 && step == 1000 {
                db.client().execute(
                    "UPDATE igla_race_trials SET status = 'pruned' WHERE trial_id = $1",
                    &[trial_id],
                ).await?;
                pruned = true;
                break;
            }

            // ASHA completion: check if found winner (BPB < 1.5)
            if bpb < 1.5 {
                db.client().execute(
                    "UPDATE igla_race_trials SET status = 'completed', final_bpb = $1 WHERE trial_id = $2",
                    &[bpb, trial_id],
                ).await?;
                return Ok(bpb);
            }

            prev_bpb = bpb;
        }

        // Final update: mark as completed if not pruned
        if !pruned {
            db.client().execute(
                "UPDATE igla_race_trials SET status = 'completed', final_bpb = $1 WHERE trial_id = $2",
                &[prev_bpb, trial_id],
            ).await?;
        }

        // Update shared best BPB (if this is main race worker)
        if let Ok(best) = best_bpb.try_write() {
            if prev_bpb < *best {
                *best = prev_bpb;
            }
        }

        info!("Worker {} finished with best BPB: {}", worker_id, best_bpb.read().unwrap_or(f64::MAX));

        Ok(best_bpb.read().unwrap_or(f64::MAX))
}
