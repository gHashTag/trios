# Ring IR-01 — telemetry

> Часть крейта `trios-igla-race` · Scaffolded for issue #238 · Invariant I5

## Назначение / Purpose

Telemetry ring for trios-igla-race. Collects per-step BPB observations, EMA values, and heartbeat signals from running training jobs. Streams telemetry data upward to BR-OUTPUT for persistence.

## Файлы / Expected files

| File | Role |
|------|------|
| `Cargo.toml` | Workspace member, Bronze tier |
| `src/lib.rs` | Ring entry point (placeholder → migration target) |
| `RING.md` | Ring identity and laws |
| `TASK.md` | Incremental migration checklist |
| `AGENTS.md` | Agent instructions for this ring |

## Зависимости / Dependency position

See `../RING.md` (parent crate ring graph).

## Ссылки / Links

- Parent crate: [`trios-igla-race`](../../README.md)
- Anchor: `φ² + φ⁻² = 3 · TRINITY`
