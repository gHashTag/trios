# RING — BR-IO-CHAT-05 (trios-chat)

## Identity

| Field   | Value |
|---------|-------|
| Tier    | 🥉 Bronze (I/O ring) |
| Package | `trios-chat-br-io-chat-05` |
| Path    | `crates/trios-chat/rings/BR-IO-CHAT-05/` |
| Sealed  | No |

## Purpose

Concrete async SeaORM-backed Postgres implementation of the Silver
trait declared in CR-CHAT-05. Mirrors the canonical pattern
`SR-MEM-05 ↔ BR-IO-MEM-05` from `crates/trios-agent-memory/rings/`.

This ring is the **only** location in the trios-chat ring graph where
an async runtime, sqlx, or sea-orm is allowed to appear.

## Public API

| Item | Role |
|---|---|
| `entities::chat_envelope::{Model, ActiveModel, Entity, Column}` | SeaORM table mapping |
| `Migrator`                                                       | sea-orm-migration MigratorTrait |
| `AsyncStore` trait                                               | async mirror of CR-CHAT-05's `Store` |
| `PgChatStore::connect(url)`                                      | open a pool + apply opts |
| `PgChatStore::run_migrations()`                                  | idempotent up-migration |
| `PgChatStore::put / get / list_session / count`                  | trait methods |
| `PgChatStore::truncate_for_tests`                                | test-only helper |

## Dependencies

- `trios-chat-cr-chat-00`, `trios-chat-cr-chat-05` (path)
- `sea-orm` 1.1 with `sqlx-postgres + runtime-tokio-rustls + macros`
- `sea-orm-migration` 1.1 matching feature set
- `tokio` runtime (Bronze-tier exception per R-RING-DEP-002)
- `async-trait` for the AsyncStore mirror
- `chrono`, `tracing`

## Invariants

- ✅ R-CHAT-1: only sealed `EnvelopeRow::ciphertext` ever crosses the
  process boundary; no plaintext primitive lives in this ring.
- ✅ Duplicate `(session, counter)` → `Error::Invariant("persist:
  duplicate row")` for parity with `MemoryStore`.
- ✅ Integration tests gate on `$DATABASE_URL` so default
  `cargo test --workspace` stays fast (semantics covered by
  `MemoryStore` in CR-CHAT-05).

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
