# TASK — SR-ALG-00 (trios-algorithm-arena)

## Status: DONE ✅

Closes #450 · Part of #446

## Completed

- [x] Outer GOLD II crate `trios-algorithm-arena` scaffolded (`Cargo.toml` nested workspace, `src/lib.rs` ≤ 50 LoC, `RING.md`)
- [x] SR-ALG-00 ring at `rings/SR-ALG-00/` with `README.md`, `TASK.md`, `AGENTS.md`, `RING.md`, `Cargo.toml`, `src/lib.rs` (I5 mandatory)
- [x] Cargo deps limited to `serde + uuid + hex` (+ `serde_json` dev) — R-RING-DEP-002
- [x] `AlgorithmId` newtype (UUID v4) with `Display`, `Hash`, `Eq`
- [x] `EntryHash([u8; 32])` — custom serde to lowercase hex (length 64)
- [x] `GoldenState([u8; 32])` — same hex layout, optional in spec
- [x] `EnvVar`, `EnvValue` newtypes (transparent serde)
- [x] `AlgorithmSpec` with: `algorithm_id`, `name`, `entry_path: PathBuf`, `entry_hash`, `env`, `golden_state_hash?`, `theorem?`
- [x] `AlgorithmSpec::new(name, path, hash)` cheap constructor
- [x] `AlgorithmSpec::verify_hash(&[u8; 32])` boolean check
- [x] 12 unit tests — uniqueness, hash length, hex roundtrip, optional fields, full roundtrip, schema field-name stability
- [x] `forbid(unsafe_code)` + `deny(missing_docs)`
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `Agent: Arena-Architect` trailer (L14)
- [x] L7 experience entry

## Open (handed to next rings)

- [ ] SR-ALG-01 jepa
- [ ] SR-ALG-02 universal-transformer
- [ ] **SR-ALG-03 e2e-ttt** ★ P0-CRITICAL — beat parameter-golf #1837 (`val_bpb < 1.07063`)
- [ ] BR-OUTPUT — AlgorithmArena assembler

## Next ring

SR-ALG-03 e2e-ttt (#457). It is the WIN ring for EPIC #446.
