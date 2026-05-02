# TASK — SR-04 (trios-igla-race-pipeline)

## Status: DONE ✅ (Silver decision engine); follow-up scoped

Closes #456 · Part of #446

## Completed

- [x] Ring at `rings/SR-04/` with I5 (`README.md`, `TASK.md`, `AGENTS.md`, `RING.md`, `Cargo.toml`, `src/lib.rs`)
- [x] `Gardener::should_prune(&Scarab, &BpbSampleRow) -> Result<bool, GardenerErr>`
- [x] `Gardener::decide(&Scarab, &BpbSampleRow) -> Result<GardenerDecision, GardenerErr>`
- [x] `Gardener::apply_cull(&mut Scarab) -> Result<(), GardenerErr>` (SR-01 FSM `Running → Pruned`)
- [x] `Gardener::pre_flight() -> InvariantStatus` — cheap INV-2 + INV-8 check
- [x] `DEFAULT_RUNGS` ASHA schedule (1 K, 3 K, 9 K, 27 K)
- [x] `WARMUP_STEPS = 500` INV-2 guard (champion-survives)
- [x] `ARCHITECTURAL_FLOOR_BPB = 2.19` protection
- [x] `GardenerAction` (`Noop`, `Cull`, `Plateau`, `Promote`) with serde tag
- [x] `GardenerDecision` JSON-roundtrippable row
- [x] `GardenerSink` trait — object-safe, mirror of SR-03 `BpbSink`
- [x] `InvariantStatus` — INV-1..10 read-through struct
- [x] 16 unit tests incl. exhaustive champion-survives property
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `Agent: Gardener-General` trailer (L14)

## Open (follow-up rings)

- [ ] **BR-IO `gardener-pg`** — sqlx / tokio-postgres adapter for `GardenerSink`; reads `bpb_samples`, writes `gardener_decisions` + `gardener_runs`
- [ ] **BR-OUTPUT worker pool** — ports `race.rs` (24 K) into the assembler
- [ ] **SR-NCA-XX** — ports `nca.rs` (14 K) dual-band entropy, INV-4
- [ ] **SR-LESSONS-XX** — ports `lessons.rs` (8 K) mistake memory
- [ ] **SR-04b** — plateau detector (currently `Plateau` variant exists but detection loop is in BR-OUTPUT)
- [ ] **SR-04c** — promote detector (same note)
- [ ] Optional: re-export shims in `crates/trios-igla-race/src/{asha,invariants,rungs}.rs` — only *after* `tch`-feature build matrix is sorted out

## Honest disclosure (R5)

Issue #456 asks for porting 85 KB of live-fleet code (`asha + invariants + race + nca + rungs + lessons`). This PR ships the **decision engine** portion (Silver, pure logic, 552 LoC). The rest is R5-unsafe to land without an integrated tch harness and is scoped into the follow-ups above.

## Next ring

BR-IO `gardener-pg` (new issue) — the first concrete `GardenerSink`.
