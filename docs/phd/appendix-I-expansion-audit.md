# Appendix I Expansion Audit · Phase 2 STUB-KILL task 2.9 · trios#380

**Branch:** `feat/phd-phase2-stubkill-2-9` (stacked on `feat/phd-phase2-stubkill-2-10`, tip `0066a58`)
**Author:** Dmitrii Vasilev `<raoffonom@icloud.com>`, ORCID 0009-0008-4294-6159
**Anchor:** φ² + φ⁻² = 3 · DOI 10.5281/zenodo.19227877 · defense 2026-06-15

## Pre/post state

| Metric | Pre | Post |
|---|---:|---:|
| File `docs/phd/appendix/I-xdc-pin-map.tex` size (B) | 4,435 | 16,227 |
| File line count | 141 | 378 |
| New `\label{}` sites | 1 | 14 (`app:xdc-pin-map` preserved + `sec:appI-reading`, `sec:appI-show`, `sec:appI-board`, `sec:appI-clock`, `sec:appI-uart`, `sec:appI-led`, `sec:appI-jtag`, `sec:appI-listing`, `sec:appI-banks`, `sec:appI-timing`, `sec:appI-provenance`, `sec:appI-reproduction`, `sec:appI-open`, `sec:appI-falsify`) |
| `\begin/\end` environments | balanced | balanced (18/18) |
| Total monograph `\label` sites | 1170 | 1184 (0 dup, 0 dangling) |

## Sections added

- **§I.0 Reading guide** — XDC primer for non-FPGA readers (PACKAGE_PIN / IOSTANDARD / timing decomposition)
- **§I.7 I/O bank topology** — pins grouped by bank (14, 15) with VCCO; UART bank assignment marked `audit-pending` per R5
- **§I.8 Timing constraint rationale** — 20 ns clock derivation, 92 MHz PLL multiplier ($9.2/5$), false-path semantics
- **§I.9 Schematic provenance** — exact source: QMTech "Artix-7 100T Core Board" schematic rev 2.1 + Xilinx UG475; UART pins flagged as one degree of indirection
- **§I.10 Reproduction protocol** — Rust `cargo run -p trinity-fpga -- synthesize`; deterministic synthesis flags pinned; SHA-256 cross-link to App.M
- **§I.11 Open issues / audit-pending** — 4 items: UART bank inference, blank FPGA-side JTAG column (intentional/correct), XC7A200T-vs-XC7A100T-CSG324 pinout, multi-board parity
- **§I.12 Falsification hooks** — R7 pre-registered: synthesis fail / IOSTANDARD violation / SHA-256 drift / negative WNS

## Preserved verbatim

- All 11 pin coordinates: U18, D20, E19, R14, P14, N16, M16, IO18, IO19, IO23, IO35
- All XDC `\begin{verbatim}` listings (clock + UART + LED + false-paths)
- IDCODE `0x13631093`, STAT `0x401079FC`
- 50 MHz oscillator → 92 MHz PLL chain
- φ² + φ⁻² = 3 anchor

## Acceptance gates (all green)

- [x] File ≥ 8,192 B (16,227 ≥ 8,192) ✅
- [x] All 1184 `\label` sites unique ✅
- [x] 0 duplicate label keys ✅
- [x] 0 dangling `\ref` ✅
- [x] All `\begin/\end` balanced (18/18) ✅
- [x] R1: zero `.py` / `.sh` blocks (Rust subcommand `cargo run -p trinity-fpga -- synthesize`) ✅
- [x] R5 honesty: UART bank D20/E19, XC7A200T full-die pinout, multi-board parity → `audit-pending` ✅
- [x] R7 falsification hooks: 4 pre-registered observations ✅
- [x] R10 atomic commit: single commit ✅

## Files committed

| File | Δ lines | Notes |
|---|---:|---|
| `docs/phd/appendix/I-xdc-pin-map.tex` | +237 | 4,435 B → 16,227 B; sections I.0, I.7-I.12 added; I.1-I.6 preserved verbatim with `\label` injection |
| `docs/phd/appendix-I-expansion-audit.md` | +new | this file |

## Phase 2 progress: 9/10 lanes done after this merge

Remaining: task 2.7 (App.F FPGA bitstream + SHA-256, 4,932 B → ≥8 KB) — most R5-sensitive.
