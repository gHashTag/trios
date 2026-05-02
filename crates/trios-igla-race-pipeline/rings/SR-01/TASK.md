# TASK — SR-01 (trios-igla-race-pipeline)

## Status: DONE ✅

Closes #452 · Part of #446

## Completed

- [x] Ring at `rings/SR-01/` with `README.md`, `TASK.md`, `AGENTS.md`, `RING.md`, `Cargo.toml`, `src/lib.rs`
- [x] Pure `transition(current, next) -> Result<JobStatus, FsmError>` guard
- [x] `is_valid_transition` boolean helper
- [x] `StrategyQueue` (FIFO, `O(1)` push/claim) over `VecDeque<Scarab>`
- [x] `claim_one` flips `Queued → Running` and stamps `started_at`
- [x] `push` rejects non-`Queued` scarabs
- [x] `peek_next` (non-mutating)
- [x] `FsmError::InvalidTransition`, `FsmError::AlreadyClaimed` (shape pinned for the future SR-IO advisory-lock layer)
- [x] 16 unit tests — every FSM pair, FIFO ordering, claim semantics, 100k-op O(1) smoke
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `Agent: Queue-Quartermaster` trailer (L14)

## Open (handed to next rings)

- [ ] BR-IO `strategy-queue-pg` — Postgres advisory-lock / `SKIP LOCKED` adapter
- [ ] SR-02 trainer-runner — claims one scarab per worker tick
- [ ] SR-04 gardener — calls `transition(Running, Pruned)` on cull
- [ ] Optional: re-export shim in `crates/trios-igla-race/src/hive_automaton.rs`

## Next ring

SR-02 trainer-runner (#454).
