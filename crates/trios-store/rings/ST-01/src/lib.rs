//! ST-01 — SQLite repository (sqlx) over ST-00 rows.
//!
//! Opens the SAME `.db` file the TS backend uses (WAL, foreign_keys=ON)
//! so Rust and TS can coexist during migration. All queries are typed
//! against the ST-00 row structs.

use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;
use trios_store_st00::{
    Adapter, AgentDefinitionRow, DetectedBy, OAuthTokenRow, ProducedFileRow,
};

/// Thin handle around a shared sqlx SQLite pool.
#[derive(Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    /// Open (creating if missing) the SQLite file at `path` with the same
    /// PRAGMAs the TS backend sets: WAL journal + foreign keys on.
    pub async fn open(path: &str) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new().max_connections(5).connect_with(opts).await?;
        Ok(Self { pool })
    }

    /// In-memory store for tests.
    pub async fn open_memory() -> Result<Self> {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ---- agent_definitions -------------------------------------------------

    pub async fn upsert_agent(&self, row: &AgentDefinitionRow) -> Result<()> {
        sqlx::query(
            "INSERT INTO agent_definitions
               (id,name,adapter,model_id,reasoning_effort,permission_mode,
                session_key,pinned,adapter_config_json,created_at,updated_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, adapter=excluded.adapter,
               model_id=excluded.model_id, reasoning_effort=excluded.reasoning_effort,
               permission_mode=excluded.permission_mode, session_key=excluded.session_key,
               pinned=excluded.pinned, adapter_config_json=excluded.adapter_config_json,
               updated_at=excluded.updated_at",
        )
        .bind(&row.id)
        .bind(&row.name)
        .bind(row.adapter.as_str())
        .bind(&row.model_id)
        .bind(&row.reasoning_effort)
        .bind(&row.permission_mode)
        .bind(&row.session_key)
        .bind(row.pinned as i64)
        .bind(&row.adapter_config_json)
        .bind(row.created_at)
        .bind(row.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_agent(&self, id: &str) -> Result<Option<AgentDefinitionRow>> {
        let r = sqlx::query("SELECT * FROM agent_definitions WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(r.map(|row| AgentDefinitionRow {
            id: row.get("id"),
            name: row.get("name"),
            adapter: Adapter::parse(row.get::<String, _>("adapter").as_str())
                .unwrap_or(Adapter::Claude),
            model_id: row.get("model_id"),
            reasoning_effort: row.get("reasoning_effort"),
            permission_mode: row.get("permission_mode"),
            session_key: row.get("session_key"),
            pinned: row.get::<i64, _>("pinned") != 0,
            adapter_config_json: row.get("adapter_config_json"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    }

    pub async fn list_agents(&self) -> Result<Vec<AgentDefinitionRow>> {
        let rows = sqlx::query("SELECT * FROM agent_definitions ORDER BY updated_at DESC")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| AgentDefinitionRow {
                id: row.get("id"),
                name: row.get("name"),
                adapter: Adapter::parse(row.get::<String, _>("adapter").as_str())
                    .unwrap_or(Adapter::Claude),
                model_id: row.get("model_id"),
                reasoning_effort: row.get("reasoning_effort"),
                permission_mode: row.get("permission_mode"),
                session_key: row.get("session_key"),
                pinned: row.get::<i64, _>("pinned") != 0,
                adapter_config_json: row.get("adapter_config_json"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect())
    }

    pub async fn delete_agent(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM agent_definitions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- oauth_tokens ------------------------------------------------------

    pub async fn upsert_token(&self, row: &OAuthTokenRow) -> Result<()> {
        sqlx::query(
            "INSERT INTO oauth_tokens
               (browseros_id,provider,access_token,refresh_token,expires_at,email,account_id,updated_at)
             VALUES (?,?,?,?,?,?,?,?)
             ON CONFLICT(browseros_id,provider) DO UPDATE SET
               access_token=excluded.access_token, refresh_token=excluded.refresh_token,
               expires_at=excluded.expires_at, email=excluded.email,
               account_id=excluded.account_id, updated_at=excluded.updated_at",
        )
        .bind(&row.browseros_id)
        .bind(&row.provider)
        .bind(&row.access_token)
        .bind(&row.refresh_token)
        .bind(row.expires_at)
        .bind(&row.email)
        .bind(&row.account_id)
        .bind(row.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_token(&self, browseros_id: &str, provider: &str) -> Result<Option<OAuthTokenRow>> {
        let r = sqlx::query(
            "SELECT * FROM oauth_tokens WHERE browseros_id = ? AND provider = ?",
        )
        .bind(browseros_id)
        .bind(provider)
        .fetch_optional(&self.pool)
        .await?;
        Ok(r.map(|row| OAuthTokenRow {
            browseros_id: row.get("browseros_id"),
            provider: row.get("provider"),
            access_token: row.get("access_token"),
            refresh_token: row.get("refresh_token"),
            expires_at: row.get("expires_at"),
            email: row.get("email"),
            account_id: row.get("account_id"),
            updated_at: row.get("updated_at"),
        }))
    }

    // ---- produced_files ----------------------------------------------------

    pub async fn upsert_produced_file(&self, row: &ProducedFileRow) -> Result<()> {
        sqlx::query(
            "INSERT INTO produced_files
               (id,agent_definition_id,session_key,turn_id,turn_prompt,path,size,mtime_ms,created_at,detected_by)
             VALUES (?,?,?,?,?,?,?,?,?,?)
             ON CONFLICT(agent_definition_id,path) DO UPDATE SET
               session_key=excluded.session_key, turn_id=excluded.turn_id,
               turn_prompt=excluded.turn_prompt, size=excluded.size,
               mtime_ms=excluded.mtime_ms, created_at=excluded.created_at,
               detected_by=excluded.detected_by",
        )
        .bind(&row.id)
        .bind(&row.agent_definition_id)
        .bind(&row.session_key)
        .bind(&row.turn_id)
        .bind(&row.turn_prompt)
        .bind(&row.path)
        .bind(row.size)
        .bind(row.mtime_ms)
        .bind(row.created_at)
        .bind(row.detected_by.as_str())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn files_for_turn(&self, turn_id: &str) -> Result<Vec<ProducedFileRow>> {
        let rows = sqlx::query("SELECT * FROM produced_files WHERE turn_id = ?")
            .bind(turn_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| ProducedFileRow {
                id: row.get("id"),
                agent_definition_id: row.get("agent_definition_id"),
                session_key: row.get("session_key"),
                turn_id: row.get("turn_id"),
                turn_prompt: row.get("turn_prompt"),
                path: row.get("path"),
                size: row.get("size"),
                mtime_ms: row.get("mtime_ms"),
                created_at: row.get("created_at"),
                detected_by: DetectedBy::parse(row.get::<String, _>("detected_by").as_str()),
            })
            .collect())
    }
}
