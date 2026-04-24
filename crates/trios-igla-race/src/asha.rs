//! ASHA (Asynchronous Successive Halving Algorithm) implementation
//! Trinity-optimized: rungs at 1k → 3k → 9k → 27k (3^k progression)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use anyhow::Result;
use tracing::{info, warn};

use crate::neon::NeonDb;
use crate::lessons::{generate_lesson, Outcome, RungData, TrialConfig, LessonType};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AshaConfig {
    /// Target BPB to declare winner
    pub target_bpb: f64,

    /// Top fraction to keep at each rung (e.g., 0.33 = top 33%)
    pub keep_fraction: f64,

    /// Minimum trials to start pruning
    pub min_trials: usize,

    /// Whether to run infinite loop (for continuous search)
    pub continuous: bool,
}

impl Default for AshaConfig {
    fn default() -> Self {
        Self {
            target_bpb: 1.5,  // IGLA target
            keep_fraction: 0.33,
            min_trials: 10,
            continuous: true,
        }
    }
}

/// ASHA trial status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AshaTrial {
    pub trial_id: Uuid,
    pub machine_id: String,
    pub worker_id: usize,
    pub config: serde_json::Value,
    pub current_rung: Option<AshaRung>,
    pub best_bpb: f64,
    pub status: String,
    pub started_at: DateTime<Utc>,
}

/// Checkpoint data at a rung
#[derive(Debug, Clone, Serialize)]
pub struct RungCheckpoint {
    pub trial_id: Uuid,
    pub rung: AshaRung,
    pub step: usize,
    pub bpb: f64,
    pub timestamp: DateTime<Utc>,
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
    trial_id: &Uuid,
    current_bpb: f64,
    config: &AshaConfig,
) -> Result<bool> {
    // If target reached, never prune
    if current_bpb <= config.target_bpb {
        return Ok(false);
    }

    // Get all trials at this rung
    let rows = db.client().query(
        "SELECT COALESCE(rung_1000_bpb, 999) FROM igla_race_trials
         WHERE status IN ('running', 'completed') AND rung_1000_bpb IS NOT NULL",
        &[]
    ).await?;

    let all_trials: Vec<f64> = rows.iter().map(|row| row.get(0)).collect();

    if all_trials.len() < config.min_trials {
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
        let config: serde_json::Value = db.client().query_one(
            "SELECT config FROM igla_race_trials WHERE trial_id = $1",
            &[trial_id]
        ).await?.get(0);

        let lesson = format!("WINNER: config → BPB={} at step={}", final_bpb, final_step);

        db.store_lesson(
            trial_id,
            "igla_found",
            final_step as i32,
            final_bpb,
            &lesson,
            "WINNER",
        ).await?;

        info!("IGLA FOUND! trial_id={:?}, BPB={}", trial_id, final_bpb);
    }

    Ok(())
}

/// Register a new trial (Failure Memory: Record 1)
pub async fn register_trial(
    db: &NeonDb,
    machine_id: &str,
    worker_id: usize,
    config: &serde_json::Value,
) -> Result<Uuid> {
    let trial_id = Uuid::new_v4();

    db.register_trial(trial_id, machine_id, worker_id as i32, config).await?;

    info!("Trial registered: trial_id={:?}, machine={}, worker={}",
          trial_id, machine_id, worker_id);

    Ok(trial_id)
}

/// Check if config is already running (deduplication)
pub async fn is_config_running(
    db: &NeonDb,
    machine_id: &str,
    config: &serde_json::Value,
) -> Result<bool> {
    let count: i64 = db.client().query_one(
        r#"
        SELECT COUNT(*) FROM igla_race_trials
        WHERE machine_id = $1 AND config = $2
          AND status IN ('pending', 'running')
        "#,
        &[&machine_id, config]
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
    record_checkpoint(db, trial_id, rung, rung.step(), bpb).await?;

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

    // Check if this was the final rung
    if rung == AshaRung::Rung27000 {
        mark_completed(db, trial_id, rung.step(), bpb).await?;
        return Ok(false); // Stop trial (completed)
    }

    Ok(true) // Continue to next rung
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rung_progression() {
        assert_eq!(AshaRung::Rung1000.next(), Some(AshaRung::Rung3000));
        assert_eq!(AshaRung::Rung3000.next(), Some(AshaRung::Rung9000));
        assert_eq!(AshaRung::Rung9000.next(), Some(AshaRung::Rung27000));
        assert_eq!(AshaRung::Rung27000.next(), None);
    }

    #[test]
    fn test_rung_steps() {
        assert_eq!(AshaRung::Rung1000.step(), 1000);
        assert_eq!(AshaRung::Rung3000.step(), 3000);
        assert_eq!(AshaRung::Rung9000.step(), 9000);
        assert_eq!(AshaRung::Rung27000.step(), 27000);
    }

    #[test]
    fn test_asha_config_default() {
        let config = AshaConfig::default();
        assert_eq!(config.target_bpb, 1.5);
        assert_eq!(config.keep_fraction, 0.33);
        assert!(config.continuous);
    }
}
