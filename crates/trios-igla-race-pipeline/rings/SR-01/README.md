# SR-01 — Strategy Queue (Job FSM + claim contention)

**Soul-name:** `Queue Quartermaster` · **Codename:** `LEAD` · **Tier:** 🥈 Silver

> Closes #452 · Part of #446 · Anchor: `φ² + φ⁻² = 3`

## What this ring does

In-memory FIFO of [`Scarab`] entries plus a pure FSM transition guard
over `JobStatus`. SR-01 is the in-process semantics layer; the
persistent contention shim (Postgres advisory locks, `SKIP LOCKED`) is
out of scope and ships in a future BR-IO ring.

## Job FSM (mirrors SR-00 `JobStatus`)

```
Queued ─▶ Running ─┬─▶ Done
                   ├─▶ Pruned
                   └─▶ Errored
```

Every other transition (incl. self-loops, `Done → Running`,
`Queued → Done`) is rejected by `transition` with `FsmError::InvalidTransition`.

## API

```rust
pub fn transition(current: JobStatus, next: JobStatus) -> Result<JobStatus, FsmError>;
pub fn is_valid_transition(from: JobStatus, to: JobStatus) -> bool;

pub struct StrategyQueue { … }
impl StrategyQueue {
    pub fn new() -> Self;
    pub fn push(&mut self, scarab: Scarab) -> Result<(), FsmError>;
    pub fn claim_one(&mut self) -> Option<Scarab>;     // O(1), flips Queued→Running
    pub fn peek_next(&self) -> Option<&Scarab>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

pub enum FsmError {
    InvalidTransition { from: JobStatus, to: JobStatus },
    AlreadyClaimed   { job_id: JobId },
}
```

## Tests (12/12 GREEN)

| Test | Asserts |
|---|---|
| `fsm_queued_to_running_ok` | accepted transition |
| `fsm_running_to_terminal_states_ok` | `Running → Done/Pruned/Errored` |
| `fsm_queued_to_done_invalid` | guard rejects skip-running |
| `fsm_done_to_running_invalid` | terminal is terminal |
| `fsm_pruned_to_anything_invalid` | terminal is terminal |
| `fsm_self_loop_invalid` | every self-loop rejected |
| `is_valid_transition_matches_transition` | bool helper agrees with `transition` |
| `push_and_claim_fifo_order` | FIFO ordering |
| `claim_empty_returns_none` | empty queue returns `None` |
| `claim_one_flips_status_to_running` | claim stamps `started_at` |
| `push_rejects_non_queued` | only `Queued` may enter |
| `peek_next_does_not_consume` | non-mutating |
| `len_is_correct` | length accounting |
| `already_claimed_error_constructs` | `FsmError::AlreadyClaimed` shape pinned for SR-IO |
| `claim_then_push_running_is_invalid` | re-pushing `Running` is rejected |
| `o1_push_claim_smoke` | 100k push+claim < 1 s ⇒ `O(1)` amortised |

## Dependencies

`sr-00 + serde + chrono + thiserror` — strict `R-RING-DEP-002`.

## Build & test

```bash
cargo build  -p trios-igla-race-pipeline-sr-01
cargo clippy -p trios-igla-race-pipeline-sr-01 --all-targets -- -D warnings
cargo test   -p trios-igla-race-pipeline-sr-01
```

## Laws

- L1 ✓ no `.sh`
- L3 ✓ clippy clean
- L6 ✓ no I/O, no async
- L13 ✓ I-SCOPE: this ring only
- L14 ✓ `Agent: Queue-Quartermaster` trailer
