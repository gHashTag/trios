# Ring PS-00 — schedule

> Часть крейта `trios-phi-schedule` · Scaffolded for issue #238 · Invariant I5

## Назначение / Purpose

Schedule ring for trios-phi-schedule. Implements the φ-modulated (golden ratio) learning rate schedule: computes LR values at each step using the φ² + φ⁻² = 3 anchor invariant. Pure arithmetic, no I/O.

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
