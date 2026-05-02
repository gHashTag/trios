# RING — trios-algorithm-arena (Gold Crate, GOLD II)

## Identity

| Field    | Value |
|----------|-------|
| Metal    | 🥇 Gold |
| Type     | Crate (nested workspace) |
| Position | GOLD II — algorithm arena (algorithm-vs-pipeline registry) |
| Sealed   | No |

## Purpose

The canonical metadata layer for *individual algorithms* competing in
the IGLA race. While GOLD I (`trios-igla-race-pipeline`) owns the
**pipeline** primitives (BPB writer, scarabs, queues, gardener), GOLD
II owns the **algorithm spec** primitives — the manifest that pins
which Python entry point runs, which environment variables it expects,
which golden state it should converge to, and which Coq theorem (if
any) backs it.

## Why GOLD II is independent of GOLD I

- GOLD I = "how the fleet runs" (queues, BPB, ASHA, deploy)
- GOLD II = "what the fleet runs" (entry path, hash, env, theorem)

Separating the two means SR-ALG-00..03 + BR-OUTPUT (this crate) and
SR-00..05 + BR-OUTPUT (GOLD I) can ship independently and merge in
parallel.

## Ring Structure (L-ARCH-001)

```
crates/trios-algorithm-arena/
├── src/lib.rs                       ← re-export facade (≤ 50 LoC)
├── Cargo.toml                       ← outer GOLD, nested workspace
├── RING.md                          ← this file
└── rings/
    └── SR-ALG-00/                   ← arena-types (day 1)
        ├── README.md
        ├── TASK.md
        ├── AGENTS.md
        ├── RING.md
        ├── Cargo.toml
        └── src/lib.rs
```

Future rings: SR-ALG-01 jepa, SR-ALG-02 universal-transformer,
SR-ALG-03 e2e-ttt (★ P0 — beat parameter-golf #1837), BR-OUTPUT
AlgorithmArena assembler.

## Critical constraint — no Python in `crates/`

`AlgorithmSpec::entry_path` points *out* of `crates/` (typically into
`parameter-golf/records/...`). Real Python spawn is the job of SR-02
trainer-runner — not this crate. R-L6-PURE-007 forbids `.py` files
inside any Rust crate.

## Laws

- L1 — no `.sh`
- L3 — clippy clean
- L4 — tests before merge
- L13 — I-SCOPE: only this crate
- L14 — `Agent: <CODENAME>` trailer
- R-RING-FACADE-001 — `src/lib.rs` re-exports only

## Anchor

`φ² + φ⁻² = 3` · TRINITY · GOLD II
