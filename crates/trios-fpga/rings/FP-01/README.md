# Ring FP-01 — synthesis

> Часть крейта `trios-fpga` · Scaffolded for issue #238 · Invariant I5

## Назначение / Purpose

Synthesis ring for the trios-fpga crate. Drives Yosys synthesis and nextpnr place-and-route over the HDL produced by FP-00. Translates RTL to technology-mapped netlists for target FPGA families (XC7A100T, XC7A35T).

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
