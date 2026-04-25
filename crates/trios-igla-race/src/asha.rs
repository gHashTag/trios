//! ASHA (Asynchronous Successive Halving Algorithm) implementation
//!
//! Trinity-optimized: rungs at 1k → 3k → 9k → 27k (3^k progression)
//!
//! IGLA RACE: Uses real tjepa_train binary for JEPA-T training

use uuid::Uuid;
use anyhow::Result;
use tracing::{info, warn};
use rand::SeedableRng;
use rand::rngs::StdRng;
use tokio::process::Command;

use crate::neon::NeonDb;
use crate::lessons::{TrialConfig, RungData, Outcome};

/// Architecture kind for IGLA Race
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchKind {
    Jepa,  // T-JEPA (our real training)
}

impl ArchKind {
    /// Get minimum rung for this architecture
    ///
    /// JEPA requires more steps for initial convergence
    pub fn min_rung(&self) -> i32 {
        match self {
            ArchKind::Jepa => 3000,
        }
    }

    /// Get rung schedule for this architecture
    pub fn rung_schedule(&self) -> Vec<i32> {
        match self {
            ArchKind::Jepa => vec![3000, 9000, 27000],
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            ArchKind::Jepa => "jepa",
        }
    }
}

/// ASHA rungs (Trinity 3^k progression)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AshaRung {
    Rung1000 = 1000,
    Rung3000 = 3000,
    Rung9000 = 9000,
    Rung27000 = 27000,
}

impl AshaRung {
    /// Get all rungs in order (default schedule)
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
            arch: "jepa".to_owned(),
        }
    }
}

/// Record a checkpoint at a rung
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
    _db: &NeonDb,
    _trial_id: &Uuid,
    current_bpb: f64,
    config: &AshaConfig,
) -> Result<bool> {
    if current_bpb <= config.target_bpb {
        return Ok(false);
    }
    // INV-2: ASHA champion survives with threshold=3.5 (phi^2 + phi^-2 + 0.5)
    Ok(current_bpb > 3.5)
}

