# RING — SR-01 (trios-igla-race-pipeline)

## Identity

| Field   | Value |
|---------|-------|
| Metal   | 🥈 Silver |
| Package | `trios-igla-race-pipeline-sr-01` |
| Sealed  | No |

## Purpose

Job FSM + in-process FIFO queue. SR-01 owns the lifecycle contract —
which `JobStatus` transitions are legal — and an in-memory queue the
trainer fleet pulls from.

## Why SR-01 sits above SR-00

SR-00 defines the static `JobStatus` enum. SR-01 turns the enum into a
guarded transition function so SR-02 (trainer-runner), SR-04 (gardener)
and BR-IO (advisory-lock layer) all share the same legality rules. The
FSM lives here exactly once.

## API Surface (pub)

| Item | Role |
|---|---|
| `transition(current, next)` | guarded transition (Result) |
| `is_valid_transition(a, b)` | boolean fast-path |
| `StrategyQueue` | `VecDeque<Scarab>` with FIFO O(1) push/claim |
| `FsmError` | `InvalidTransition`, `AlreadyClaimed` |

## Dependencies

- `trios-igla-race-pipeline-sr-00` (path)
- `serde`
- `chrono`
- `thiserror`

No `tokio`, no `sqlx`, no `reqwest`.

## Laws

- R1 — pure Rust
- L6 — no I/O, no async
- L13 — I-SCOPE: this ring only
- R-RING-DEP-002 — strict dep list above
- FSM contract frozen — extension via SR-00 ADR only

## Anchor

`φ² + φ⁻² = 3`
