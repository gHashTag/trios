# TASK — CR-CHAT-04 (padding)

## Status: DONE — Wave-4 ring decomposition

Refs trinity-fpga#28, trinity-fpga#35 · part of `feat/trios-chat-rings`

## Done

- [x] Ring scaffold (RING.md / AGENTS.md / TASK.md / Cargo.toml / src/lib.rs / README.md)
- [x] `CLASSES = [256, 1024, 4096, 16384]` constant
- [x] `MAX_PAYLOAD = 16380` constant
- [x] `pad_class(&[u8]) -> Vec<u8>`
- [x] `unpad(&[u8]) -> Result<&[u8]>`
- [x] 7 unit tests (boundaries, round-trip, falsifier × 3, size-leak, max-payload)
- [x] `cargo clippy --all-targets -- -D warnings` clean

## Open

- [ ] (none — sealed-tier consumes via CR-CHAT-01)

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