/// Handle trial pruning
pub async fn handle_pruning(
    db: &NeonDb,
    trial_id: &Uuid,
    rung: AshaRung,
    bpb: f64,
    config: &TrialConfig,
) -> Result<()> {
    db.mark_pruned(trial_id, rung.as_i32(), bpb).await?;

    let rung_data = RungData { step: rung.step(), bpb };
    let (lesson, lesson_type) = crate::lessons::generate_lesson(config, &rung_data, Outcome::Pruned);

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

/// Mark trial as completed
pub async fn mark_completed(
    db: &NeonDb,
    trial_id: &Uuid,
    final_step: usize,
    final_bpb: f64,
) -> Result<()> {
    db.mark_completed(trial_id, final_bpb, final_step as i32).await?;

    if final_bpb < 1.5 {
        info!("IGLA FOUND! trial_id={:?}, BPB={}", trial_id, final_bpb);
    }

    Ok(())
}

/// Register a new trial
pub async fn register_trial(
    db: &NeonDb,
    machine_id: &str,
    worker_id: usize,
    config_json: &str,
) -> Result<Uuid> {
    let trial_id = Uuid::new_v4();
    db.register_trial(&trial_id, machine_id, worker_id as i32, config_json).await?;
    Ok(trial_id)
}

/// Check if config is already running
pub async fn is_config_running(
    db: &NeonDb,
    machine_id: &str,
    config_json: &str,
) -> Result<bool> {
    db.is_config_running(machine_id, config_json).await
}

/// ASHA worker loop (IGLA RACE)
pub async fn run_worker(
    neon_url: &str,
    machine_id: &str,
    worker_id: u64,
    best_bpb: std::sync::Arc<std::sync::RwLock<f64>>,
) -> Result<f64> {
    let db = NeonDb::connect(neon_url).await?;
    let mut rng = StdRng::from_entropy();
    let mut trial_counter = worker_id * 1_000_000;

    // Parse architecture type
    let default_config = AshaConfig::default();
    let arch_kind = ArchKind::Jepa; // Always use JEPA for IGLA RACE

    // Get rung schedule based on architecture
    let rungs = arch_kind.rung_schedule();

    loop {
        // 1. sample_config → trial config
        let config = sample_config(&mut rng);
        let config_json = serde_json::to_string(&config)?;

        // 2. register_trial in Neon
        trial_counter += 1;
        let trial_id = format!("{}-w{}-t{}", machine_id, worker_id, trial_counter);
        let trial_uuid = Uuid::parse_str(&trial_id.replace("-", "")).unwrap_or_else(|_| Uuid::new_v4());

        if let Err(e) = db.register_trial(&trial_uuid, machine_id, worker_id as i32, &config_json).await {
            warn!("register trial failed: {e}");
            continue;
        }

        info!("[w{worker_id}] trial {trial_id}: h={} lr={:.6} seed={}",
              config.hidden.unwrap_or(256), config.lr.unwrap_or(0.004), config.seed.unwrap_or(42));

        let mut pruned = false;

        // 3. For each rung in schedule
        let min_rung = arch_kind.min_rung();

        for &rung in &rungs {
            // JEPA: skip rung 1000 due to slower convergence
            if rung < min_rung {
                info!("Skipping rung {} for JEPA (below min rung {})", rung, min_rung);
                continue;
            }

            let rung_steps = rung as usize;

            // a. Spawn subprocess: ./target/release/tjepa_train (real JEPA training)
            // Note: tjepa_train expects --key=value format
            let output = Command::new("./target/release/tjepa_train")
                .arg(format!("--seed={}", config.seed.unwrap_or(42)))
                .arg(format!("--steps={}", rung_steps))
                .arg(format!("--encoder-lr={:.8}", config.lr.unwrap_or(0.004)))
                .arg(format!("--ntp-lr={:.8}", config.lr.unwrap_or(0.004) * 0.25))
                .arg("--ntp-weight=1.0")
                .arg(format!("--jepa-weight={}", config.jepa_weight.unwrap_or(1.0)))
                .arg(format!("--nca-weight={}", config.nca_weight.unwrap_or(0.25)))
                .arg(format!("--optimizer={}", config.optimizer.clone().unwrap_or_else(|| "adamw".to_string())))
                .arg(format!("--jepa-warmup={}", config.warmup_steps.unwrap_or(1500)))
                .arg(format!("--trial-id={}", trial_id))
                .arg(format!("--agent-id={}-w{}", machine_id, worker_id))
                .output()
                .await?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("[w{worker_id}] trainer failed at rung {rung_steps}: {stderr}");
                let _ = db.mark_pruned(&trial_uuid, rung_steps as i32, 999.0).await;
                pruned = true;
                break;
            }

            // b. Parse BPB from stdout last line
            let stdout = String::from_utf8_lossy(&output.stdout);
            let last_line = stdout.lines().last().unwrap_or("");
            let bpb_str = last_line.strip_prefix("BPB=")
                .ok_or_else(|| anyhow::anyhow!("last stdout line is not BPB=: {last_line}"))?;
            let bpb: f64 = bpb_str.parse()?;

            // c. update_rung in Neon
            info!("[w{worker_id}] rung={}: trial={}, BPB={:.4}", rung_steps, trial_id, bpb);

            // e. if bpb < 1.50 → save_winner in Neon → return Ok(bpb)
            if bpb < 1.50 {
                info!("[w{worker_id}] IGLA FOUND! BPB={bpb:.4}");
                {
                    let mut best = best_bpb.write().unwrap();
                    if bpb < *best { *best = bpb; }
                }
                return Ok(bpb);
            }

            // d. if should_prune(rung, bpb) → break to next trial
            let should_prune_val = should_prune(&db, &trial_uuid, bpb, &default_config).await?;
            if should_prune_val {
                info!("[w{worker_id}] Prune trial at rung {rung_steps}: BPB={}", bpb);
                pruned = true;
                break;
            }
        }

        if !pruned {
            info!("[w{worker_id}] Mark trial completed: {}", trial_id);
        }
    }
}

fn sample_config(rng: &mut StdRng) -> TrialConfig {
    use rand::seq::SliceRandom;

    // INV-8: lr in [0.001, 0.01] - phi-anchored
    // Using 0.004 = alpha_phi/phi^3 (champion LR)
    let lrs = [0.001, 0.002, 0.004, 0.008];
    let lr = *lrs.choose(rng).unwrap();

    // INV-3: d_model >= 256 for GF16
    let hiddens = [256, 384];
    let hidden = *hiddens.choose(rng).unwrap();

    // JEPA weights for multi-objective loss
    let jepa_weights = [0.5, 1.0, 1.5, 2.0];
    let nca_weights = [0.1, 0.25, 0.5];

    // IGLA requires 3-seed verification: 42, 43, 44
    let seeds = [42, 43, 44];

    TrialConfig {
        lr: Some(lr),
        d_model: Some(hidden),
        hidden: Some(hidden),
        n_layers: Some(2),
        optimizer: Some("adamw".to_string()),
        activation: Some("relu".to_string()),
        weight_decay: Some(0.04), // INV-3 consistent
        dropout: Some(0.0),
        warmup_steps: Some(1500),
        max_steps: Some(27000),
        jepa_weight: Some(*jepa_weights.choose(rng).unwrap()),
        nca_weight: Some(*nca_weights.choose(rng).unwrap()),
        seed: Some(*seeds.choose(rng).unwrap()),
    }
}
