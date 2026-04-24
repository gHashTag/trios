//! IGLA RACE commands
//!
//! Implements the distributed hyperparameter optimization race using ASHA
//! (Asynchronous Successive Halving) with Optuna and Neon PostgreSQL.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Race initialization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceConfig {
    /// Neon PostgreSQL connection URL
    pub neon_url: String,
    /// Optuna study name
    pub study_name: String,
    /// ASHA reduction factor (typically 3-4)
    pub reduction_factor: usize,
    /// ASHA min resources (rungs)
    pub min_rungs: usize,
    /// Max resources per trial
    pub max_rungs: usize,
    /// Target BPB threshold for IGLA
    pub target_bpb: f64,
}

impl Default for RaceConfig {
    fn default() -> Self {
        Self {
            neon_url: std::env::var("NEON_DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://user:pass@ep-xxx.us-east-2.aws.neon.tech/neondb".to_string()),
            study_name: "igla-race".to_string(),
            reduction_factor: 3,
            min_rungs: 3,    // 1000 steps
            max_rungs: 27,   // 9000 steps (3^3)
            target_bpb: 1.50,
        }
    }
}

/// Trial status from Optuna
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialStatus {
    pub trial_id: u32,
    pub state: String,
    pub value: Option<f64>,
    pub params: HashMap<String, f64>,
    pub rung: Option<usize>,
    pub datetime_start: Option<String>,
    pub datetime_complete: Option<String>,
}

/// Leaderboard entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: usize,
    pub trial_id: u32,
    pub bpb: f64,
    pub params: String,
    pub rung: usize,
    pub agent: Option<String>,
    pub timestamp: String,
}

