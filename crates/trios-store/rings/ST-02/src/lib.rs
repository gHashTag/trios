//! ST-02 — DDL migrations mirroring the drizzle SQLite schema.
//!
//! Idempotent `CREATE TABLE IF NOT EXISTS` + indexes matching
//! `browseros-agent/apps/server/src/lib/db/schema/*`. Safe to run against
//! an existing TS-created database (all `IF NOT EXISTS`).
//! Statements are applied through SeaORM (`execute_unprepared`) inside a
//! single transaction.

use anyhow::Result;
use sea_orm::{ConnectionTrait, TransactionTrait};
use trios_store_st01::Store;

/// The full DDL, applied in one batch.
pub const SCHEMA_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS agent_definitions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    adapter TEXT NOT NULL,
    model_id TEXT NOT NULL,
    reasoning_effort TEXT NOT NULL,
    permission_mode TEXT NOT NULL DEFAULT 'approve-all',
    session_key TEXT NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0,
    adapter_config_json TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS agent_definitions_session_key_unique ON agent_definitions (session_key);
CREATE INDEX IF NOT EXISTS agent_definitions_updated_at_idx ON agent_definitions (updated_at);
CREATE INDEX IF NOT EXISTS agent_definitions_adapter_updated_at_idx ON agent_definitions (adapter, updated_at);

CREATE TABLE IF NOT EXISTS oauth_tokens (
    browseros_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    access_token TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    email TEXT,
    account_id TEXT,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (browseros_id, provider)
);
CREATE INDEX IF NOT EXISTS oauth_tokens_browseros_id_idx ON oauth_tokens (browseros_id);

CREATE TABLE IF NOT EXISTS produced_files (
    id TEXT PRIMARY KEY NOT NULL,
    agent_definition_id TEXT NOT NULL REFERENCES agent_definitions(id) ON DELETE CASCADE,
    session_key TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    turn_prompt TEXT NOT NULL,
    path TEXT NOT NULL,
    size INTEGER NOT NULL,
    mtime_ms INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    detected_by TEXT NOT NULL DEFAULT 'diff'
);
CREATE UNIQUE INDEX IF NOT EXISTS produced_files_agent_path_unique ON produced_files (agent_definition_id, path);
CREATE INDEX IF NOT EXISTS produced_files_agent_created_idx ON produced_files (agent_definition_id, created_at);
CREATE INDEX IF NOT EXISTS produced_files_turn_idx ON produced_files (turn_id);
CREATE INDEX IF NOT EXISTS produced_files_session_idx ON produced_files (session_key);
"#;

/// Apply the schema to a store. Idempotent.
pub async fn migrate(store: &Store) -> Result<()> {
    let tx = store.conn().begin().await?;
    for raw in SCHEMA_DDL.split(';') {
        let sql = raw.trim();
        if sql.is_empty() {
            continue;
        }
        tx.execute_unprepared(sql).await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use trios_store_st00::{Adapter, AgentDefinitionRow, DetectedBy, OAuthTokenRow, ProducedFileRow};

    #[tokio::test]
    async fn migrate_and_roundtrip_all_tables() {
        let store = Store::open_memory().await.unwrap();
        migrate(&store).await.unwrap();
        // idempotent second run
        migrate(&store).await.unwrap();

        let agent = AgentDefinitionRow {
            id: "a1".into(),
            name: "Claw".into(),
            adapter: Adapter::Openclaw,
            model_id: "gpt".into(),
            reasoning_effort: "high".into(),
            permission_mode: "approve-all".into(),
            session_key: "sess-1".into(),
            pinned: false,
            adapter_config_json: Some("{}".into()),
            created_at: 100,
            updated_at: 200,
        };
        store.upsert_agent(&agent).await.unwrap();
        assert_eq!(store.get_agent("a1").await.unwrap().unwrap(), agent);
        assert_eq!(store.list_agents().await.unwrap().len(), 1);

        let tok = OAuthTokenRow {
            browseros_id: "b1".into(),
            provider: "google".into(),
            access_token: "at".into(),
            refresh_token: "rt".into(),
            expires_at: 999,
            email: Some("x@y.z".into()),
            account_id: None,
            updated_at: 5,
        };
        store.upsert_token(&tok).await.unwrap();
        assert_eq!(store.get_token("b1", "google").await.unwrap().unwrap(), tok);

        let file = ProducedFileRow {
            id: "f1".into(),
            agent_definition_id: "a1".into(),
            session_key: "sess-1".into(),
            turn_id: "t1".into(),
            turn_prompt: "make a report".into(),
            path: "reports/q1.pdf".into(),
            size: 1024,
            mtime_ms: 123,
            created_at: 300,
            detected_by: DetectedBy::Diff,
        };
        store.upsert_produced_file(&file).await.unwrap();
        assert_eq!(store.files_for_turn("t1").await.unwrap(), vec![file]);

        // FK cascade: deleting the agent sweeps produced files.
        store.delete_agent("a1").await.unwrap();
        assert!(store.files_for_turn("t1").await.unwrap().is_empty());
    }
}
