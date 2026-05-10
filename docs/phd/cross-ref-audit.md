# Cross-Reference Audit — Phase 1 UNIFY task 1.5

**Branch:** `feat/phd-phase1-unify-1-5` (stacked on `feat/phd-phase1-unify-1-2`, PR #595)
**Issue:** [trios#380](https://github.com/gHashTag/trios/issues/380) task 1.5
**Anchor:** φ² + φ⁻² = 3 · DOI [10.5281/zenodo.19227877](https://zenodo.org/records/19227877)

## Summary

- **Total `\label{}` sites:** 1145
- **Unique label keys:** 1145
- **Duplicate label keys:** 0 (was 126 pre-patch — all eliminated)
- **Dangling refs:** 0 (was 0 — preserved)
- **Original referenced keys still resolved:** 119/119 (no breakage)

## Acceptance criteria (#380 task 1.5)

| Criterion | Status |
|---|---|
| Label→file map produced | ✅ this document |
| All `\ref` resolve to a label | ✅ 0 dangling |
| No duplicate labels remain | ✅ 0 duplicates |
| No referenced label was broken | ✅ 119/119 resolved |

## Patch logic

Originally, 7 structural keys (`abstract`, `introduction`, `results-evidence`, `qed-assertions`, `sealed-seeds`, `discussion`, `references`) plus ~119 unreferenced content-section keys were defined identically across 70 chapter files in `docs/phd/chapters/`. Because none were consumed by `\ref`/`\autoref`/`\eqref`/`\Cref`/`\pageref`, they produced LaTeX duplicate-label warnings without breaking any cross-reference.

**Rule:** for every `\label{KEY}` in a chapter file `<stem>.tex`:
- if `KEY` appears in any `\ref{KEY}` in the corpus → **leave bare** (protected)
- otherwise → rewrite to `\label{<stem>:KEY}` (idempotent: skip if already prefixed)

This eliminates collisions with zero risk of breaking cross-refs because protected keys (the 119 referenced ones) are already namespaced and unique.

## Label → File map

Total entries: 1145

<details><summary>Click to expand full map (1145 keys)</summary>

| Label key | File(s) |
|---|---|
| `app:A` | `appendix/A-catalogue.tex` |
| `app:F` | `appendix/F-coq-citation-map.tex` |
| `app:acm-ae` | `appendix/H-acm-ae-checklist.tex` |
| `app:data-availability` | `appendix/G-data-availability.tex` |
| `app:falsification` | `appendix/B-falsification.tex` |
| `app:fpga-bitstream` | `appendix/F-fpga-bitstream.tex` |
| `app:golden-benchmark` | `appendix/C-golden-benchmark.tex` |
| `app:golden-mirror` | `appendix/D-golden-mirror.tex` |
| `app:troubleshooting` | `appendix/J-troubleshooting.tex` |
| `app:xdc-pin-map` | `appendix/I-xdc-pin-map.tex` |
| `app:zenodo-doi` | `appendix/H-zenodo-doi.tex` |
| `ch:1` | `chapters/fa_01.tex` |
| `ch:11` | `chapters/fa_11.tex` |
| `ch:13` | `chapters/fa_13.tex` |
| `ch:15` | `chapters/fa_15.tex` |
| `ch:17-spiral` | `chapters/fa_17.tex` |
| `ch:18` | `chapters/fa_18.tex` |
| `ch:19` | `chapters/fa_19.tex` |
| `ch:21-experiments-jepa` | `chapters/fa_21.tex` |
| `ch:23-gf16-algebra` | `chapters/fa_23.tex` |
| `ch:24` | `chapters/fa_24.tex` |
| `ch:24-igla-arch` | `chapters/fa_24.tex` |
| `ch:25` | `chapters/fa_25.tex` |
| `ch:25-benchmarks` | `chapters/fa_25.tex` |
| `ch:26-data-analysis` | `chapters/fa_26.tex` |
| `ch:28` | `chapters/fa_28.tex` |
| `ch:28-momentum-algebra` | `chapters/fa_28.tex` |
| `ch:32` | `chapters/fa_32.tex` |
| `ch:33` | `chapters/fa_33.tex` |
| `ch:34` | `chapters/fa_33.tex` |
| `ch:6` | `chapters/fa_06.tex` |
| `ch:9` | `chapters/fa_09.tex` |
| `ch:benchmarks` | `chapters/fa_25.tex` |
| `ch:data-analysis` | `chapters/fa_26.tex` |
| `ch:e8-symmetry` | `chapters/fa_22.tex` |
| `ch:energy` | `chapters/fa_28.tex` |
| `ch:experiments-asha` | `chapters/fa_21.tex` |
| `ch:experiments-bpb` | `chapters/fa_21.tex` |
| `ch:experiments-gf16` | `chapters/fa_23.tex` |
| `ch:fibonacci` | `chapters/fa_07.tex` |
| `ch:fibonacci-tesselation` | `chapters/fa_07.tex` |
| `ch:gf16-algebra` | `chapters/fa_23.tex` |
| `ch:golden-egg` | `chapters/fa_01.tex` |
| `ch:golden-seed` | `chapters/fa_01.tex` |
| `ch:igla-architecture` | `chapters/fa_24.tex` |
| `ch:igla-race` | `chapters/fa_24.tex` |
| `ch:jepa` | `chapters/fa_21.tex` |
| `ch:lucas-closure` | `chapters/fa_29.tex` |
| `ch:lucas-ladder` | `chapters/fa_29.tex` |
| `ch:lucas-ring` | `chapters/fa_27.tex` |
| `ch:monad` | `chapters/fa_00.tex` |
| `ch:nca` | `chapters/fa_29.tex` |
| `ch:plm` | `chapters/fa_24.tex` |
| `ch:standard-model` | `chapters/fa_20.tex` |
| `ch:three-strands` | `chapters/fa_27.tex` |
| `ch:trinity-identity` | `chapters/fa_27.tex` |
| `ch:vesica-piscis` | `chapters/fa_11.tex` |
| `ch:vsa` | `chapters/fa_29.tex` |
| `ch_00:ch:0` | `chapters/ch_00.tex` |
| `ch_00:thm:0:1` | `chapters/ch_00.tex` |
| `ch_00:thm:0:2` | `chapters/ch_00.tex` |
| `ch_01:abstract` | `chapters/ch_01.tex` |
| `ch_01:ch1-s1-vision-extended` | `chapters/ch_01.tex` |
| `ch_01:ch1-s2-contributions` | `chapters/ch_01.tex` |
| `ch_01:ch1-s3-lineage` | `chapters/ch_01.tex` |
| `ch_01:ch1-s4-theorem-xref` | `chapters/ch_01.tex` |
| `ch_01:ch1-s5-roadmap` | `chapters/ch_01.tex` |
| `ch_01:ch1-s6-notation` | `chapters/ch_01.tex` |
| `ch_01:discussion` | `chapters/ch_01.tex` |
| `ch_01:introduction` | `chapters/ch_01.tex` |
| `ch_01:qed-assertions` | `chapters/ch_01.tex` |
| `ch_01:references` | `chapters/ch_01.tex` |
| `ch_01:research-questions-and-scope` | `chapters/ch_01.tex` |
| `ch_01:results-evidence` | `chapters/ch_01.tex` |
| `ch_01:sealed-seeds` | `chapters/ch_01.tex` |
| `ch_01:tab:ch1-falsification-matrix` | `chapters/ch_01.tex` |
| `ch_01:the-trinity-architecture-and-its-algebraic-substrate` | `chapters/ch_01.tex` |
| `ch_01:thm:ch1-alpha-phi-closed` | `chapters/ch_01.tex` |
| `ch_01:thm:ch1-lucas-closure` | `chapters/ch_01.tex` |
| `ch_02:abstract` | `chapters/ch_02.tex` |
| `ch_02:ch2-s1-kart-kan` | `chapters/ch_02.tex` |
| `ch_02:ch2-s2-finite-field` | `chapters/ch_02.tex` |
| `ch_02:ch2-s3-ternary` | `chapters/ch_02.tex` |
| `ch_02:ch2-s4-vsa` | `chapters/ch_02.tex` |
| `ch_02:ch2-s5-ltn` | `chapters/ch_02.tex` |
| `ch_02:ch2-s6-cliffs` | `chapters/ch_02.tex` |
| `ch_02:ch2-s7-gap` | `chapters/ch_02.tex` |
| `ch_02:ch2-s8-theorems` | `chapters/ch_02.tex` |
| `ch_02:discussion` | `chapters/ch_02.tex` |
| `ch_02:early-symbolicconnectionist-hybrids` | `chapters/ch_02.tex` |
| `ch_02:fibonacci-and-lucas-lattices-as-basis-sets` | `chapters/ch_02.tex` |
| `ch_02:gap-in-prior-art` | `chapters/ch_02.tex` |
| `ch_02:introduction` | `chapters/ch_02.tex` |
| `ch_02:logic-tensor-networks-and-differentiable-reasoning` | `chapters/ch_02.tex` |
| `ch_02:qed-assertions` | `chapters/ch_02.tex` |
| `ch_02:references` | `chapters/ch_02.tex` |
| `ch_02:representational-bottleneck-and-the-ux3c6-structural-prior` | `chapters/ch_02.tex` |
| `ch_02:results-evidence` | `chapters/ch_02.tex` |
| `ch_02:sealed-seeds` | `chapters/ch_02.tex` |
| `ch_02:sparse-and-ternary-neural-computation` | `chapters/ch_02.tex` |
| `ch_02:taxonomy-of-neuro-symbolic-paradigms` | `chapters/ch_02.tex` |
| `ch_02:the-normalisation-problem` | `chapters/ch_02.tex` |
| `ch_02:thm:ch2-phi-square` | `chapters/ch_02.tex` |
| `ch_02:thm:ch2-trinity` | `chapters/ch_02.tex` |
| `ch_02:vector-symbolic-architectures` | `chapters/ch_02.tex` |
| `ch_03:abstract` | `chapters/ch_03.tex` |
| `ch_03:ch3-s1-trinity-detail` | `chapters/ch_03.tex` |
| `ch_03:ch3-s2-phi-family` | `chapters/ch_03.tex` |
| `ch_03:ch3-s3-coq-listing` | `chapters/ch_03.tex` |
| `ch_03:ch3-s4-numeric` | `chapters/ch_03.tex` |
| `ch_03:ch3-s5-arch` | `chapters/ch_03.tex` |
| `ch_03:coq-mechanisation-and-sac-0-invariant` | `chapters/ch_03.tex` |
| `ch_03:derivation-of-the-anchor-identity` | `chapters/ch_03.tex` |
| `ch_03:discussion` | `chapters/ch_03.tex` |
| `ch_03:introduction` | `chapters/ch_03.tex` |
| `ch_03:invariant-sac-0` | `chapters/ch_03.tex` |
| `ch_03:minimal-polynomial-and-basic-consequences` | `chapters/ch_03.tex` |
| `ch_03:power-survey` | `chapters/ch_03.tex` |
| `ch_03:proof-architecture` | `chapters/ch_03.tex` |
| `ch_03:qed-assertions` | `chapters/ch_03.tex` |
| `ch_03:references` | `chapters/ch_03.tex` |
| `ch_03:relation-to-fibonacci-arithmetic` | `chapters/ch_03.tex` |
| `ch_03:results-evidence` | `chapters/ch_03.tex` |
| `ch_03:sealed-seeds` | `chapters/ch_03.tex` |
| `ch_03:the-integer-3-coincidence` | `chapters/ch_03.tex` |
| `ch_04:abstract` | `chapters/ch_04.tex` |
| `ch_04:ch4-s1-alpha-phi` | `chapters/ch_04.tex` |
| `ch_04:ch4-s2-dimensional` | `chapters/ch_04.tex` |
| `ch_04:ch4-s3-alpha-qed` | `chapters/ch_04.tex` |
| `ch_04:ch4-s4-derivation-levels` | `chapters/ch_04.tex` |
| `ch_04:ch4-s5-runtime` | `chapters/ch_04.tex` |
| `ch_04:ch4-s6-gate` | `chapters/ch_04.tex` |
| `ch_04:derivation-of-the-closed-form` | `chapters/ch_04.tex` |
| `ch_04:discussion` | `chapters/ch_04.tex` |
| `ch_04:introduction` | `chapters/ch_04.tex` |
| `ch_04:multiplicative-identity-and-kernel-integration` | `chapters/ch_04.tex` |
| `ch_04:qed-assertions` | `chapters/ch_04.tex` |
| `ch_04:references` | `chapters/ch_04.tex` |
| `ch_04:results-evidence` | `chapters/ch_04.tex` |
| `ch_04:sealed-seeds` | `chapters/ch_04.tex` |
| `ch_04:tab:ch4-dimensional` | `chapters/ch_04.tex` |
| `ch_05:abstract` | `chapters/ch_05.tex` |
| `ch_05:ch5-s1-lucas-closure` | `chapters/ch_05.tex` |
| `ch_05:ch5-s2-basin` | `chapters/ch_05.tex` |
| `ch_05:ch5-s3-seeds` | `chapters/ch_05.tex` |
| `ch_05:ch5-s4-coq-listing` | `chapters/ch_05.tex` |
| `ch_05:ch5-s5-admissibility` | `chapters/ch_05.tex` |
| `ch_05:ch5-s6-arch` | `chapters/ch_05.tex` |
| `ch_05:discussion` | `chapters/ch_05.tex` |
| `ch_05:fibonacci-lucas-seeds-and-their-contractive-basin` | `chapters/ch_05.tex` |
| `ch_05:introduction` | `chapters/ch_05.tex` |
| `ch_05:qed-assertions` | `chapters/ch_05.tex` |
| `ch_05:references` | `chapters/ch_05.tex` |
| `ch_05:results-evidence` | `chapters/ch_05.tex` |
| `ch_05:sealed-seeds` | `chapters/ch_05.tex` |
| `ch_05:the-ux3c6-distance-metric-and-the-balancing-fixed-point` | `chapters/ch_05.tex` |
| `ch_06:abstract` | `chapters/ch_06.tex` |
| `ch_06:coq-encoding` | `chapters/ch_06.tex` |
| `ch_06:discussion` | `chapters/ch_06.tex` |
| `ch_06:goldenfloat-format-definitions` | `chapters/ch_06.tex` |
| `ch_06:introduction` | `chapters/ch_06.tex` |
| `ch_06:key-theorems-and-proof-sketches` | `chapters/ch_06.tex` |
| `ch_06:lucas-closure-on-gf16` | `chapters/ch_06.tex` |
| `ch_06:preliminaries` | `chapters/ch_06.tex` |
| `ch_06:qed-assertions` | `chapters/ch_06.tex` |
| `ch_06:references` | `chapters/ch_06.tex` |
| `ch_06:results-evidence` | `chapters/ch_06.tex` |
| `ch_06:sealed-seeds` | `chapters/ch_06.tex` |
| `ch_07:abstract` | `chapters/ch_07.tex` |
| `ch_07:discussion` | `chapters/ch_07.tex` |
| `ch_07:from-the-trinity-identity-to-the-golden-angle` | `chapters/ch_07.tex` |
| `ch_07:h4-root-system-e8-lattice-and-the-varphi-scaled-block-decomposition` | `chapters/ch_07.tex` |
| `ch_07:introduction` | `chapters/ch_07.tex` |
| `ch_07:qed-assertions` | `chapters/ch_07.tex` |
| `ch_07:references` | `chapters/ch_07.tex` |
| `ch_07:results-evidence` | `chapters/ch_07.tex` |
| `ch_07:sealed-seeds` | `chapters/ch_07.tex` |
| `ch_08:abstract` | `chapters/ch_08.tex` |
| `ch_08:discussion` | `chapters/ch_08.tex` |
| `ch_08:gain-admissibility` | `chapters/ch_08.tex` |
| `ch_08:hybrid-qk-gain-invariant-inv-6` | `chapters/ch_08.tex` |
| `ch_08:introduction` | `chapters/ch_08.tex` |
| `ch_08:proof-sketch-for-admit_phi_sq` | `chapters/ch_08.tex` |
| `ch_08:qed-assertions` | `chapters/ch_08.tex` |
| `ch_08:references` | `chapters/ch_08.tex` |
| `ch_08:results-evidence` | `chapters/ch_08.tex` |
| `ch_08:sealed-seeds` | `chapters/ch_08.tex` |
| `ch_08:tf3-and-tf9-algebraic-structure` | `chapters/ch_08.tex` |
| `ch_08:tf9-product-encoding` | `chapters/ch_08.tex` |
| `ch_08:trit-encoding` | `chapters/ch_08.tex` |
| `ch_08:ux3c6-normalisation` | `chapters/ch_08.tex` |
| `ch_09:ablation-matrix-tier-abc-m1m6` | `chapters/ch_09.tex` |
| `ch_09:abstract` | `chapters/ch_09.tex` |
| `ch_09:competitor-format-summaries` | `chapters/ch_09.tex` |
| `ch_09:discussion` | `chapters/ch_09.tex` |
| `ch_09:gf16-format-specification` | `chapters/ch_09.tex` |
| `ch_09:gf16-phi_bias60-and-the-inv-3-safe-domain` | `chapters/ch_09.tex` |
| `ch_09:introduction` | `chapters/ch_09.tex` |
| `ch_09:inv-3-nine-coq-precision-bounds` | `chapters/ch_09.tex` |
| `ch_09:qed-assertions` | `chapters/ch_09.tex` |
| `ch_09:references` | `chapters/ch_09.tex` |
| `ch_09:results-evidence` | `chapters/ch_09.tex` |
| `ch_09:sealed-seeds` | `chapters/ch_09.tex` |
| `ch_10:abstract` | `chapters/ch_10.tex` |
| `ch_10:discussion` | `chapters/ch_10.tex` |
| `ch_10:gf16-range-and-precision-formalisation` | `chapters/ch_10.tex` |
| `ch_10:introduction` | `chapters/ch_10.tex` |
| `ch_10:qed-assertions` | `chapters/ch_10.tex` |
| `ch_10:references` | `chapters/ch_10.tex` |
| `ch_10:results-evidence` | `chapters/ch_10.tex` |
| `ch_10:sealed-seeds` | `chapters/ch_10.tex` |
| `ch_10:the-pareto-frontier-and-conjecture-c1` | `chapters/ch_10.tex` |
| `ch_11:abstract` | `chapters/ch_11.tex` |
| `ch_11:discussion` | `chapters/ch_11.tex` |
| `ch_11:hypothesis-formalisation-and-registration-protocol` | `chapters/ch_11.tex` |
| `ch_11:introduction` | `chapters/ch_11.tex` |
| `ch_11:inv-7-invariant-and-coq-formalisation` | `chapters/ch_11.tex` |
| `ch_11:qed-assertions` | `chapters/ch_11.tex` |
| `ch_11:references` | `chapters/ch_11.tex` |
| `ch_11:results-evidence` | `chapters/ch_11.tex` |
| `ch_11:sealed-seeds` | `chapters/ch_11.tex` |
| `ch_12:abstract` | `chapters/ch_12.tex` |
| `ch_12:bridge-architecture-and-interface-contracts` | `chapters/ch_12.tex` |
| `ch_12:clock-domain-analysis-and-timing` | `chapters/ch_12.tex` |
| `ch_12:discussion` | `chapters/ch_12.tex` |
| `ch_12:error-handling-protocol` | `chapters/ch_12.tex` |
| `ch_12:frequency-ratios-and-the-golden-ratio` | `chapters/ch_12.tex` |
| `ch_12:introduction` | `chapters/ch_12.tex` |
| `ch_12:logical-structure` | `chapters/ch_12.tex` |
| `ch_12:power-accounting` | `chapters/ch_12.tex` |
| `ch_12:qed-assertions` | `chapters/ch_12.tex` |
| `ch_12:references` | `chapters/ch_12.tex` |
| `ch_12:results-evidence` | `chapters/ch_12.tex` |
| `ch_12:sealed-seeds` | `chapters/ch_12.tex` |
| `ch_12:signal-naming-convention` | `chapters/ch_12.tex` |
| `ch_12:throughput-budget` | `chapters/ch_12.tex` |
| `ch_13:abstract` | `chapters/ch_13.tex` |
| `ch_13:discussion` | `chapters/ch_13.tex` |
| `ch_13:introduction` | `chapters/ch_13.tex` |
| `ch_13:qed-assertions` | `chapters/ch_13.tex` |
| `ch_13:references` | `chapters/ch_13.tex` |
| `ch_13:results-evidence` | `chapters/ch_13.tex` |
| `ch_13:sealed-seeds` | `chapters/ch_13.tex` |
| `ch_13:the-runtime-mirror-contract-and-igla_assertions.json` | `chapters/ch_13.tex` |
| `ch_13:the-strobe-seed-admissibility-criterion` | `chapters/ch_13.tex` |
| `ch_14:abstract` | `chapters/ch_14.tex` |
| `ch_14:bpb-definition-and-algebraic-properties` | `chapters/ch_14.tex` |
| `ch_14:byte-level-normalisation` | `chapters/ch_14.tex` |
| `ch_14:cross-entropy-and-perplexity` | `chapters/ch_14.tex` |
| `ch_14:discussion` | `chapters/ch_14.tex` |
| `ch_14:gate-2-bpb-1.85` | `chapters/ch_14.tex` |
| `ch_14:gate-3-bpb-1.50` | `chapters/ch_14.tex` |
| `ch_14:gate-thresholds-and-their-derivation` | `chapters/ch_14.tex` |
| `ch_14:introduction` | `chapters/ch_14.tex` |
| `ch_14:qed-assertions` | `chapters/ch_14.tex` |
| `ch_14:references` | `chapters/ch_14.tex` |
| `ch_14:relationship-to-the-darpa-energy-goal` | `chapters/ch_14.tex` |
| `ch_14:results-evidence` | `chapters/ch_14.tex` |
| `ch_14:sealed-seeds` | `chapters/ch_14.tex` |
| `ch_14:ux3c6-weighted-bpb` | `chapters/ch_14.tex` |
| `ch_15:abstract` | `chapters/ch_15.tex` |
| `ch_15:bpb-protocol-and-monotone-backward-invariant-inv-1` | `chapters/ch_15.tex` |
| `ch_15:database-schema` | `chapters/ch_15.tex` |
| `ch_15:discussion` | `chapters/ch_15.tex` |
| `ch_15:evaluation-protocol` | `chapters/ch_15.tex` |
| `ch_15:gate-evaluation` | `chapters/ch_15.tex` |
| `ch_15:introduction` | `chapters/ch_15.tex` |
| `ch_15:inv-1-bpb-monotone-backward` | `chapters/ch_15.tex` |
| `ch_15:qed-assertions` | `chapters/ch_15.tex` |
| `ch_15:railway-write-back-architecture` | `chapters/ch_15.tex` |
| `ch_15:references` | `chapters/ch_15.tex` |
| `ch_15:results-evidence` | `chapters/ch_15.tex` |
| `ch_15:sealed-seeds` | `chapters/ch_15.tex` |
| `ch_15:warmup-gate` | `chapters/ch_15.tex` |
| `ch_15:write-back-protocol` | `chapters/ch_15.tex` |
| `ch_16:abstract` | `chapters/ch_16.tex` |
| `ch_16:discussion` | `chapters/ch_16.tex` |
| `ch_16:grid-construction-and-sparsity-analysis` | `chapters/ch_16.tex` |
| `ch_16:introduction` | `chapters/ch_16.tex` |
| `ch_16:qed-assertions` | `chapters/ch_16.tex` |
| `ch_16:references` | `chapters/ch_16.tex` |
| `ch_16:results-evidence` | `chapters/ch_16.tex` |
| `ch_16:sealed-seeds` | `chapters/ch_16.tex` |
| `ch_16:the-phi-distance-function` | `chapters/ch_16.tex` |
| `ch_17:abstract` | `chapters/ch_17.tex` |
| `ch_17:analysis-of-effects-and-golden-ratio-structure` | `chapters/ch_17.tex` |
| `ch_17:discussion` | `chapters/ch_17.tex` |
| `ch_17:factor-definitions-and-experimental-design` | `chapters/ch_17.tex` |
| `ch_17:introduction` | `chapters/ch_17.tex` |
| `ch_17:qed-assertions` | `chapters/ch_17.tex` |
| `ch_17:references` | `chapters/ch_17.tex` |
| `ch_17:results-evidence` | `chapters/ch_17.tex` |
| `ch_17:sealed-seeds` | `chapters/ch_17.tex` |
| `ch_18:abstract` | `chapters/ch_18.tex` |
| `ch_18:coq.interval-upgrade-lane` | `chapters/ch_18.tex` |
| `ch_18:discussion` | `chapters/ch_18.tex` |
| `ch_18:hardware-and-runtime-limitations` | `chapters/ch_18.tex` |
| `ch_18:introduction` | `chapters/ch_18.tex` |
| `ch_18:qed-assertions` | `chapters/ch_18.tex` |
| `ch_18:references` | `chapters/ch_18.tex` |
| `ch_18:sealed-seeds` | `chapters/ch_18.tex` |
| `ch_18:state-of-the-art-comparison-clara-soa-snapshot` | `chapters/ch_18.tex` |
| `ch_19:abstract` | `chapters/ch_19.tex` |
| `ch_19:discussion` | `chapters/ch_19.tex` |
| `ch_19:introduction` | `chapters/ch_19.tex` |
| `ch_19:qed-assertions` | `chapters/ch_19.tex` |
| `ch_19:references` | `chapters/ch_19.tex` |
| `ch_19:results-evidence` | `chapters/ch_19.tex` |
| `ch_19:sealed-seeds` | `chapters/ch_19.tex` |
| `ch_19:test-design-and-hypotheses` | `chapters/ch_19.tex` |
| `ch_19:welch-t-statistic-and-degrees-of-freedom` | `chapters/ch_19.tex` |
| `ch_20:abstract` | `chapters/ch_20.tex` |
| `ch_20:algebraic-basis` | `chapters/ch_20.tex` |
| `ch_20:discussion` | `chapters/ch_20.tex` |
| `ch_20:hardware-and-software-specification` | `chapters/ch_20.tex` |
| `ch_20:hardware-pinning` | `chapters/ch_20.tex` |
| `ch_20:introduction` | `chapters/ch_20.tex` |
| `ch_20:non-determinism-budget` | `chapters/ch_20.tex` |
| `ch_20:qed-assertions` | `chapters/ch_20.tex` |
| `ch_20:references` | `chapters/ch_20.tex` |
| `ch_20:results-evidence` | `chapters/ch_20.tex` |
| `ch_20:sanctioned-seed-protocol` | `chapters/ch_20.tex` |
| `ch_20:sealed-seeds` | `chapters/ch_20.tex` |
| `ch_20:seed-assignment-to-experiments` | `chapters/ch_20.tex` |
| `ch_20:seed-verification` | `chapters/ch_20.tex` |
| `ch_20:software-environment` | `chapters/ch_20.tex` |
| `ch_21:abstract` | `chapters/ch_21.tex` |
| `ch_21:agent-topology` | `chapters/ch_21.tex` |
| `ch_21:definitions` | `chapters/ch_21.tex` |
| `ch_21:discussion` | `chapters/ch_21.tex` |
| `ch_21:formal-victory-criterion-inv-7` | `chapters/ch_21.tex` |
| `ch_21:introduction` | `chapters/ch_21.tex` |
| `ch_21:multi-agent-fleet-architecture` | `chapters/ch_21.tex` |
| `ch_21:qed-assertions` | `chapters/ch_21.tex` |
| `ch_21:rainbow-bridge-consistency-inv-7b` | `chapters/ch_21.tex` |
| `ch_21:references` | `chapters/ch_21.tex` |
| `ch_21:relation-to-varphi2-varphi-2-3` | `chapters/ch_21.tex` |
| `ch_21:results-evidence` | `chapters/ch_21.tex` |
| `ch_21:sealed-seeds` | `chapters/ch_21.tex` |
| `ch_21:six-refutation-theorems` | `chapters/ch_21.tex` |
| `ch_21:victory-declaration-protocol` | `chapters/ch_21.tex` |
| `ch_22:abstract` | `chapters/ch_22.tex` |
| `ch_22:discussion` | `chapters/ch_22.tex` |
| `ch_22:introduction` | `chapters/ch_22.tex` |
| `ch_22:qed-assertions` | `chapters/ch_22.tex` |
| `ch_22:references` | `chapters/ch_22.tex` |
| `ch_22:results-evidence` | `chapters/ch_22.tex` |
| `ch_22:satisfaction-witness-and-victory-predicate` | `chapters/ch_22.tex` |
| `ch_22:sealed-seeds` | `chapters/ch_22.tex` |
| `ch_22:worker-pool-invariants-and-falsification-witnesses` | `chapters/ch_22.tex` |
| `ch_23:abstract` | `chapters/ch_23.tex` |
| `ch_23:discussion` | `chapters/ch_23.tex` |
| `ch_23:introduction` | `chapters/ch_23.tex` |
| `ch_23:mcp-adapter-layer-architecture` | `chapters/ch_23.tex` |
| `ch_23:protocol-implementation-and-latency-analysis` | `chapters/ch_23.tex` |
| `ch_23:qed-assertions` | `chapters/ch_23.tex` |
| `ch_23:references` | `chapters/ch_23.tex` |
| `ch_23:results-evidence` | `chapters/ch_23.tex` |
| `ch_23:sealed-seeds` | `chapters/ch_23.tex` |
| `ch_24:abstract` | `chapters/ch_24.tex` |
| `ch_24:agent-model` | `chapters/ch_24.tex` |
| `ch_24:coq-encoding` | `chapters/ch_24.tex` |
| `ch_24:discussion` | `chapters/ch_24.tex` |
| `ch_24:formal-model-of-the-period-locked-monitor` | `chapters/ch_24.tex` |
| `ch_24:implementation-and-hardware-interface` | `chapters/ch_24.tex` |
| `ch_24:interrupt-interface-with-the-hardware-bridge` | `chapters/ch_24.tex` |
| `ch_24:introduction` | `chapters/ch_24.tex` |
| `ch_24:period-ratio-and-non-resonance` | `chapters/ch_24.tex` |
| `ch_24:priority-queue-and-phi-weighted-scheduling` | `chapters/ch_24.tex` |
| `ch_24:qed-assertions` | `chapters/ch_24.tex` |
| `ch_24:references` | `chapters/ch_24.tex` |
| `ch_24:results-evidence` | `chapters/ch_24.tex` |
| `ch_24:rtl-implementation` | `chapters/ch_24.tex` |
| `ch_24:sealed-seeds` | `chapters/ch_24.tex` |
| `ch_25:abstract` | `chapters/ch_25.tex` |
| `ch_25:cycle-classification-and-attention-periodicity` | `chapters/ch_25.tex` |
| `ch_25:discussion` | `chapters/ch_25.tex` |
| `ch_25:introduction` | `chapters/ch_25.tex` |
| `ch_25:qed-assertions` | `chapters/ch_25.tex` |
| `ch_25:references` | `chapters/ch_25.tex` |
| `ch_25:results-evidence` | `chapters/ch_25.tex` |
| `ch_25:sealed-seeds` | `chapters/ch_25.tex` |
| `ch_25:varphi-lattice-structure-and-the-cycle-map` | `chapters/ch_25.tex` |
| `ch_26:abstract` | `chapters/ch_26.tex` |
| `ch_26:discussion` | `chapters/ch_26.tex` |
| `ch_26:gf16_quant-galois-field-16-quantisation` | `chapters/ch_26.tex` |
| `ch_26:instruction-encoding` | `chapters/ch_26.tex` |
| `ch_26:introduction` | `chapters/ch_26.tex` |
| `ch_26:isa-register-file-and-encoding` | `chapters/ch_26.tex` |
| `ch_26:opcode-specifications` | `chapters/ch_26.tex` |
| `ch_26:phi_rope-ux3c6-rotary-position-encoding` | `chapters/ch_26.tex` |
| `ch_26:qed-assertions` | `chapters/ch_26.tex` |
| `ch_26:references` | `chapters/ch_26.tex` |
| `ch_26:register-file` | `chapters/ch_26.tex` |
| `ch_26:results-evidence` | `chapters/ch_26.tex` |
| `ch_26:sealed-seeds` | `chapters/ch_26.tex` |
| `ch_26:tf3_add-ternary-addition` | `chapters/ch_26.tex` |
| `ch_26:tf3_mul-ternary-multiplication` | `chapters/ch_26.tex` |
| `ch_26:vsa_bind-hyperdimensional-binding` | `chapters/ch_26.tex` |
| `ch_26:vsa_bundle-hyperdimensional-bundling` | `chapters/ch_26.tex` |
| `ch_26:vsa_unbind-hyperdimensional-unbinding` | `chapters/ch_26.tex` |
| `ch_27:abstract` | `chapters/ch_27.tex` |
| `ch_27:abstract-syntax` | `chapters/ch_27.tex` |
| `ch_27:discussion` | `chapters/ch_27.tex` |
| `ch_27:environments-and-evaluation` | `chapters/ch_27.tex` |
| `ch_27:introduction` | `chapters/ch_27.tex` |
| `ch_27:mechanised-proofs-determinism-and-exhaustiveness` | `chapters/ch_27.tex` |
| `ch_27:qed-assertions` | `chapters/ch_27.tex` |
| `ch_27:references` | `chapters/ch_27.tex` |
| `ch_27:relation-to-gf16-and-varphi-arithmetic` | `chapters/ch_27.tex` |
| `ch_27:results-evidence` | `chapters/ch_27.tex` |
| `ch_27:sealed-seeds` | `chapters/ch_27.tex` |
| `ch_27:ternary-arithmetic` | `chapters/ch_27.tex` |
| `ch_27:theorem-eval_det-determinism` | `chapters/ch_27.tex` |
| `ch_27:theorem-trit_exhaustive-exhaustiveness` | `chapters/ch_27.tex` |
| `ch_27:tri27-syntax-and-denotational-semantics` | `chapters/ch_27.tex` |
| `ch_28:abstract` | `chapters/ch_28.tex` |
| `ch_28:architecture-zero-dsp-ternary-datapath` | `chapters/ch_28.tex` |
| `ch_28:discussion` | `chapters/ch_28.tex` |
| `ch_28:introduction` | `chapters/ch_28.tex` |
| `ch_28:qed-assertions` | `chapters/ch_28.tex` |
| `ch_28:references` | `chapters/ch_28.tex` |
| `ch_28:resource-utilisation-and-timing-closure` | `chapters/ch_28.tex` |
| `ch_28:results-evidence` | `chapters/ch_28.tex` |
| `ch_28:sealed-seeds` | `chapters/ch_28.tex` |
| `ch_29:abstract` | `chapters/ch_29.tex` |
| `ch_29:coq-formalisation-and-ckm-unitarity-seed` | `chapters/ch_29.tex` |
| `ch_29:discussion` | `chapters/ch_29.tex` |
| `ch_29:introduction` | `chapters/ch_29.tex` |
| `ch_29:qed-assertions` | `chapters/ch_29.tex` |
| `ch_29:references` | `chapters/ch_29.tex` |
| `ch_29:results-evidence` | `chapters/ch_29.tex` |
| `ch_29:sealed-seeds` | `chapters/ch_29.tex` |
| `ch_29:the-sacred-formula-v-conjecture-and-ux3c6-monomial-parameterisation` | `chapters/ch_29.tex` |
| `ch_30:abstract` | `chapters/ch_30.tex` |
| `ch_30:associative-recall-memory` | `chapters/ch_30.tex` |
| `ch_30:discussion` | `chapters/ch_30.tex` |
| `ch_30:goldenfloat-encoding-of-hypervectors` | `chapters/ch_30.tex` |
| `ch_30:hypervector-definition` | `chapters/ch_30.tex` |
| `ch_30:introduction` | `chapters/ch_30.tex` |
| `ch_30:phi-rotary-position-encoding-phi-rope-in-vsa-context` | `chapters/ch_30.tex` |
| `ch_30:qed-assertions` | `chapters/ch_30.tex` |
| `ch_30:references` | `chapters/ch_30.tex` |
| `ch_30:results-evidence` | `chapters/ch_30.tex` |
| `ch_30:sealed-seeds` | `chapters/ch_30.tex` |
| `ch_30:ternary-vsa-over-the-goldenfloat-substrate` | `chapters/ch_30.tex` |
| `ch_31:abstract` | `chapters/ch_31.tex` |
| `ch_31:discussion` | `chapters/ch_31.tex` |
| `ch_31:formal-seal-297-coq-theorems` | `chapters/ch_31.tex` |
| `ch_31:hardware-architecture` | `chapters/ch_31.tex` |
| `ch_31:introduction` | `chapters/ch_31.tex` |
| `ch_31:qed-assertions` | `chapters/ch_31.tex` |
| `ch_31:references` | `chapters/ch_31.tex` |
| `ch_31:results-evidence` | `chapters/ch_31.tex` |
| `ch_31:sealed-seeds` | `chapters/ch_31.tex` |
| `ch_32:abstract` | `chapters/ch_32.tex` |
| `ch_32:crc-16ccitt-polynomial` | `chapters/ch_32.tex` |
| `ch_32:discussion` | `chapters/ch_32.tex` |
| `ch_32:error-recovery-automaton` | `chapters/ch_32.tex` |
| `ch_32:frame-grammar` | `chapters/ch_32.tex` |
| `ch_32:frame-structure-and-grammar` | `chapters/ch_32.tex` |
| `ch_32:introduction` | `chapters/ch_32.tex` |
| `ch_32:physical-layer` | `chapters/ch_32.tex` |
| `ch_32:qed-assertions` | `chapters/ch_32.tex` |
| `ch_32:references` | `chapters/ch_32.tex` |
| `ch_32:results-evidence` | `chapters/ch_32.tex` |
| `ch_32:sealed-seeds` | `chapters/ch_32.tex` |
| `ch_32:sync-frame-payload` | `chapters/ch_32.tex` |
| `ch_32:sync-frame-trigger` | `chapters/ch_32.tex` |
| `ch_32:ux3c6-synchronisation-frames` | `chapters/ch_32.tex` |
| `ch_33:abstract` | `chapters/ch_33.tex` |
| `ch_33:diagnosis-and-root-cause` | `chapters/ch_33.tex` |
| `ch_33:discussion` | `chapters/ch_33.tex` |
| `ch_33:flash_no_sudo.sh` | `chapters/ch_33.tex` |
| `ch_33:fxload-cross-compilation` | `chapters/ch_33.tex` |
| `ch_33:introduction` | `chapters/ch_33.tex` |
| `ch_33:qed-assertions` | `chapters/ch_33.tex` |
| `ch_33:references` | `chapters/ch_33.tex` |
| `ch_33:results-evidence` | `chapters/ch_33.tex` |
| `ch_33:sealed-seeds` | `chapters/ch_33.tex` |
| `ch_33:usb-enumeration-on-macos-arm` | `chapters/ch_33.tex` |
| `ch_33:verified-hardware-configuration-post-blk-001` | `chapters/ch_33.tex` |
| `ch_34:abstract` | `chapters/ch_34.tex` |
| `ch_34:discussion` | `chapters/ch_34.tex` |
| `ch_34:energy-accounting-framework` | `chapters/ch_34.tex` |
| `ch_34:introduction` | `chapters/ch_34.tex` |
| `ch_34:qed-assertions` | `chapters/ch_34.tex` |
| `ch_34:references` | `chapters/ch_34.tex` |
| `ch_34:results-evidence` | `chapters/ch_34.tex` |
| `ch_34:sealed-seeds` | `chapters/ch_34.tex` |
| `ch_34:ternary-mechanism-analysis` | `chapters/ch_34.tex` |
| `ch_35_mesh_node:ch:mesh-node` | `chapters/ch_35_mesh_node.tex` |
| `ch_35_mesh_node:fig:asic-block` | `chapters/ch_35_mesh_node.tex` |
| `ch_35_mesh_node:tab:comparison` | `chapters/ch_35_mesh_node.tex` |
| `ch_35_mesh_node:tab:rns-packets` | `chapters/ch_35_mesh_node.tex` |
| `ch_35_mesh_node:thm:mru-liveness` | `chapters/ch_35_mesh_node.tex` |
| `ch_35_mesh_node:thm:phi-id` | `chapters/ch_35_mesh_node.tex` |
| `ch_35_mesh_node:thm:power-budget` | `chapters/ch_35_mesh_node.tex` |
| `cor:01-l2-three` | `chapters/fa_01.tex` |
| `cor:01-lucas-as-trace` | `chapters/fa_01.tex` |
| `cor:01-reciprocal` | `chapters/fa_01.tex` |
| `cor:05-asymptotic-rate` | `chapters/fa_05.tex` |
| `cor:05-binet` | `chapters/fa_05.tex` |
| `def:13-lucas-12` | `chapters/fa_13.tex` |
| `eq:ch0-fit` | `chapters/ch_00.tex` |
| `eq:gf-def` | `appendix/C-golden-benchmark.tex` |
| `fa_00:thm:trinity-identity-prologue` | `chapters/fa_00.tex` |
| `fa_01:cor:01-approx-quality` | `chapters/fa_01.tex` |
| `fa_01:cor:01-cascade` | `chapters/fa_01.tex` |
| `fa_01:cor:01-fp-rate` | `chapters/fa_01.tex` |
| `fa_01:cor:01-pentagon-vesica` | `chapters/fa_01.tex` |
| `fa_01:fig:vesica` | `chapters/fa_01.tex` |
| `fa_01:lem:01-gm-limit` | `chapters/fa_01.tex` |
| `fa_01:lem:01-golden-angle` | `chapters/fa_01.tex` |
| `fa_01:lem:01-hex-vesica` | `chapters/fa_01.tex` |
| `fa_01:lem:01-newton-phi` | `chapters/fa_01.tex` |
| `fa_01:lem:01-pentagram-self` | `chapters/fa_01.tex` |
| `fa_01:lem:01-small-n` | `chapters/fa_01.tex` |
| `fa_01:lem:01-vesica-area` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-A` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AA` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AB` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AC` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AD` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AE` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AF` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AG` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AH` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AI` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AJ` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AK` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AL` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AM` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AN` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AO` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AP` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AQ` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AR` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AS` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-AT` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-B` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-C` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-D` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-E` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-F` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-G` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-H` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-I` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-J` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-K` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-L` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-M` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-N` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-O` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-P` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-Q` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-R` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-S` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-T` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-U` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-V` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-W` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-X` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-Y` | `chapters/fa_01.tex` |
| `fa_01:sec:01-app-Z` | `chapters/fa_01.tex` |
| `fa_01:sec:01-strand-I` | `chapters/fa_01.tex` |
| `fa_01:sec:01-strand-II` | `chapters/fa_01.tex` |
| `fa_01:sec:01-strand-III` | `chapters/fa_01.tex` |
| `fa_01:thm:01-dodec-circ` | `chapters/fa_01.tex` |
| `fa_01:thm:01-lucas-anchor` | `chapters/fa_01.tex` |
| `fa_01:thm:01-lucas-as-trace` | `chapters/fa_01.tex` |
| `fa_01:thm:01-ring-int` | `chapters/fa_01.tex` |
| `fa_01:thm:01-three-witnesses` | `chapters/fa_01.tex` |
| `fa_01:thm:01-trace-as-Z` | `chapters/fa_01.tex` |
| `fa_01:thm:01-universal-anchor` | `chapters/fa_01.tex` |
| `fa_02:abstract` | `chapters/fa_02.tex` |
| `fa_02:discussion` | `chapters/fa_02.tex` |
| `fa_02:early-symbolicconnectionist-hybrids` | `chapters/fa_02.tex` |
| `fa_02:fibonacci-and-lucas-lattices-as-basis-sets` | `chapters/fa_02.tex` |
| `fa_02:gap-in-prior-art` | `chapters/fa_02.tex` |
| `fa_02:introduction` | `chapters/fa_02.tex` |
| `fa_02:logic-tensor-networks-and-differentiable-reasoning` | `chapters/fa_02.tex` |
| `fa_02:qed-assertions` | `chapters/fa_02.tex` |
| `fa_02:references` | `chapters/fa_02.tex` |
| `fa_02:representational-bottleneck-and-the-ux3c6-structural-prior` | `chapters/fa_02.tex` |
| `fa_02:results-evidence` | `chapters/fa_02.tex` |
| `fa_02:sealed-seeds` | `chapters/fa_02.tex` |
| `fa_02:sparse-and-ternary-neural-computation` | `chapters/fa_02.tex` |
| `fa_02:taxonomy-of-neuro-symbolic-paradigms` | `chapters/fa_02.tex` |
| `fa_02:the-normalisation-problem` | `chapters/fa_02.tex` |
| `fa_02:vector-symbolic-architectures` | `chapters/fa_02.tex` |
| `fa_03:abstract` | `chapters/fa_03.tex` |
| `fa_03:coq-mechanisation-and-sac-0-invariant` | `chapters/fa_03.tex` |
| `fa_03:derivation-of-the-anchor-identity` | `chapters/fa_03.tex` |
| `fa_03:discussion` | `chapters/fa_03.tex` |
| `fa_03:introduction` | `chapters/fa_03.tex` |
| `fa_03:invariant-sac-0` | `chapters/fa_03.tex` |
| `fa_03:minimal-polynomial-and-basic-consequences` | `chapters/fa_03.tex` |
| `fa_03:power-survey` | `chapters/fa_03.tex` |
| `fa_03:proof-architecture` | `chapters/fa_03.tex` |
| `fa_03:qed-assertions` | `chapters/fa_03.tex` |
| `fa_03:references` | `chapters/fa_03.tex` |
| `fa_03:relation-to-fibonacci-arithmetic` | `chapters/fa_03.tex` |
| `fa_03:results-evidence` | `chapters/fa_03.tex` |
| `fa_03:sealed-seeds` | `chapters/fa_03.tex` |
| `fa_03:the-integer-3-coincidence` | `chapters/fa_03.tex` |
| `fa_04:abstract` | `chapters/fa_04.tex` |
| `fa_04:derivation-of-the-closed-form` | `chapters/fa_04.tex` |
| `fa_04:discussion` | `chapters/fa_04.tex` |
| `fa_04:introduction` | `chapters/fa_04.tex` |
| `fa_04:multiplicative-identity-and-kernel-integration` | `chapters/fa_04.tex` |
| `fa_04:qed-assertions` | `chapters/fa_04.tex` |
| `fa_04:references` | `chapters/fa_04.tex` |
| `fa_04:results-evidence` | `chapters/fa_04.tex` |
| `fa_04:sealed-seeds` | `chapters/fa_04.tex` |
| `fa_05:ch:golden-bridge` | `chapters/fa_05.tex` |
| `fa_05:cor:05-anchor-via-genfn` | `chapters/fa_05.tex` |
| `fa_05:cor:05-matrix-cassini` | `chapters/fa_05.tex` |
| `fa_05:def:05-egf` | `chapters/fa_05.tex` |
| `fa_05:def:05-fib` | `chapters/fa_05.tex` |
| `fa_05:def:05-hankel` | `chapters/fa_05.tex` |
| `fa_05:def:05-luc` | `chapters/fa_05.tex` |
| `fa_05:lem:05-D-fib` | `chapters/fa_05.tex` |
| `fa_05:lem:05-D-luc` | `chapters/fa_05.tex` |
| `fa_05:lem:05-coupling-check` | `chapters/fa_05.tex` |
| `fa_05:lem:05-degree` | `chapters/fa_05.tex` |
| `fa_05:lem:05-degree-Q-phi` | `chapters/fa_05.tex` |
| `fa_05:lem:05-fib-egf` | `chapters/fa_05.tex` |
| `fa_05:lem:05-fib-hankel` | `chapters/fa_05.tex` |
| `fa_05:lem:05-fib-vals` | `chapters/fa_05.tex` |
| `fa_05:lem:05-luc-egf` | `chapters/fa_05.tex` |
| `fa_05:lem:05-luc-vals` | `chapters/fa_05.tex` |
| `fa_05:lem:05-pole-locations` | `chapters/fa_05.tex` |
| `fa_05:lem:05-pole-residue` | `chapters/fa_05.tex` |
| `fa_05:lem:05-product-coefs` | `chapters/fa_05.tex` |
| `fa_05:lem:05-rational` | `chapters/fa_05.tex` |
| `fa_05:lem:05-riordan-small` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-A` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-AA` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-AB` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-AC` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-AD` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-AE` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-AF` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-AG` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-AH` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-B` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-C` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-D` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-E` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-G` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-H` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-I` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-J` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-L` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-M` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-N` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-O` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-P` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-Q` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-R` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-S` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-T` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-U` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-V` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-W` | `chapters/fa_05.tex` |
| `fa_05:sec:05-app-Z` | `chapters/fa_05.tex` |
| `fa_05:sec:05-closing` | `chapters/fa_05.tex` |
| `fa_05:sec:05-intro` | `chapters/fa_05.tex` |
| `fa_05:sec:05-library` | `chapters/fa_05.tex` |
| `fa_05:thm:05-bridge-to-l4` | `chapters/fa_05.tex` |
| `fa_05:thm:05-bridge-to-l6` | `chapters/fa_05.tex` |
| `fa_05:thm:05-cassini-triangle` | `chapters/fa_05.tex` |
| `fa_05:thm:05-gf-product` | `chapters/fa_05.tex` |
| `fa_06:abstract` | `chapters/fa_06.tex` |
| `fa_06:coq-encoding` | `chapters/fa_06.tex` |
| `fa_06:discussion` | `chapters/fa_06.tex` |
| `fa_06:goldenfloat-format-definitions` | `chapters/fa_06.tex` |
| `fa_06:introduction` | `chapters/fa_06.tex` |
| `fa_06:key-theorems-and-proof-sketches` | `chapters/fa_06.tex` |
| `fa_06:lucas-closure-on-gf16` | `chapters/fa_06.tex` |
| `fa_06:preliminaries` | `chapters/fa_06.tex` |
| `fa_06:qed-assertions` | `chapters/fa_06.tex` |
| `fa_06:references` | `chapters/fa_06.tex` |
| `fa_06:results-evidence` | `chapters/fa_06.tex` |
| `fa_06:sealed-seeds` | `chapters/fa_06.tex` |
| `fa_07:abstract` | `chapters/fa_07.tex` |
| `fa_07:discussion` | `chapters/fa_07.tex` |
| `fa_07:from-the-trinity-identity-to-the-golden-angle` | `chapters/fa_07.tex` |
| `fa_07:h4-root-system-e8-lattice-and-the-varphi-scaled-block-decomposition` | `chapters/fa_07.tex` |
| `fa_07:introduction` | `chapters/fa_07.tex` |
| `fa_07:qed-assertions` | `chapters/fa_07.tex` |
| `fa_07:references` | `chapters/fa_07.tex` |
| `fa_07:results-evidence` | `chapters/fa_07.tex` |
| `fa_07:sealed-seeds` | `chapters/fa_07.tex` |
| `fa_08:abstract` | `chapters/fa_08.tex` |
| `fa_08:discussion` | `chapters/fa_08.tex` |
| `fa_08:gain-admissibility` | `chapters/fa_08.tex` |
| `fa_08:hybrid-qk-gain-invariant-inv-6` | `chapters/fa_08.tex` |
| `fa_08:introduction` | `chapters/fa_08.tex` |
| `fa_08:proof-sketch-for-admit_phi_sq` | `chapters/fa_08.tex` |
| `fa_08:qed-assertions` | `chapters/fa_08.tex` |
| `fa_08:references` | `chapters/fa_08.tex` |
| `fa_08:results-evidence` | `chapters/fa_08.tex` |
| `fa_08:sealed-seeds` | `chapters/fa_08.tex` |
| `fa_08:sec:falsification:ch08` | `chapters/fa_08.tex` |
| `fa_08:tf3-and-tf9-algebraic-structure` | `chapters/fa_08.tex` |
| `fa_08:tf9-product-encoding` | `chapters/fa_08.tex` |
| `fa_08:trit-encoding` | `chapters/fa_08.tex` |
| `fa_08:ux3c6-normalisation` | `chapters/fa_08.tex` |
| `fa_09:ablation-matrix-tier-abc-m1m6` | `chapters/fa_09.tex` |
| `fa_09:abstract` | `chapters/fa_09.tex` |
| `fa_09:competitor-format-summaries` | `chapters/fa_09.tex` |
| `fa_09:discussion` | `chapters/fa_09.tex` |
| `fa_09:gf16-format-specification` | `chapters/fa_09.tex` |
| `fa_09:gf16-phi_bias60-and-the-inv-3-safe-domain` | `chapters/fa_09.tex` |
| `fa_09:introduction` | `chapters/fa_09.tex` |
| `fa_09:inv-3-nine-coq-precision-bounds` | `chapters/fa_09.tex` |
| `fa_09:qed-assertions` | `chapters/fa_09.tex` |
| `fa_09:references` | `chapters/fa_09.tex` |
| `fa_09:results-evidence` | `chapters/fa_09.tex` |
| `fa_09:sealed-seeds` | `chapters/fa_09.tex` |
| `fa_10:abstract` | `chapters/fa_10.tex` |
| `fa_10:discussion` | `chapters/fa_10.tex` |
| `fa_10:gf16-range-and-precision-formalisation` | `chapters/fa_10.tex` |
| `fa_10:introduction` | `chapters/fa_10.tex` |
| `fa_10:qed-assertions` | `chapters/fa_10.tex` |
| `fa_10:references` | `chapters/fa_10.tex` |
| `fa_10:results-evidence` | `chapters/fa_10.tex` |
| `fa_10:sealed-seeds` | `chapters/fa_10.tex` |
| `fa_10:the-pareto-frontier-and-conjecture-c1` | `chapters/fa_10.tex` |
| `fa_11:abstract` | `chapters/fa_11.tex` |
| `fa_11:discussion` | `chapters/fa_11.tex` |
| `fa_11:hypothesis-formalisation-and-registration-protocol` | `chapters/fa_11.tex` |
| `fa_11:introduction` | `chapters/fa_11.tex` |
| `fa_11:inv-7-invariant-and-coq-formalisation` | `chapters/fa_11.tex` |
| `fa_11:qed-assertions` | `chapters/fa_11.tex` |
| `fa_11:references` | `chapters/fa_11.tex` |
| `fa_11:results-evidence` | `chapters/fa_11.tex` |
| `fa_11:sealed-seeds` | `chapters/fa_11.tex` |
| `fa_12:abstract` | `chapters/fa_12.tex` |
| `fa_12:bridge-architecture-and-interface-contracts` | `chapters/fa_12.tex` |
| `fa_12:clock-domain-analysis-and-timing` | `chapters/fa_12.tex` |
| `fa_12:discussion` | `chapters/fa_12.tex` |
| `fa_12:error-handling-protocol` | `chapters/fa_12.tex` |
| `fa_12:frequency-ratios-and-the-golden-ratio` | `chapters/fa_12.tex` |
| `fa_12:introduction` | `chapters/fa_12.tex` |
| `fa_12:logical-structure` | `chapters/fa_12.tex` |
| `fa_12:power-accounting` | `chapters/fa_12.tex` |
| `fa_12:qed-assertions` | `chapters/fa_12.tex` |
| `fa_12:references` | `chapters/fa_12.tex` |
| `fa_12:results-evidence` | `chapters/fa_12.tex` |
| `fa_12:sealed-seeds` | `chapters/fa_12.tex` |
| `fa_12:signal-naming-convention` | `chapters/fa_12.tex` |
| `fa_12:throughput-budget` | `chapters/fa_12.tex` |
| `fa_13:ch:13-metatron` | `chapters/fa_13.tex` |
| `fa_13:sec:13-appD` | `chapters/fa_13.tex` |
| `fa_13:sec:13-appE` | `chapters/fa_13.tex` |
| `fa_13:sec:13-appF` | `chapters/fa_13.tex` |
| `fa_13:sec:13-appG` | `chapters/fa_13.tex` |
| `fa_13:sec:13-appI` | `chapters/fa_13.tex` |
| `fa_13:sec:13-appJ` | `chapters/fa_13.tex` |
| `fa_13:sec:13-appK` | `chapters/fa_13.tex` |
| `fa_13:sec:13-appL` | `chapters/fa_13.tex` |
| `fa_13:sec:13-appM` | `chapters/fa_13.tex` |
| `fa_13:sec:13-arch` | `chapters/fa_13.tex` |
| `fa_13:sec:13-arch-summary` | `chapters/fa_13.tex` |
| `fa_13:sec:13-cartesian-rim` | `chapters/fa_13.tex` |
| `fa_13:sec:13-conn-23` | `chapters/fa_13.tex` |
| `fa_13:sec:13-connection-to-17` | `chapters/fa_13.tex` |
| `fa_13:sec:13-coords-bookkeeping` | `chapters/fa_13.tex` |
| `fa_13:sec:13-coq-map` | `chapters/fa_13.tex` |
| `fa_13:sec:13-counting-arch` | `chapters/fa_13.tex` |
| `fa_13:sec:13-cube-vs-spiral` | `chapters/fa_13.tex` |
| `fa_13:sec:13-diagram` | `chapters/fa_13.tex` |
| `fa_13:sec:13-disc-est` | `chapters/fa_13.tex` |
| `fa_13:sec:13-disc-not` | `chapters/fa_13.tex` |
| `fa_13:sec:13-disc-open` | `chapters/fa_13.tex` |
| `fa_13:sec:13-disc-summary` | `chapters/fa_13.tex` |
| `fa_13:sec:13-discussion` | `chapters/fa_13.tex` |
| `fa_13:sec:13-edge-counts` | `chapters/fa_13.tex` |
| `fa_13:sec:13-emp-26` | `chapters/fa_13.tex` |
| `fa_13:sec:13-filt-def` | `chapters/fa_13.tex` |
| `fa_13:sec:13-filt-quotients` | `chapters/fa_13.tex` |
| `fa_13:sec:13-filt-why` | `chapters/fa_13.tex` |
| `fa_13:sec:13-five-platonic` | `chapters/fa_13.tex` |
| `fa_13:sec:13-gf16` | `chapters/fa_13.tex` |
| `fa_13:sec:13-identities` | `chapters/fa_13.tex` |
| `fa_13:sec:13-layer-distances` | `chapters/fa_13.tex` |
| `fa_13:sec:13-lucas-12-orbit` | `chapters/fa_13.tex` |
| `fa_13:sec:13-lucas-ring-coords` | `chapters/fa_13.tex` |
| `fa_13:sec:13-notation` | `chapters/fa_13.tex` |
| `fa_13:sec:13-origin` | `chapters/fa_13.tex` |
| `fa_13:sec:13-polar` | `chapters/fa_13.tex` |
| `fa_13:sec:13-projection` | `chapters/fa_13.tex` |
| `fa_13:sec:13-strand-I` | `chapters/fa_13.tex` |
| `fa_13:sec:13-strand-I-takeaway` | `chapters/fa_13.tex` |
| `fa_13:sec:13-strand-II` | `chapters/fa_13.tex` |
| `fa_13:sec:13-strand-II-wrap` | `chapters/fa_13.tex` |
| `fa_13:sec:13-strand-III` | `chapters/fa_13.tex` |
| `fa_13:sec:13-strand-III-wrap` | `chapters/fa_13.tex` |
| `fa_13:sec:13-thirteen` | `chapters/fa_13.tex` |
| `fa_13:sec:13-three-cubes` | `chapters/fa_13.tex` |
| `fa_13:sec:13-three-strands` | `chapters/fa_13.tex` |
| `fa_13:sec:13-trinity-plane` | `chapters/fa_13.tex` |
| `fa_14:abstract` | `chapters/fa_14.tex` |
| `fa_14:bpb-definition-and-algebraic-properties` | `chapters/fa_14.tex` |
| `fa_14:byte-level-normalisation` | `chapters/fa_14.tex` |
| `fa_14:cross-entropy-and-perplexity` | `chapters/fa_14.tex` |
| `fa_14:discussion` | `chapters/fa_14.tex` |
| `fa_14:gate-2-bpb-1.85` | `chapters/fa_14.tex` |
| `fa_14:gate-3-bpb-1.50` | `chapters/fa_14.tex` |
| `fa_14:gate-thresholds-and-their-derivation` | `chapters/fa_14.tex` |
| `fa_14:introduction` | `chapters/fa_14.tex` |
| `fa_14:qed-assertions` | `chapters/fa_14.tex` |
| `fa_14:references` | `chapters/fa_14.tex` |
| `fa_14:relationship-to-the-darpa-energy-goal` | `chapters/fa_14.tex` |
| `fa_14:results-evidence` | `chapters/fa_14.tex` |
| `fa_14:sealed-seeds` | `chapters/fa_14.tex` |
| `fa_14:ux3c6-weighted-bpb` | `chapters/fa_14.tex` |
| `fa_15:abstract` | `chapters/fa_15.tex` |
| `fa_15:bpb-protocol-and-monotone-backward-invariant-inv-1` | `chapters/fa_15.tex` |
| `fa_15:database-schema` | `chapters/fa_15.tex` |
| `fa_15:discussion` | `chapters/fa_15.tex` |
| `fa_15:evaluation-protocol` | `chapters/fa_15.tex` |
| `fa_15:gate-evaluation` | `chapters/fa_15.tex` |
| `fa_15:introduction` | `chapters/fa_15.tex` |
| `fa_15:inv-1-bpb-monotone-backward` | `chapters/fa_15.tex` |
| `fa_15:qed-assertions` | `chapters/fa_15.tex` |
| `fa_15:railway-write-back-architecture` | `chapters/fa_15.tex` |
| `fa_15:references` | `chapters/fa_15.tex` |
| `fa_15:results-evidence` | `chapters/fa_15.tex` |
| `fa_15:sealed-seeds` | `chapters/fa_15.tex` |
| `fa_15:warmup-gate` | `chapters/fa_15.tex` |
| `fa_15:write-back-protocol` | `chapters/fa_15.tex` |
| `fa_16:abstract` | `chapters/fa_16.tex` |
| `fa_16:discussion` | `chapters/fa_16.tex` |
| `fa_16:grid-construction-and-sparsity-analysis` | `chapters/fa_16.tex` |
| `fa_16:introduction` | `chapters/fa_16.tex` |
| `fa_16:qed-assertions` | `chapters/fa_16.tex` |
| `fa_16:references` | `chapters/fa_16.tex` |
| `fa_16:results-evidence` | `chapters/fa_16.tex` |
| `fa_16:sealed-seeds` | `chapters/fa_16.tex` |
| `fa_16:the-phi-distance-function` | `chapters/fa_16.tex` |
| `fa_17:abstract` | `chapters/fa_17.tex` |
| `fa_17:analysis-of-effects-and-golden-ratio-structure` | `chapters/fa_17.tex` |
| `fa_17:discussion` | `chapters/fa_17.tex` |
| `fa_17:factor-definitions-and-experimental-design` | `chapters/fa_17.tex` |
| `fa_17:introduction` | `chapters/fa_17.tex` |
| `fa_17:qed-assertions` | `chapters/fa_17.tex` |
| `fa_17:references` | `chapters/fa_17.tex` |
| `fa_17:results-evidence` | `chapters/fa_17.tex` |
| `fa_17:sealed-seeds` | `chapters/fa_17.tex` |
| `fa_18:abstract` | `chapters/fa_18.tex` |
| `fa_18:coq.interval-upgrade-lane` | `chapters/fa_18.tex` |
| `fa_18:discussion` | `chapters/fa_18.tex` |
| `fa_18:hardware-and-runtime-limitations` | `chapters/fa_18.tex` |
| `fa_18:introduction` | `chapters/fa_18.tex` |
| `fa_18:qed-assertions` | `chapters/fa_18.tex` |
| `fa_18:references` | `chapters/fa_18.tex` |
| `fa_18:sealed-seeds` | `chapters/fa_18.tex` |
| `fa_18:sec:falsification:ch18` | `chapters/fa_18.tex` |
| `fa_18:state-of-the-art-comparison-clara-soa-snapshot` | `chapters/fa_18.tex` |
| `fa_19:abstract` | `chapters/fa_19.tex` |
| `fa_19:discussion` | `chapters/fa_19.tex` |
| `fa_19:introduction` | `chapters/fa_19.tex` |
| `fa_19:qed-assertions` | `chapters/fa_19.tex` |
| `fa_19:references` | `chapters/fa_19.tex` |
| `fa_19:results-evidence` | `chapters/fa_19.tex` |
| `fa_19:sealed-seeds` | `chapters/fa_19.tex` |
| `fa_19:test-design-and-hypotheses` | `chapters/fa_19.tex` |
| `fa_19:welch-t-statistic-and-degrees-of-freedom` | `chapters/fa_19.tex` |
| `fa_20:ch:20` | `chapters/fa_20.tex` |
| `fa_20:def:alpha` | `chapters/fa_20.tex` |
| `fa_20:def:ckm` | `chapters/fa_20.tex` |
| `fa_20:def:gauge-boson` | `chapters/fa_20.tex` |
| `fa_20:def:higgs` | `chapters/fa_20.tex` |
| `fa_20:def:koide` | `chapters/fa_20.tex` |
| `fa_20:def:lepton` | `chapters/fa_20.tex` |
| `fa_20:def:pmns` | `chapters/fa_20.tex` |
| `fa_20:def:quark` | `chapters/fa_20.tex` |
| `fa_20:def:su2` | `chapters/fa_20.tex` |
| `fa_20:def:su3` | `chapters/fa_20.tex` |
| `fa_20:def:u1` | `chapters/fa_20.tex` |
| `fa_20:prop:ckm-golden` | `chapters/fa_20.tex` |
| `fa_20:prop:golden-alpha` | `chapters/fa_20.tex` |
| `fa_20:prop:golden-koide` | `chapters/fa_20.tex` |
| `fa_20:prop:higgs-mass` | `chapters/fa_20.tex` |
| `fa_20:prop:pmns-golden` | `chapters/fa_20.tex` |
| `fa_20:prop:su3-dim` | `chapters/fa_20.tex` |
| `fa_20:prop:u1-charge` | `chapters/fa_20.tex` |
| `fa_20:sec:20-falsify` | `chapters/fa_20.tex` |
| `fa_20:thm:pauli` | `chapters/fa_20.tex` |
| `fa_20:thm:sm-symmetry` | `chapters/fa_20.tex` |
| `fa_20:thm:strong-golden` | `chapters/fa_20.tex` |
| `fa_20:thm:weak-golden` | `chapters/fa_20.tex` |
| `fa_21:ch:21` | `chapters/fa_21.tex` |
| `fa_21:def:dim-reg` | `chapters/fa_21.tex` |
| `fa_21:def:eft` | `chapters/fa_21.tex` |
| `fa_21:def:field-ops` | `chapters/fa_21.tex` |
| `fa_21:def:fock` | `chapters/fa_21.tex` |
| `fa_21:def:higgs-pot` | `chapters/fa_21.tex` |
| `fa_21:def:kg` | `chapters/fa_21.tex` |
| `fa_21:def:lagrangian` | `chapters/fa_21.tex` |
| `fa_21:def:path-integral` | `chapters/fa_21.tex` |
| `fa_21:def:qed` | `chapters/fa_21.tex` |
| `fa_21:def:yang-mills` | `chapters/fa_21.tex` |
| `fa_21:prop:beta-golden` | `chapters/fa_21.tex` |
| `fa_21:prop:feynman` | `chapters/fa_21.tex` |
| `fa_21:prop:goldstone` | `chapters/fa_21.tex` |
| `fa_21:prop:kg-eq` | `chapters/fa_21.tex` |
| `fa_21:prop:non-abelian` | `chapters/fa_21.tex` |
| `fa_21:sec:21-falsify` | `chapters/fa_21.tex` |
| `fa_21:thm:mode-expansion` | `chapters/fa_21.tex` |
| `fa_21:thm:n-point` | `chapters/fa_21.tex` |
| `fa_21:thm:rg` | `chapters/fa_21.tex` |
| `fa_21:thm:ssb` | `chapters/fa_21.tex` |
| `fa_21:thm:weinberg` | `chapters/fa_21.tex` |
| `fa_22:abstract` | `chapters/fa_22.tex` |
| `fa_22:discussion` | `chapters/fa_22.tex` |
| `fa_22:introduction` | `chapters/fa_22.tex` |
| `fa_22:qed-assertions` | `chapters/fa_22.tex` |
| `fa_22:references` | `chapters/fa_22.tex` |
| `fa_22:results-evidence` | `chapters/fa_22.tex` |
| `fa_22:satisfaction-witness-and-victory-predicate` | `chapters/fa_22.tex` |
| `fa_22:sealed-seeds` | `chapters/fa_22.tex` |
| `fa_22:worker-pool-invariants-and-falsification-witnesses` | `chapters/fa_22.tex` |
| `fa_23:abstract` | `chapters/fa_23.tex` |
| `fa_23:discussion` | `chapters/fa_23.tex` |
| `fa_23:introduction` | `chapters/fa_23.tex` |
| `fa_23:mcp-adapter-layer-architecture` | `chapters/fa_23.tex` |
| `fa_23:protocol-implementation-and-latency-analysis` | `chapters/fa_23.tex` |
| `fa_23:qed-assertions` | `chapters/fa_23.tex` |
| `fa_23:references` | `chapters/fa_23.tex` |
| `fa_23:results-evidence` | `chapters/fa_23.tex` |
| `fa_23:sealed-seeds` | `chapters/fa_23.tex` |
| `fa_24:abstract` | `chapters/fa_24.tex` |
| `fa_24:agent-model` | `chapters/fa_24.tex` |
| `fa_24:coq-encoding` | `chapters/fa_24.tex` |
| `fa_24:discussion` | `chapters/fa_24.tex` |
| `fa_24:formal-model-of-the-period-locked-monitor` | `chapters/fa_24.tex` |
| `fa_24:implementation-and-hardware-interface` | `chapters/fa_24.tex` |
| `fa_24:interrupt-interface-with-the-hardware-bridge` | `chapters/fa_24.tex` |
| `fa_24:introduction` | `chapters/fa_24.tex` |
| `fa_24:period-ratio-and-non-resonance` | `chapters/fa_24.tex` |
| `fa_24:priority-queue-and-phi-weighted-scheduling` | `chapters/fa_24.tex` |
| `fa_24:qed-assertions` | `chapters/fa_24.tex` |
| `fa_24:references` | `chapters/fa_24.tex` |
| `fa_24:results-evidence` | `chapters/fa_24.tex` |
| `fa_24:rtl-implementation` | `chapters/fa_24.tex` |
| `fa_24:sealed-seeds` | `chapters/fa_24.tex` |
| `fa_24:sec:falsification:ch24` | `chapters/fa_24.tex` |
| `fa_25:abstract` | `chapters/fa_25.tex` |
| `fa_25:cycle-classification-and-attention-periodicity` | `chapters/fa_25.tex` |
| `fa_25:discussion` | `chapters/fa_25.tex` |
| `fa_25:introduction` | `chapters/fa_25.tex` |
| `fa_25:qed-assertions` | `chapters/fa_25.tex` |
| `fa_25:references` | `chapters/fa_25.tex` |
| `fa_25:results-evidence` | `chapters/fa_25.tex` |
| `fa_25:sealed-seeds` | `chapters/fa_25.tex` |
| `fa_25:sec:falsification:ch25` | `chapters/fa_25.tex` |
| `fa_25:varphi-lattice-structure-and-the-cycle-map` | `chapters/fa_25.tex` |
| `fa_26:abstract` | `chapters/fa_26.tex` |
| `fa_26:discussion` | `chapters/fa_26.tex` |
| `fa_26:gf16_quant-galois-field-16-quantisation` | `chapters/fa_26.tex` |
| `fa_26:instruction-encoding` | `chapters/fa_26.tex` |
| `fa_26:introduction` | `chapters/fa_26.tex` |
| `fa_26:isa-register-file-and-encoding` | `chapters/fa_26.tex` |
| `fa_26:opcode-specifications` | `chapters/fa_26.tex` |
| `fa_26:phi_rope-ux3c6-rotary-position-encoding` | `chapters/fa_26.tex` |
| `fa_26:qed-assertions` | `chapters/fa_26.tex` |
| `fa_26:references` | `chapters/fa_26.tex` |
| `fa_26:register-file` | `chapters/fa_26.tex` |
| `fa_26:results-evidence` | `chapters/fa_26.tex` |
| `fa_26:sealed-seeds` | `chapters/fa_26.tex` |
| `fa_26:tf3_add-ternary-addition` | `chapters/fa_26.tex` |
| `fa_26:tf3_mul-ternary-multiplication` | `chapters/fa_26.tex` |
| `fa_26:vsa_bind-hyperdimensional-binding` | `chapters/fa_26.tex` |
| `fa_26:vsa_bundle-hyperdimensional-bundling` | `chapters/fa_26.tex` |
| `fa_26:vsa_unbind-hyperdimensional-unbinding` | `chapters/fa_26.tex` |
| `fa_27:abstract` | `chapters/fa_27.tex` |
| `fa_27:abstract-syntax` | `chapters/fa_27.tex` |
| `fa_27:discussion` | `chapters/fa_27.tex` |
| `fa_27:environments-and-evaluation` | `chapters/fa_27.tex` |
| `fa_27:introduction` | `chapters/fa_27.tex` |
| `fa_27:mechanised-proofs-determinism-and-exhaustiveness` | `chapters/fa_27.tex` |
| `fa_27:qed-assertions` | `chapters/fa_27.tex` |
| `fa_27:references` | `chapters/fa_27.tex` |
| `fa_27:relation-to-gf16-and-varphi-arithmetic` | `chapters/fa_27.tex` |
| `fa_27:results-evidence` | `chapters/fa_27.tex` |
| `fa_27:sealed-seeds` | `chapters/fa_27.tex` |
| `fa_27:ternary-arithmetic` | `chapters/fa_27.tex` |
| `fa_27:theorem-eval_det-determinism` | `chapters/fa_27.tex` |
| `fa_27:theorem-trit_exhaustive-exhaustiveness` | `chapters/fa_27.tex` |
| `fa_27:tri27-syntax-and-denotational-semantics` | `chapters/fa_27.tex` |
| `fa_28:abstract` | `chapters/fa_28.tex` |
| `fa_28:architecture-zero-dsp-ternary-datapath` | `chapters/fa_28.tex` |
| `fa_28:discussion` | `chapters/fa_28.tex` |
| `fa_28:introduction` | `chapters/fa_28.tex` |
| `fa_28:qed-assertions` | `chapters/fa_28.tex` |
| `fa_28:references` | `chapters/fa_28.tex` |
| `fa_28:resource-utilisation-and-timing-closure` | `chapters/fa_28.tex` |
| `fa_28:results-evidence` | `chapters/fa_28.tex` |
| `fa_28:sealed-seeds` | `chapters/fa_28.tex` |
| `fa_29:ch:29` | `chapters/fa_29.tex` |
| `fa_29:def:lucas` | `chapters/fa_29.tex` |
| `fa_29:def:lucas-primes` | `chapters/fa_29.tex` |
| `fa_29:def:lucas-spiral` | `chapters/fa_29.tex` |
| `fa_29:def:lucas-tiling` | `chapters/fa_29.tex` |
| `fa_29:def:lucas-trinity` | `chapters/fa_29.tex` |
| `fa_29:prop:golden-lucas-mixing` | `chapters/fa_29.tex` |
| `fa_29:prop:lucas-golden` | `chapters/fa_29.tex` |
| `fa_29:prop:lucas-mod` | `chapters/fa_29.tex` |
| `fa_29:prop:lucas-tiling` | `chapters/fa_29.tex` |
| `fa_29:sec:29-falsify` | `chapters/fa_29.tex` |
| `fa_29:thm:cassini` | `chapters/fa_29.tex` |
| `fa_29:thm:lucas-div` | `chapters/fa_29.tex` |
| `fa_29:thm:lucas-fibo` | `chapters/fa_29.tex` |
| `fa_29:thm:lucas-prime-density` | `chapters/fa_29.tex` |
| `fa_29:thm:lucas-spiral` | `chapters/fa_29.tex` |
| `fa_29:thm:neutrino-lucas` | `chapters/fa_29.tex` |
| `fa_29:thm:product` | `chapters/fa_29.tex` |
| `fa_30:abstract` | `chapters/fa_30.tex` |
| `fa_30:associative-recall-memory` | `chapters/fa_30.tex` |
| `fa_30:discussion` | `chapters/fa_30.tex` |
| `fa_30:goldenfloat-encoding-of-hypervectors` | `chapters/fa_30.tex` |
| `fa_30:hypervector-definition` | `chapters/fa_30.tex` |
| `fa_30:introduction` | `chapters/fa_30.tex` |
| `fa_30:phi-rotary-position-encoding-phi-rope-in-vsa-context` | `chapters/fa_30.tex` |
| `fa_30:qed-assertions` | `chapters/fa_30.tex` |
| `fa_30:references` | `chapters/fa_30.tex` |
| `fa_30:results-evidence` | `chapters/fa_30.tex` |
| `fa_30:sealed-seeds` | `chapters/fa_30.tex` |
| `fa_30:ternary-vsa-over-the-goldenfloat-substrate` | `chapters/fa_30.tex` |
| `fa_31:ch:31` | `chapters/fa_31.tex` |
| `fa_31:def:antirealism` | `chapters/fa_31.tex` |
| `fa_31:def:apriori` | `chapters/fa_31.tex` |
| `fa_31:def:beauty` | `chapters/fa_31.tex` |
| `fa_31:def:constants` | `chapters/fa_31.tex` |
| `fa_31:def:empiricism` | `chapters/fa_31.tex` |
| `fa_31:def:muh` | `chapters/fa_31.tex` |
| `fa_31:def:platonism` | `chapters/fa_31.tex` |
| `fa_31:def:pythagorean` | `chapters/fa_31.tex` |
| `fa_31:def:realism` | `chapters/fa_31.tex` |
| `fa_31:def:structuralism` | `chapters/fa_31.tex` |
| `fa_31:prop:empirical-golden` | `chapters/fa_31.tex` |
| `fa_31:prop:golden-anthropic` | `chapters/fa_31.tex` |
| `fa_31:prop:golden-cat` | `chapters/fa_31.tex` |
| `fa_31:prop:golden-effective` | `chapters/fa_31.tex` |
| `fa_31:prop:golden-muh` | `chapters/fa_31.tex` |
| `fa_31:prop:golden-platonism` | `chapters/fa_31.tex` |
| `fa_31:prop:instrumental-golden` | `chapters/fa_31.tex` |
| `fa_31:thm:apriori-golden` | `chapters/fa_31.tex` |
| `fa_31:thm:golden-beauty` | `chapters/fa_31.tex` |
| `fa_31:thm:golden-constants` | `chapters/fa_31.tex` |
| `fa_31:thm:golden-struct` | `chapters/fa_31.tex` |
| `fa_31:thm:platonic-golden` | `chapters/fa_31.tex` |
| `fa_31:thm:pythagorean-golden` | `chapters/fa_31.tex` |
| `fa_31:thm:realist-golden` | `chapters/fa_31.tex` |
| `fa_32:prop:golden-opt` | `chapters/fa_32.tex` |
| `fa_32:thm:alpha-summary` | `chapters/fa_32.tex` |
| `fa_32:thm:e8-summary` | `chapters/fa_32.tex` |
| `fa_32:thm:golden-entropy` | `chapters/fa_32.tex` |
| `fa_32:thm:golden-unif` | `chapters/fa_32.tex` |
| `fa_32:thm:trinity-summary` | `chapters/fa_32.tex` |
| `fa_33:abstract` | `chapters/fa_33.tex` |
| `fa_33:diagnosis-and-root-cause` | `chapters/fa_33.tex` |
| `fa_33:discussion` | `chapters/fa_33.tex` |
| `fa_33:flash_no_sudo.sh` | `chapters/fa_33.tex` |
| `fa_33:fxload-cross-compilation` | `chapters/fa_33.tex` |
| `fa_33:introduction` | `chapters/fa_33.tex` |
| `fa_33:qed-assertions` | `chapters/fa_33.tex` |
| `fa_33:references` | `chapters/fa_33.tex` |
| `fa_33:results-evidence` | `chapters/fa_33.tex` |
| `fa_33:sealed-seeds` | `chapters/fa_33.tex` |
| `fa_33:usb-enumeration-on-macos-arm` | `chapters/fa_33.tex` |
| `fa_33:verified-hardware-configuration-post-blk-001` | `chapters/fa_33.tex` |
| `fig:<slug>-<n>` | `frontmatter/list-of-figures.tex` |
| `lem:01-best-rational` | `chapters/fa_01.tex` |
| `lem:01-cf-rec` | `chapters/fa_01.tex` |
| `lem:05-coef-limit` | `chapters/fa_05.tex` |
| `lem:05-luc-hankel` | `chapters/fa_05.tex` |
| `lem:05-matrix-power` | `chapters/fa_05.tex` |
| `lem:13-galois` | `chapters/fa_13.tex` |
| `lem:13-gf16-floor` | `chapters/fa_13.tex` |
| `lem:13-primary` | `chapters/fa_13.tex` |
| `lem:13-secondary` | `chapters/fa_13.tex` |
| `lem:13-tertiary` | `chapters/fa_13.tex` |
| `lem:13-trinity` | `chapters/fa_13.tex` |
| `sec:05-anchor-coeff` | `chapters/fa_05.tex` |
| `sec:05-app-F` | `chapters/fa_05.tex` |
| `sec:05-app-K` | `chapters/fa_05.tex` |
| `sec:05-app-X` | `chapters/fa_05.tex` |
| `sec:05-app-Y` | `chapters/fa_05.tex` |
| `sec:05-closed-form` | `chapters/fa_05.tex` |
| `sec:05-coupling` | `chapters/fa_05.tex` |
| `sec:05-falsification` | `chapters/fa_05.tex` |
| `sec:05-partial-frac` | `chapters/fa_05.tex` |
| `sec:05-prelim` | `chapters/fa_05.tex` |
| `sec:05-radius` | `chapters/fa_05.tex` |
| `sec:05-strand-i` | `chapters/fa_05.tex` |
| `sec:05-strand-ii` | `chapters/fa_05.tex` |
| `sec:05-strand-iii` | `chapters/fa_05.tex` |
| `sec:13-appA` | `chapters/fa_13.tex` |
| `sec:13-appB` | `chapters/fa_13.tex` |
| `sec:13-appC` | `chapters/fa_13.tex` |
| `sec:13-appH` | `chapters/fa_13.tex` |
| `sec:13-arch-scaffold` | `chapters/fa_13.tex` |
| `sec:13-filt-coq` | `chapters/fa_13.tex` |
| `sec:13-filtration` | `chapters/fa_13.tex` |
| `sec:13-seventy-eight` | `chapters/fa_13.tex` |
| `sec:13-symmetry-group` | `chapters/fa_13.tex` |
| `sec:13-trinity-bookkeeping` | `chapters/fa_13.tex` |
| `sec:ckm` | `chapters/fa_20.tex` |
| `sec:mass` | `chapters/fa_20.tex` |
| `sec:mesh-roadmap` | `chapters/ch_35_mesh_node.tex` |
| `sec:xvc-bridge` | `appendix/F-fpga-bitstream.tex` |
| `tab:<slug>-<n>` | `frontmatter/list-of-tables.tex` |
| `tab:ch0-fits` | `chapters/ch_00.tex` |
| `tab:power` | `chapters/ch_35_mesh_node.tex` |
| `thm:01-anchor` | `chapters/fa_01.tex` |
| `thm:01-convergent-fib` | `chapters/fa_01.tex` |
| `thm:01-fixed` | `chapters/fa_01.tex` |
| `thm:01-pentagon` | `chapters/fa_01.tex` |
| `thm:01-pentagon-alg` | `chapters/fa_01.tex` |
| `thm:01-quadratic` | `chapters/fa_01.tex` |
| `thm:01-vesica-lens` | `chapters/fa_01.tex` |
| `thm:05-anchor-as-coeff` | `chapters/fa_05.tex` |
| `thm:05-asymptotic` | `chapters/fa_05.tex` |
| `thm:05-bridge` | `chapters/fa_05.tex` |
| `thm:05-cassini-fib` | `chapters/fa_05.tex` |
| `thm:05-cassini-luc` | `chapters/fa_05.tex` |
| `thm:05-coupling` | `chapters/fa_05.tex` |
| `thm:05-fl-conv` | `chapters/fa_05.tex` |
| `thm:05-genfn-closed` | `chapters/fa_05.tex` |
| `thm:05-partial-frac` | `chapters/fa_05.tex` |
| `thm:05-radius` | `chapters/fa_05.tex` |
| `thm:13-projection` | `chapters/fa_13.tex` |
| `thm:13-total-edges` | `chapters/fa_13.tex` |
| `thm:D:1` | `appendix/D-golden-mirror.tex` |
| `thm:ch1-trinity-identity` | `chapters/ch_01.tex` |
| `thm:ch3-trinity-canonical` | `chapters/ch_03.tex` |
| `thm:euler-lagrange` | `chapters/fa_21.tex` |
| `thm:lucas-binet` | `chapters/fa_29.tex` |
| `thm:lucas-trinity` | `chapters/fa_29.tex` |

</details>

## Referenced keys (119)

These keys are consumed by at least one `\ref`/`\autoref`/`\eqref`/`\Cref`/`\pageref` and were preserved in their bare form:

<details><summary>Click to expand</summary>

| Key | Defined in | Referenced from |
|---|---|---|
| `ch:1` | `chapters/fa_01.tex` | `chapters/ch_00.tex` |
| `ch:11` | `chapters/fa_11.tex` | `chapters/ch_00.tex` |
| `ch:13` | `chapters/fa_13.tex` | `appendix/B-falsification.tex`, `appendix/J-troubleshooting.tex` |
| `ch:15` | `chapters/fa_15.tex` | `appendix/B-falsification.tex`, `appendix/G-data-availability.tex` |
| `ch:17-spiral` | `chapters/fa_17.tex` | `chapters/fa_13.tex` |
| `ch:18` | `chapters/fa_18.tex` | `appendix/B-falsification.tex`, `appendix/G-data-availability.tex` |
| `ch:19` | `chapters/fa_19.tex` | `chapters/ch_00.tex` |
| `ch:21-experiments-jepa` | `chapters/fa_21.tex` | `chapters/fa_13.tex` |
| `ch:23-gf16-algebra` | `chapters/fa_23.tex` | `chapters/fa_13.tex` |
| `ch:24` | `chapters/fa_24.tex` | `appendix/B-falsification.tex` |
| `ch:24-igla-arch` | `chapters/fa_24.tex` | `chapters/fa_13.tex` |
| `ch:25` | `chapters/fa_25.tex` | `appendix/B-falsification.tex` |
| `ch:25-benchmarks` | `chapters/fa_25.tex` | `chapters/fa_13.tex` |
| `ch:26-data-analysis` | `chapters/fa_26.tex` | `chapters/fa_13.tex` |
| `ch:28` | `chapters/fa_28.tex` | `appendix/F-fpga-bitstream.tex` |
| `ch:28-momentum-algebra` | `chapters/fa_28.tex` | `chapters/fa_13.tex` |
| `ch:32` | `chapters/fa_32.tex` | `appendix/I-xdc-pin-map.tex` |
| `ch:33` | `chapters/fa_33.tex` | `appendix/F-fpga-bitstream.tex` |
| `ch:34` | `chapters/fa_33.tex` | `appendix/B-falsification.tex`, `appendix/F-fpga-bitstream.tex` |
| `ch:6` | `chapters/fa_06.tex` | `appendix/C-golden-benchmark.tex` |
| `ch:9` | `chapters/fa_09.tex` | `appendix/B-falsification.tex`, `appendix/C-golden-benchmark.tex` |
| `ch:benchmarks` | `chapters/fa_25.tex` | `chapters/fa_00.tex` |
| `ch:data-analysis` | `chapters/fa_26.tex` | `chapters/fa_00.tex` |
| `ch:e8-symmetry` | `chapters/fa_22.tex` | `chapters/fa_00.tex` |
| `ch:energy` | `chapters/fa_28.tex` | `frontmatter/notation.tex` |
| `ch:experiments-asha` | `chapters/fa_21.tex` | `frontmatter/notation.tex` |
| `ch:experiments-bpb` | `chapters/fa_21.tex` | `frontmatter/notation.tex` |
| `ch:experiments-gf16` | `chapters/fa_23.tex` | `frontmatter/notation.tex` |
| `ch:fibonacci` | `chapters/fa_07.tex` | `frontmatter/notation.tex` |
| `ch:fibonacci-tesselation` | `chapters/fa_07.tex` | `chapters/fa_00.tex` |
| `ch:gf16-algebra` | `chapters/fa_23.tex` | `chapters/fa_00.tex` |
| `ch:golden-egg` | `chapters/fa_01.tex` | `frontmatter/notation.tex` |
| `ch:golden-seed` | `chapters/fa_01.tex` | `frontmatter/notation.tex` |
| `ch:igla-architecture` | `chapters/fa_24.tex` | `chapters/fa_00.tex` |
| `ch:igla-race` | `chapters/fa_24.tex` | `frontmatter/notation.tex` |
| `ch:jepa` | `chapters/fa_21.tex` | `frontmatter/notation.tex` |
| `ch:lucas-closure` | `chapters/fa_29.tex` | `chapters/fa_00.tex` |
| `ch:lucas-ladder` | `chapters/fa_29.tex` | `chapters/fa_05.tex` |
| `ch:lucas-ring` | `chapters/fa_27.tex` | `chapters/fa_00.tex`, `chapters/fa_05.tex`, `frontmatter/notation.tex` |
| `ch:monad` | `chapters/fa_00.tex` | `chapters/fa_00.tex` |
| `ch:nca` | `chapters/fa_29.tex` | `frontmatter/notation.tex` |
| `ch:plm` | `chapters/fa_24.tex` | `appendix/F-fpga-bitstream.tex` |
| `ch:standard-model` | `chapters/fa_20.tex` | `chapters/fa_00.tex` |
| `ch:three-strands` | `chapters/fa_27.tex` | `chapters/fa_00.tex`, `frontmatter/notation.tex` |
| `ch:trinity-identity` | `chapters/fa_27.tex` | `chapters/fa_00.tex` |
| `ch:vesica-piscis` | `chapters/fa_11.tex` | `chapters/fa_00.tex` |
| `ch:vsa` | `chapters/fa_29.tex` | `frontmatter/notation.tex` |
| `cor:01-l2-three` | `chapters/fa_01.tex` | `chapters/fa_01.tex` |
| `cor:01-lucas-as-trace` | `chapters/fa_01.tex` | `chapters/fa_01.tex` |
| `cor:01-reciprocal` | `chapters/fa_01.tex` | `chapters/fa_01.tex` |
| `cor:05-asymptotic-rate` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `cor:05-binet` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `def:13-lucas-12` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `eq:ch0-fit` | `chapters/ch_00.tex` | `chapters/ch_00.tex` |
| `lem:01-best-rational` | `chapters/fa_01.tex` | `chapters/fa_01.tex` |
| `lem:01-cf-rec` | `chapters/fa_01.tex` | `chapters/fa_01.tex` |
| `lem:05-coef-limit` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `lem:05-luc-hankel` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `lem:05-matrix-power` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `lem:13-galois` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `lem:13-gf16-floor` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `lem:13-primary` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `lem:13-secondary` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `lem:13-tertiary` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `lem:13-trinity` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `sec:05-anchor-coeff` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:05-app-F` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:05-app-K` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:05-app-X` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:05-app-Y` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:05-closed-form` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:05-coupling` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:05-falsification` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:05-partial-frac` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:05-prelim` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:05-radius` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:05-strand-i` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:05-strand-ii` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:05-strand-iii` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `sec:13-appA` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `sec:13-appB` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `sec:13-appC` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `sec:13-appH` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `sec:13-arch-scaffold` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `sec:13-filt-coq` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `sec:13-filtration` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `sec:13-seventy-eight` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `sec:13-symmetry-group` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `sec:13-trinity-bookkeeping` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `sec:ckm` | `chapters/fa_20.tex` | `chapters/fa_20.tex` |
| `sec:mass` | `chapters/fa_20.tex` | `chapters/fa_20.tex` |
| `sec:mesh-roadmap` | `chapters/ch_35_mesh_node.tex` | `chapters/ch_35_mesh_node.tex` |
| `sec:xvc-bridge` | `appendix/F-fpga-bitstream.tex` | `appendix/F-fpga-bitstream.tex` |
| `tab:ch0-fits` | `chapters/ch_00.tex` | `chapters/ch_00.tex` |
| `tab:power` | `chapters/ch_35_mesh_node.tex` | `chapters/ch_35_mesh_node.tex` |
| `thm:01-anchor` | `chapters/fa_01.tex` | `chapters/fa_01.tex` |
| `thm:01-convergent-fib` | `chapters/fa_01.tex` | `chapters/fa_01.tex` |
| `thm:01-fixed` | `chapters/fa_01.tex` | `chapters/fa_01.tex` |
| `thm:01-pentagon` | `chapters/fa_01.tex` | `chapters/fa_01.tex` |
| `thm:01-pentagon-alg` | `chapters/fa_01.tex` | `chapters/fa_01.tex` |
| `thm:01-quadratic` | `chapters/fa_01.tex` | `chapters/fa_01.tex` |
| `thm:01-vesica-lens` | `chapters/fa_01.tex` | `chapters/fa_01.tex` |
| `thm:05-anchor-as-coeff` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `thm:05-asymptotic` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `thm:05-bridge` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `thm:05-cassini-fib` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `thm:05-cassini-luc` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `thm:05-coupling` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `thm:05-fl-conv` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `thm:05-genfn-closed` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `thm:05-partial-frac` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `thm:05-radius` | `chapters/fa_05.tex` | `chapters/fa_05.tex` |
| `thm:13-projection` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `thm:13-total-edges` | `chapters/fa_13.tex` | `chapters/fa_13.tex` |
| `thm:ch1-trinity-identity` | `chapters/ch_01.tex` | `chapters/ch_01.tex` |
| `thm:ch3-trinity-canonical` | `chapters/ch_03.tex` | `chapters/ch_03.tex` |
| `thm:euler-lagrange` | `chapters/fa_21.tex` | `chapters/fa_21.tex` |
| `thm:lucas-binet` | `chapters/fa_29.tex` | `chapters/fa_05.tex` |
| `thm:lucas-trinity` | `chapters/fa_29.tex` | `chapters/fa_29.tex` |

</details>

## Skill provenance

Authored under skills `phd-chapter-author v1.1` + `phd-monograph-auditor v1.2`.
Per R5 (honesty): all renames are mechanical, none flip Admitted↔Proven; no `.py`/`.sh` were committed (R1).
