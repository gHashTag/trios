# TRIOS ISSUE RESOLUTION PLAN — 179 Open Issues

Generated: 2026-05-20
Status: ACTIVE

---

## PHASE 1 — 🔴 Critical Bugs (Day 1)

| # | Issue | Type | Scope |
|---|-------|------|-------|
| #827 | fake_quant.rs broken for ≤8-bit formats (R7 falsified by TIER1-posit8) | bug | trios-doctor / trios-phd |
| #779 | postrun-sidecar: parse format/algo/hidden from canon_name (8110 poisoned rows) | bug | igla pipeline |
| #721 | closure_gate.py seed_phi column missing on ssot.bpb_samples | bug, P1 | CI/infra |

---

## PHASE 2 — 🔴 P0 One-Shots (Day 1-2)

| # | Issue | Scope |
|---|-------|-------|
| #736 | L-CG-SCHEMA-FIX — repair closure_gate.py to match ssot.bpb_samples schema | CI fix |
| #740 | L-DOI-HONEST — cross-repo DOI/Coq provenance correction | docs/provenance |
| #844 | L-DPC29 — Wave-32 SYSTEM INTEGRATION PROBE | dispatch mirror |
| #845 | L-DPC26 — Wave-29 TENET Sparsity-Aware LUT (Target TOPS/W 195) | hardware |
| #846 | L-DPC30 — Wave-33 TENET UNSTRUCTURED ZERO-SKIP | dispatch mirror |
| #600 | L-SEED-CANON — Restore canonical seed discipline {47, 89, 144, 123} | runtime/tooling |
| #597 | L-MR-POSTRUN — Retrieve matrix_samples.jsonl from Railway runner | infra |

---

## PHASE 3 — 🟡 P1 Issues (Day 2-3)

| # | Issue | Scope |
|---|-------|-------|
| #641 | Wave 25 · L-MATRIX-DSN-ROTATE: rotate secrets.MATRIX_DATABASE_URL | infra/secrets |
| #645 | Wave 26 · L-GPTQ-ON-GF16: replicate calibration lever on Trinity GF16 | ML/experiment |
| #709 | Wave 29 ONE SHOT — IGLA pipeline repair (seed=43 + ON CONFLICT + DSN) | infra |
| #454 | [SR-02] trainer-runner — E2E TTT O(1) per-chunk core | architecture, ring |
| #762 | PASS-8 R5-honest deep-sweep — Appendix F→M rename + image-gate | docs |
| #766 | PASS-9 R5-honest deep-sweep — image-gate root-fix + broken refs | docs |

---

## PHASE 4 — Coq Proof Tracking (Day 3-5)

| # | Issue | Scope |
|---|-------|-------|
| #436 | Theorem-First Cascade — Phase 0–12 (master tracking) | meta |
| #437 | Phase 0.1 — Import CorePhi.v from t27 (4 trivial Qed) | Coq |
| #438 | Phase 0.8 — Import bpb_decreases_with_real_gradient (1 Qed) | Coq |
| #439 | Phase 0.9 — Import entropy_band_non_empty (1 Qed) | Coq |
| #440 | Phase 1 — Derived TrainerConfig.v | Coq |
| #559 | L-T27-PROOFSYNC — eliminate 32 stale Admitted in t27 fork | Coq |
| #587 | L-COQ-WITNESS-TODO: runtime-witness implementation tracking | Coq |
| #791 | L-S36 Coq: MultiPrecLucasCorrect proof tracking | Coq |
| #793 | L-S37 PhiPriorQuantCorrect: phi-prior weight quantizer correctness | Coq |
| #797 | L-S40 Coq: MeshDeterminismCorrect proof tracking | Coq |
| #799 | L-S43 Coq: PowerStateSafety proof tracking | Coq |
| #801 | L-S46 Merkle Aggregation + Replay Safety Coq Proof | Coq |
| #803 | L-S48 MultiDieAggCorrect.v — 8-die merkle aggregation | Coq |

---

## PHASE 5 — KAT Epic + Math/Theorem (Day 5-7)

| # | Issue | Scope |
|---|-------|-------|
| #572 | EPIC L-KAT — Kolmogorov–Arnold ↔ Trinity GF16 integration | meta |
| #574 | L-KAT-CH23: 23-gf16-algebra.tex §Kolmogorov–Arnold Foundation | LaTeX |
| #575 | L-KAT-CH24: 24-igla-architecture.tex §VSA matmul as KAT forward-pass | LaTeX |
| #576 | L-KAT-CH34: NEW 34-hardware-bridge.tex (≥1500 lines) | LaTeX |
| #578 | L-KAT-APX-G: NEW appendices/G-kat-bridge.tex | LaTeX |
| #610 | L-KAT-RW: ch_27.tex Related Work patch | LaTeX |
| #611 | L-KAT-12: Theorem 12.7 KART-GF(16) isomorphism | Coq + Rust |

