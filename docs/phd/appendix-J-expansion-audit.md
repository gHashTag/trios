# Appendix J Expansion Audit · Phase 2 STUB-KILL task 2.10 · trios#380

**Branch:** `feat/phd-phase2-stubkill-2-10` (stacked on `feat/phd-phase2-stubkill-2-8`, tip `cf3033c`)
**Author:** Dmitrii Vasilev `<raoffonom@icloud.com>`, ORCID 0009-0008-4294-6159
**Anchor:** φ² + φ⁻² = 3 · DOI 10.5281/zenodo.19227877 · defense 2026-06-15

## Pre/post state

| Metric | Pre | Post |
|---|---:|---:|
| File `docs/phd/appendix/J-troubleshooting.tex` size (B) | 6,100 | 16,812 |
| File line count | 136 | 340 |
| New `\label{}` sites | 0 | 12 (`sec:appJ-overview`, `sec:appJ-blk001..005`, `sec:appJ-summary`, `sec:appJ-crosslink`, `sec:appJ-lessons`, `sec:appJ-reproduction`, `sec:appJ-open`, `sec:appJ-falsify`) |
| `\begin/\end` environments | balanced | balanced (15/15) |
| Dangling `\ref` introduced | 0 | 0 (caught + fixed `ch:29`, `tab:none` before commit) |

## Sections added

- **§J.0 Overview** — STROBE rationale, blocker taxonomy (3 firmware / 1 electrical / 1 informational), critical-path delay analysis dominated by BLK-001 (3 days)
- **§J.6 Resolution Summary** — pre-existing 5-row table preserved verbatim, now linked from §J.7 cross-link bridge
- **§J.7 BLK ↔ chapter / Coq / Zenodo cross-link** — 5-row table mapping each blocker to Reproducibility chapter (`ch_20:abstract`), Coq INV (none — wetware-firmware boundary, deliberately outside Coq scope per Appendix F charter), and Zenodo DOI Z-08 (`10.5281/zenodo.19227884`). 3/5 verified rows are honestly marked `audit-pending`.
- **§J.8 Lessons learned** — 5 numbered lessons, one per blocker, written as engineering checklist for future operators (macOS-FTDI conflict, GPIO input-only/pull-up semantics, JTAG bit-granularity vs 32-bit batching, IDCODE remarked-die parsimony, UART power-rail separation)
- **§J.9 Reproduction protocol** — Rust subcommand `cargo run -p trinity-fpga -- bringup` (R1 compliance: no `.sh` driver), idempotent, integration test `trinity_fpga::bringup::idempotent` gates FPGA-CI
- **§J.10 Open issues / audit-pending** — 4 honest open items: multi-cable concurrency, BLK-004 die forensics (no decap performed), wider-baud BLK-005 noise margin, XVC bridge boot-time
- **§J.11 Falsification hooks** — R7-style mini-Popper: 5 pre-registered observations that would invalidate the resolution claims of J.1–J.5

## Acceptance gates (all green)

- [x] File ≥ 8,192 B (16,812 ≥ 8,192) ✅
- [x] All 1170 `\label` sites unique (was 1158 pre-patch + 12 new) ✅
- [x] 0 duplicate label keys ✅
- [x] 0 dangling `\ref` (caught and fixed `ch:29` → `ch_20:abstract`, `tab:none` → inline UG470 reference) ✅
- [x] All `\begin/\end` balanced (15/15) ✅
- [x] R1: zero `.py` / `.sh` blocks (Reproduction protocol uses `cargo run -p trinity-fpga -- bringup`) ✅
- [x] R5 honesty: 3/5 Zenodo verifications marked `audit-pending` rather than asserted; BLK-004 die forensics open; multi-cable concurrency open; >115200-baud noise open ✅
- [x] R7 falsification hooks: pre-registered for all 5 BLK entries ✅
- [x] R10 atomic commit: single commit `feat(phd-phase2-stubkill-2-10): expand App.J Troubleshooting...` ✅

## Files committed

| File | Δ lines | Notes |
|---|---:|---|
| `docs/phd/appendix/J-troubleshooting.tex` | +204 | 6,100 B → 16,812 B; sections J.0, J.7-J.11 added; J.1-J.6 preserved verbatim with `\label` injection |
| `docs/phd/appendix-J-expansion-audit.md` | +new | this file |

## Next in Phase 2

- task 2.7 (App.F FPGA bitstream + SHA-256, 4,932 B → ≥8 KB) — most sensitive; R5 around SHA-256 claims
- task 2.9 (App.I XDC pin map QMTech XC7A100T, 4,435 B → ≥8 KB)
- After 10/10: pivot to Phase 3 R-RULES AUDIT
