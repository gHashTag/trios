# TASK — BR-IO-CHAT-05 (trios-chat)

## Status: IN-PROGRESS — Wave-3 ring decomposition

Refs trinity-fpga#28 #33 · part of `feat/trios-chat-rings`

## Done

- [x] Ring scaffold (RING.md / AGENTS.md / TASK.md / Cargo.toml / src/)
- [x] SeaORM 1.1 dependency wired with `sqlx-postgres +
  runtime-tokio-rustls + macros + with-chrono`
- [x] `entities/chat_envelope.rs` — Entity / Model / ActiveModel
- [x] `migrations/mod.rs` — Migrator entry-point
- [x] `migrations/m2026_05_09_000001_create_chat_envelope.rs` — full
  CREATE TABLE + CREATE INDEX with composite primary key
- [x] `store.rs` — `AsyncStore` trait + `PgChatStore` impl
  (`connect / run_migrations / put / get / list_session / count /
  truncate_for_tests`)
- [x] Duplicate-key (Postgres 23505) maps to
  `Error::Invariant("persist: duplicate row")` for parity with
  `MemoryStore`
- [x] Integration test gated on `$DATABASE_URL`
- [x] `cargo build -p trios-chat-br-io-chat-05` passes

## Open

- [ ] Wire `BR-OUTPUT-CHAT` (re-export) to expose either MemoryStore
  or PgChatStore behind a `BackendChoice` enum
- [ ] Add `group_state` / `welcome` entities + migrations once
  CR-CHAT-03 (group ring) is decomposed
- [ ] Replace string-match unique-violation detection with
  `DbErr::sql_state == "23505"` once SeaORM 1.2 lands

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
