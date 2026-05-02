//! Worker pool — parallel trial orchestration.
//!
//! Spawns and manages N parallel workers, each running trials
//! and reporting results back.

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use anyhow::Result;
use tracing::{info, error};

use super::trial::{execute_trial, TrialResult};
use crate::TrialConfig;

/// Worker pool configuration.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Number of workers to spawn
    pub workers: u32,
    /// Trials per worker before joining
    pub trials_per_worker: u32,
}

/// Result from a worker.
#[derive(Debug)]
pub struct WorkerResult {
    pub worker_id: u32,
    pub results: Vec<TrialResult>,
}

/// Worker pool for parallel trial execution.
pub struct WorkerPool {
    config: WorkerConfig,
    best_bpb: Arc<RwLock<f64>>,
}

impl WorkerPool {
    /// Create a new worker pool.
    pub fn new(config: WorkerConfig) -> Self {
        WorkerPool {
            config,
            best_bpb: Arc::new(RwLock::new(f64::MAX)),
        }
    }

    /// Run the worker pool to completion.
    ///
    /// Returns all trial results from all workers.
    pub async fn run(&self) -> Result<Vec<WorkerResult>> {
        info!("Starting worker pool: {} workers", self.config.workers);
        info!("Trials per worker: {}", self.config.trials_per_worker);

        let mut set = JoinSet::new();
        let mut results = Vec::new();

        // Spawn workers
        for wid in 0..self.config.workers {
            let config = self.config.clone();
            let best = Arc::clone(&self.best_bpb);
            set.spawn(async move {
                self::run_worker(wid, config, best).await
            });
        }

        // Collect results
        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(worker_result)) => {
                    info!("Worker {} completed: {} trials",
                           worker_result.worker_id,
                           worker_result.results.len());
                    results.push(worker_result);
                }
                Ok(Err(e)) => error!("Worker error: {}", e),
                Err(e) => error!("Join error: {}", e),
            }
        }

        Ok(results)
    }

    /// Check if victory has been achieved.
    pub async fn check_victory(&self) -> Result<bool> {
        let best = *self.best_bpb.read().await;
        Ok(best < 1.5)
    }

    /// Get current best BPB.
    pub async fn best_bpb(&self) -> f64 {
        *self.best_bpb.read().await
    }
}

/// Run a single worker.
async fn run_worker(
    worker_id: u32,
    config: WorkerConfig,
    best_bpb: Arc<RwLock<f64>>,
) -> Result<WorkerResult> {
    let mut results = Vec::new();

    for trial_idx in 0..config.trials_per_worker {
        // Create trial configuration
        let trial_config = TrialConfig {
            trial_id: ((worker_id as u64) * (config.trials_per_worker as u64)) + (trial_idx as u64),
            lr: 0.004, // TODO: sample from LrSampler
            d_model: 256,
            gradient_mode: trios_algorithm_arena::invariants::GradientMode::L2,
        };

        // Execute trial
        match execute_trial(&trial_config).await {
            Ok(result) => {
                results.push(result.clone());

                // Update best BPB
                if result.final_bpb < 1.5 {
                    info!("VICTORY! Worker {} trial {} BPB={:.4}",
                           worker_id, trial_idx, result.final_bpb);
                    let mut best = best_bpb.write().await;
                    *best = result.final_bpb.min(*best);
                }
            }
            Err(e) => {
                error!("Trial {} failed: {}", trial_config.trial_id, e);
            }
        }
    }

    Ok(WorkerResult { worker_id, results })
}
