//! Async `Store` mirror backed by SeaORM + Postgres.
//!
//! The synchronous `Store` trait lives in CR-CHAT-05; this ring
//! provides the async surface so callers under tokio can persist sealed
//! envelopes against a real database.

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, Database, DatabaseConnection, DbErr,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use sea_orm_migration::MigratorTrait;
use std::time::Duration;

use trios_chat_cr_chat_00::{Counter, DestHash, Error, Result, SessionId};
use trios_chat_cr_chat_05::EnvelopeRow;

use crate::entities::chat_envelope::{self, ActiveModel as EnvelopeActive, Entity as Envelope};
use crate::migrations::Migrator;

/// Async mirror of `trios_chat_cr_chat_05::Store`.
///
/// Kept narrow on purpose; matches the sync trait's CRUD shape so
/// higher-level rings can pick a backend at boot.
#[async_trait]
pub trait AsyncStore: Send + Sync {
    /// Insert a row. Duplicate `(session, counter)` returns
    /// `Error::Invariant("persist: duplicate row")`.
    async fn put(&self, row: EnvelopeRow) -> Result<()>;

    /// Fetch one row by primary key.
    async fn get(&self, session: &SessionId, counter: Counter) -> Result<Option<EnvelopeRow>>;

    /// All rows for a session, ordered by counter ASC.
    async fn list_session(&self, session: &SessionId) -> Result<Vec<EnvelopeRow>>;

    /// Total rows currently stored.
    async fn count(&self) -> Result<usize>;
}

/// Concrete SeaORM-backed Postgres store.
///
/// `[VERIFIED]` query shape matches the migration's `chat_envelope`
/// table; `[ASPIRATIONAL]` real connection pooling tuning lives in a
/// follow-up PR.
pub struct PgChatStore {
    db: DatabaseConnection,
}

impl PgChatStore {
    /// Open a connection against `database_url`, applying sensible
    /// defaults for a chat workload.
    pub async fn connect(database_url: &str) -> Result<Self> {
        let mut opts = ConnectOptions::new(database_url.to_owned());
        opts.max_connections(8)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(5))
            .idle_timeout(Duration::from_secs(300))
            .sqlx_logging(false);
        let db = Database::connect(opts).await.map_err(map_dberr)?;
        Ok(Self { db })
    }

    /// Run pending migrations (idempotent).
    pub async fn run_migrations(&self) -> Result<()> {
        Migrator::up(&self.db, None).await.map_err(map_dberr)
    }

    /// Borrow the underlying connection (e.g. for the higher-level
    /// Trinity registry).
    pub fn connection(&self) -> &DatabaseConnection {
        &self.db
    }

    /// Test-only helper: wipe every row in `chat_envelope`.
    /// Behind a function rather than a trait method on purpose — only
    /// integration tests need it.
    pub async fn truncate_for_tests(&self) -> Result<()> {
        self.db
            .transaction::<_, (), DbErr>(|txn| {
                Box::pin(async move {
                    Envelope::delete_many().exec(txn).await?;
                    Ok(())
                })
            })
            .await
            .map_err(|e| Error::Persist(format!("truncate: {e}")))?;
        Ok(())
    }
}

#[async_trait]
impl AsyncStore for PgChatStore {
    async fn put(&self, row: EnvelopeRow) -> Result<()> {
        let active = EnvelopeActive {
            session_id: Set(row.session.0.to_vec()),
            counter: Set(row.counter.get() as i64),
            dest_hash: Set(row.dest.0.to_vec()),
            ciphertext: Set(row.ciphertext.clone()),
            stored_at: sea_orm::ActiveValue::NotSet,
        };
        match active.insert(&self.db).await {
            Ok(_) => Ok(()),
            Err(DbErr::Exec(rt)) => {
                // Postgres unique-violation maps to invariant for
                // parity with MemoryStore.
                let s = format!("{rt}");
                if s.contains("23505") || s.contains("duplicate") {
                    Err(Error::Invariant("persist: duplicate row"))
                } else {
                    Err(Error::Persist(s))
                }
            }
            Err(e) => Err(map_dberr(e)),
        }
    }

    async fn get(&self, session: &SessionId, counter: Counter) -> Result<Option<EnvelopeRow>> {
        let model = Envelope::find_by_id((session.0.to_vec(), counter.get() as i64))
            .one(&self.db)
            .await
            .map_err(map_dberr)?;
        model.map(model_to_row).transpose()
    }

    async fn list_session(&self, session: &SessionId) -> Result<Vec<EnvelopeRow>> {
        let models = Envelope::find()
            .filter(chat_envelope::Column::SessionId.eq(session.0.to_vec()))
            .order_by_asc(chat_envelope::Column::Counter)
            .all(&self.db)
            .await
            .map_err(map_dberr)?;
        models.into_iter().map(model_to_row).collect()
    }

    async fn count(&self) -> Result<usize> {
        let n = Envelope::find().count(&self.db).await.map_err(map_dberr)?;
        Ok(n as usize)
    }
}

fn map_dberr(e: DbErr) -> Error {
    Error::Persist(format!("{e}"))
}

fn model_to_row(m: chat_envelope::Model) -> Result<EnvelopeRow> {
    if m.session_id.len() != 32 {
        return Err(Error::Persist(format!(
            "session_id wrong length: {}",
            m.session_id.len()
        )));
    }
    if m.dest_hash.len() != 16 {
        return Err(Error::Persist(format!(
            "dest_hash wrong length: {}",
            m.dest_hash.len()
        )));
    }
    let mut sid = [0u8; 32];
    sid.copy_from_slice(&m.session_id);
    let mut dest = [0u8; 16];
    dest.copy_from_slice(&m.dest_hash);
    EnvelopeRow::new(
        SessionId(sid),
        Counter(m.counter.max(0) as u64),
        DestHash(dest),
        m.ciphertext,
    )
}

// ---------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    //! Integration tests run only when `DATABASE_URL` is set so the
    //! default `cargo test --workspace` stays fast. Use
    //!   `DATABASE_URL=postgres://... cargo test -p trios-chat-br-io-chat-05`
    //! to exercise. Semantics are otherwise covered by the
    //! `MemoryStore` tests in CR-CHAT-05.

    use super::*;

    #[tokio::test]
    async fn pg_round_trip_when_database_url_present() {
        let url = match std::env::var("DATABASE_URL") {
            Ok(v) => v,
            Err(_) => return,
        };
        let store = PgChatStore::connect(&url).await.expect("connect");
        store.run_migrations().await.expect("migrate");
        store.truncate_for_tests().await.expect("truncate");

        let row = EnvelopeRow::new(
            SessionId([0xAB; 32]),
            Counter(0),
            DestHash([0xCD; 16]),
            vec![0xEEu8; 64],
        )
        .unwrap();

        store.put(row.clone()).await.expect("insert");
        let back = store
            .get(&SessionId([0xAB; 32]), Counter(0))
            .await
            .expect("select")
            .expect("present");
        assert_eq!(back, row);
        assert_eq!(store.count().await.unwrap(), 1);

        // duplicate => invariant
        let dup = store.put(row).await;
        assert!(matches!(dup, Err(Error::Invariant("persist: duplicate row"))));

        store.truncate_for_tests().await.unwrap();
    }
}
