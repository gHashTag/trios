# RING — trios-fpga

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Type  | Crate (rings scaffolded for issue #238) |
| Sealed | No |

## Purpose

HDL, synthesis, bitstream and bitstream-output rings for trios-fpga — scaffolded under L-ARCH-001 (Tier 4 FPGA toolchain).

## Ring Structure (L-ARCH-001)

Rings: FP-00 (hdl), FP-01 (synthesis), FP-02 (bitstream), BR-BITSTREAM (bitstream-output)

```
crates/trios-fpga/
├── src/                 ← existing logic (untouched, re-export facade)
└── rings/               ← scaffolded for issue #238 (additive)
    ├── FP-00/  ← hdl
    ├── FP-01/  ← synthesis
    ├── FP-02/  ← bitstream
    ├── BR-BITSTREAM/  ← bitstream-output
```

## Dependency flow

```
BR-BITSTREAM → FP-02 → FP-01 → FP-00
```

## Anchor

`phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

## Laws

- R1 / R5 / R9: Ring isolation
- L7: Additive scaffold only — no behavior change to parent `src/`
- L-ARCH-001: `rings/` is the future home of all logic
