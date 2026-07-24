//! trios-store — unified persistence for the consolidated Rust backend.
//!
//! Re-export facade only (L-ARCH-001: logic lives in `rings/`).
//!
//! ```no_run
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! let store = trios_store::open_and_migrate("./trios.db").await?;
//! let agents = store.list_agents().await?;
//! # let _ = agents; Ok(()) }
//! ```
//!
//! Ring structure:
//!   ST-00 — schema types (rows, enums) — pure data
//!   ST-01 — sqlx SQLite repository
//!   ST-02 — DDL migrations (mirror drizzle)
//!   BR-OUTPUT — open_and_migrate assembly

pub use trios_store_br_output::{migrations, open_and_migrate, types, Store};
pub use trios_store_st00 as schema;
