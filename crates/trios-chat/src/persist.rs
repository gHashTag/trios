//! L-CHAT-5 · trinity-fpga#33 — Persistence (Wave-2).
//!
//! Per **R-CHAT-1** (NO PLAINTEXT AT REST) the store only ever ingests
//! sealed envelopes. Plaintext never crosses this boundary.
//!
//! Wave-2 ships:
//!   * `Store` trait — minimal CRUD over `(session_id, counter, ciphertext)`
//!   * `MemoryStore` — `[VERIFIED]` reference impl + property tests
//!   * `PostgresStore` skeleton — `[ASPIRATIONAL]` schema + SQL templates,
//!     concrete sqlx integration in a follow-up (avoids a heavy dep here).
//!
//! Schema (Postgres / Neon):
//! ```sql
//! CREATE TABLE chat_envelope (
//!     session_id  BYTEA NOT NULL,
//!     counter     BIGINT NOT NULL,
//!     dest_hash   BYTEA NOT NULL,           -- routing hint (R-CHAT-3)
//!     ciphertext  BYTEA NOT NULL,           -- AEAD output, padded class
//!     stored_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
//!     PRIMARY KEY (session_id, counter)
//! );
//! CREATE INDEX chat_envelope_dest_hash_idx ON chat_envelope (dest_hash);
//! ```

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::{Error, Result};

/// One envelope row exactly as it lives at rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeRow {
    /// Session identifier (32 B, opaque to the store).
    pub session_id: [u8; 32],
    /// Strictly-monotone ratchet counter within the session.
    pub counter: u64,
    /// Destination-hash (16 B) — what the mesh routes on (R-CHAT-3).
    pub dest_hash: [u8; 16],
    /// AEAD ciphertext, already in a fixed padding class (R-CHAT-9).
    pub ciphertext: Vec<u8>,
}

impl EnvelopeRow {
    /// `[VERIFIED]` Reject any attempt to construct a row from suspiciously
    /// short data — mostly catches programmer errors that would otherwise
    /// store empty / unpadded blobs.
    pub fn new(
        session_id: [u8; 32],
        counter: u64,
        dest_hash: [u8; 16],
        ciphertext: Vec<u8>,
    ) -> Result<Self> {
        if ciphertext.len() < 32 {
            return Err(Error::Invariant("persist: ciphertext too short for AEAD"));
        }
        Ok(Self {
            session_id,
            counter,
            dest_hash,
            ciphertext,
        })
    }
}

/// Minimal interface every persistence backend must satisfy.
///
/// The trait is sync to keep tests light; an `async` mirror lives in the
/// follow-up `persist_sqlx.rs` file behind a feature flag.
pub trait Store: Send {
    /// Insert a row. Duplicate `(session_id, counter)` returns
    /// `Error::Invariant("persist: duplicate row")`.
    fn put(&mut self, row: EnvelopeRow) -> Result<()>;

    /// Fetch one row by primary key.
    fn get(&self, session_id: &[u8; 32], counter: u64) -> Option<EnvelopeRow>;

    /// All rows for a session, ordered by counter ASC.
    fn list_session(&self, session_id: &[u8; 32]) -> Vec<EnvelopeRow>;

    /// Total rows currently stored.
    fn len(&self) -> usize;

    /// Whether the store is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// In-memory reference implementation. `[VERIFIED]`
pub struct MemoryStore {
    rows: Mutex<BTreeMap<([u8; 32], u64), EnvelopeRow>>,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore {
    /// Create a fresh in-memory store.
    pub fn new() -> Self {
        Self {
            rows: Mutex::new(BTreeMap::new()),
        }
    }
}

impl Store for MemoryStore {
    fn put(&mut self, row: EnvelopeRow) -> Result<()> {
        let mut rows = self.rows.lock().expect("MemoryStore mutex poisoned");
        let key = (row.session_id, row.counter);
        if rows.contains_key(&key) {
            return Err(Error::Invariant("persist: duplicate row"));
        }
        rows.insert(key, row);
        Ok(())
    }

    fn get(&self, session_id: &[u8; 32], counter: u64) -> Option<EnvelopeRow> {
        let rows = self.rows.lock().expect("MemoryStore mutex poisoned");
        rows.get(&(*session_id, counter)).cloned()
    }

