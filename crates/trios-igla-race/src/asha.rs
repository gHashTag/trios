//! ASHA (Asynchronous Successive Halving Algorithm) implementation
//!
//! Trinity-optimized: rungs at 1k → 3k → 9k → 27k (3^k progression)
//!
//! ASHA progressively prunes trials that don't perform well at intermediate checkpoints.
//! At each rung, only the top 33% of trials continue.
//!
//! ## TASK-5A.7 — Arch-aware rung schedules
//!
//! Law L-R10: JEPA first rung = 3000 (1.4× slower convergence vs NTP/N-gram).
//! Use `AshaSchedule::for_arch(arch)` to get the correct rung list.
//! Pure functions are in `trios-train-cpu::objective::{get_rung_schedule, should_skip_rung}`.

use chrono::{DateTime, Utc};
use uuid::Uuid;
use anyhow::Result;
use tracing::{info, warn};

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

    /// Construct from step count (returns None if not a valid rung)
    pub fn from_step(step: u32) -> Option<AshaRung> {
        match step {
            1000 => Some(AshaRung::Rung1000),
            3000 => Some(AshaRung::Rung3000),
            9000 => Some(AshaRung::Rung9000),
            27000 => Some(AshaRung::Rung27000),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// TASK-5A.7 — Arch-aware ASHA schedule
// Law L-R10: JEPA uses 3000-step first rung (1.4× slower convergence)
// Pure stat functions live in trios-train-cpu::objective
// ---------------------------------------------------------------------------

/// Architecture-specific ASHA rung schedule.
#[derive(Debug, Clone)]
pub struct AshaSchedule {
    pub arch: String,
    rungs: Vec<AshaRung>,
}

impl AshaSchedule {
    /// Build schedule for the given arch.
    ///
    /// | arch    | rungs                     | law      |
    /// |---------|---------------------------|----------|
    /// | `jepa`  | 3000 → 9000 → 27000       | L-R10    |
    /// | `hybrid`| 2000 → 6000 → 18000 (≈)  | default  |
    /// | `attn`  | 1000 → 3000 → 9000 → 27000| default  |
    /// | other   | 1000 → 3000 → 9000 → 27000| default  |
    pub fn for_arch(arch: &str) -> Self {
        let steps: Vec<u32> = match arch {
            "jepa" => vec![3000, 9000, 27000],
            // hybrid rounds to existing rung boundaries
            "hybrid" => vec![3000, 9000, 27000],
            _ => vec![1000, 3000, 9000, 27000],
        };
        let rungs = steps
            .into_iter()
            .filter_map(AshaRung::from_step)
            .collect();
        Self { arch: arch.to_owned(), rungs }
    }

    /// Ordered list of rungs for this schedule.
    pub fn rungs(&self) -> &[AshaRung] {
        &self.rungs
    }

    /// First rung step count.
    pub fn first_rung_steps(&self) -> usize {
        self.rungs.first().map(|r| r.step()).unwrap_or(1000)
    }

    /// True if this rung should be skipped for the arch.
    /// (e.g., `Rung1000` is skipped for `jepa`).
    pub fn skips_rung(&self, rung: AshaRung) -> bool {
        !self.rungs.contains(&rung)
    }

    /// Next rung after `current`, respecting arch schedule.
    pub fn next_rung(&self, current: AshaRung) -> Option<AshaRung> {
        let pos = self.rungs.iter().position(|&r| r == current)?;
        self.rungs.get(pos + 1).copied()
    }
}

// ---------------------------------------------------------------------------

/// ASHA trial configuration
#[derive(Debug, Clone)]
pub struct AshaConfig {
    /// Target BPB to declare winner
    pub target_bpb: f64,

    /// Top fraction to keep at each rung (e.g., 0.33 = top 33%)
    pub keep_fraction: f64,

    /// Minimum trials to start pruning
    pub min_trials: usize,

    /// Whether to run infinite loop (for continuous search)
    pub continuous: bool,

    /// Architecture name — controls rung schedule (TASK-5A.7)
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

impl AshaConfig {
    /// Convenience: create config for a specific arch.
    pub fn for_arch(arch: &str) -> Self {
        Self { arch: arch.to_owned(), ..Self::default() }
    }

    /// Get the arch-aware schedule.
    pub fn schedule(&self) -> AshaSchedule {
        AshaSchedule::for_arch(&self.arch)
    }

    /// True if `rung` should be skipped for this config's arch.
    pub fn skips_rung(&self, rung: AshaRung) -> bool {
        self.schedule().skips_rung(rung)
    }
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

/// Checkpoint data at a rung
#[derive(Debug, Clone)]
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
    _trial_id: &Uuid,
    current_bpb: f64,
    config: &AshaConfig,
) -> Result<bool> {
    if current_bpb <= config.target_bpb {
        return Ok(false);
    }

    let rows = db.client().query(
        "SELECT COALESCE(rung_1000_bpb, 999) FROM igla_race_trials
         WHERE status IN ('running', 'completed') AND rung_1000_bpb IS NOT NULL",
        &[]
    ).await?;

    let all_trials: Vec<f64> = rows.iter().map(|row| row.get(0)).collect();

    if all_trials.len() < config.min_trials {
        return Ok(false);
    }

    let mut sorted = all_trials.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let keep_count = (sorted.len() as f64 * config.keep_fraction).ceil() as usize;
    let threshold = if keep_count < sorted.len() {
        sorted[keep_count]
    } else {
        *sorted.last().unwrap_or(&current_bpb)
    };

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
    db.mark_pruned(trial_id, rung.as_i32(), bpb).await?;

    let rung_data = RungData { step: rung.step(), bpb };
    let (lesson, lesson_type) = generate_lesson(config, &rung_data, Outcome::Pruned);

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

    if final_bpb < 1.5 {
        let config_json: String = db.client().query_one(
            "SELECT config::text FROM igla_race_trials WHERE trial_id = $1",
            &[trial_id]
        ).await?.get(0);

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
///
/// TASK-5A.7: respects arch-aware schedule — skips rung if not in schedule.
pub async fn process_rung(
    db: &NeonDb,
    trial_id: &Uuid,
    rung: AshaRung,
    bpb: f64,
    asha_config: &AshaConfig,
    trial_config: &TrialConfig,
) -> Result<bool> {
    // Skip rung if not in arch schedule (e.g. Rung1000 for jepa)
    if asha_config.skips_rung(rung) {
        info!("Rung {:?} skipped for arch='{}' (Law L-R10)", rung, asha_config.arch);
        return Ok(true); // continue without recording
    }

    record_checkpoint(db, trial_id, rung, rung.step(), bpb).await?;

    if bpb <= asha_config.target_bpb {
        mark_completed(db, trial_id, rung.step(), bpb).await?;
        return Ok(false);
    }

    if should_prune(db, trial_id, bpb, asha_config).await? {
        handle_pruning(db, trial_id, rung, bpb, trial_config).await?;
        return Ok(false);
    }

    if rung == AshaRung::Rung27000 {
        mark_completed(db, trial_id, rung.step(), bpb).await?;
        return Ok(false);
    }

    Ok(true)
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
        assert_eq!(config.arch, "attn");
    }

    // --- TASK-5A.7 ---

    #[test]
    fn test_jepa_schedule_first_rung_3000() {
        let s = AshaSchedule::for_arch("jepa");
        assert_eq!(s.first_rung_steps(), 3000, "Law L-R10: JEPA first rung must be 3000");
    }

    #[test]
    fn test_jepa_skips_rung_1000() {
        let s = AshaSchedule::for_arch("jepa");
        assert!(s.skips_rung(AshaRung::Rung1000),
            "JEPA must skip Rung1000 per Law L-R10");
    }

    #[test]
    fn test_jepa_does_not_skip_rung_3000() {
        let s = AshaSchedule::for_arch("jepa");
        assert!(!s.skips_rung(AshaRung::Rung3000));
    }

    #[test]
    fn test_attn_schedule_starts_at_1000() {
        let s = AshaSchedule::for_arch("attn");
        assert_eq!(s.first_rung_steps(), 1000);
        assert!(!s.skips_rung(AshaRung::Rung1000));
    }

    #[test]
    fn test_next_rung_jepa() {
        let s = AshaSchedule::for_arch("jepa");
        assert_eq!(s.next_rung(AshaRung::Rung3000), Some(AshaRung::Rung9000));
        assert_eq!(s.next_rung(AshaRung::Rung9000), Some(AshaRung::Rung27000));
        assert_eq!(s.next_rung(AshaRung::Rung27000), None);
    }

    #[test]
    fn test_asha_config_for_arch_jepa() {
        let cfg = AshaConfig::for_arch("jepa");
        assert!(cfg.skips_rung(AshaRung::Rung1000));
        assert!(!cfg.skips_rung(AshaRung::Rung3000));
    }

    #[test]
    fn test_rung_from_step() {
        assert_eq!(AshaRung::from_step(3000), Some(AshaRung::Rung3000));
        assert_eq!(AshaRung::from_step(2000), None);
    }
}
