# Ring FP-00 — hdl

> Часть крейта `trios-fpga` · Scaffolded for issue #238 · Invariant I5

## Назначение / Purpose

HDL source ring for the trios-fpga FPGA build pipeline. Contains hardware description logic (Verilog generation, module definitions) for the Trinity S3AI FPGA build system. This is the bottom ring of the FPGA dependency graph.

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