---

## PHASE 6 — IGLA Race + Experiments (Day 5-7)

| # | Issue | Scope |
|---|-------|-------|
| #508 | IGLA RACE — Active Tracker | meta |
| #507 | INCIDENT: IGLA race champion-loop deadlocked | incident |
| #505 | ПРИКАЗ №1 — ALPHA→FINEWEB: Штурм Gate-3 | race-critical |
| #502 | EPIC: R0 Experiment Matrix — 5 waves | experiments |
| #445 | P0 ARCH: IGLA RACE 6-account Railway cycle | infra |
| #444 | BUG: trios-trainer-igla does not write BPB to NEON bpb_samples | bug |
| #442 | Parameter Golf Wish-List PRs: JEPA + Universal Transformer + E2E TTT | competition |
| #774 | R7 falsifiability: LANE×LR×seed diversity = 1 | experiment |
| #712 | Critic response — concrete evidence (5 blocks) | research |

---

## PHASE 7 — PhD Chapter Expansion (Day 7-14)

### Wave-9c: Thinnest chapters → ≥1000 LoC
| # | Issue |
|---|-------|
| #795 | Expand 5 thinnest PhD chapters to ≥1000 LoC |

### Wave-13c: Round-2 expansion
| # | Issue |
|---|-------|
| #805 | Round-2 thin chapter expansion (5 chapters) |

### Wave-14c: Round-3 expansion
| # | Issue |
|---|-------|
| #808 | flos_53, flos_57, flos_47, flos_41, flos_45 (≥1000 LoC each) |

### Wave-38/39 siblings
| # | Issue |
|---|-------|
| #879 | Wave-38 — Reversible dendritic NULLOR (Glava 84) |
| #890 | Wave-39 — Speculative early-exit (Glava 85) |

### PhD flos_71..74 (Capstone chapters)
| # | Issue |
|---|-------|
| #813 | flos_71 — TRI-27 Coptic ISA & 3-bank Register File |
| #814 | flos_72 — Sacred ALU FPGA → SKY130 Silicon Port |
| #815 | flos_73 — 21 Brain Modules as TRI-27 Microcode |
| #816 | flos_74 — Trinity DNA: Three-Strand Integration & TRI NET DePIN |

---

## PHASE 8 — PhD Book Chapters (Day 7-21, parallel)

### 🔴 P0 Chapters
| # | Chapter | Words |
|---|---------|-------|
| #381 | Ch.0 — Front Matter / Abstract / Cover | 250 |
| #385 | Ch.4 — GoldenFloat Family GF4 to GF64 | 900 |
| #387 | Ch.6 — Pre-registration and H1 | 500 |
| #388 | Ch.7 — Empirical Bridge (BPB benchmark results) | 1200 |
| #389 | Ch.8 — Negative Controls and Limitations | 600 |
| #399 | Ch.9 — GF vs FP16 BF16 MXFP4 baseline | 700 |
| #403 | Ch.16 — 360-lane phi-distance grid | 700 |
| #405 | Ch.19 — Statistical analysis | 500 |
| #416 | App.E — Pre-reg PDF + OSF + IGLA RACE results | 500 |
| #420 | Ch.26 — KOSCHEI ISA | 1100 |
| #422 | Ch.28 — QMTech φ-Numeric ALU (MEASURED) | 1300 |
| #425 | Ch.31 — Hardware-Numerics Empirical Bridge | 800 |
| #428 | Ch.34 — Energy Efficiency vs GPU baseline | 600 |
| #429 | App.F — Bitstream archive | 300 |
| #430 | App.H — Zenodo DOI registry | 400 |

### 🟡 P1 Chapters
| # | Chapter | Words |
|---|---------|-------|
| #383 | Ch.2 — Phyllotaxis and Vogel | 600 |
| #390 | Ch.9 — Live Dashboard | 500 |
| #391 | Ch.10 — Reproducibility | 400 |
| #395 | Ch.14 — Acknowledgments and Bibliography | 300 |
| #401 | Ch.13 — Sealed seeds protocol | 400 |
| #402 | Ch.14 — STROBE-style checklist | 400 |
| #404 | Ch.17 — Ablation 6/9 vs alternatives | 500 |
| #406 | Ch.21 — IGLA RACE distributed training | 500 |
| #407 | Ch.22 — Railway deployment + Tailscale | 400 |
| #408 | Ch.23 — MCP Server v2.2 endpoints | 500 |
| #415 | App.D — Reproduction kit | 300 |
| #418 | Ch.24 — Period-Locked Runtime Monitor | 700 |
| #421 | Ch.27 — TRI27 DSL and Dual-Target Codegen | 800 |
| #423 | Ch.29 — Sacred Formula V | 900 |
| #426 | Ch.32 — UART v6 protocol | 500 |
| #427 | Ch.33 — JTAG Access on macOS-ARM | 500 |
| #431 | App.I — XDC pin maps | 200 |

