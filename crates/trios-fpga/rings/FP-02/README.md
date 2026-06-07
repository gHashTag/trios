# Ring FP-02 — bitstream

> Часть крейта `trios-fpga` · Scaffolded for issue #238 · Invariant I5

## Назначение / Purpose

Bitstream generation ring for trios-fpga. Consumes the placed-and-routed netlist from FP-01 and produces a binary bitstream file ready for JTAG flashing via openFPGALoader. Feeds into BR-BITSTREAM.

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
