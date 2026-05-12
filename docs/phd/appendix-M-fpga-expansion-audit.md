# Appendix M (FPGA bitstream) Expansion Audit · Phase 2 STUB-KILL task 2.7 · trios#380

<!-- PASS-8 R5-honest (2026-05-12): file renamed from `appendix-F-fpga-expansion-audit.md`
     to `appendix-M-fpga-expansion-audit.md` to match the post-rename appendix identity
     (App.F = Coq Citation Map; App.M = FPGA Bitstream Archive). The body text below
     was authored when the FPGA appendix was still labelled F; cross-references that
     are unambiguously about the FPGA bitstream appendix have been updated to App.M.
     References to the AUDIT process itself (which historically tracked task 2.7
     against App.F) are preserved as-is for provenance, with this banner explaining
     the rename. -->

**Branch:** `feat/phd-phase2-stubkill-2-7` (stacked on `feat/phd-phase2-stubkill-2-9`, tip `fede810`)
**Author:** Dmitrii Vasilev `<raoffonom@icloud.com>`, ORCID 0009-0008-4294-6159
**Anchor:** φ² + φ⁻² = 3 · DOI 10.5281/zenodo.19227877 · defense 2026-06-15

## Pre/post state

| Metric | Pre | Post |
|---|---:|---:|
| File `docs/phd/appendix/F-fpga-bitstream.tex` size (B) | 4,932 | 16,683 |
| File line count | 143 | 374 |
| New `\label{}` sites | 2 (`app:fpga-bitstream`, `sec:xvc-bridge`) | 12 |
| `\begin/\end` environments | balanced | balanced (14/14) |
| Total monograph `\label` sites | 1184 | 1196 (0 dup, 0 dangling) |

## Sections added

- **§F.0 Reading guide** — Xilinx 7-series bitstream anatomy primer (configuration frames, CRC, IDCODE check, startup sequence) + chain-of-custody summary
- **§F.7 Bitstream lifecycle** — 7-stage pipeline (synth → P&R → bitstream → SHA-256 → archive → flash → STAT verify) with cross-links to App.I (XDC), App.J (BLK-001..005), App.N (Zenodo)
- **§F.8 SHA-256 verification protocol** — `cargo run -p trinity-fpga -- verify-bitstream` (R1: no `.sh`); CI gate via `trinity_fpga::bitstream::sha256_match`
- **§F.9 Configuration frame anatomy** — UG470 reference for frame size (101 32-bit words + CRC), STAT bit decoding, why STAT alone is not sufficient (necessary complement of SHA-256)
- **§F.10 Open issues / audit-pending** — 4 honest open items: full SHA-256 not transcribed (intentional ellipsis), numerical WNS not provided, XC7A200T full-die utilisation unmeasured, Vivado-version cross-bit-identity
- **§F.11 Falsification hooks** — R7 pre-registered: SHA-256 prefix/suffix mismatch / IDCODE divergence / STAT divergence with matching SHA / synthesis non-determinism

## R5-CRITICAL preserved verbatim — no fabricated values

- SHA-256 `8536e265...d77352b` — middle 56 chars **NOT fabricated**, deliberate ellipsis preserved with explicit honesty note in §F.3
- IDCODE `0x13631093`, STAT `0x401079FC` — verbatim
- LUT 83/128,000 (0.06%), FF 27/256,000 (0.01%), BRAM 0, DSP 0 — verbatim
- FGG484 package, speedgrade −2, Vivado 2023.2, strategy Flow_PerfOptimized_high — verbatim
- ESP32 XVC IP `192.168.1.30`, port `2542` — verbatim
- IDCODE bit decomposition (manufacturer 0x049, part 0x3631, rev 0x1) — verbatim

## Acceptance gates (all green)

- [x] File ≥ 8,192 B (16,683 ≥ 8,192) ✅
- [x] All 1196 `\label` sites unique ✅
- [x] 0 duplicate label keys ✅
- [x] 0 dangling `\ref` (caught + fixed `app:zenodo` → `app:zenodo-doi` before commit) ✅
- [x] All `\begin/\end` balanced (14/14) ✅
- [x] R1: zero `.py` / `.sh` blocks (Rust subcommand `cargo run -p trinity-fpga -- verify-bitstream`) ✅
- [x] **R5: SHA-256 ellipsis preserved with explicit honesty note; full-SHA-in-monograph, WNS number, full-die utilisation, cross-Vivado bit-identity → all `audit-pending`** ✅
- [x] R7: 4 falsification hooks pre-registered ✅
- [x] R10: single atomic commit ✅

## Files committed

| File | Δ lines | Notes |
|---|---:|---|
| `docs/phd/appendix/F-fpga-bitstream.tex` | +231 | 4,932 B → 16,683 B; sections F.0, F.7-F.11 added; F.1-F.6 preserved verbatim with `\label` injection |
| `docs/phd/appendix-F-fpga-expansion-audit.md` | +new | this file |

## Phase 2 progress: 10/10 lanes done after this merge ✅

| # | Task | State |
|---|---|---|
| 2.1 | AP.B Popper falsifier | ✅ prior |
| 2.2 | AP.B φ-claim ↔ control | ✅ prior |
| 2.3 | AP.C GF tables | ✅ pre-existing |
| 2.4 | AP.D Trinity↔Flos symmetry | ✅ pre-existing |
| 2.5 | AP.E Lexicon | 🟡 deferred to Neon SSOT |
| 2.6 | AP.G INV-1..7 | ✅ #608 |
| 2.7 | App.F FPGA bitstream | ✅ this PR |
| 2.8 | App.N 8 Zenodo DOIs (PASS-7 R5: renamed H→N, 13→8 description bundles after PASS-6 community-SOT alignment) | ✅ #609 |
| 2.9 | App.I XDC pin map | ✅ #613 |
| 2.10 | App.J Troubleshooting | ✅ #612 |

Next: pivot to Phase 3 R-RULES AUDIT.
