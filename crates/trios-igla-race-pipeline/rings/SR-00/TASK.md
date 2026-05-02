# TASK — SR-00 (trios-igla-race-pipeline)

## Status: DONE ✅ (initial release)

Closes #448 · Part of #446

## Completed

- [x] Outer GOLD I crate `trios-igla-race-pipeline` scaffolded (`Cargo.toml`, `src/lib.rs` ≤ 50 LoC, `RING.md`)
- [x] SR-00 ring at `rings/SR-00/` with `README.md`, `TASK.md`, `AGENTS.md`, `RING.md`, `Cargo.toml`, `src/lib.rs` (I5 mandatory)
- [x] Cargo deps limited to `serde + serde_json + uuid + chrono` (no `tokio`, no `sqlx`, no `reqwest`)
- [x] `JobId`, `WorkerId`, `Seed`, `StrategyId` newtypes with `Display`, `Hash`, `Eq`
- [x] `JobStatus` enum (`Queued`, `Running`, `Done`, `Pruned`, `Errored`) with snake_case serde
- [x] `Heartbeat` row (job_id, worker_id, ts, step?, bpb?) — matches `heartbeats` table
- [x] `BpbSampleRow` (job_id, step, bpb, ema?, ts) — matches `bpb_samples` table
- [x] `Scarab` composite (job_id, strategy_id, worker_id?, seed, status, created_at, started_at?, completed_at?, best_bpb?, best_step?, config) — matches `scarabs` table
- [x] `Scarab::queued()` builder
- [x] Full JSON roundtrip for every public type
- [x] Schema parity tests — assert field names emitted by serde match SQL columns
- [x] `forbid(unsafe_code)` + `deny(missing_docs)`
- [x] 13 unit tests GREEN, < 0.01 s
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `Agent: Constitutional-Cartographer` trailer on commit (L14)
- [x] L7 experience entry written

## Open (handed to next rings)

- [ ] SR-01 strategy-queue — Job FSM + claim contention (uses `StrategyId`, `JobStatus`)
- [ ] SR-02 trainer-runner — E2E TTT O(1) per-chunk core (consumes `Scarab`, emits `Heartbeat`)
- [ ] SR-03 bpb-writer — BPB+EMA+Neon write path (consumes `BpbSampleRow`)
- [ ] SR-04 gardener — ASHA pruner (reads `Scarab` snapshots)
- [ ] SR-05 railway-deployer — fleet integration
- [ ] BR-OUTPUT — IglaRacePipeline assembler
- [ ] Optional follow-up PR: backward-compat re-exports in `crates/trios-igla-race/src/{neon.rs,status.rs}`. Not required for SR-00 to land — SR-00 stands on its own.

## Next ring

SR-03 bpb-writer (#451) — first ring to depend on SR-00 + SR-01.
