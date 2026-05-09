# TASK — CR-CHAT-01 (identity + sealed)

## Status: DONE — Wave-4 ring decomposition

Refs trinity-fpga#28, #29, #32 · part of `feat/trios-chat-rings`

## Done

- [x] Ring scaffold (RING.md / AGENTS.md / TASK.md / Cargo.toml / src/* / README.md)
- [x] `identity::Identity` migrated from monolith — `generate`, `lt_verifying`, `pre_x25519_pub`, `pre_mlkem_pub`, `build_bundle`, `sign`, `safety_number`
- [x] `identity::PrekeyBundle` + `PrekeyBundleBody` migrated — `verify`, `verify_at`, canonical bytes, signed
- [x] ML-KEM-768 placeholder (`MLKEM_PUB_LEN = 1184`, `MLKEM_SEC_LEN = 32`) preserved as `[ASPIRATIONAL]`
- [x] `sealed::SealedEnvelope::seal`/`unseal`, `dest_hash`, `symmetric_kdf` migrated from monolith
- [x] 13 unit tests (6 identity + 7 sealed)
- [x] `cargo clippy --all-targets -- -D warnings` clean

## Open (consumed by next rings)

- [ ] CR-CHAT-02 ratchet — depends on CR-CHAT-01 for `Identity` + `SealedEnvelope`
- [ ] CR-CHAT-03 group — depends on CR-CHAT-01 for `Identity::sign`
- [ ] BR-OUTPUT-CHAT — re-export `Identity`, `PrekeyBundle`, `SealedEnvelope`

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
