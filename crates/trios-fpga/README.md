# trios-fpga

FPGA build pipeline, JTAG flash, and board support for Trinity S3AI.

Extracted from the t27 compiler bootstrap, this crate provides:

- **Board profiles** — QMTECH XC7A100T, Arty A7-100T, Arty A7-35T
- **Build pipeline** — Verilog generation, Yosys synthesis, nextpnr P&R, bitstream generation
- **Flash pipeline** — JTAG bitstream loading via openFPGALoader
- **XDC constraints** — Parsing, preprocessing for nextpnr, minimal pin sets
- **Synth readiness** — Scan specs for synthesis readiness (parse, typecheck, Verilog gen, TDD)

## Usage

```rust
use trios_fpga::{BuildConfig, BuildPipeline, FlashConfig, FlashPipeline, KnownBoard};

// Build
let config = BuildConfig::new(".", KnownBoard::QmtechA100t).minimal(true).smoke(true);
let pipeline = BuildPipeline::new(config);
let result = pipeline.run(std::path::Path::new("t27c"))?;

// Flash
let flash = FlashConfig::new(KnownBoard::QmtechA100t);
let flash_pipeline = FlashPipeline::new(flash);
flash_pipeline.run()?;
```

## Supported Boards

| Board | FPGA | Clock | LEDs | SPI | MAC Debug |
|-------|------|-------|------|-----|-----------|
| QMTECH XC7A100T | xc7a100tcsg324 | 12 MHz | 8 | Yes | Yes |
| Arty A7-100T | xc7a100tcsg324 | 100 MHz | 4 | Yes | No |
| Arty A7-35T | xc7a35tcsg324 | 100 MHz | 4 | Yes | No |

## FPGA Module Catalog (33 modules)

mac, uart, spi, bridge, top_level, hir, hw_types, memory, clock_domain, fifo,
axi4, apb_bridge, gf16_accel, formal, ternary_isa, stdlib, simulator, assembler,
testbench, vcd_trace, e2e_demo, linker, timing, power, placement, partition,
router, dft, cts, crossopt, bootrom, sv_emit, firrtl, cdc, lint, coverage

phi^2 + 1/phi^2 = 3 | TRINITY
