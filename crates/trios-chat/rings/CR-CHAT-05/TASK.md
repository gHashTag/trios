# TASK — CR-CHAT-05 (trios-chat)

## Status: IN-PROGRESS — Wave-3 ring decomposition

Refs trinity-fpga#28 #33 · part of `feat/trios-chat-rings`

## Done

- [x] Ring scaffold (RING.md / AGENTS.md / TASK.md / Cargo.toml / src/lib.rs)
- [x] `EnvelopeRow { session, counter, dest, ciphertext }` newtype
- [x] `Store` sync trait — `put / get / list_session / len / is_empty`
- [x] `MemoryStore` reference impl backed by `BTreeMap`
- [x] `EnvelopeRow::new` rejects ciphertexts <32 B
- [x] `Store::put` rejects duplicate `(session, counter)`
- [x] 7 unit tests — round-trip / duplicate / list-order / isolation /
  short-ct rejection / non-existent get / empty store

## Open (handed to next rings)

- [ ] BR-IO-CHAT-05 — concrete SeaORM-Postgres async impl
  (entities + Migrator + ActiveModel)
- [ ] BR-OUTPUT-CHAT — re-export Store trait alongside the rest

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
