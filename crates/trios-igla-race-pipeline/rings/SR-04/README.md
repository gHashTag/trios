# SR-04 — Gardener (ASHA pruner + INV status)

**Soul-name:** `Gardener General` · **Codename:** `LEAD` · **Tier:** 🥈 Silver

> Closes #456 · Part of #446 · Anchor: `φ² + φ⁻² = 3`

## Honest scope (R5)

This ring is the **Silver decision engine**. It does *not*:

- read from `gardener_runs` / `gardener_decisions` (Postgres I/O ships in a future BR-IO `gardener-pg` ring)
- run the worker pool (`race.rs`, 24 KB) — that loop ships in BR-OUTPUT
- compute `nca.rs` dual-band entropy (INV-4) — that ring is a separate SR-NCA-XX
- compute `lessons.rs` — out of scope for SR-04

What SR-04 owns: **`Gardener::should_prune` / `decide` / `apply_cull` / `pre_flight`** and the `GardenerSink` trait the future BR-IO adapter implements. Issue #456 also asks for a port of `asha.rs + invariants.rs + race.rs + nca.rs + rungs.rs + lessons.rs` — that is tracked as a **follow-up** because moving 85 KB of live-fleet code without an integrated tch / tokio harness is R5-unsafe and would silently break Phase-1 trainers.

## What landed

- `Gardener` — stateless ASHA decision engine.
- `DEFAULT_RUNGS` — 4-rung schedule (1 K, 3 K, 9 K, 27 K).
- `WARMUP_STEPS = 500` — INV-2 warmup guard (no cull legal under this).
- `ARCHITECTURAL_FLOOR_BPB = 2.19` — protects healthy seeds.
- `GardenerAction` (`Noop`, `Cull`, `Plateau`, `Promote`).
- `GardenerDecision` row, JSON-roundtrippable.
- `GardenerSink` trait — pluggable persistence (mirror of SR-03 `BpbSink`).
- `InvariantStatus` — read-through for INV-1..10.
- `Gardener::pre_flight()` — cheaply evaluates INV-2 + INV-8 (rest flow through the Coq bridge).

## Tests (16/16 GREEN)

| Group | Tests |
|---|---|
| INV-2 warmup | `warmup_never_prunes`, `warmup_emits_noop_decision`, `inv2_champion_always_survives_warmup` (exhaustive 0..=500 step sweep) |
| Architectural floor | `architectural_floor_protects` |
| Cull | `cull_fires_past_rung_when_bpb_above_threshold`, `cull_does_not_fire_below_rung_threshold`, `decide_cull_attaches_rung` |
| FSM | `non_running_never_prunes`, `apply_cull_flips_status`, `apply_cull_on_queued_fails_via_fsm` |
| Errors | `mismatched_job_id_errors` |
| Pre-flight | `pre_flight_default_rungs_inv2_ok`, `pre_flight_bad_rungs_inv2_fails` |
| Serde | `decision_roundtrip_json`, `action_serializes_tagged_kind` |
| Sink | `sink_trait_object_is_send` |

## Dependencies

- `trios-igla-race-pipeline-sr-00` (path) — wire format
- `trios-igla-race-pipeline-sr-01` (path) — FSM
- `trios-igla-race-pipeline-sr-03` (path) — `PHI_BAND_*`
- `serde`, `chrono`, `thiserror`
- `serde_json` (dev only)

No `sqlx`, no `tokio-postgres`, no `reqwest`. R-RING-DEP-002.

## Build & test

```bash
cargo build  -p trios-igla-race-pipeline-sr-04
cargo clippy -p trios-igla-race-pipeline-sr-04 --all-targets -- -D warnings
cargo test   -p trios-igla-race-pipeline-sr-04
```

## Laws

- L1 ✓ no `.sh`
- L3 ✓ clippy clean
- L4 ✓ tests before merge
- L6 ✓ async via the trait, no global runtime in lib
- L13 ✓ I-SCOPE: only `crates/trios-igla-race-pipeline/rings/SR-04/`
- L14 ✓ `Agent: Gardener-General` trailer
- INV-2 ✓ champion-survives property test
- R-RING-DEP-002 ✓ strict dep list above
- R-RING-FACADE-001 ✓ outer crate `src/lib.rs` re-exports only
