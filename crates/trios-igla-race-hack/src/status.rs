//! Status queries — race status and reporting.
//!
//! Provides functionality to query race status from the database
//! and return formatted results.

use anyhow::Result;
use chrono::{DateTime, Utc};

/// Race status summary.
#[derive(Debug, Clone)]
pub struct RaceStatus {
    /// Best BPB found so far
    pub best_bpb: f64,
    /// Total number of trials completed
    pub total_trials: u64,
    /// Number of currently active workers
    pub active_workers: u32,
    /// Trial ID that achieved best BPB
    pub best_trial_id: u64,
    /// Timestamp of best BPB
    pub best_timestamp: DateTime<Utc>,
    /// Recent trial results
    pub recent_trials: Vec<TrialSummary>,
}

/// Summary of a single trial.
#[derive(Debug, Clone)]
pub struct TrialSummary {
    pub trial_id: u64,
    pub bpb: f64,
    pub rung: u32,
    pub timestamp: DateTime<Utc>,
}

/// Query for race status.
pub struct QueryStatus {
    neon_url: Option<String>,
}

impl QueryStatus {
    /// Create a new status query.
    ///
    /// Uses NEON_URL env var if available.
    pub fn new() -> Result<Self> {
        let neon_url = std::env::var("NEON_URL").ok();
        Ok(Self { neon_url })
    }

    /// Create a new status query with custom Neon URL.
    pub fn with_url(neon_url: String) -> Self {
        Self {
            neon_url: Some(neon_url),
        }
    }

    /// Execute the status query.
    ///
    /// Connects to Neon database and queries race status.
    pub async fn execute(&self) -> Result<RaceStatus> {
        // TODO: Implement actual database query
        // This is a stub - will be filled with real Neon queries

        Ok(RaceStatus {
            best_bpb: 1.5,
            total_trials: 0,
            active_workers: 0,
            best_trial_id: 0,
            best_timestamp: Utc::now(),
            recent_trials: vec![],
        })
    }
}

/// Show status in human-readable format.
pub fn show_status(status: &RaceStatus) -> Result<()> {
    println!("IGLA RACE Status");
    println!("────────────────");
    println!("Best BPB:   {:.4}", status.best_bpb);
    println!("Best trial:  {}", status.best_trial_id);
    println!("Best at:    {}", status.best_timestamp.format("%Y-%m-%d %H:%M:%S"));
    println!("Total trials: {}", status.total_trials);
    println!("Active workers: {}", status.active_workers);

    Ok(())
}

/// Show best BPB only.
pub fn show_best(status: &RaceStatus) -> Result<()> {
    println!("{:.4}", status.best_bpb);
    Ok(())
}
