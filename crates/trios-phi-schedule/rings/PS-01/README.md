# Ring PS-01 — executor

> Часть крейта `trios-phi-schedule` · Scaffolded for issue #238 · Invariant I5

## Назначение / Purpose

Executor ring for trios-phi-schedule. Applies the φ-LR schedule produced by PS-00 to a training loop step, returning the scheduled learning rate for the current step. Consumes PS-00 types.

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

- Parent crate: [`trios-phi-schedule`](../../README.md)
- Anchor: `φ² + φ⁻² = 3 · TRINITY`