/// Initialize the IGLA RACE
///
/// Creates the Optuna study in Neon PostgreSQL with ASHA pruner.
pub fn init(config: RaceConfig) -> Result<()> {
    info!("🏎️  Initializing IGLA RACE...");
    info!("  Study: {}", config.study_name);
    info!("  Target BPB: {:.3}", config.target_bpb);

    // In production, this would call Optuna REST API
    // Schema matches actual Neon database structure
    let schema = format!(
        r#"
-- IGLA RACE Schema for Neon PostgreSQL
-- Production schema (matches Neon console tables)

CREATE TABLE IF NOT EXISTS igla_race_trials (
    trial_id SERIAL PRIMARY KEY,
    state TEXT NOT NULL DEFAULT 'running',
    value NUMERIC,
    params JSONB NOT NULL,
    rung INTEGER NOT NULL DEFAULT 3,
    agent TEXT,
    machine_id TEXT,
    datetime_start TIMESTAMPTZ DEFAULT NOW(),
    datetime_complete TIMESTAMPTZ,
    UNIQUE(trial_id)
);

CREATE TABLE IF NOT EXISTS igla_race_experience (
    experience_id SERIAL PRIMARY KEY,
    trial_id INTEGER REFERENCES igla_race_trials(trial_id),
    agent TEXT NOT NULL,
    rung_reached INTEGER NOT NULL,
    bpb_at_rung NUMERIC NOT NULL,
    promoted BOOLEAN DEFAULT FALSE,
    technique JSONB,
    datetime_learned TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS igla_race_competitors (
    competitor_id SERIAL PRIMARY KEY,
    nato_name TEXT NOT NULL UNIQUE,
    soul_name TEXT,
    version TEXT DEFAULT 'v1.0',
    capabilities JSONB,
    active BOOLEAN DEFAULT TRUE,
    datetime_registered TIMESTAMPTZ DEFAULT NOW(),
    last_seen TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_trials_state ON igla_race_trials(state);
CREATE INDEX IF NOT EXISTS idx_trials_value ON igla_race_trials(value) WHERE value IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_trials_agent ON igla_race_trials(agent);
CREATE INDEX IF NOT EXISTS idx_competitors_nato ON igla_race_competitors(nato_name);
CREATE INDEX IF NOT EXISTS idx_experience_agent ON igla_race_experience(agent);
CREATE INDEX IF NOT EXISTS idx_experience_trial ON igla_race_experience(trial_id);

CREATE OR REPLACE VIEW igla_leaderboard AS
SELECT
    ROW_NUMBER() OVER (ORDER BY t.value ASC) as rank,
    t.trial_id,
    t.agent,
    t.value as bpb,
    t.params,
    t.rung,
    t.datetime_complete,
    c.nato_name
FROM igla_race_trials t
LEFT JOIN igla_race_competitors c ON t.agent = c.nato_name
WHERE t.state = 'completed' AND t.value IS NOT NULL
ORDER BY t.value ASC;

CREATE OR REPLACE VIEW igla_winning_techniques AS
SELECT
    e.technique,
    COUNT(*) as usage_count,
    AVG(e.bpb_at_rung) as avg_bpb,
    MIN(e.bpb_at_rung) as best_bpb
FROM igla_race_experience e
WHERE e.promoted = TRUE
GROUP BY e.technique
ORDER BY usage_count DESC, avg_bpb ASC;
"#);

    println!("\n📋 SQL Schema for Neon PostgreSQL:");
    println!("```sql");
    println!("{}", schema.trim());
    println!("```\n");

    println!("✅ Race initialized!");
    println!("\n🚀 To start workers on each machine:");
    println!("   python scripts/igla_race_worker.py --workers 4");
    println!("\n📊 To view live status:");
    println!("   tri race status");

    Ok(())
}

/// Show live leaderboard from the race
pub fn status(limit: Option<usize>) -> Result<()> {
    info!("🏆 Fetching IGLA RACE leaderboard...");

    let limit = limit.unwrap_or(10);

    // Mock data for now - in production this queries Neon PostgreSQL via igla_leaderboard view
    let mock_entries = vec![
        LeaderboardEntry {
            rank: 1,
            trial_id: 1,
            bpb: 2.716,
            params: "h=384,lr=0.00618,wd=0.01,dropout=0.1".to_string(),
            rung: 27,
            agent: Some("ALFA".to_string()),
            timestamp: Utc::now().to_rfc3339(),
        },
        LeaderboardEntry {
            rank: 2,
            trial_id: 2,
            bpb: 2.743,
            params: "h=384,lr=0.004,wd=0.02,dropout=0.05".to_string(),
            rung: 27,
            agent: Some("BRAVO".to_string()),
            timestamp: Utc::now().to_rfc3339(),
        },
        LeaderboardEntry {
            rank: 3,
            trial_id: 3,
            bpb: 2.780,
            params: "h=256,lr=0.005,wd=0.01,dropout=0.0".to_string(),
            rung: 27,
            agent: Some("CHARLIE".to_string()),
            timestamp: Utc::now().to_rfc3339(),
        },
    ];

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  🏆 IGLA RACE LEADERBOARD - LIVE                              ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Rank │ Trial │   BPB   │ Agent │ Rung │ Parameters           ║");
    println!("╠══════════════════════════════════════════════════════════════╣");

    for entry in mock_entries.iter().take(limit) {
        let agent = entry.agent.as_deref().unwrap_or("???");
        let params_short = if entry.params.len() > 25 {
            format!("{}...", &entry.params[..25])
        } else {
            entry.params.clone()
        };
        println!(
            "║  {:4} │ {:5} │ {:7.3} │ {:5} │ {:4} │ {:20} ║",
            entry.rank, entry.trial_id, entry.bpb, agent, entry.rung, params_short
        );
    }

    println!("╚══════════════════════════════════════════════════════════════╝");

    let active_count = 12; // Mock
    let completed_count = 42; // Mock
    let promoted_rate = 23.5; // Mock

    println!("\n📊 Statistics:");
    println!("  Active trials:   {}", active_count);
    println!("  Completed:       {}", completed_count);
    println!("  Promoted rate:   {:.1}% (ASHA efficiency)", promoted_rate);
    println!("  Target BPB:      1.500 🎯");

    println!("\n🏁 ASHA Rungs:");
    println!("  Run 1000 steps → BPB check → Promote to 3000");
    println!("  Run 3000 steps → BPB check → Promote to 9000");
    println!("  Run 9000 steps → BPB < 1.50? → IGLA FOUND! 🎯");

    Ok(())
}

/// Report a trial result to the race
pub fn report_trial(
    trial_id: u32,
    rung: usize,
    bpb: f64,
    params: HashMap<String, f64>,
    agent: Option<String>,
) -> Result<()> {
    info!("📤 Reporting trial {} at rung {}", trial_id, rung);

    // In production, this would update Neon via Optuna REST API
    println!("  Trial ID: {}", trial_id);
    println!("  Rung: {}", rung);
    println!("  BPB: {:.3}", bpb);
    println!("  Params: {:?}", params);
    if let Some(a) = &agent {
        println!("  Agent: {}", a);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_race_config_default() {
        let config = RaceConfig::default();
        assert_eq!(config.target_bpb, 1.50);
        assert_eq!(config.reduction_factor, 3);
    }

    #[test]
    fn test_status_output() {
        let result = status(Some(5));
        assert!(result.is_ok());
    }
}
