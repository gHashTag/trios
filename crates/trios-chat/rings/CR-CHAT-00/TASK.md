# TASK — CR-CHAT-00 (trios-chat)

## Status: IN-PROGRESS — Wave-3 ring decomposition

Refs trinity-fpga#28 · part of `feat/trios-chat-rings`

## Done

- [x] Ring scaffold (RING.md / AGENTS.md / TASK.md / Cargo.toml / src/lib.rs)
- [x] `SessionId([u8; 32])` newtype, serde, hex-validated
- [x] `Counter(u64)` newtype with `next()`
- [x] `DestHash([u8; 16])` newtype
- [x] `EnvelopeMeta { session, counter, dest, padded_len }`
- [x] `Error` enum (thiserror) + `Result<T>` shorthand
- [x] `chat_laws()` returning the 12 R-CHAT laws as a static slice
- [x] 9 unit tests — newtype roundtrip, hex parse, every Error variant, law-table integrity
- [x] `cargo clippy --all-targets -- -D warnings` clean

## Open (handed to next rings)

- [ ] CR-CHAT-01 sealed — depends on CR-CHAT-00 for `EnvelopeMeta`
- [ ] CR-CHAT-02 ratchet — depends on CR-CHAT-00 for `Counter`
- [ ] CR-CHAT-03 group — depends on CR-CHAT-00 for `Error`/`Result`
- [ ] CR-CHAT-04 injection / capability / padding — `Error` consumer
- [ ] CR-CHAT-05 persist (Silver trait) — `EnvelopeMeta` consumer
- [ ] BR-IO-CHAT-05 SeaORM impl — entities mapped to CR-CHAT-00 newtypes
- [ ] BR-OUTPUT-CHAT — re-export ring assembling the whole stack

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
