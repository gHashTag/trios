# RING — CR-CHAT-05 (trios-chat)

## Identity

| Field   | Value |
|---------|-------|
| Tier    | 🥈 Silver (Core ring) |
| Package | `trios-chat-cr-chat-05` |
| Path    | `crates/trios-chat/rings/CR-CHAT-05/` |
| Sealed  | No |

## Purpose

Persistence contract for Trinity Secure Chat. Defines the synchronous
`Store` trait and ships a `[VERIFIED]` in-memory reference impl
(`MemoryStore`) so every other ring can be tested without standing up
Postgres.

The concrete SeaORM-Postgres backend lives in the sibling Bronze ring
**BR-IO-CHAT-05** — that is the only place where async / sea-orm /
sqlx ever appear in the trios-chat ring graph.

## Why CR-CHAT-05 stays Silver-tier (no I/O)

- Lets every higher-level ring (group / ratchet / falsifier runner)
  unit-test against `MemoryStore` without spinning Postgres.
- Keeps `R-CHAT-1` (NO PLAINTEXT AT REST) enforceable at the trait
  boundary: nothing on this side ever sees plaintext.
- Mirrors the canonical `SR-MEM-05 ↔ BR-IO-MEM-05` split established
  in `crates/trios-agent-memory/rings/`.

## Public API

| Item | Role |
|---|---|
| `EnvelopeRow`              | one row at rest (sealed envelope + meta) |
| `Store` trait              | sync CRUD over `(SessionId, Counter)` |
| `MemoryStore`              | `[VERIFIED]` reference impl |
| `MemoryStore::new()`       | fresh store |
| `MemoryStore::len() / put() / get() / list_session()` | trait methods |

## Dependencies

- `trios-chat-cr-chat-00` (path) — types only
- `serde`, `serde_json`, `thiserror`

## Invariants

- ❌ no tokio, sqlx, sea-orm, reqwest
- ✅ R-CHAT-1: no public API touches plaintext
- ✅ `EnvelopeRow::new` rejects ciphertexts shorter than 32 bytes
- ✅ `Store::put` rejects duplicate `(session, counter)` with
  `Error::Invariant("persist: duplicate row")`

## Sibling Bronze ring

The concrete async sea-orm impl lives in
`crates/trios-chat/rings/BR-IO-CHAT-05/` — it implements an `async`
mirror of this trait against a real Postgres pool.

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
