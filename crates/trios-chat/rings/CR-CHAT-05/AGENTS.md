# AGENTS.md — CR-CHAT-05 (trios-chat)

## Identity

- Ring: CR-CHAT-05
- Package: `trios-chat-cr-chat-05`
- Role: persistence trait + in-memory reference store
- Codename: `LEAD`

## What this ring does

Defines `Store` (sync CRUD) and `MemoryStore` (in-memory reference).
Concrete async SeaORM impl lives in **sibling BR-IO-CHAT-05**.

## Rules (ABSOLUTE)

- R1   — pure Rust
- L6   — no I/O, no async runtime in this ring
- L13  — I-SCOPE: only this ring
- R-RING-DEP-002 — deps = `cr-chat-00 + serde + serde_json + thiserror`
- **R-CHAT-1 enforcement** — public API surface MUST NOT have any
  plaintext-bearing argument. `EnvelopeRow::ciphertext: Vec<u8>` is
  the only payload field and the type system gives no decryption path
  inside this crate.

## You MAY

- ✅ Add helper queries (`list_session_paginated`, `count_session`, …)
- ✅ Add property tests
- ✅ Tighten invariants on `EnvelopeRow::new`

## You MAY NOT

- ❌ Add tokio / sqlx / sea-orm / reqwest
- ❌ Expose decryption helpers on `Store`
- ❌ Change the `Store::put` duplicate-error wording — downstream
  matches on it (`"persist: duplicate row"`)

## Build

```bash
cargo build  -p trios-chat-cr-chat-05
cargo clippy -p trios-chat-cr-chat-05 --all-targets -- -D warnings
cargo test   -p trios-chat-cr-chat-05
```

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
