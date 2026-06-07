# Ring BR-BITSTREAM — bitstream-output

> Часть крейта `trios-fpga` · Scaffolded for issue #238 · Invariant I5

## Назначение / Purpose

Facade / output ring for trios-fpga. Re-exports FP-00, FP-01, FP-02 under the unified `trios_fpga` API surface. Owns `BuildConfig`, `BuildPipeline`, `FlashConfig`, `FlashPipeline`, and `KnownBoard` that are exposed to downstream crates.

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

- Parent crate: [`trios-fpga`](../../README.md)
- Anchor: `φ² + φ⁻² = 3 · TRINITY`
