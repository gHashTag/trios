# Ring IR-00 — orchestration

> Часть крейта `trios-igla-race` · Scaffolded for issue #238 · Invariant I5

## Назначение / Purpose

Orchestration ring for the trios-igla-race crate. Coordinates the IGLA race training loop: job dispatch, worker assignment, and lifecycle management of training runs. Bottom of the igla-race dependency graph.

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
