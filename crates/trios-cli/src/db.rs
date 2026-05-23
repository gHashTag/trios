//! SQLite storage for leaderboard
//!
//! Local persistent storage for experiment results.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Leaderboard entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: Option<i64>,
    pub agent: String,
    pub exp_id: String,
    pub config: String,
    pub train_bpb: f64,
    pub val_bpb: f64,
    pub params: u64,
    pub time_sec: f64,
    pub timestamp: String,
}

pub struct Leaderboard {
    db: Connection,
}

impl Leaderboard {
    /// Open or create leaderboard database
    pub fn open() -> Result<Self> {
        let path = Self::path();

        // Create parent dir if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = Connection::open(&path).context("Failed to open leaderboard DB")?;

        // Create table if not exists
        db.execute(
            "CREATE TABLE IF NOT EXISTS leaderboard (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                agent TEXT NOT NULL,
                exp_id TEXT NOT NULL,
                config TEXT NOT NULL,
                train_bpb REAL NOT NULL,
                val_bpb REAL NOT NULL,
                params INTEGER NOT NULL,
                time_sec REAL NOT NULL,
                timestamp TEXT NOT NULL
            )",
            [],
        )
        .context("Failed to create leaderboard table")?;

        // Create indexes for common queries
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_val_bpb ON leaderboard(val_bpb)",
            [],
        )
        .context("Failed to create val_bpb index")?;

        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_agent ON leaderboard(agent)",
            [],
        )
        .context("Failed to create agent index")?;

        Ok(Self { db })
    }

    /// Insert new entry
    pub fn insert(&self, entry: &Entry) -> Result<i64> {
        self.db.execute(
            "INSERT INTO leaderboard (agent, exp_id, config, train_bpb, val_bpb, params, time_sec, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.agent,
                entry.exp_id,
                entry.config,
                entry.train_bpb,
                entry.val_bpb,
                entry.params,
                entry.time_sec,
                entry.timestamp,
            ],
        )
        .context("Failed to insert leaderboard entry")?;

        Ok(self.db.last_insert_rowid())
    }

    /// Get top N entries by val_bpb
    pub fn top(&self, limit: usize) -> Result<Vec<Entry>> {
        let mut stmt = self.db.prepare_cached(
            "SELECT id, agent, exp_id, config, train_bpb, val_bpb, params, time_sec, timestamp
             FROM leaderboard
             ORDER BY val_bpb ASC
             LIMIT ?1",
        )?;

        let mut entries = Vec::new();
        let mut rows = stmt.query(params![limit as i64])?;

        while let Some(row) = rows.next()? {
            entries.push(Entry {
                id: Some(row.get(0)?),
                agent: row.get(1)?,
                exp_id: row.get(2)?,
                config: row.get(3)?,
                train_bpb: row.get(4)?,
                val_bpb: row.get(5)?,
                params: row.get(6)?,
                time_sec: row.get(7)?,
                timestamp: row.get(8)?,
            });
        }

        Ok(entries)
    }

    /// Get entries for specific agent
    pub fn by_agent(&self, agent: &str) -> Result<Vec<Entry>> {
        let mut stmt = self.db.prepare_cached(
            "SELECT id, agent, exp_id, config, train_bpb, val_bpb, params, time_sec, timestamp
             FROM leaderboard
             WHERE agent = ?1
             ORDER BY timestamp DESC",
        )?;

        let mut entries = Vec::new();
        let mut rows = stmt.query(params![agent])?;

        while let Some(row) = rows.next()? {
            entries.push(Entry {
                id: Some(row.get(0)?),
                agent: row.get(1)?,
                exp_id: row.get(2)?,
                config: row.get(3)?,
                train_bpb: row.get(4)?,
                val_bpb: row.get(5)?,
                params: row.get(6)?,
                time_sec: row.get(7)?,
                timestamp: row.get(8)?,
            });
        }

        Ok(entries)
    }

    /// Get entry by exp_id
    pub fn by_exp_id(&self, exp_id: &str) -> Result<Option<Entry>> {
        let mut stmt = self.db.prepare_cached(
            "SELECT id, agent, exp_id, config, train_bpb, val_bpb, params, time_sec, timestamp
             FROM leaderboard
             WHERE exp_id = ?1",
        )?;

        let mut rows = stmt.query(params![exp_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Entry {
                id: Some(row.get(0)?),
                agent: row.get(1)?,
                exp_id: row.get(2)?,
                config: row.get(3)?,
                train_bpb: row.get(4)?,
                val_bpb: row.get(5)?,
                params: row.get(6)?,
                time_sec: row.get(7)?,
                timestamp: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Delete entry
    pub fn delete(&self, exp_id: &str) -> Result<()> {
        self.db
            .execute("DELETE FROM leaderboard WHERE exp_id = ?1", params![exp_id])?;
        Ok(())
    }

    /// Get stats
    pub fn stats(&self) -> Result<Stats> {
        let count: i64 = self
            .db
            .query_row("SELECT COUNT(*) FROM leaderboard", [], |r| r.get(0))?;

        let min_bpb: f64 = self
            .db
            .query_row("SELECT MIN(val_bpb) FROM leaderboard", [], |r| r.get(0))?;

        let avg_bpb: f64 = self
            .db
            .query_row("SELECT AVG(val_bpb) FROM leaderboard", [], |r| r.get(0))?;

        Ok(Stats {
            count,
            min_bpb,
            avg_bpb,
        })
    }
}

#[derive(Debug)]
pub struct Stats {
    pub count: i64,
    pub min_bpb: f64,
    pub avg_bpb: f64,
}

impl Leaderboard {
    fn path() -> PathBuf {
        PathBuf::from(".trinity/leaderboard.db")
    }
}
