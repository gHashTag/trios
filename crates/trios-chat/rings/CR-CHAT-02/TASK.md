# TASK — CR-CHAT-02 (ratchet)

## Status: DONE — Wave-4 ring decomposition

Refs trinity-fpga#28, #30 · part of `feat/trios-chat-rings`

## Done

- [x] Ring scaffold (RING.md / AGENTS.md / TASK.md / Cargo.toml / src/lib.rs / README.md)
- [x] `RootKey` / `ChainKey` / `MessageKey` migrated
- [x] `Chain::from_root`, `send_next`, `recv_accept`, `dh_step` migrated
- [x] `SKIPPED_KEYS_CAP = 1024` exposed as a public constant
- [x] 10 unit tests (8 ratchet + 1 DH-symmetry + 1 memory-cap falsifier)
- [x] `cargo clippy --all-targets -- -D warnings` clean

## Open (consumed by next rings)

- [ ] BR-OUTPUT-CHAT — re-export `Chain`, `MessageKey`
- [ ] CR-CHAT-02-pq — concrete ML-KEM-768 mix-in (future PR)

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
