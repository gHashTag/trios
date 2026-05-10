# AGENTS.md — BR-IO-CHAT-05 (trios-chat)

## Identity

- Ring: BR-IO-CHAT-05
- Package: `trios-chat-br-io-chat-05`
- Role: SeaORM-backed Postgres impl of the persistence trait
- Codename: `BRONZE`

## What this ring does

Concrete `AsyncStore` (mirror of CR-CHAT-05's sync `Store`) wired to a
SeaORM pool. Owns the `chat_envelope` Entity, the Migrator, and the
ActiveModel CRUD path.

## Rules (ABSOLUTE)

- Bronze-tier exception to R-RING-DEP-002: tokio + sea-orm allowed
  **only here**.
- Migrations are append-only; `down()` is best-effort, callers must
  not rely on it for production rollback.
- Duplicate-key error MUST be reported as
  `Error::Invariant("persist: duplicate row")` (Silver-tier parity).

## You MAY

- ✅ Add new SeaORM entities for additional tables (group_state, …)
- ✅ Add new migrations (append-only)
- ✅ Tune connect-options on `PgChatStore::connect`
- ✅ Add integration tests gated on `$DATABASE_URL`

## You MAY NOT

- ❌ Re-export sea-orm types from CR-CHAT-* (Silver rings stay clean)
- ❌ Decrypt or inspect `ciphertext` (R-CHAT-1)
- ❌ Drop the `dest_hash` index — used for sealed-sender routing

## Build

```bash
cargo build  -p trios-chat-br-io-chat-05
cargo clippy -p trios-chat-br-io-chat-05 --all-targets -- -D warnings
cargo test   -p trios-chat-br-io-chat-05      # MemoryStore semantics
DATABASE_URL=postgres://... cargo test -p trios-chat-br-io-chat-05
```

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
