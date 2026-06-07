# Ring OS-00 — smoke

> Часть крейта `trios-operator-smoke` · Scaffolded for issue #238 · Invariant I5

## Назначение / Purpose

Smoke execution ring for trios-operator-smoke. Runs operator-level smoke tests against live or stubbed backends. Verifies that core trios operators (train, flash, race) are reachable and return correct status codes.

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

- Parent crate: [`trios-operator-smoke`](../../README.md)
- Anchor: `φ² + φ⁻² = 3 · TRINITY`
