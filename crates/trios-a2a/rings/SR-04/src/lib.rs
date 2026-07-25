//! SR-04 — durable persistence for the A2A registry.
//!
//! The in-memory `A2ARegistry` (SR-02) loses all agent cards and tasks on
//! restart. SR-04 adds an optional durable backend behind a small trait so
//! `trios-server` can survive restarts and share registry state across
//! processes. Storage is SQLite (WAL) accessed through SeaORM — the same ORM
//! already used by trios-chat (BR-IO-CHAT-05) — with tables owned by this
//! ring (`a2a_agents`, `a2a_tasks`, `a2a_wire_cards`, `a2a_pending`) so the
//! A2A contract stays decoupled from the adapter-definition schema.
//!
//! Ring isolation: SR-04 depends only on SR-00 (AgentCard) and SR-01 (Task).
//! It does NOT import SR-02 — the registry composes a store, not vice-versa.

use anyhow::Result;
use async_trait::async_trait;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveValue::Set, ConnectOptions, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
};
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

    // --- REST/wire surface (Волна 5+): client-shaped cards & offline queues ---

    /// Persist the verbatim client (wire) card for an agent.
    async fn save_wire_card(&self, id: &str, card: &serde_json::Value) -> Result<()>;
    /// Load all wire cards as `(agent_id, card)` pairs.
    async fn load_wire_cards(&self) -> Result<Vec<(String, serde_json::Value)>>;

    /// Persist the full pending-queue snapshot for a recipient
    /// (empty queue deletes the row).
    async fn save_pending(&self, recipient: &str, queue: &[serde_json::Value]) -> Result<()>;
    /// Load all pending queues as `(recipient, messages)` pairs.
    async fn load_pending(&self) -> Result<Vec<(String, Vec<serde_json::Value>)>>;
}

// --- SeaORM entities (one module per table, TEXT-JSON payload columns) ---

mod agent_row {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "a2a_agents")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub card_json: String,
        pub updated_at: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

mod task_row {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "a2a_tasks")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub task_json: String,
        pub updated_at: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

mod wire_card_row {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "a2a_wire_cards")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub card_json: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

mod pending_row {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "a2a_pending")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub recipient: String,
        pub queue_json: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

/// SQLite-backed [`A2AStore`] (SeaORM over the shared SQLite engine).
#[derive(Clone)]
pub struct SqliteA2AStore {
    db: DatabaseConnection,
}

impl SqliteA2AStore {
    /// Open (creating if missing) the SQLite file and ensure the A2A tables.
    pub async fn open(path: &str) -> Result<Self> {
        // `mode=rwc` — read/write/create, parity with sqlx create_if_missing.
        let mut opts = ConnectOptions::new(format!("sqlite://{path}?mode=rwc"));
        opts.max_connections(5).sqlx_logging(false);
        let db = Database::connect(opts).await?;
        db.execute_unprepared("PRAGMA journal_mode=WAL;").await?;
        db.execute_unprepared("PRAGMA foreign_keys=ON;").await?;
        let s = Self { db };
        s.migrate().await?;
        Ok(s)
    }

    /// In-memory store for tests (single connection — one shared memory DB).
    pub async fn open_memory() -> Result<Self> {
        let mut opts = ConnectOptions::new("sqlite::memory:");
        opts.max_connections(1).sqlx_logging(false);
        let db = Database::connect(opts).await?;
        let s = Self { db };
        s.migrate().await?;
        Ok(s)
    }

    async fn migrate(&self) -> Result<()> {
        self.db
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS a2a_agents (
                    id TEXT PRIMARY KEY NOT NULL,
                    card_json TEXT NOT NULL,
                    updated_at INTEGER NOT NULL DEFAULT 0
                )",
            )
            .await?;
        self.db
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS a2a_tasks (
                    id TEXT PRIMARY KEY NOT NULL,
                    task_json TEXT NOT NULL,
                    updated_at INTEGER NOT NULL DEFAULT 0
                )",
            )
            .await?;
        self.db
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS a2a_wire_cards (
                    id TEXT PRIMARY KEY NOT NULL,
                    card_json TEXT NOT NULL
                )",
            )
            .await?;
        self.db
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS a2a_pending (
                    recipient TEXT PRIMARY KEY NOT NULL,
                    queue_json TEXT NOT NULL
                )",
            )
            .await?;
        Ok(())
    }
}

