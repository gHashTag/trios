//! Helper functions for Neon database operations

use tokio_postgres::Client;
use anyhow::Result;
use uuid::Uuid;

/// Register trial in Neon
pub async fn register_trial(
    client: &Client,
    trial_id: Uuid,
    machine_id: &str,
    worker_id: i32,
    config: &serde_json::Value,
) -> Result<()> {
    let config_str = serde_json::to_string(config)?;
    client.execute(
        "INSERT INTO igla_race_trials (trial_id, machine_id, worker_id, config, status, started_at)
         VALUES ($1, $2, $3, $4, 'running', NOW())",
        &[&trial_id, &machine_id, &worker_id, &config_str],
    ).await?;
    Ok(())
}
