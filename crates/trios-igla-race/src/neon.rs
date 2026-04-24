//! Neon PostgreSQL connection and database operations
//!
//! Provides async interface to Neon cloud PostgreSQL for:
//! - Trial registration and status tracking
//! - Checkpoint recording at ASHA rungs
//! - Lesson storage for failure memory
//! - Leaderboard queries

use tokio_postgres::{NoTls};
use anyhow::Result;
use tracing::info;
use uuid::Uuid;

/// Neon database connection
pub struct NeonDb {
    client: tokio_postgres::Client,
}

impl NeonDb {
    /// Create new Neon connection from connection string
    pub async fn connect(conn_str: &str) -> Result<Self> {
        info!("Connecting to Neon database...");

        let (client, connection) = tokio_postgres::connect(conn_str, NoTls).await?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        info!("Connected to Neon database successfully");

        Ok(Self { client })
    }

    /// Get reference to client
    pub fn client(&self) -> &tokio_postgres::Client {
        &self.client
    }

    /// Initialize database (check tables exist)
    pub async fn initialize(&self) -> Result<()> {
        let table_exists: bool = self.client
            .query_one(
                "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'igla_race_trials')",
                &[],
            )
            .await?
            .get(0);

        if table_exists {
            info!("Database tables already exist");
        } else {
            info!("Database tables not found, please run migrations");
        }

        Ok(())
    }

    /// Register a new trial
    pub async fn register_trial(
        &self,
        trial_id: Uuid,
        machine_id: &str,
        worker_id: i32,
        config_json: &str,
    ) -> Result<()> {
        self.client
            .execute(
                "INSERT INTO igla_race_trials
                 (trial_id, machine_id, worker_id, config, status, started_at)
                 VALUES ($1, $2, $3, $4::jsonb, 'running', NOW())",
                &[&trial_id, &machine_id, &worker_id, &config_json],
            )
            .await?;

        info!("Trial registered: trial_id={}, machine={}, worker={}",
              trial_id, machine_id, worker_id);

        Ok(())
    }

    /// Record checkpoint at rung
    pub async fn record_checkpoint(
        &self,
        trial_id: &Uuid,
        rung: i32,
        bpb: f64,
    ) -> Result<()> {
        let column = match rung {
            1000 => "rung_1000",
            3000 => "rung_3000",
            9000 => "rung_9000",
            27000 => "rung_27000",
            _ => return Err(anyhow::anyhow!("Invalid rung: {}", rung)),
        };

        let query = format!(
            "UPDATE igla_race_trials
             SET {}_step = $1, {}_bpb = $2, final_step = $1, final_bpb = $2
             WHERE trial_id = $3",
            column, column
        );

        self.client
            .execute(&query, &[&rung, &bpb, trial_id])
            .await?;

        info!("Checkpoint recorded: trial_id={:?}, rung={}, BPB={}",
              trial_id, rung, bpb);

        Ok(())
    }

    /// Mark trial as pruned
    pub async fn mark_pruned(
        &self,
        trial_id: &Uuid,
        rung: i32,
        bpb: f64,
    ) -> Result<()> {
        self.client
            .execute(
                "UPDATE igla_race_trials
                 SET status = 'pruned', pruned_at = NOW()
                 WHERE trial_id = $1",
                &[trial_id],
            )
            .await?;

        info!("Trial pruned: trial_id={:?}, rung={}, BPB={}",
              trial_id, rung, bpb);

        Ok(())
    }

    /// Mark trial as completed
    pub async fn mark_completed(
        &self,
        trial_id: &Uuid,
        final_step: i32,
        final_bpb: f64,
    ) -> Result<()> {
        self.client
            .execute(
                "UPDATE igla_race_trials
                 SET status = 'completed', completed_at = NOW(),
                     final_step = $1, final_bpb = $2
                 WHERE trial_id = $3",
                &[&final_step, &final_bpb, trial_id],
            )
            .await?;

        info!("Trial completed: trial_id={:?}, BPB={}", trial_id, final_bpb);

        Ok(())
    }

    /// Store lesson in experience
    pub async fn store_lesson(
        &self,
        trial_id: &Uuid,
        outcome: &str,
        pruned_at_rung: i32,
        bpb_at_pruned: f64,
        lesson: &str,
        lesson_type: &str,
    ) -> Result<()> {
        self.client
            .execute(
                "INSERT INTO igla_race_experience
                 (trial_id, outcome, pruned_at_rung, bpb_at_pruned, lesson, lesson_type)
                 VALUES ($1, $2, $3, $4, $5, $6)",
                &[trial_id, &outcome, &pruned_at_rung, &bpb_at_pruned, &lesson, &lesson_type],
            )
            .await?;

        Ok(())
    }

    /// Check if config is already running
    pub async fn is_config_running(
        &self,
        machine_id: &str,
        config_json: &str,
    ) -> Result<bool> {
        let count: i64 = self.client
            .query_one(
                "SELECT COUNT(*) FROM igla_race_trials
                 WHERE machine_id = $1 AND config = $2::jsonb
                   AND status IN ('pending', 'running')",
                &[&machine_id, &config_json],
            )
            .await?
            .get(0);

        Ok(count > 0)
    }

    /// Get top lessons from experience
    pub async fn get_top_lessons(&self, limit: i32) -> Result<Vec<LessonEntry>> {
        let rows = self.client
            .query(
                "SELECT lesson, lesson_type, pattern_count
                 FROM igla_race_experience
                 ORDER BY pattern_count DESC, confidence DESC
                 LIMIT $1",
                &[&limit],
            )
            .await?;

        let lessons = rows.iter().map(|row| {
            LessonEntry {
                lesson: row.get(0),
                lesson_type: row.get(1),
                pattern_count: row.get(2),
            }
        }).collect();

        Ok(lessons)
    }
}

/// Lesson entry
#[derive(Debug, Clone)]
pub struct LessonEntry {
    pub lesson: String,
    pub lesson_type: String,
    pub pattern_count: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neon_db_creation() {
        // Unit test only - requires real connection string for integration tests
    }
}