#[async_trait]
impl A2AStore for SqliteA2AStore {
    async fn save_agent(&self, card: &AgentCard) -> Result<()> {
        let row = agent_row::ActiveModel {
            id: Set(card.id.as_str().to_string()),
            card_json: Set(serde_json::to_string(card)?),
            updated_at: Set(0),
        };
        agent_row::Entity::insert(row)
            .on_conflict(
                OnConflict::column(agent_row::Column::Id)
                    .update_column(agent_row::Column::CardJson)
                    .to_owned(),
            )
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn load_agents(&self) -> Result<Vec<AgentCard>> {
        let rows = agent_row::Entity::find().all(&self.db).await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(serde_json::from_str(&r.card_json)?);
        }
        Ok(out)
    }

    async fn remove_agent(&self, id: &str) -> Result<()> {
        agent_row::Entity::delete_by_id(id).exec(&self.db).await?;
        wire_card_row::Entity::delete_by_id(id).exec(&self.db).await?;
        pending_row::Entity::delete_by_id(id).exec(&self.db).await?;
        Ok(())
    }

    async fn save_task(&self, task: &Task) -> Result<()> {
        let row = task_row::ActiveModel {
            id: Set(task.id.clone()),
            task_json: Set(serde_json::to_string(task)?),
            updated_at: Set(0),
        };
        task_row::Entity::insert(row)
            .on_conflict(
                OnConflict::column(task_row::Column::Id)
                    .update_column(task_row::Column::TaskJson)
                    .to_owned(),
            )
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn load_tasks(&self) -> Result<Vec<Task>> {
        let rows = task_row::Entity::find().all(&self.db).await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(serde_json::from_str(&r.task_json)?);
        }
        Ok(out)
    }

    async fn save_wire_card(&self, id: &str, card: &serde_json::Value) -> Result<()> {
        let row = wire_card_row::ActiveModel {
            id: Set(id.to_string()),
            card_json: Set(card.to_string()),
        };
        wire_card_row::Entity::insert(row)
            .on_conflict(
                OnConflict::column(wire_card_row::Column::Id)
                    .update_column(wire_card_row::Column::CardJson)
                    .to_owned(),
            )
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn load_wire_cards(&self) -> Result<Vec<(String, serde_json::Value)>> {
        let rows = wire_card_row::Entity::find().all(&self.db).await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push((r.id, serde_json::from_str(&r.card_json)?));
        }
        Ok(out)
    }

    async fn save_pending(&self, recipient: &str, queue: &[serde_json::Value]) -> Result<()> {
        if queue.is_empty() {
            pending_row::Entity::delete_by_id(recipient).exec(&self.db).await?;
            return Ok(());
        }
        let row = pending_row::ActiveModel {
            recipient: Set(recipient.to_string()),
            queue_json: Set(serde_json::to_string(queue)?),
        };
        pending_row::Entity::insert(row)
            .on_conflict(
                OnConflict::column(pending_row::Column::Recipient)
                    .update_column(pending_row::Column::QueueJson)
                    .to_owned(),
            )
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn load_pending(&self) -> Result<Vec<(String, Vec<serde_json::Value>)>> {
        let rows = pending_row::Entity::find().all(&self.db).await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push((r.recipient, serde_json::from_str(&r.queue_json)?));
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

    #[tokio::test]
    async fn wire_cards_and_pending_persist() {
        use serde_json::json;
        let store = SqliteA2AStore::open_memory().await.unwrap();
        store
            .save_wire_card("a", &json!({"id": "a", "name": "A", "version": "1.0"}))
            .await
            .unwrap();
        store
            .save_pending("a", &[json!({"n": 1}), json!({"n": 2})])
            .await
            .unwrap();

        let cards = store.load_wire_cards().await.unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].1["version"], "1.0");
        let pending = store.load_pending().await.unwrap();
        assert_eq!(pending[0].1.len(), 2);

        // wire-card upsert (same id) does not duplicate
        store
            .save_wire_card("a", &json!({"id": "a", "name": "A", "version": "2.0"}))
            .await
            .unwrap();
        let cards = store.load_wire_cards().await.unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].1["version"], "2.0");

        // empty snapshot deletes the row
        store.save_pending("a", &[]).await.unwrap();
        assert!(store.load_pending().await.unwrap().is_empty());

        // remove_agent clears wire card + pending too
        store
            .save_pending("a", &[json!({"n": 3})])
            .await
            .unwrap();
        store.remove_agent("a").await.unwrap();
        assert!(store.load_wire_cards().await.unwrap().is_empty());
        assert!(store.load_pending().await.unwrap().is_empty());
    }
}
