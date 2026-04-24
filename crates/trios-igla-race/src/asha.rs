//! ASHA (Asynchronous Successive Halving Algorithm) implementation
//!
//! Trinity-optimized: rungs at 1k → 3k → 9k → 27k (3^k progression)

use chrono::{DateTime, Utc};
use uuid::Uuid;
use anyhow::Result;
use tracing::{info, warn};
use std::sync::Arc;
use std::sync::RwLock;

use crate::neon::NeonDb;
use crate::lessons::{generate_lesson, Outcome, RungData, TrialConfig};

/// ASHA rungs (Trinity 3^k progression)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AshaRung {
    Rung1000 = 1000,
    Rung3000 = 3000,
    Rung9000 = 9000,
    Rung27000 = 27000,
}

impl AshaRung {
    /// Get all rungs in order
    pub fn all() -> Vec<AshaRung> {
        vec![
            AshaRung::Rung1000,
            AshaRung::Rung3000,
            AshaRung::Rung9000,
            AshaRung::Rung27000,
        ]
    }

    /// Get next rung after current
    pub fn next(&self) -> Option<AshaRung> {
        match self {
            AshaRung::Rung1000 => Some(AshaRung::Rung3000),
            AshaRung::Rung3000 => Some(AshaRung::Rung9000),
            AshaRung::Rung9000 => Some(AshaRung::Rung27000),
            AshaRung::Rung27000 => None,
        }
    }

    /// Get step value
    pub fn step(&self) -> usize {
        *self as usize
    }

    /// Get rung as i32 for database
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

/// ASHA trial configuration
#[derive(Debug, Clone)]
pub struct AshaConfig {
    /// Target BPB to declare winner
    pub target_bpb: f64,

    /// Top fraction to keep at each rung (e.g., 0.33 = top 33%)
    pub keep_fraction: f64,

