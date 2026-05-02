# AGENTS — SR-05

## Active soul

- `Loop-Locksmith` — author, contract design, R7 audit triplet, mock RailwayApi backend, observe-only state machine.

## Constitutional rules honoured

- **R1** — pure Rust (no `.py` inside `crates/`, no `.sh`).
- **R5-honest** — concrete reqwest GraphQL client + git dep on `trios-railway-audit` deferred to BR-IO ring; documented in README §Honest scope.
- **R-RING-DEP-002** — Silver-tier deps only (`serde`, `serde_json`, `chrono`, `thiserror`, `tracing`, sibling SR-00..04 rings).
- **R-L6-PURE-007** — no `.py` files inside this ring.
- **L1** — no `.sh` files inside this ring tree (also asserted by `R-L1-ECHO-006`).
- **L13** (I-SCOPE) — only this ring touched by the SR-05 contract definitions.
- **L14** — every commit carries `Agent: Loop-Locksmith` trailer.
- **I5** — README, TASK, AGENTS, RING, Cargo.toml, src/lib.rs all present.
- **R7 audit triplet** — `RAIL=<verb> @ project=<8c> service=<8c> sha=<8c> ts=<rfc3339>` enforced by `R7Triplet::format`; the trimming-to-8 invariant is unit-tested.

## Anchor

`phi^2 + phi^-2 = 3`
