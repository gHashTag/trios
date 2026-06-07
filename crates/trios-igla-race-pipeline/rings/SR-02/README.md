# SR-02 — Trainer Runner (E2E TTT O(1) per-chunk core)

**Soul-name:** `Race Runner` · **Codename:** `LEAD` · **Tier:** 🥉 Bronze

> Part of #446 · Anchor: `φ² + φ⁻² = 3`

## Назначение / What this ring does

End-to-end TTT (Test-Time Training) inner loop, O(1) per chunk.
SR-02 consumes a [`Scarab`] record from SR-01 (strategy queue) and
drives the training core: load model weights, iterate over chunks,
emit [`Heartbeat`] events, and return a final [`BpbSampleRow`] to
SR-03 (bpb-writer). CPU-only by default; the `tch` feature gates the
real libtorch backend.

## Dependency position

```
SR-02 (trainer-runner)
  ├─ SR-00  (scarab-types — wire format primitives)
  └─ SR-01  (strategy-queue — Job FSM + claim contention)
```

## Файлы / Expected files

| File | Role |
|------|------|
| `Cargo.toml` | Workspace member, Bronze tier; `tch` feature for libtorch |
| `src/lib.rs` | Ring entry point: `TrainerRunner` + `RunResult` |
| `RING.md` | Ring identity and laws |
| `TASK.md` | Incremental migration checklist |
| `AGENTS.md` | Agent instructions for this ring |

## Build

```bash
cargo build  -p trios-igla-race-pipeline-sr-02
cargo clippy -p trios-igla-race-pipeline-sr-02 --all-targets -- -D warnings
cargo test   -p trios-igla-race-pipeline-sr-02
```

## Ссылки / Links

- Parent crate: [`trios-igla-race-pipeline`](../../README.md)
- SR-00 scarab-types: [`../SR-00`](../SR-00/README.md)
- SR-01 strategy-queue: [`../SR-01`](../SR-01/README.md)
- Anchor: `φ² + φ⁻² = 3 · TRINITY`
