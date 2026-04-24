//! ASHA (Asynchronous Successive Halving Algorithm) implementation (STUB for TASK-1)
//!
//! Trinity-optimized: rungs at 1k → 3k → 9k → 27k (3^k progression)
//!
//! For TASK-1, this is a stub that returns simple values without database queries.

use uuid::Uuid;
use anyhow::Result;
use tracing::{info, warn};

use crate::neon::NeonDb;
use crate::lessons::{Outcome, RungData, TrialConfig};

/// ASHA rungs (Trinity 3^k progression)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AshaRung {
    Rung1000 = 1000,
    Rung3000 = 3000,
    Rung9000 = 9000,
    Rung27000 = 27000,
}

impl AshaRung {
    /// Get all rungs in order (default NTP schedule)
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
    pub target_bpb: f64,
    pub keep_fraction: f64,
    pub min_trials: usize,
    pub continuous: bool,
    pub arch: String,
}

impl Default for AshaConfig {
    fn default() -> Self {
        Self {
            target_bpb: 1.5,
            keep_fraction: 0.33,
            min_trials: 10,
            continuous: true,
            arch: "attn".to_owned(),
        }
    }
}

/// Record a checkpoint at a rung (STUB)
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

/// Determine if trial should be pruned at this rung (STUB)
pub async fn should_prune(
    _db: &NeonDb,
    _trial_id: &Uuid,
    current_bpb: f64,
    config: &AshaConfig,
) -> Result<bool> {
    if current_bpb <= config.target_bpb {
        return Ok(false);
    }
    // STUB: simple heuristic - prune if BPB > 2.7 at first rung
    Ok(current_bpb > 2.7)
}

/// Handle trial pruning (STUB)
pub async fn handle_pruning(
    db: &NeonDb,
    trial_id: &Uuid,
    rung: AshaRung,
    bpb: f64,
    config: &TrialConfig,
) -> Result<()> {
    db.mark_pruned(trial_id, rung.as_i32(), bpb).await?;

    let rung_data = RungData { step: rung.step(), bpb };
    let (lesson, _lesson_type) = crate::lessons::generate_lesson(config, &rung_data, Outcome::Pruned);

    db.store_lesson(
        trial_id,
        &Outcome::Pruned.to_string(),
        rung.as_i32(),
        bpb,
        &lesson,
        "PATTERN",
    ).await?;

    warn!("Trial pruned: trial_id={:?}, rung={:?}, BPB={}, lesson={}",
           trial_id, rung, bpb, lesson);

    Ok(())
}

/// Mark trial as completed (STUB)
pub async fn mark_completed(
    db: &NeonDb,
    trial_id: &Uuid,
    final_step: usize,
    final_bpb: f64,
) -> Result<()> {
    db.mark_completed(trial_id, final_step as i32, final_bpb).await?;

    if final_bpb < 1.5 {
        info!("IGLA FOUND! trial_id={:?}, BPB={}", trial_id, final_bpb);
    }

    Ok(())
}

/// Register a new trial (STUB)
pub async fn register_trial(
    db: &NeonDb,
    machine_id: &str,
    worker_id: usize,
    config_json: &str,
) -> Result<Uuid> {
    let trial_id = Uuid::new_v4();
    db.register_trial(trial_id, machine_id, worker_id as i32, config_json).await?;
    Ok(trial_id)
}

/// Check if config is already running (STUB)
pub async fn is_config_running(
    db: &NeonDb,
    machine_id: &str,
    config_json: &str,
) -> Result<bool> {
    db.is_config_running(machine_id, config_json).await
}
