# TASK — SR-HACK-00 (trios-igla-race-hack)

## Status: DONE ✅ (initial release)

Closes #447 · Part of #446

## Completed

- [x] Outer GOLD III crate `trios-igla-race-hack` scaffolded (`Cargo.toml`, `src/lib.rs` ≤ 50 LoC, `RING.md`)
- [x] SR-HACK-00 ring at `rings/SR-HACK-00/` with `README.md`, `TASK.md`, `AGENTS.md`, `RING.md`, `Cargo.toml`, `src/lib.rs` (I5 mandatory)
- [x] Cargo deps limited to `serde + serde_json` (dev) — R-RING-DEP-002
- [x] `RingTier` enum (`Gold`, `Silver`, `Bronze`, `ColorVariant(String)`)
- [x] `Lane`     enum (`Algorithm`, `TttLora`, `Quantization`, `Megakernels`, `Theory`)
- [x] `Gate`     enum (`G1`, `G2`, `G3`)
- [x] `Term`     enum (16 canonical variants — exceeds the ≥ 15 acceptance threshold)
- [x] `Display` impl for every public type
- [x] Full `Serialize / Deserialize` roundtrip verified
- [x] `all_terms()` helper for SR-HACK-01..05 completeness audits
- [x] 9 unit tests — JSON roundtrip, snake_case / UPPERCASE serde format, stable Display strings, kind-tag presence
- [x] `forbid(unsafe_code)` + `deny(missing_docs)`
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `Agent: LEAD` trailer on commits (L14)

## Open (handed to SR-HACK-01..05)

- [ ] SR-HACK-01 — DM template engine consuming `Term`
- [ ] SR-HACK-02 — PR-comment renderer
- [ ] SR-HACK-03 — Leaderboard row formatter
- [ ] SR-HACK-04 — Discord embed builder
- [ ] SR-HACK-05 — Outreach audit (asserts every artefact uses `Term::*`, no free strings)

## Next ring

SR-HACK-01 — DM template engine (long-tail tracker #463).
