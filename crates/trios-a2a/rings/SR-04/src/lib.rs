//! SR-04 — durable persistence for the A2A registry.
//!
//! The in-memory `A2ARegistry` (SR-02) loses all agent cards and tasks on
//! restart. SR-04 adds an optional durable backend behind a small trait so
//! `trios-server` can survive restarts and share registry state across
//! processes. It reuses the same SQLite engine as `trios-store` (WAL), but
//! owns its own tables (`a2a_agents`, `a2a_tasks`) so the A2A contract stays
//! decoupled from the adapter-definition schema.
//!
//! Ring isolation: SR-04 depends only on SR-00 (AgentCard) and SR-01 (Task).
//! It does NOT import SR-02 — the registry composes a store, not vice-versa.

use anyhow::Result;
use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;
use trios_a2a_sr00::AgentCard;
use trios_a2a_sr01::Task;

/// Durable backend for A2A registry state. Cards and tasks are stored as
/// their canonical JSON (the SR-00 / SR-01 serde form), so the wire contract
/// is the single source of truth for persistence too.
#[async_trait]
pub trait A2AStore: Send + Sync {
    async fn save_agent(&self, card: &AgentCard) -> Result<()>;
    async fn load_agents(&self) -> Result<Vec<AgentCard>>;
    async fn remove_agent(&self, id: &str) -> Result<()>;

    async fn save_task(&self, task: &Task) -> Result<()>;
    async fn load_tasks(&self) -> Result<Vec<Task>>;
}

/// SQLite-backed [`A2AStore`] (shares the trios-store SQLite engine).
#[derive(Clone)]
pub struct SqliteA2AStore {
    pool: SqlitePool,
}

impl SqliteA2AStore {
    /// Open (creating if missing) the SQLite file and ensure the A2A tables.
    pub async fn open(path: &str) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new().max_connections(5).connect_with(opts).await?;
        let s = Self { pool };
        s.migrate().await?;
        Ok(s)
    }

    /// In-memory store for tests.
    pub async fn open_memory() -> Result<Self> {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")?;
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await?;
        let s = Self { pool };
        s.migrate().await?;
        Ok(s)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS a2a_agents (
                id TEXT PRIMARY KEY NOT NULL,
                card_json TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS a2a_tasks (
                id TEXT PRIMARY KEY NOT NULL,
                task_json TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[async_trait]
impl A2AStore for SqliteA2AStore {
    async fn save_agent(&self, card: &AgentCard) -> Result<()> {
        let json = serde_json::to_string(card)?;
        sqlx::query(
            "INSERT INTO a2a_agents (id, card_json) VALUES (?, ?)
             ON CONFLICT(id) DO UPDATE SET card_json = excluded.card_json",
        )
        .bind(card.id.as_str())
        .bind(json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_agents(&self) -> Result<Vec<AgentCard>> {
        let rows = sqlx::query("SELECT card_json FROM a2a_agents")
            .fetch_all(&self.pool)
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let j: String = r.get("card_json");
            out.push(serde_json::from_str(&j)?);
        }
        Ok(out)
    }

    async fn remove_agent(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM a2a_agents WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn save_task(&self, task: &Task) -> Result<()> {
        let json = serde_json::to_string(task)?;
        sqlx::query(
            "INSERT INTO a2a_tasks (id, task_json) VALUES (?, ?)
             ON CONFLICT(id) DO UPDATE SET task_json = excluded.task_json",
        )
        .bind(&task.id)
        .bind(json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_tasks(&self) -> Result<Vec<Task>> {
        let rows = sqlx::query("SELECT task_json FROM a2a_tasks")
            .fetch_all(&self.pool)
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let j: String = r.get("task_json");
            out.push(serde_json::from_str(&j)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trios_a2a_sr00::AgentId;

    #[tokio::test]
    async fn agents_persist_and_reload() {
        let store = SqliteA2AStore::open_memory().await.unwrap();
        let card = AgentCard::new("alpha", "Alpha");
        store.save_agent(&card).await.unwrap();

        let loaded = store.load_agents().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id.as_str(), "alpha");
        assert_eq!(loaded[0].name, "Alpha");

        // upsert (same id) does not duplicate
        store.save_agent(&AgentCard::new("alpha", "Alpha2")).await.unwrap();
        let loaded = store.load_agents().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Alpha2");

        store.remove_agent("alpha").await.unwrap();
        assert!(store.load_agents().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn tasks_persist_via_canonical_json() {
        let store = SqliteA2AStore::open_memory().await.unwrap();
        let task = Task::new("Build", AgentId::new("lead"));
        store.save_task(&task).await.unwrap();
        let loaded = store.load_tasks().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].title, "Build");
    }
}