### 🟢 P2 / Deferred Chapters
| # | Chapter | Words |
|---|---------|-------|
| #382 | Ch.1 — Introduction Why Golden | 800 |
| #384 | Ch.3 — Trinity Identity | 700 |
| #386 | Ch.5 — Conjecture C1 | 700 |
| #392 | Ch.11 — Multi-Agent Methodology | 600 |
| #393 | Ch.12 — Hardware Bridge (deferred v3.0) | — |
| #394 | Ch.13 — Discussion and Future Work | 800 |
| #396 | Ch.4 — Sacred Formula V (Foundations) | 600 |
| #397 | Ch.5 — phi-distance and Fibonacci-Lucas seeds | 500 |
| #398 | Ch.8 — Ternary Floats TF3 TF9 | 600 |
| #400 | Ch.10 — Coq L1 range times precision | 600 |
| #411 | Ch.27 — Connections | 500 |
| #412 | Ch.28 — Future Work | 400 |
| #419 | Ch.25 — Phi-period Constants | 600 |
| #424 | Ch.30 — Trinity SAI Architecture | 700 |
| #432 | App.J — Troubleshoot log | 300 |
| #413 | App.B — Sacred constants catalogue (defer) | 500 |
| #414 | App.C — Verify checks (defer) | 400 |

---

## PHASE 9 — Sacred Geometry Chapters (Lane D, Day 14-21)

| # | Chapter |
|---|---------|
| #64 | Ch.0 — The Monad |
| #66 | Ch.2 — Vesica Piscis |
| #67 | Ch.3 — Golden Trident |
| #68 | Ch.4 — Golden Cut |
| #69 | Ch.5 — Golden Egg |
| #70 | Ch.6 — Seed of Life |
| #71 | Ch.7 — Flower of Life |
| #72 | Ch.8 — Fruit of Life |
| #73 | Ch.9 — Metatron's Cube |
| #74 | Ch.10 — Flower → Quasicrystal Extension |
| #75 | Ch.11 — Tetrahedron (Fire) |
| #76 | Ch.12 — Cube (Earth) |
| #77 | Ch.13 — Octahedron (Air) |
| #78 | Ch.14 — Dodecahedron (Cosmos) |
| #79 | Ch.15 — Icosahedron (Water) |
| #90 | Ch.26 — Golden Crystal |
| #91 | Ch.27 — Golden Architecture |
| #92 | Ch.28 — Golden Helix |
| #93 | Ch.29 — Golden Phyllotaxis |
| #94 | Ch.30 — Golden Harmony |
| #97 | Ch.33 — Golden Bloom |

---

## PHASE 10 — Lane Tasks (Day 14-28)

| # | Lane | Scope |
|---|------|-------|
| #41 | Lane A — LaTeX scaffold, build pipeline, 33 chapter placeholders | scaffolding |
| #43 | Lane B — Algebra core + Trinity Identity proof | math |
| #44 | Lane C — Physics constants + methodology | physics |
| #46 | Lane D — Sacred geometry + imagery | geometry |
| #47 | Lane E — GF16 + IGLA + benchmarks | experiments |
| #48 | Lane A2 — Bibliography, source-of-truth manifest | docs |
| #49 | Lane A3 — Appendix E (Lexicon) + Appendix F (Genealogy) | docs |
| #50 | Lane A4 — TikZ figures system | visuals |
| #51 | Lane A5 — Final integration, compile audit, Zenodo upload | publish |

---

## PHASE 11 — Golden Sunflowers PR Series (Day 14-28)

| # | PR | Scope |
|---|-----|-------|
| #372 | GOLDEN SUNFLOWERS — SINGLE SOURCE OF TRUTH | meta |
| #373 | MASTER EPIC: Complete PhD Field Map | meta |
| #374 | PR-1: §11 Empirical Bridge | chapter |
| #375 | PR-2: §4.4 GoldenFloat Family Theorem T6 | chapter |
| #376 | PR-3: Falsification Layer + Pre-registration | chapter |
| #377 | PR-4: §2 Vogel Phyllotaxis + Unifying Narrative | chapter |
| #378 | PR-5: §13 Multi-Agent Reproducibility | chapter |
| #379 | PR-6: Live BPB Dashboard | infra |

---

## PHASE 12 — Appendices + Genealogy (Day 21-28)

