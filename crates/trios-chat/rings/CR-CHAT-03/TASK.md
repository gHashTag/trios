# TASK — CR-CHAT-03 (group)

## Status: DONE — Wave-4 ring decomposition

Refs trinity-fpga#28, #31 · part of `feat/trios-chat-rings`

## Done

- [x] Ring scaffold (RING.md / AGENTS.md / TASK.md / Cargo.toml / src/lib.rs / README.md)
- [x] `GroupId`, `Epoch`, `LeafIndex` newtypes migrated
- [x] `Welcome`, `Commit`, `Op` types migrated
- [x] `Group::create`, `process_commit`, `welcome_for` migrated
- [x] 7 unit tests (happy + 4 falsifiers + remove + welcome)
- [x] `cargo clippy --all-targets -- -D warnings` clean

## Open

- [ ] CR-CHAT-03-mls — concrete openmls integration (future PR)
- [ ] BR-OUTPUT-CHAT — re-export `Group`, `Commit`, `Welcome`

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
