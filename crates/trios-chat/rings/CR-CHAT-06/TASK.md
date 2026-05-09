# TASK — CR-CHAT-06 (capability + injection)

## Status: DONE — Wave-4 ring decomposition

Refs trinity-fpga#28, #34 (capability), #36 (injection) · part of `feat/trios-chat-rings`

## Done

- [x] Ring scaffold (RING.md / AGENTS.md / TASK.md / Cargo.toml / src/* / README.md)
- [x] `capability::CapabilityToken` migrated — `issue` / `verify` / `signing_bytes`
- [x] `capability::ToolManifest` migrated — `sign` / `verify` / `signing_bytes`
- [x] `injection::classify_input` / `quarantine_wrap` / `validate_output` migrated
- [x] 49+ canonical deny patterns preserved
- [x] 11 unit tests (6 capability + 5 injection)
- [x] `cargo clippy --all-targets -- -D warnings` clean

## Open

- [ ] BR-OUTPUT-CHAT — re-export `CapabilityToken`, `Scope`, `ToolManifest`, `validate_output`

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