| # | Issue |
|---|-------|
| #99 | App.B — Golden Ledger (84 Coq proofs + SHA-256) |
| #103 | App.F — Golden Genealogy (Euclid→Trinity) |
| #104 | App.G — Golden Data (PDG/CODATA/Planck raw) |
| #105 | App.H — Sacred Geometry Constructions (Monad→Metatron) |

---

## PHASE 13 — Infrastructure + Security (Day 14-21)

| # | Issue | Scope |
|---|-------|-------|
| #632 | EPIC: Trinity Secure Chat | comms |
| #735 | Wave-24: Key Schedule transcript binding + Forward-secrecy ratchet | crypto |
| #231 | Sidepanel AgentChat v2 — markdown + rendering | UI |
| #569 | L-APIARY-REFRESH: prune closed lanes | maintenance |
| #571 | L-APIARY-T27-SCAN-L1: cross-repo array refactor | maintenance |

---

## PHASE 14 — PhD + Research Meta (Day 21-28)

| # | Issue | Scope |
|---|-------|-------|
| #39 | PhD Chapter 5: Golden Scales | chapter |
| #40 | PhD Chapter 4: Golden Harvest | chapter |
| #57 | PhD Chapter 9: Golden Stars | chapter |
| #62 | feat: crates/trios-phd — Rust-native PhD LaTeX pipeline | tooling |
| #63 | Golden Chain Orchestrator — The Learned Cat Pipeline | meta |
| #19 | OpenAI Parameter Golf — Competitive 16MB LM Submission | competition |
| #594 | L-CONSOLIDATE-16: restructure 48 → 16 chapters (PLAN ONLY) | planning |
| #619 | Defense rehearsal tracker — 3 sessions before viva 2026-06-15 | defense |
| #789 | PhD Defense Pack — Falsification appendix + Coq citation map | defense |
| #809 | L-S51: Trinity Loss — φ-prior-aware ternary contrastive loss | ML |
| #807 | JEPA-T ternary ingest pipeline (L-S50) | ML |

---

## PHASE 15 — Long-tail + Architecture (Day 28+)

| # | Issue | Scope |
|---|-------|-------|
| #940 | Queen Hive Scarab Strategy Plane — Sovereign Scarab v4 | strategy |
| #829 | Operator: publish B008 in trinity-s3ai community | documentation |
| #552 | L-DOI-SWEEP — cross-repo Zenodo citation sweep | documentation |
| #463 | Long-tail tracker — SR-HACK-01..05 + SR-MEM-02/03/04/06 | architecture |
| #264 | TRINITY HIVE — Queen's Registry & ONE SHOT Dispatch | meta |

---

## EXECUTION ORDER

```
Phase 1  (Day 1)     → #827, #779, #721              [3 bugs]
Phase 2  (Day 1-2)   → #736, #740, #844, #845, #846, #600, #597  [7 P0 one-shots]
Phase 3  (Day 2-3)   → #641, #645, #709, #454, #762, #766        [6 P1 issues]
Phase 4  (Day 3-5)   → #436-#440, #559, #587, #791, #793, #797, #799, #801, #803  [13 Coq]
Phase 5  (Day 5-7)   → #572, #574-#578, #610, #611              [7 KAT epic]
Phase 6  (Day 5-7)   → #508, #507, #505, #502, #445, #444, #442, #774, #712  [9 IGLA]
Phase 7  (Day 7-14)  → #795, #805, #808, #879, #890, #813-#816  [10 chapter expansion]
Phase 8  (Day 7-21)  → #381-#432 (parallel by priority tier)     [32 PhD chapters]
Phase 9  (Day 14-21) → #64-#97 Lane D chapters                  [21 sacred geometry]
Phase 10 (Day 14-28) → #41-#51 Lane tasks                        [9 lane tasks]
Phase 11 (Day 14-28) → #372-#379 Golden Sunflowers PR series     [7 PR chapters]
Phase 12 (Day 21-28) → #99, #103, #104, #105 Appendices          [4 appendices]
Phase 13 (Day 14-21) → #632, #735, #231, #569, #571 Infra       [5 infra]
Phase 14 (Day 21-28) → #39, #40, #57, #62, #63, #19, etc.       [12 PhD meta]
Phase 15 (Day 28+)   → #940, #829, #552, #463, #264 Long-tail   [5 long-term]

TOTAL: 179 issues across 15 phases, ~28 days
```

---

## Notes

- Phases 4-6, 7-9, 10-12 can run in parallel with different agents
- Phase 8 is the largest (32 chapters) — needs dedicated writing agents
- Phase 6 (IGLA Race) is time-sensitive — races don't wait
- Phase 15 items may be deferred or cancelled based on viva outcome (2026-06-15)
