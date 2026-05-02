# RING — trios-igla-race-pipeline (Gold Crate, GOLD I)

## Identity

| Field    | Value |
|----------|-------|
| Metal    | 🥇 Gold |
| Type     | Crate (nested workspace) |
| Position | GOLD I — E2E TTT pipeline `O(1)` per chunk |
| Sealed   | No |

## Purpose

The canonical assembler for the E2E TTT (Test-Time Training) pipeline
whose per-chunk cost is independent of context length. All other GOLD I
rings (SR-01 strategy-queue, SR-02 trainer-runner, SR-03 bpb-writer,
SR-04 gardener, SR-05 railway-deployer, BR-OUTPUT IglaRacePipeline)
build on the typed primitives defined in SR-00.

## Why this crate exists

The legacy `crates/trios-igla-race/` mixes types, I/O, and async logic
in one file. Splitting the typed primitives into a dependency-free ring
lets every downstream ring depend on the same wire format without
inheriting tokio / sqlx / reqwest. SR-00 is the bottom of the GOLD I
graph and stays forever pure.

## Ring Structure (L-ARCH-001)

```
crates/trios-igla-race-pipeline/
├── src/lib.rs                     ← re-export facade (≤ 50 LoC)
├── Cargo.toml                     ← outer GOLD, nested workspace
├── RING.md                        ← this file
└── rings/
    └── SR-00/                     ← scarab-types (day 1)
        ├── README.md
        ├── TASK.md
        ├── AGENTS.md
        ├── RING.md
        ├── Cargo.toml
        └── src/lib.rs
```

Future rings (SR-01..05 + BR-OUTPUT) plug in here.

## Laws

- L1 — no `.sh` files
- L3 — `cargo clippy --all-targets -- -D warnings` clean
- L4 — tests before merge
- L13 — I-SCOPE: only this crate + 2 backward-compat re-export shims in `crates/trios-igla-race/`
- L14 — `Agent: <CODENAME>` trailer on every commit
- R-RING-FACADE-001 — `src/lib.rs` is re-exports only

## Anchor

`φ² + φ⁻² = 3` · TRINITY · GOLD I