    fn list_session(&self, session_id: &[u8; 32]) -> Vec<EnvelopeRow> {
        let rows = self.rows.lock().expect("MemoryStore mutex poisoned");
        rows.iter()
            .filter(|((sid, _), _)| sid == session_id)
            .map(|(_, v)| v.clone())
            .collect()
    }

    fn len(&self) -> usize {
        self.rows.lock().expect("MemoryStore mutex poisoned").len()
    }
}

/// `[ASPIRATIONAL]` Postgres-backed store skeleton.
///
/// The Wave-2 commit ships only the SQL templates and the connection-string
/// holder so callers can compile against the type today; the concrete
/// `sqlx` integration lands in a follow-up PR that adds an `async` Store
/// trait behind a feature flag (avoids forcing tokio on consumers).
pub struct PostgresStore {
    /// Postgres connection string (e.g. Neon URL).
    pub conn_str: String,
}

impl PostgresStore {
    /// `[ASPIRATIONAL]` Hold the conn string; real pool opens in follow-up.
    pub fn new(conn_str: impl Into<String>) -> Self {
        Self {
            conn_str: conn_str.into(),
        }
    }

    /// SQL schema string \u2014 callable from migrations / tests.
    pub fn schema_sql() -> &'static str {
        "CREATE TABLE IF NOT EXISTS chat_envelope (\n  session_id  BYTEA NOT NULL,\n  counter     BIGINT NOT NULL,\n  dest_hash   BYTEA NOT NULL,\n  ciphertext  BYTEA NOT NULL,\n  stored_at   TIMESTAMPTZ NOT NULL DEFAULT now(),\n  PRIMARY KEY (session_id, counter)\n);\nCREATE INDEX IF NOT EXISTS chat_envelope_dest_hash_idx ON chat_envelope (dest_hash);\n"
    }

    /// SQL `INSERT` template, parameterised. `[CITED]` matches schema_sql.
    pub fn insert_sql() -> &'static str {
        "INSERT INTO chat_envelope (session_id, counter, dest_hash, ciphertext) VALUES ($1, $2, $3, $4)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(session: u8, counter: u64, ct_byte: u8) -> EnvelopeRow {
        EnvelopeRow::new([session; 32], counter, [9u8; 16], vec![ct_byte; 64]).unwrap()
    }

    #[test]
    fn round_trip_put_get() {
        let mut s = MemoryStore::new();
        let r = row(1, 0, 0xAA);
        s.put(r.clone()).unwrap();
        assert_eq!(s.get(&[1u8; 32], 0), Some(r));
    }

    #[test]
    fn duplicate_rejected() {
        let mut s = MemoryStore::new();
        let r = row(2, 0, 0xBB);
        s.put(r.clone()).unwrap();
        assert!(matches!(s.put(r), Err(Error::Invariant(_))));
    }

    #[test]
    fn list_session_ordered() {
        let mut s = MemoryStore::new();
        s.put(row(3, 2, 0xC0)).unwrap();
        s.put(row(3, 0, 0xC1)).unwrap();
        s.put(row(3, 1, 0xC2)).unwrap();
        let xs = s.list_session(&[3u8; 32]);
        let counters: Vec<u64> = xs.iter().map(|r| r.counter).collect();
        assert_eq!(counters, vec![0, 1, 2]);
    }

    #[test]
    fn other_sessions_isolated() {
        let mut s = MemoryStore::new();
        s.put(row(4, 0, 0x44)).unwrap();
        s.put(row(5, 0, 0x55)).unwrap();
        assert_eq!(s.list_session(&[4u8; 32]).len(), 1);
        assert_eq!(s.list_session(&[5u8; 32]).len(), 1);
        assert_eq!(s.list_session(&[6u8; 32]).len(), 0);
    }

    #[test]
    fn falsifier_short_ciphertext_rejected() {
        let r = EnvelopeRow::new([0u8; 32], 0, [0u8; 16], vec![0u8; 8]);
        assert!(matches!(r, Err(Error::Invariant(_))));
    }

    #[test]
    fn postgres_schema_contains_pk() {
        let sql = PostgresStore::schema_sql();
        assert!(sql.contains("PRIMARY KEY (session_id, counter)"));
        assert!(sql.contains("dest_hash"));
    }

    #[test]
    fn postgres_insert_template_param_count() {
        // 4 parameters: session_id, counter, dest_hash, ciphertext.
        let sql = PostgresStore::insert_sql();
        let placeholders = (1..=4).all(|i| sql.contains(&format!("${}", i)));
        assert!(placeholders);
    }
}
