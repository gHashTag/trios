//! SeaORM Entity for `chat_envelope` — sealed envelopes at rest.
//!
//! Schema:
//! ```sql
//! CREATE TABLE chat_envelope (
//!     session_id  BYTEA NOT NULL,         -- 32 B opaque session id
//!     counter     BIGINT NOT NULL,        -- strictly-monotone
//!     dest_hash   BYTEA NOT NULL,         -- 16 B routing hint (R-CHAT-3)
//!     ciphertext  BYTEA NOT NULL,         -- AEAD output, padded class
//!     stored_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
//!     PRIMARY KEY (session_id, counter)
//! );
//! ```
//!
//! Per **R-CHAT-1** (NO PLAINTEXT AT REST) the `ciphertext` column is
//! the only payload field; it has already been AEAD-sealed by the
//! sender before reaching this layer.

use sea_orm::entity::prelude::*;

/// SeaORM Entity. Derived `EnumIter` is gated on the `with-uuid`
/// feature internally — we use chrono for timestamps and Vec<u8> for
/// the bytea columns.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "chat_envelope")]
pub struct Model {
    /// 32-byte opaque session identifier (BYTEA, primary key part).
    #[sea_orm(primary_key, auto_increment = false)]
    pub session_id: Vec<u8>,
    /// Strictly-monotone ratchet counter (BIGINT, primary key part).
    #[sea_orm(primary_key, auto_increment = false)]
    pub counter: i64,
    /// 16-byte routing hint; the mesh routes on this hash (R-CHAT-3).
    pub dest_hash: Vec<u8>,
    /// AEAD ciphertext, already padded to a fixed length class.
    pub ciphertext: Vec<u8>,
    /// Insertion timestamp — set to `now()` by Postgres on insert.
    pub stored_at: ChronoDateTimeUtc,
}

/// No outgoing relations from this table — sealed envelopes are
/// content-addressed by `(session_id, counter)`.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