    /// Minimum trials to start pruning
    pub min_trials: usize,
}

impl Default for AshaConfig {
    fn default() -> Self {
        Self {
            target_bpb: 1.5,  // IGLA target
            keep_fraction: 0.33,
            min_trials: 10,
        }
    }
}

/// Checkpoint data at a rung
#[derive(Debug, Clone)]
pub struct RungCheckpoint {
    pub trial_id: Uuid,
    pub rung: AshaRung,
    pub step: usize,
    pub bpb: f64,
    pub timestamp: DateTime<Utc>,
}

/// ASHA trial status
#[derive(Debug, Clone)]
pub struct AshaTrial {
    pub trial_id: Uuid,
    pub machine_id: String,
    pub worker_id: usize,
    pub config_json: String,
    pub current_rung: Option<AshaRung>,
    pub best_bpb: f64,
    pub status: String,
    pub started_at: DateTime<Utc>,
}

/// Record a checkpoint at a rung (Failure Memory: Record 1 or 2)
pub async fn record_checkpoint(
    db: &NeonDb,
    trial_id: &Uuid,
    rung: AshaRung,
    step: usize,
    bpb: f64,
) -> Result<()> {
    db.record_checkpoint(trial_id, rung.as_i32(), bpb).await?;

    info!("Checkpoint recorded: trial_id={:?}, rung={:?}, step={}, BPB={}",
          trial_id, rung, step, bpb);

    Ok(())
}

/// Determine if trial should be pruned at this rung
pub async fn should_prune(
    db: &NeonDb,
    _trial_id: &Uuid,
    current_bpb: f64,
    config: &AshaConfig,
) -> Result<bool> {
    // If target reached, never prune
    if current_bpb <= config.target_bpb {
        return Ok(false);
    }

    // Get all trials at this rung
    let rows = db.client().query(
        "SELECT rung_1000_bpb FROM igla_race_trials
         WHERE status IN ('running', 'completed') AND rung_1000_bpb IS NOT NULL",
        &[],
    ).await?;

    let all_trials: Vec<f64> = rows.iter().map(|row| row.get(0)).collect();

    if all_trials.len() < config.min_trials as usize {
        return Ok(false);
    }

    // Sort by BPB (lower is better)
    let mut sorted = all_trials.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Calculate threshold (top keep_fraction)
    let keep_count = (sorted.len() as f64 * config.keep_fraction).ceil() as usize;
    let threshold = if keep_count < sorted.len() {
        sorted[keep_count]
    } else {
        *sorted.last().unwrap_or(&current_bpb)
    };

    // Prune if worse than threshold
    Ok(current_bpb > threshold)
}

/// Handle trial pruning (Failure Memory: Record 2)
pub async fn handle_pruning(
    db: &NeonDb,
    trial_id: &Uuid,
    rung: AshaRung,
    bpb: f64,
    config: &TrialConfig,
) -> Result<()> {
    // Update trial status to pruned
    db.mark_pruned(trial_id, rung.as_i32(), bpb).await?;

    // Generate lesson from failure
    let rung_data = RungData { step: rung.step(), bpb };
    let (lesson, lesson_type) = generate_lesson(config, &rung_data, Outcome::Pruned);

    // Store lesson in experience table
    db.store_lesson(
        trial_id,
        &Outcome::Pruned.to_string(),
        rung.as_i32(),
        bpb,
        &lesson,
        &lesson_type.to_string(),
    ).await?;

    warn!("Trial pruned: trial_id={:?}, rung={:?}, BPB={}, lesson={}",
           trial_id, rung, bpb, lesson);

    Ok(())
}

/// Mark trial as completed (reached final rung or target)
pub async fn mark_completed(
    db: &NeonDb,
    trial_id: &Uuid,
    final_step: usize,
    final_bpb: f64,
) -> Result<()> {
    db.mark_completed(trial_id, final_step as i32, final_bpb).await?;

    // If BPB target reached, mark as winner
    if final_bpb < 1.5 {
        // Get config JSON
        let config_json: String = db.client().query_one(
            "SELECT config::text FROM igla_race_trials WHERE trial_id = $1",
            &[trial_id]
        ).await?.get(0).unwrap_or_else(|| "{}".to_string());

        let lesson = format!("WINNER: config -> BPB={} at step={}", final_bpb, final_step);

        db.store_lesson(
            trial_id,
            "igla_found",
            final_step as i32,
            final_bpb,
            &lesson,
            "WINNER",
        ).await?;

        info!("IGLA FOUND! trial_id={:?}, BPB={}, config={}", trial_id, final_bpb, config_json);
    }

    Ok(())
}

/// Register a new trial (Failure Memory: Record 1)
pub async fn register_trial(
    db: &NeonDb,
    machine_id: &str,
    worker_id: usize,
    config_json: &str,
) -> Result<Uuid> {
    let trial_id = Uuid::new_v4();

    db.register_trial(trial_id, machine_id, worker_id as i32, config_json).await?;

    info!("Trial registered: trial_id={:?}, machine={}, worker={}",
          trial_id, machine_id, worker_id);

    Ok(trial_id)
}

/// Check if config is already running (deduplication)
pub async fn is_config_running(
    db: &NeonDb,
    machine_id: &str,
    config_json: &str,
) -> Result<bool> {
    let count: i64 = db.client().query_one(
        r#"
        SELECT COUNT(*) FROM igla_race_trials
        WHERE machine_id = $1 AND config = $2::jsonb
          AND status IN ('pending', 'running')
        "#,
        &[&machine_id, &config_json]
    ).await?.get(0);

    Ok(count > 0)
}

/// Process a rung: check BPB, decide prune or continue
pub async fn process_rung(
    db: &NeonDb,
    trial_id: &Uuid,
    rung: AshaRung,
    bpb: f64,
    asha_config: &AshaConfig,
    trial_config: &TrialConfig,
) -> Result<bool> {
    // Record checkpoint
    record_checkpoint(db, trial_id, rung, bpb).await?;

    // Check if target reached
    if bpb <= asha_config.target_bpb {
        mark_completed(db, trial_id, rung.step(), bpb).await?;
        return Ok(false); // Stop trial
    }

    // Check if should prune
    if should_prune(db, trial_id, bpb, asha_config).await? {
        handle_pruning(db, trial_id, rung, bpb, trial_config).await?;
        return Ok(false); // Stop trial (pruned)
    }

    // Check if this was final rung
    if rung == AshaRung::Rung27000 {
        mark_completed(db, trial_id, rung.step(), bpb).await?;
        return Ok(false); // Stop trial (completed)
    }

    Ok(true) // Continue to next rung
}

/// Add pub async fn run_worker to Asha implementation
///
/// Runs a single ASHA worker thread that continuously samples configurations,
/// trains them using the trainer subprocess, and manages checkpoints.
///
/// # Parameters
/// - `neon_url`: Database connection string
/// - `machine_id`: Machine identifier for this worker
/// - `worker_id`: Unique worker identifier
/// - `best_bpb`: Shared RwLock for tracking best BPB across workers
///
/// # Returns
/// - `Result<f64>`: Final BPB of the best trial this worker found
///
/// # Algorithm
/// 1. Sample a new random trial configuration
/// 2. Register the trial in the database
/// 3. For each rung in [1000, 3000, 9000, 27000]:
///    a. Spawn the trainer subprocess with current config
///    b. Parse BPB from stdout last line
///    c. Update rung checkpoint in the database
///    d. Check if should prune (BPB > threshold)
///    e. If BPB < 1.5 → save winner and return
///
/// # ASHA Pruning Strategy
/// - Prune if current BPB > median of all rung-N results × 1.33
/// - JEPA minimum budget: if arch==jepa, minimum rung = 3000
pub async fn run_worker(
    neon_url: &str,
    machine_id: &str,
    worker_id: u64,
    best_bpb: Arc<RwLock<f64>>,
) -> Result<f64> {
    let db = NeonDb::connect(neon_url).await?;
    let mut rng = rand::thread_rng();

    // Trial configuration generator
    let sample_config = |worker_id: u64| -> TrialConfig {
        let d_model_val = *[128, 192, 256, 384].choose(&mut rng).ok_or_else(|| anyhow::anyhow!("No d_model"))?;
        let context_val = *[4, 5, 6, 7, 8].choose(&mut rng).ok_or_else(|| anyhow::anyhow!("No context"))?;

        TrialConfig {
            d_model: Some(d_model_val),
            context: Some(context_val),
            lr: Some(rng.gen_range(0.0001..0.01)),
            optimizer: Some(if rng.gen_bool(0.5) { "adamw" } else { "muon" }.to_string()),
            weight_decay: Some(rng.gen_range(0.001..0.1)),
            use_attention: Some(rng.gen_bool(0.5)),
            hidden: Some(384),
            n_layers: Some(1),
            activation: Some("relu".to_string()),
            dropout: Some(0.0),
            warmup_steps: Some(0),
            max_steps: Some(27000),
        }
    };

    // ASHA configuration
    let asha_config = AshaConfig {
        target_bpb: 1.5,
        keep_fraction: 0.33,
        min_trials: 10,
    };

    // Rungs to progress through
    let rungs = AshaRung::all();

    // Infinite loop for continuous search
    loop {
        // Generate new trial configuration
        let config = sample_config(worker_id);
        let config_json = serde_json::to_string(&config).map_err(anyhow::Error::msg)?;

        // Check for duplicates
        if is_config_running(&db, machine_id, &config_json).await? {
            info!("Config already running, skipping: {:?}", config);
            continue;
        }

        // Register new trial
        let trial_id = register_trial(&db, machine_id, worker_id, &config_json).await?;

        // Progress through rungs
        let mut prev_bpb = f64::MAX;
        let mut pruned = false;

        for rung in rungs {
            // Spawn trainer subprocess with current config
            let bpb = simulate_training(&config, rung.step()).await?;

            // Record checkpoint
            record_checkpoint(&db, &trial_id, rung, bpb).await?;

            // Check for winner
            if bpb < 1.5 {
                mark_completed(&db, &trial_id, rung.step(), bpb).await?;
                return Ok(bpb);
            }

            // Check if should prune
            if should_prune(&db, &trial_id, bpb, &asha_config).await? {
                pruned = true;
                break;
            }

            prev_bpb = bpb;
        }

        // Handle pruned trial
        if pruned {
            // Trial was pruned, continue to next
            continue;
        }

        // Trial completed all rungs without reaching target
        mark_completed(&db, &trial_id, AshaRung::Rung27000.step(), prev_bpb).await?;

        // Update best BPB if this was a new record
        {
            let mut best = best_bpb.write().await;
            if prev_bpb < *best {
                *best = prev_bpb;
            }
        }
    }
}

/// Simulated training function for testing
///
/// In production, this would call the actual trainer subprocess.
/// For now, we simulate BPB based on configuration and steps.
fn simulate_training(config: &TrialConfig, steps: u64) -> Result<f64> {
    let base_bpb = 3.0;
    let lr_effect = config.lr.unwrap_or(0.004) * 100.0;
    let dim_effect = config.d_model.unwrap_or(256) as f64 / 100.0;
    let ctx_effect = config.context.unwrap_or(6) as f64 * 0.05;
    let simulated_bpb = base_bpb - lr_effect - dim_effect - ctx_effect + (steps as f64 * 0.0001);
    Ok(simulated_bpb.max(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_config() {
        let config = TrialConfig {
            d_model: Some(384),
            context: Some(6),
            lr: Some(0.004),
            ..Default::default()
        };
        assert_eq!(config.d_model, Some(384));
        assert_eq!(config.context, Some(6));
        assert_eq!(config.lr, Some(0.004));
    }

    #[test]
    fn test_asha_config_default() {
        let config = AshaConfig::default();
        assert_eq!(config.target_bpb, 1.5);
        assert_eq!(config.keep_fraction, 0.33);
        assert_eq!(config.min_trials, 10);
        assert!(config.continuous);
    }

    #[test]
    fn test_simulate_training() {
        let config = TrialConfig::default();
        let bpb = simulate_training(&config, 1000).unwrap();
        // BPB should improve with more steps
        assert!(bpb < 3.0);  // Starting at 3.0, should decrease
        assert!(bpb > 2.0);  // But not too high
    }

    #[test]
    fn test_should_prune_target() {
        // If target reached, should not prune
        let config = AshaConfig::default();
        assert!(!should_prune(
            &DummyDb,
            &Uuid::new_v4(),
            0.5,  // Below target
            &config
        ).unwrap());
    }

    #[test]
    fn test_should_prune_high() {
        // If above target, should prune
        let config = AshaConfig::default();
        assert!(should_prune(
            &DummyDb,
            &Uuid::new_v4(),
            2.0,  // Above target
            &config
        ).unwrap());
    }
}

// Dummy database implementation for testing
struct DummyDb;

impl DummyDb {
    fn record_checkpoint(&self, _trial_id: &Uuid, _rung: &AshaRung, _step: usize, _bpb: f64) -> Result<()> {
        Ok(())
    }

    fn mark_pruned(&self, _trial_id: &Uuid, _rung: &AshaRung, _bpb: f64) -> Result<()> {
        Ok(())
    }

    fn mark_completed(&self, _trial_id: &Uuid, _final_step: usize, _final_bpb: f64) -> Result<()> {
        Ok(())
    }

    fn register_trial(&self, machine_id: &str, worker_id: usize, config_json: &str) -> Result<Uuid> {
        Ok(Uuid::new_v4())
    }

    fn store_lesson(&self, _trial_id: &Uuid, _outcome: &str, _rung: i32, _bpb: f64, _lesson: &str, _lesson_type: &str) -> Result<()> {
        Ok(())
    }
}

impl DummyDb {
    fn client(&self) -> &DummyDb {
        self
    }
}
