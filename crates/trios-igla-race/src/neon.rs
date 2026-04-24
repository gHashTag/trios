//! Neon PostgreSQL connection and database operations

use tokio_postgres::{NoTls, Error as PgError};
use anyhow::Result;
use tracing::info;
use uuid::Uuid;
use chrono::{DateTime, Utc};

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

    /// Execute raw SQL query
    pub async fn execute(&self, query: &str) -> Result<u64> {
        Ok(self.client.execute(query, &[]).await?)
    }

    /// Query for leaderboard entries
    pub async fn get_leaderboard(&self, limit: i32) -> Result<Vec<LeaderboardEntry>> {
        let rows = self.client
            .query(
                "SELECT trial_id, machine_id, config, status, final_bpb, final_step,
                        started_at, completed_at, best_rung, lesson, bpb_rank
                 FROM igla_race_leaderboard
                 ORDER BY bpb_rank ASC
                 LIMIT $1",
                &[&limit],
            )
            .await?;

        let entries = rows.iter().map(|row| {
            LeaderboardEntry {
                trial_id: row.get(0),
                machine_id: row.get(1),
                config: row.get(2),
                status: row.get(3),
                final_bpb: row.get(4),
                final_step: row.get(5),
                started_at: row.get(6),
                completed_at: row.get(7),
                best_rung: row.get(8),
                lesson: row.get(9),
                bpb_rank: row.get(10),
            }
        }).collect();

        Ok(entries)
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

    /// Register a new trial
    pub async fn register_trial(
        &self,
        trial_id: Uuid,
        machine_id: &str,
        worker_id: i32,
        config: &serde_json::Value,
    ) -> Result<()> {
        self.client
            .execute(
                "INSERT INTO igla_race_trials
                 (trial_id, machine_id, worker_id, config, status, started_at)
                 VALUES ($1, $2, $3, $4, 'running', NOW())",
                &[&trial_id, &machine_id, &worker_id, config],
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
        config: &serde_json::Value,
    ) -> Result<bool> {
        let count: i64 = self.client
            .query_one(
                "SELECT COUNT(*) FROM igla_race_trials
                 WHERE machine_id = $1 AND config = $2
                   AND status IN ('pending', 'running')",
                &[&machine_id, config],
            )
            .await?
            .get(0);

        Ok(count > 0)
    }
}

/// Leaderboard entry
#[derive(Debug, Clone)]
pub struct LeaderboardEntry {
    pub trial_id: Uuid,
    pub machine_id: String,
    pub config: serde_json::Value,
    pub status: String,
    pub final_bpb: Option<f64>,
    pub final_step: Option<i32>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub best_rung: i32,
    pub lesson: Option<String>,
    pub bpb_rank: i64,
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
