//! ST-01 — SQLite repository (SeaORM) over ST-00 rows.
//!
//! Opens the SAME `.db` file the TS backend used (WAL, foreign_keys=ON)
//! so the data written during the TS→Rust migration stays readable.
//! Query layer is SeaORM; the underlying sqlx pool is constructed with
//! explicit `SqliteConnectOptions` (via SeaORM's sqlx re-export) so the
//! per-connection PRAGMAs match what the TS backend set.

pub mod entities;

use anyhow::Result;
use sea_orm::sea_query::OnConflict;
use sea_orm::sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions,
};
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    SqlxSqliteConnector,
};
use std::str::FromStr;
use trios_store_st00::{
    Adapter, AgentDefinitionRow, DetectedBy, OAuthTokenRow, ProducedFileRow,
};

use entities::{agent_definitions, oauth_tokens, produced_files};

/// Thin handle around a shared SeaORM SQLite connection (pooled).
#[derive(Clone)]
pub struct Store {
    conn: DatabaseConnection,
}

impl Store {
    /// Open (creating if missing) the SQLite file at `path` with the same
    /// PRAGMAs the TS backend set: WAL journal + foreign keys on.
    pub async fn open(path: &str) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await?;
        Ok(Self {
            conn: SqlxSqliteConnector::from_sqlx_sqlite_pool(pool),
        })
    }

    /// In-memory store for tests.
    pub async fn open_memory() -> Result<Self> {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        Ok(Self {
            conn: SqlxSqliteConnector::from_sqlx_sqlite_pool(pool),
        })
    }

    /// The underlying SeaORM connection (for migrations / raw statements).
    pub fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }

    // ---- agent_definitions -------------------------------------------------

    pub async fn upsert_agent(&self, row: &AgentDefinitionRow) -> Result<()> {
        let model = agent_definitions::ActiveModel {
            id: Set(row.id.clone()),
            name: Set(row.name.clone()),
            adapter: Set(row.adapter.as_str().to_string()),
            model_id: Set(row.model_id.clone()),
            reasoning_effort: Set(row.reasoning_effort.clone()),
            permission_mode: Set(row.permission_mode.clone()),
            session_key: Set(row.session_key.clone()),
            pinned: Set(row.pinned),
            adapter_config_json: Set(row.adapter_config_json.clone()),
            created_at: Set(row.created_at),
            updated_at: Set(row.updated_at),
        };
        agent_definitions::Entity::insert(model)
            .on_conflict(
                OnConflict::column(agent_definitions::Column::Id)
                    .update_columns([
                        agent_definitions::Column::Name,
                        agent_definitions::Column::Adapter,
                        agent_definitions::Column::ModelId,
                        agent_definitions::Column::ReasoningEffort,
                        agent_definitions::Column::PermissionMode,
                        agent_definitions::Column::SessionKey,
                        agent_definitions::Column::Pinned,
                        agent_definitions::Column::AdapterConfigJson,
                        agent_definitions::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&self.conn)
            .await?;
        Ok(())
    }

    pub async fn get_agent(&self, id: &str) -> Result<Option<AgentDefinitionRow>> {
        let found = agent_definitions::Entity::find_by_id(id)
            .one(&self.conn)
            .await?;
        Ok(found.map(agent_row_from_model))
    }

    pub async fn list_agents(&self) -> Result<Vec<AgentDefinitionRow>> {
        let rows = agent_definitions::Entity::find()
            .order_by_desc(agent_definitions::Column::UpdatedAt)
            .all(&self.conn)
            .await?;
        Ok(rows.into_iter().map(agent_row_from_model).collect())
    }

    pub async fn delete_agent(&self, id: &str) -> Result<()> {
        agent_definitions::Entity::delete_by_id(id)
            .exec(&self.conn)
            .await?;
        Ok(())
    }

    // ---- oauth_tokens ------------------------------------------------------

    pub async fn upsert_token(&self, row: &OAuthTokenRow) -> Result<()> {
        let model = oauth_tokens::ActiveModel {
            browseros_id: Set(row.browseros_id.clone()),
            provider: Set(row.provider.clone()),
            access_token: Set(row.access_token.clone()),
            refresh_token: Set(row.refresh_token.clone()),
            expires_at: Set(row.expires_at),
            email: Set(row.email.clone()),
            account_id: Set(row.account_id.clone()),
            updated_at: Set(row.updated_at),
        };
        oauth_tokens::Entity::insert(model)
            .on_conflict(
                OnConflict::columns([
                    oauth_tokens::Column::BrowserosId,
                    oauth_tokens::Column::Provider,
                ])
                .update_columns([
                    oauth_tokens::Column::AccessToken,
                    oauth_tokens::Column::RefreshToken,
                    oauth_tokens::Column::ExpiresAt,
                    oauth_tokens::Column::Email,
                    oauth_tokens::Column::AccountId,
                    oauth_tokens::Column::UpdatedAt,
                ])
                .to_owned(),
            )
            .exec(&self.conn)
            .await?;
        Ok(())
    }

    pub async fn get_token(
        &self,
        browseros_id: &str,
        provider: &str,
    ) -> Result<Option<OAuthTokenRow>> {
        let found = oauth_tokens::Entity::find_by_id((
            browseros_id.to_string(),
            provider.to_string(),
        ))
        .one(&self.conn)
        .await?;
        Ok(found.map(|m| OAuthTokenRow {
            browseros_id: m.browseros_id,
            provider: m.provider,
            access_token: m.access_token,
            refresh_token: m.refresh_token,
            expires_at: m.expires_at,
            email: m.email,
            account_id: m.account_id,
            updated_at: m.updated_at,
        }))
    }

    // ---- produced_files ----------------------------------------------------

    pub async fn upsert_produced_file(&self, row: &ProducedFileRow) -> Result<()> {
        let model = produced_files::ActiveModel {
            id: Set(row.id.clone()),
            agent_definition_id: Set(row.agent_definition_id.clone()),
            session_key: Set(row.session_key.clone()),
            turn_id: Set(row.turn_id.clone()),
            turn_prompt: Set(row.turn_prompt.clone()),
            path: Set(row.path.clone()),
            size: Set(row.size),
            mtime_ms: Set(row.mtime_ms),
            created_at: Set(row.created_at),
            detected_by: Set(row.detected_by.as_str().to_string()),
        };
        produced_files::Entity::insert(model)
            .on_conflict(
                OnConflict::columns([
                    produced_files::Column::AgentDefinitionId,
                    produced_files::Column::Path,
                ])
                .update_columns([
                    produced_files::Column::SessionKey,
                    produced_files::Column::TurnId,
                    produced_files::Column::TurnPrompt,
                    produced_files::Column::Size,
                    produced_files::Column::MtimeMs,
                    produced_files::Column::CreatedAt,
                    produced_files::Column::DetectedBy,
                ])
                .to_owned(),
            )
            .exec(&self.conn)
            .await?;
        Ok(())
    }

    pub async fn files_for_turn(&self, turn_id: &str) -> Result<Vec<ProducedFileRow>> {
        let rows = produced_files::Entity::find()
            .filter(produced_files::Column::TurnId.eq(turn_id))
            .all(&self.conn)
            .await?;
        Ok(rows
            .into_iter()
            .map(|m| ProducedFileRow {
                id: m.id,
                agent_definition_id: m.agent_definition_id,
                session_key: m.session_key,
                turn_id: m.turn_id,
                turn_prompt: m.turn_prompt,
                path: m.path,
                size: m.size,
                mtime_ms: m.mtime_ms,
                created_at: m.created_at,
                detected_by: DetectedBy::parse(&m.detected_by),
            })
            .collect())
    }
}

fn agent_row_from_model(m: agent_definitions::Model) -> AgentDefinitionRow {
    AgentDefinitionRow {
        id: m.id,
        name: m.name,
        adapter: Adapter::parse(&m.adapter).unwrap_or(Adapter::Claude),
        model_id: m.model_id,
        reasoning_effort: m.reasoning_effort,
        permission_mode: m.permission_mode,
        session_key: m.session_key,
        pinned: m.pinned,
        adapter_config_json: m.adapter_config_json,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}
