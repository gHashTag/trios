//! BR-IO-CHAT-05 — Chat persistence: SeaORM-backed Postgres backend.
//!
//! Anchor: `phi^2 + phi^-2 = 3 · TRINITY · CHAT · ZERO-METADATA`
//!
//! Bronze-tier sibling of CR-CHAT-05. This ring is the **only** place
//! in the trios-chat ring graph where an async runtime, sqlx, or
//! sea-orm is allowed to appear — keeping every Silver-tier ring
//! mock-testable and wasm-friendly.
//!
//! # Layout
//!
//! ```text
//! src/
//! ├── entities/      ← SeaORM Entity / Model / ActiveModel per table
//! │   ├── mod.rs
//! │   └── chat_envelope.rs
//! ├── migrations/    ← sea-orm-migration MigrationTrait per file
//! │   ├── mod.rs
//! │   └── m2026_05_09_000001_create_chat_envelope.rs
//! ├── store.rs       ← AsyncStore trait + PgChatStore impl
//! └── lib.rs         ← public re-exports
//! ```

#![forbid(unsafe_code)]

pub mod entities;
pub mod migrations;
pub mod store;

pub use migrations::Migrator;
pub use store::{AsyncStore, PgChatStore};
