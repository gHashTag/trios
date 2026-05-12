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
| `ch:1` | `chapters/flos_01.tex` |
| `ch:11` | `chapters/flos_11.tex` |
| `ch:13` | `chapters/flos_13.tex` |
| `ch:15` | `chapters/flos_15.tex` |
| `ch:17-spiral` | `chapters/flos_17.tex` |
| `ch:18` | `chapters/flos_18.tex` |
| `ch:19` | `chapters/flos_19.tex` |
| `ch:21-experiments-jepa` | `chapters/flos_21.tex` |
| `ch:23-gf16-algebra` | `chapters/flos_23.tex` |
| `ch:24` | `chapters/flos_24.tex` |
| `ch:24-igla-arch` | `chapters/flos_24.tex` |
| `ch:25` | `chapters/flos_25.tex` |
| `ch:25-benchmarks` | `chapters/flos_25.tex` |
| `ch:26-data-analysis` | `chapters/flos_26.tex` |
| `ch:28` | `chapters/flos_28.tex` |
| `ch:28-momentum-algebra` | `chapters/flos_28.tex` |
| `ch:32` | `chapters/flos_32.tex` |
| `ch:33` | `chapters/flos_33.tex` |
| `ch:34` | `chapters/flos_33.tex` |
| `ch:6` | `chapters/flos_06.tex` |
| `ch:9` | `chapters/flos_09.tex` |
| `ch:benchmarks` | `chapters/flos_25.tex` |
| `ch:data-analysis` | `chapters/flos_26.tex` |
| `ch:e8-symmetry` | `chapters/flos_22.tex` |
| `ch:energy` | `chapters/flos_28.tex` |
| `ch:experiments-asha` | `chapters/flos_21.tex` |
| `ch:experiments-bpb` | `chapters/flos_21.tex` |
| `ch:experiments-gf16` | `chapters/flos_23.tex` |
| `ch:fibonacci` | `chapters/flos_07.tex` |
| `ch:fibonacci-tesselation` | `chapters/flos_07.tex` |
| `ch:gf16-algebra` | `chapters/flos_23.tex` |
| `ch:golden-egg` | `chapters/flos_01.tex` |
| `ch:golden-seed` | `chapters/flos_01.tex` |
| `ch:igla-architecture` | `chapters/flos_24.tex` |
| `ch:igla-race` | `chapters/flos_24.tex` |
| `ch:jepa` | `chapters/flos_21.tex` |
| `ch:lucas-closure` | `chapters/flos_29.tex` |
| `ch:lucas-ladder` | `chapters/flos_29.tex` |
| `ch:lucas-ring` | `chapters/flos_27.tex` |
| `ch:monad` | `chapters/flos_00.tex` |
| `ch:nca` | `chapters/flos_29.tex` |
| `ch:plm` | `chapters/flos_24.tex` |
| `ch:standard-model` | `chapters/flos_20.tex` |
| `ch:three-strands` | `chapters/flos_27.tex` |
| `ch:trinity-identity` | `chapters/flos_27.tex` |
| `ch:vesica-piscis` | `chapters/flos_11.tex` |
| `ch:vsa` | `chapters/flos_29.tex` |
| `flos_34:ch:0` | `chapters/flos_34.tex` |
| `flos_34:thm:0:1` | `chapters/flos_34.tex` |
| `flos_34:thm:0:2` | `chapters/flos_34.tex` |
| `flos_35:abstract` | `chapters/flos_35.tex` |
| `flos_35:ch1-s1-vision-extended` | `chapters/flos_35.tex` |
| `flos_35:ch1-s2-contributions` | `chapters/flos_35.tex` |
| `flos_35:ch1-s3-lineage` | `chapters/flos_35.tex` |
| `flos_35:ch1-s4-theorem-xref` | `chapters/flos_35.tex` |
| `flos_35:ch1-s5-roadmap` | `chapters/flos_35.tex` |
| `flos_35:ch1-s6-notation` | `chapters/flos_35.tex` |
| `flos_35:discussion` | `chapters/flos_35.tex` |
| `flos_35:introduction` | `chapters/flos_35.tex` |
| `flos_35:qed-assertions` | `chapters/flos_35.tex` |
| `flos_35:references` | `chapters/flos_35.tex` |
| `flos_35:research-questions-and-scope` | `chapters/flos_35.tex` |
| `flos_35:results-evidence` | `chapters/flos_35.tex` |
| `flos_35:sealed-seeds` | `chapters/flos_35.tex` |
| `flos_35:tab:ch1-falsification-matrix` | `chapters/flos_35.tex` |
| `flos_35:the-trinity-architecture-and-its-algebraic-substrate` | `chapters/flos_35.tex` |
| `flos_35:thm:ch1-alpha-phi-closed` | `chapters/flos_35.tex` |
| `flos_35:thm:ch1-lucas-closure` | `chapters/flos_35.tex` |
| `flos_36:abstract` | `chapters/flos_36.tex` |
| `flos_36:ch2-s1-kart-kan` | `chapters/flos_36.tex` |
| `flos_36:ch2-s2-finite-field` | `chapters/flos_36.tex` |
| `flos_36:ch2-s3-ternary` | `chapters/flos_36.tex` |
| `flos_36:ch2-s4-vsa` | `chapters/flos_36.tex` |
| `flos_36:ch2-s5-ltn` | `chapters/flos_36.tex` |
| `flos_36:ch2-s6-cliffs` | `chapters/flos_36.tex` |
| `flos_36:ch2-s7-gap` | `chapters/flos_36.tex` |
| `flos_36:ch2-s8-theorems` | `chapters/flos_36.tex` |
| `flos_36:discussion` | `chapters/flos_36.tex` |
| `flos_36:early-symbolicconnectionist-hybrids` | `chapters/flos_36.tex` |
| `flos_36:fibonacci-and-lucas-lattices-as-basis-sets` | `chapters/flos_36.tex` |
| `flos_36:gap-in-prior-art` | `chapters/flos_36.tex` |
| `flos_36:introduction` | `chapters/flos_36.tex` |
| `flos_36:logic-tensor-networks-and-differentiable-reasoning` | `chapters/flos_36.tex` |
| `flos_36:qed-assertions` | `chapters/flos_36.tex` |
| `flos_36:references` | `chapters/flos_36.tex` |
| `flos_36:representational-bottleneck-and-the-ux3c6-structural-prior` | `chapters/flos_36.tex` |
| `flos_36:results-evidence` | `chapters/flos_36.tex` |
| `flos_36:sealed-seeds` | `chapters/flos_36.tex` |
| `flos_36:sparse-and-ternary-neural-computation` | `chapters/flos_36.tex` |
| `flos_36:taxonomy-of-neuro-symbolic-paradigms` | `chapters/flos_36.tex` |
| `flos_36:the-normalisation-problem` | `chapters/flos_36.tex` |
| `flos_36:thm:ch2-phi-square` | `chapters/flos_36.tex` |
| `flos_36:thm:ch2-trinity` | `chapters/flos_36.tex` |
| `flos_36:vector-symbolic-architectures` | `chapters/flos_36.tex` |
| `flos_37:abstract` | `chapters/flos_37.tex` |
| `flos_37:ch3-s1-trinity-detail` | `chapters/flos_37.tex` |
| `flos_37:ch3-s2-phi-family` | `chapters/flos_37.tex` |
| `flos_37:ch3-s3-coq-listing` | `chapters/flos_37.tex` |
| `flos_37:ch3-s4-numeric` | `chapters/flos_37.tex` |
| `flos_37:ch3-s5-arch` | `chapters/flos_37.tex` |
| `flos_37:coq-mechanisation-and-sac-0-invariant` | `chapters/flos_37.tex` |
| `flos_37:derivation-of-the-anchor-identity` | `chapters/flos_37.tex` |
| `flos_37:discussion` | `chapters/flos_37.tex` |
| `flos_37:introduction` | `chapters/flos_37.tex` |
| `flos_37:invariant-sac-0` | `chapters/flos_37.tex` |
| `flos_37:minimal-polynomial-and-basic-consequences` | `chapters/flos_37.tex` |
| `flos_37:power-survey` | `chapters/flos_37.tex` |
| `flos_37:proof-architecture` | `chapters/flos_37.tex` |
| `flos_37:qed-assertions` | `chapters/flos_37.tex` |
| `flos_37:references` | `chapters/flos_37.tex` |
| `flos_37:relation-to-fibonacci-arithmetic` | `chapters/flos_37.tex` |
| `flos_37:results-evidence` | `chapters/flos_37.tex` |
| `flos_37:sealed-seeds` | `chapters/flos_37.tex` |
| `flos_37:the-integer-3-coincidence` | `chapters/flos_37.tex` |
| `flos_38:abstract` | `chapters/flos_38.tex` |
| `flos_38:ch4-s1-alpha-phi` | `chapters/flos_38.tex` |
| `flos_38:ch4-s2-dimensional` | `chapters/flos_38.tex` |
| `flos_38:ch4-s3-alpha-qed` | `chapters/flos_38.tex` |
| `flos_38:ch4-s4-derivation-levels` | `chapters/flos_38.tex` |
| `flos_38:ch4-s5-runtime` | `chapters/flos_38.tex` |
| `flos_38:ch4-s6-gate` | `chapters/flos_38.tex` |
| `flos_38:derivation-of-the-closed-form` | `chapters/flos_38.tex` |
| `flos_38:discussion` | `chapters/flos_38.tex` |
| `flos_38:introduction` | `chapters/flos_38.tex` |
| `flos_38:multiplicative-identity-and-kernel-integration` | `chapters/flos_38.tex` |
| `flos_38:qed-assertions` | `chapters/flos_38.tex` |
| `flos_38:references` | `chapters/flos_38.tex` |
| `flos_38:results-evidence` | `chapters/flos_38.tex` |
| `flos_38:sealed-seeds` | `chapters/flos_38.tex` |
| `flos_38:tab:ch4-dimensional` | `chapters/flos_38.tex` |
| `flos_39:abstract` | `chapters/flos_39.tex` |
| `flos_39:ch5-s1-lucas-closure` | `chapters/flos_39.tex` |
| `flos_39:ch5-s2-basin` | `chapters/flos_39.tex` |
| `flos_39:ch5-s3-seeds` | `chapters/flos_39.tex` |
| `flos_39:ch5-s4-coq-listing` | `chapters/flos_39.tex` |
| `flos_39:ch5-s5-admissibility` | `chapters/flos_39.tex` |
| `flos_39:ch5-s6-arch` | `chapters/flos_39.tex` |
| `flos_39:discussion` | `chapters/flos_39.tex` |
| `flos_39:fibonacci-lucas-seeds-and-their-contractive-basin` | `chapters/flos_39.tex` |
| `flos_39:introduction` | `chapters/flos_39.tex` |
| `flos_39:qed-assertions` | `chapters/flos_39.tex` |
| `flos_39:references` | `chapters/flos_39.tex` |
| `flos_39:results-evidence` | `chapters/flos_39.tex` |
| `flos_39:sealed-seeds` | `chapters/flos_39.tex` |
| `flos_39:the-ux3c6-distance-metric-and-the-balancing-fixed-point` | `chapters/flos_39.tex` |
| `flos_40:abstract` | `chapters/flos_40.tex` |
| `flos_40:coq-encoding` | `chapters/flos_40.tex` |
| `flos_40:discussion` | `chapters/flos_40.tex` |
| `flos_40:goldenfloat-format-definitions` | `chapters/flos_40.tex` |
| `flos_40:introduction` | `chapters/flos_40.tex` |
| `flos_40:key-theorems-and-proof-sketches` | `chapters/flos_40.tex` |
| `flos_40:lucas-closure-on-gf16` | `chapters/flos_40.tex` |
| `flos_40:preliminaries` | `chapters/flos_40.tex` |
| `flos_40:qed-assertions` | `chapters/flos_40.tex` |
| `flos_40:references` | `chapters/flos_40.tex` |
| `flos_40:results-evidence` | `chapters/flos_40.tex` |
| `flos_40:sealed-seeds` | `chapters/flos_40.tex` |
| `flos_41:abstract` | `chapters/flos_41.tex` |
| `flos_41:discussion` | `chapters/flos_41.tex` |
| `flos_41:from-the-trinity-identity-to-the-golden-angle` | `chapters/flos_41.tex` |
| `flos_41:h4-root-system-e8-lattice-and-the-varphi-scaled-block-decomposition` | `chapters/flos_41.tex` |
| `flos_41:introduction` | `chapters/flos_41.tex` |
| `flos_41:qed-assertions` | `chapters/flos_41.tex` |
| `flos_41:references` | `chapters/flos_41.tex` |
| `flos_41:results-evidence` | `chapters/flos_41.tex` |
| `flos_41:sealed-seeds` | `chapters/flos_41.tex` |
| `flos_42:abstract` | `chapters/flos_42.tex` |
| `flos_42:discussion` | `chapters/flos_42.tex` |
| `flos_42:gain-admissibility` | `chapters/flos_42.tex` |
| `flos_42:hybrid-qk-gain-invariant-inv-6` | `chapters/flos_42.tex` |
| `flos_42:introduction` | `chapters/flos_42.tex` |
| `flos_42:proof-sketch-for-admit_phi_sq` | `chapters/flos_42.tex` |
| `flos_42:qed-assertions` | `chapters/flos_42.tex` |
| `flos_42:references` | `chapters/flos_42.tex` |
| `flos_42:results-evidence` | `chapters/flos_42.tex` |
| `flos_42:sealed-seeds` | `chapters/flos_42.tex` |
| `flos_42:tf3-and-tf9-algebraic-structure` | `chapters/flos_42.tex` |
| `flos_42:tf9-product-encoding` | `chapters/flos_42.tex` |
| `flos_42:trit-encoding` | `chapters/flos_42.tex` |
| `flos_42:ux3c6-normalisation` | `chapters/flos_42.tex` |
| `flos_43:ablation-matrix-tier-abc-m1m6` | `chapters/flos_43.tex` |
| `flos_43:abstract` | `chapters/flos_43.tex` |
| `flos_43:competitor-format-summaries` | `chapters/flos_43.tex` |
| `flos_43:discussion` | `chapters/flos_43.tex` |
| `flos_43:gf16-format-specification` | `chapters/flos_43.tex` |
| `flos_43:gf16-phi_bias60-and-the-inv-3-safe-domain` | `chapters/flos_43.tex` |
| `flos_43:introduction` | `chapters/flos_43.tex` |
| `flos_43:inv-3-nine-coq-precision-bounds` | `chapters/flos_43.tex` |
| `flos_43:qed-assertions` | `chapters/flos_43.tex` |
| `flos_43:references` | `chapters/flos_43.tex` |
| `flos_43:results-evidence` | `chapters/flos_43.tex` |
| `flos_43:sealed-seeds` | `chapters/flos_43.tex` |
| `flos_44:abstract` | `chapters/flos_44.tex` |
| `flos_44:discussion` | `chapters/flos_44.tex` |
| `flos_44:gf16-range-and-precision-formalisation` | `chapters/flos_44.tex` |
| `flos_44:introduction` | `chapters/flos_44.tex` |
| `flos_44:qed-assertions` | `chapters/flos_44.tex` |
| `flos_44:references` | `chapters/flos_44.tex` |
| `flos_44:results-evidence` | `chapters/flos_44.tex` |
| `flos_44:sealed-seeds` | `chapters/flos_44.tex` |
| `flos_44:the-pareto-frontier-and-conjecture-c1` | `chapters/flos_44.tex` |
| `flos_45:abstract` | `chapters/flos_45.tex` |
| `flos_45:discussion` | `chapters/flos_45.tex` |
| `flos_45:hypothesis-formalisation-and-registration-protocol` | `chapters/flos_45.tex` |
| `flos_45:introduction` | `chapters/flos_45.tex` |
| `flos_45:inv-7-invariant-and-coq-formalisation` | `chapters/flos_45.tex` |
| `flos_45:qed-assertions` | `chapters/flos_45.tex` |
| `flos_45:references` | `chapters/flos_45.tex` |
| `flos_45:results-evidence` | `chapters/flos_45.tex` |
| `flos_45:sealed-seeds` | `chapters/flos_45.tex` |
| `flos_46:abstract` | `chapters/flos_46.tex` |
| `flos_46:bridge-architecture-and-interface-contracts` | `chapters/flos_46.tex` |
| `flos_46:clock-domain-analysis-and-timing` | `chapters/flos_46.tex` |
| `flos_46:discussion` | `chapters/flos_46.tex` |
| `flos_46:error-handling-protocol` | `chapters/flos_46.tex` |
| `flos_46:frequency-ratios-and-the-golden-ratio` | `chapters/flos_46.tex` |
| `flos_46:introduction` | `chapters/flos_46.tex` |
| `flos_46:logical-structure` | `chapters/flos_46.tex` |
| `flos_46:power-accounting` | `chapters/flos_46.tex` |
| `flos_46:qed-assertions` | `chapters/flos_46.tex` |
| `flos_46:references` | `chapters/flos_46.tex` |
| `flos_46:results-evidence` | `chapters/flos_46.tex` |
| `flos_46:sealed-seeds` | `chapters/flos_46.tex` |
| `flos_46:signal-naming-convention` | `chapters/flos_46.tex` |
| `flos_46:throughput-budget` | `chapters/flos_46.tex` |
| `flos_47:abstract` | `chapters/flos_47.tex` |
| `flos_47:discussion` | `chapters/flos_47.tex` |
| `flos_47:introduction` | `chapters/flos_47.tex` |
| `flos_47:qed-assertions` | `chapters/flos_47.tex` |
| `flos_47:references` | `chapters/flos_47.tex` |
| `flos_47:results-evidence` | `chapters/flos_47.tex` |
| `flos_47:sealed-seeds` | `chapters/flos_47.tex` |
| `flos_47:the-runtime-mirror-contract-and-igla_assertions.json` | `chapters/flos_47.tex` |
| `flos_47:the-strobe-seed-admissibility-criterion` | `chapters/flos_47.tex` |
| `flos_48:abstract` | `chapters/flos_48.tex` |
| `flos_48:bpb-definition-and-algebraic-properties` | `chapters/flos_48.tex` |
| `flos_48:byte-level-normalisation` | `chapters/flos_48.tex` |
| `flos_48:cross-entropy-and-perplexity` | `chapters/flos_48.tex` |
| `flos_48:discussion` | `chapters/flos_48.tex` |
| `flos_48:gate-2-bpb-1.85` | `chapters/flos_48.tex` |
| `flos_48:gate-3-bpb-1.50` | `chapters/flos_48.tex` |
| `flos_48:gate-thresholds-and-their-derivation` | `chapters/flos_48.tex` |
| `flos_48:introduction` | `chapters/flos_48.tex` |
| `flos_48:qed-assertions` | `chapters/flos_48.tex` |
| `flos_48:references` | `chapters/flos_48.tex` |
| `flos_48:relationship-to-the-darpa-energy-goal` | `chapters/flos_48.tex` |
| `flos_48:results-evidence` | `chapters/flos_48.tex` |
| `flos_48:sealed-seeds` | `chapters/flos_48.tex` |
| `flos_48:ux3c6-weighted-bpb` | `chapters/flos_48.tex` |
| `flos_49:abstract` | `chapters/flos_49.tex` |
| `flos_49:bpb-protocol-and-monotone-backward-invariant-inv-1` | `chapters/flos_49.tex` |
| `flos_49:database-schema` | `chapters/flos_49.tex` |
| `flos_49:discussion` | `chapters/flos_49.tex` |
| `flos_49:evaluation-protocol` | `chapters/flos_49.tex` |
| `flos_49:gate-evaluation` | `chapters/flos_49.tex` |
| `flos_49:introduction` | `chapters/flos_49.tex` |
| `flos_49:inv-1-bpb-monotone-backward` | `chapters/flos_49.tex` |
| `flos_49:qed-assertions` | `chapters/flos_49.tex` |
| `flos_49:railway-write-back-architecture` | `chapters/flos_49.tex` |
| `flos_49:references` | `chapters/flos_49.tex` |
| `flos_49:results-evidence` | `chapters/flos_49.tex` |
| `flos_49:sealed-seeds` | `chapters/flos_49.tex` |
| `flos_49:warmup-gate` | `chapters/flos_49.tex` |
| `flos_49:write-back-protocol` | `chapters/flos_49.tex` |
| `flos_50:abstract` | `chapters/flos_50.tex` |
| `flos_50:discussion` | `chapters/flos_50.tex` |
| `flos_50:grid-construction-and-sparsity-analysis` | `chapters/flos_50.tex` |
| `flos_50:introduction` | `chapters/flos_50.tex` |
| `flos_50:qed-assertions` | `chapters/flos_50.tex` |
| `flos_50:references` | `chapters/flos_50.tex` |
| `flos_50:results-evidence` | `chapters/flos_50.tex` |
| `flos_50:sealed-seeds` | `chapters/flos_50.tex` |
| `flos_50:the-phi-distance-function` | `chapters/flos_50.tex` |
| `flos_51:abstract` | `chapters/flos_51.tex` |
| `flos_51:analysis-of-effects-and-golden-ratio-structure` | `chapters/flos_51.tex` |
| `flos_51:discussion` | `chapters/flos_51.tex` |
| `flos_51:factor-definitions-and-experimental-design` | `chapters/flos_51.tex` |
| `flos_51:introduction` | `chapters/flos_51.tex` |
| `flos_51:qed-assertions` | `chapters/flos_51.tex` |
| `flos_51:references` | `chapters/flos_51.tex` |
| `flos_51:results-evidence` | `chapters/flos_51.tex` |
| `flos_51:sealed-seeds` | `chapters/flos_51.tex` |
| `flos_52:abstract` | `chapters/flos_52.tex` |
| `flos_52:coq.interval-upgrade-lane` | `chapters/flos_52.tex` |
| `flos_52:discussion` | `chapters/flos_52.tex` |
| `flos_52:hardware-and-runtime-limitations` | `chapters/flos_52.tex` |
| `flos_52:introduction` | `chapters/flos_52.tex` |
| `flos_52:qed-assertions` | `chapters/flos_52.tex` |
| `flos_52:references` | `chapters/flos_52.tex` |
| `flos_52:sealed-seeds` | `chapters/flos_52.tex` |
| `flos_52:state-of-the-art-comparison-clara-soa-snapshot` | `chapters/flos_52.tex` |
| `flos_53:abstract` | `chapters/flos_53.tex` |
| `flos_53:discussion` | `chapters/flos_53.tex` |
| `flos_53:introduction` | `chapters/flos_53.tex` |
| `flos_53:qed-assertions` | `chapters/flos_53.tex` |
| `flos_53:references` | `chapters/flos_53.tex` |
| `flos_53:results-evidence` | `chapters/flos_53.tex` |
| `flos_53:sealed-seeds` | `chapters/flos_53.tex` |
| `flos_53:test-design-and-hypotheses` | `chapters/flos_53.tex` |
| `flos_53:welch-t-statistic-and-degrees-of-freedom` | `chapters/flos_53.tex` |
| `flos_54:abstract` | `chapters/flos_54.tex` |
| `flos_54:algebraic-basis` | `chapters/flos_54.tex` |
| `flos_54:discussion` | `chapters/flos_54.tex` |
| `flos_54:hardware-and-software-specification` | `chapters/flos_54.tex` |
| `flos_54:hardware-pinning` | `chapters/flos_54.tex` |
| `flos_54:introduction` | `chapters/flos_54.tex` |
| `flos_54:non-determinism-budget` | `chapters/flos_54.tex` |
| `flos_54:qed-assertions` | `chapters/flos_54.tex` |
| `flos_54:references` | `chapters/flos_54.tex` |
| `flos_54:results-evidence` | `chapters/flos_54.tex` |
| `flos_54:sanctioned-seed-protocol` | `chapters/flos_54.tex` |
| `flos_54:sealed-seeds` | `chapters/flos_54.tex` |
| `flos_54:seed-assignment-to-experiments` | `chapters/flos_54.tex` |
| `flos_54:seed-verification` | `chapters/flos_54.tex` |
| `flos_54:software-environment` | `chapters/flos_54.tex` |
| `flos_55:abstract` | `chapters/flos_55.tex` |
| `flos_55:agent-topology` | `chapters/flos_55.tex` |
| `flos_55:definitions` | `chapters/flos_55.tex` |
| `flos_55:discussion` | `chapters/flos_55.tex` |
| `flos_55:formal-victory-criterion-inv-7` | `chapters/flos_55.tex` |
| `flos_55:introduction` | `chapters/flos_55.tex` |
| `flos_55:multi-agent-fleet-architecture` | `chapters/flos_55.tex` |
| `flos_55:qed-assertions` | `chapters/flos_55.tex` |
| `flos_55:rainbow-bridge-consistency-inv-7b` | `chapters/flos_55.tex` |
| `flos_55:references` | `chapters/flos_55.tex` |
| `flos_55:relation-to-varphi2-varphi-2-3` | `chapters/flos_55.tex` |
| `flos_55:results-evidence` | `chapters/flos_55.tex` |
| `flos_55:sealed-seeds` | `chapters/flos_55.tex` |
| `flos_55:six-refutation-theorems` | `chapters/flos_55.tex` |
| `flos_55:victory-declaration-protocol` | `chapters/flos_55.tex` |
| `flos_56:abstract` | `chapters/flos_56.tex` |
| `flos_56:discussion` | `chapters/flos_56.tex` |
| `flos_56:introduction` | `chapters/flos_56.tex` |
| `flos_56:qed-assertions` | `chapters/flos_56.tex` |
| `flos_56:references` | `chapters/flos_56.tex` |
| `flos_56:results-evidence` | `chapters/flos_56.tex` |
| `flos_56:satisfaction-witness-and-victory-predicate` | `chapters/flos_56.tex` |
| `flos_56:sealed-seeds` | `chapters/flos_56.tex` |
| `flos_56:worker-pool-invariants-and-falsification-witnesses` | `chapters/flos_56.tex` |
| `flos_57:abstract` | `chapters/flos_57.tex` |
| `flos_57:discussion` | `chapters/flos_57.tex` |
| `flos_57:introduction` | `chapters/flos_57.tex` |
| `flos_57:mcp-adapter-layer-architecture` | `chapters/flos_57.tex` |
| `flos_57:protocol-implementation-and-latency-analysis` | `chapters/flos_57.tex` |
| `flos_57:qed-assertions` | `chapters/flos_57.tex` |
| `flos_57:references` | `chapters/flos_57.tex` |
| `flos_57:results-evidence` | `chapters/flos_57.tex` |
| `flos_57:sealed-seeds` | `chapters/flos_57.tex` |
| `flos_58:abstract` | `chapters/flos_58.tex` |
| `flos_58:agent-model` | `chapters/flos_58.tex` |
| `flos_58:coq-encoding` | `chapters/flos_58.tex` |
| `flos_58:discussion` | `chapters/flos_58.tex` |
| `flos_58:formal-model-of-the-period-locked-monitor` | `chapters/flos_58.tex` |
| `flos_58:implementation-and-hardware-interface` | `chapters/flos_58.tex` |
| `flos_58:interrupt-interface-with-the-hardware-bridge` | `chapters/flos_58.tex` |
| `flos_58:introduction` | `chapters/flos_58.tex` |
| `flos_58:period-ratio-and-non-resonance` | `chapters/flos_58.tex` |
| `flos_58:priority-queue-and-phi-weighted-scheduling` | `chapters/flos_58.tex` |
| `flos_58:qed-assertions` | `chapters/flos_58.tex` |
| `flos_58:references` | `chapters/flos_58.tex` |
| `flos_58:results-evidence` | `chapters/flos_58.tex` |
| `flos_58:rtl-implementation` | `chapters/flos_58.tex` |
| `flos_58:sealed-seeds` | `chapters/flos_58.tex` |
| `flos_59:abstract` | `chapters/flos_59.tex` |
| `flos_59:cycle-classification-and-attention-periodicity` | `chapters/flos_59.tex` |
| `flos_59:discussion` | `chapters/flos_59.tex` |
| `flos_59:introduction` | `chapters/flos_59.tex` |
| `flos_59:qed-assertions` | `chapters/flos_59.tex` |
| `flos_59:references` | `chapters/flos_59.tex` |
| `flos_59:results-evidence` | `chapters/flos_59.tex` |
| `flos_59:sealed-seeds` | `chapters/flos_59.tex` |
| `flos_59:varphi-lattice-structure-and-the-cycle-map` | `chapters/flos_59.tex` |
| `flos_60:abstract` | `chapters/flos_60.tex` |
| `flos_60:discussion` | `chapters/flos_60.tex` |
| `flos_60:gf16_quant-galois-field-16-quantisation` | `chapters/flos_60.tex` |
| `flos_60:instruction-encoding` | `chapters/flos_60.tex` |
| `flos_60:introduction` | `chapters/flos_60.tex` |
| `flos_60:isa-register-file-and-encoding` | `chapters/flos_60.tex` |
| `flos_60:opcode-specifications` | `chapters/flos_60.tex` |
| `flos_60:phi_rope-ux3c6-rotary-position-encoding` | `chapters/flos_60.tex` |
| `flos_60:qed-assertions` | `chapters/flos_60.tex` |
| `flos_60:references` | `chapters/flos_60.tex` |
| `flos_60:register-file` | `chapters/flos_60.tex` |
| `flos_60:results-evidence` | `chapters/flos_60.tex` |
| `flos_60:sealed-seeds` | `chapters/flos_60.tex` |
| `flos_60:tf3_add-ternary-addition` | `chapters/flos_60.tex` |
| `flos_60:tf3_mul-ternary-multiplication` | `chapters/flos_60.tex` |
| `flos_60:vsa_bind-hyperdimensional-binding` | `chapters/flos_60.tex` |
| `flos_60:vsa_bundle-hyperdimensional-bundling` | `chapters/flos_60.tex` |
| `flos_60:vsa_unbind-hyperdimensional-unbinding` | `chapters/flos_60.tex` |
| `flos_61:abstract` | `chapters/flos_61.tex` |
| `flos_61:abstract-syntax` | `chapters/flos_61.tex` |
| `flos_61:discussion` | `chapters/flos_61.tex` |
| `flos_61:environments-and-evaluation` | `chapters/flos_61.tex` |
| `flos_61:introduction` | `chapters/flos_61.tex` |
| `flos_61:mechanised-proofs-determinism-and-exhaustiveness` | `chapters/flos_61.tex` |
| `flos_61:qed-assertions` | `chapters/flos_61.tex` |
| `flos_61:references` | `chapters/flos_61.tex` |
| `flos_61:relation-to-gf16-and-varphi-arithmetic` | `chapters/flos_61.tex` |
| `flos_61:results-evidence` | `chapters/flos_61.tex` |
| `flos_61:sealed-seeds` | `chapters/flos_61.tex` |
| `flos_61:ternary-arithmetic` | `chapters/flos_61.tex` |
| `flos_61:theorem-eval_det-determinism` | `chapters/flos_61.tex` |
| `flos_61:theorem-trit_exhaustive-exhaustiveness` | `chapters/flos_61.tex` |
| `flos_61:tri27-syntax-and-denotational-semantics` | `chapters/flos_61.tex` |
| `flos_62:abstract` | `chapters/flos_62.tex` |
| `flos_62:architecture-zero-dsp-ternary-datapath` | `chapters/flos_62.tex` |
| `flos_62:discussion` | `chapters/flos_62.tex` |
| `flos_62:introduction` | `chapters/flos_62.tex` |
| `flos_62:qed-assertions` | `chapters/flos_62.tex` |
| `flos_62:references` | `chapters/flos_62.tex` |
| `flos_62:resource-utilisation-and-timing-closure` | `chapters/flos_62.tex` |
| `flos_62:results-evidence` | `chapters/flos_62.tex` |
| `flos_62:sealed-seeds` | `chapters/flos_62.tex` |
| `flos_63:abstract` | `chapters/flos_63.tex` |
| `flos_63:coq-formalisation-and-ckm-unitarity-seed` | `chapters/flos_63.tex` |
| `flos_63:discussion` | `chapters/flos_63.tex` |
| `flos_63:introduction` | `chapters/flos_63.tex` |
| `flos_63:qed-assertions` | `chapters/flos_63.tex` |
| `flos_63:references` | `chapters/flos_63.tex` |
| `flos_63:results-evidence` | `chapters/flos_63.tex` |
| `flos_63:sealed-seeds` | `chapters/flos_63.tex` |
| `flos_63:the-sacred-formula-v-conjecture-and-ux3c6-monomial-parameterisation` | `chapters/flos_63.tex` |
| `flos_64:abstract` | `chapters/flos_64.tex` |
| `flos_64:associative-recall-memory` | `chapters/flos_64.tex` |
| `flos_64:discussion` | `chapters/flos_64.tex` |
| `flos_64:goldenfloat-encoding-of-hypervectors` | `chapters/flos_64.tex` |
| `flos_64:hypervector-definition` | `chapters/flos_64.tex` |
| `flos_64:introduction` | `chapters/flos_64.tex` |
| `flos_64:phi-rotary-position-encoding-phi-rope-in-vsa-context` | `chapters/flos_64.tex` |
| `flos_64:qed-assertions` | `chapters/flos_64.tex` |
| `flos_64:references` | `chapters/flos_64.tex` |
| `flos_64:results-evidence` | `chapters/flos_64.tex` |
| `flos_64:sealed-seeds` | `chapters/flos_64.tex` |
| `flos_64:ternary-vsa-over-the-goldenfloat-substrate` | `chapters/flos_64.tex` |
| `flos_65:abstract` | `chapters/flos_65.tex` |
| `flos_65:discussion` | `chapters/flos_65.tex` |
| `flos_65:formal-seal-297-coq-theorems` | `chapters/flos_65.tex` |
| `flos_65:hardware-architecture` | `chapters/flos_65.tex` |
| `flos_65:introduction` | `chapters/flos_65.tex` |
| `flos_65:qed-assertions` | `chapters/flos_65.tex` |
| `flos_65:references` | `chapters/flos_65.tex` |
| `flos_65:results-evidence` | `chapters/flos_65.tex` |
| `flos_65:sealed-seeds` | `chapters/flos_65.tex` |
| `flos_66:abstract` | `chapters/flos_66.tex` |
| `flos_66:crc-16ccitt-polynomial` | `chapters/flos_66.tex` |
| `flos_66:discussion` | `chapters/flos_66.tex` |
| `flos_66:error-recovery-automaton` | `chapters/flos_66.tex` |
| `flos_66:frame-grammar` | `chapters/flos_66.tex` |
| `flos_66:frame-structure-and-grammar` | `chapters/flos_66.tex` |
| `flos_66:introduction` | `chapters/flos_66.tex` |
| `flos_66:physical-layer` | `chapters/flos_66.tex` |
| `flos_66:qed-assertions` | `chapters/flos_66.tex` |
| `flos_66:references` | `chapters/flos_66.tex` |
| `flos_66:results-evidence` | `chapters/flos_66.tex` |
| `flos_66:sealed-seeds` | `chapters/flos_66.tex` |
| `flos_66:sync-frame-payload` | `chapters/flos_66.tex` |
| `flos_66:sync-frame-trigger` | `chapters/flos_66.tex` |
| `flos_66:ux3c6-synchronisation-frames` | `chapters/flos_66.tex` |
| `flos_67:abstract` | `chapters/flos_67.tex` |
| `flos_67:diagnosis-and-root-cause` | `chapters/flos_67.tex` |
| `flos_67:discussion` | `chapters/flos_67.tex` |
| `flos_67:flash_no_sudo.sh` | `chapters/flos_67.tex` |
| `flos_67:fxload-cross-compilation` | `chapters/flos_67.tex` |
| `flos_67:introduction` | `chapters/flos_67.tex` |
| `flos_67:qed-assertions` | `chapters/flos_67.tex` |
| `flos_67:references` | `chapters/flos_67.tex` |
| `flos_67:results-evidence` | `chapters/flos_67.tex` |
| `flos_67:sealed-seeds` | `chapters/flos_67.tex` |
| `flos_67:usb-enumeration-on-macos-arm` | `chapters/flos_67.tex` |
| `flos_67:verified-hardware-configuration-post-blk-001` | `chapters/flos_67.tex` |
| `flos_68:abstract` | `chapters/flos_68.tex` |
| `flos_68:discussion` | `chapters/flos_68.tex` |
| `flos_68:energy-accounting-framework` | `chapters/flos_68.tex` |
| `flos_68:introduction` | `chapters/flos_68.tex` |
| `flos_68:qed-assertions` | `chapters/flos_68.tex` |
| `flos_68:references` | `chapters/flos_68.tex` |
| `flos_68:results-evidence` | `chapters/flos_68.tex` |
| `flos_68:sealed-seeds` | `chapters/flos_68.tex` |
| `flos_68:ternary-mechanism-analysis` | `chapters/flos_68.tex` |
| `flos_69:ch:mesh-node` | `chapters/flos_69.tex` |
| `flos_69:fig:asic-block` | `chapters/flos_69.tex` |
| `flos_69:tab:comparison` | `chapters/flos_69.tex` |
| `flos_69:tab:rns-packets` | `chapters/flos_69.tex` |
| `flos_69:thm:mru-liveness` | `chapters/flos_69.tex` |
| `flos_69:thm:phi-id` | `chapters/flos_69.tex` |
| `flos_69:thm:power-budget` | `chapters/flos_69.tex` |
| `cor:01-l2-three` | `chapters/flos_01.tex` |
| `cor:01-lucas-as-trace` | `chapters/flos_01.tex` |
| `cor:01-reciprocal` | `chapters/flos_01.tex` |
| `cor:05-asymptotic-rate` | `chapters/flos_05.tex` |
| `cor:05-binet` | `chapters/flos_05.tex` |
| `def:13-lucas-12` | `chapters/flos_13.tex` |
| `eq:ch0-fit` | `chapters/flos_34.tex` |
| `eq:gf-def` | `appendix/C-golden-benchmark.tex` |
| `flos_00:thm:trinity-identity-prologue` | `chapters/flos_00.tex` |
| `flos_01:cor:01-approx-quality` | `chapters/flos_01.tex` |
| `flos_01:cor:01-cascade` | `chapters/flos_01.tex` |
| `flos_01:cor:01-fp-rate` | `chapters/flos_01.tex` |
| `flos_01:cor:01-pentagon-vesica` | `chapters/flos_01.tex` |
| `flos_01:fig:vesica` | `chapters/flos_01.tex` |
| `flos_01:lem:01-gm-limit` | `chapters/flos_01.tex` |
| `flos_01:lem:01-golden-angle` | `chapters/flos_01.tex` |
| `flos_01:lem:01-hex-vesica` | `chapters/flos_01.tex` |
| `flos_01:lem:01-newton-phi` | `chapters/flos_01.tex` |
| `flos_01:lem:01-pentagram-self` | `chapters/flos_01.tex` |
| `flos_01:lem:01-small-n` | `chapters/flos_01.tex` |
| `flos_01:lem:01-vesica-area` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-A` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AA` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AB` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AC` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AD` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AE` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AF` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AG` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AH` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AI` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AJ` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AK` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AL` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AM` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AN` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AO` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AP` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AQ` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AR` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AS` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-AT` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-B` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-C` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-D` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-E` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-F` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-G` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-H` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-I` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-J` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-K` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-L` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-M` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-N` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-O` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-P` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-Q` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-R` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-S` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-T` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-U` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-V` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-W` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-X` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-Y` | `chapters/flos_01.tex` |
| `flos_01:sec:01-app-Z` | `chapters/flos_01.tex` |
| `flos_01:sec:01-strand-I` | `chapters/flos_01.tex` |
| `flos_01:sec:01-strand-II` | `chapters/flos_01.tex` |
| `flos_01:sec:01-strand-III` | `chapters/flos_01.tex` |
| `flos_01:thm:01-dodec-circ` | `chapters/flos_01.tex` |
| `flos_01:thm:01-lucas-anchor` | `chapters/flos_01.tex` |
| `flos_01:thm:01-lucas-as-trace` | `chapters/flos_01.tex` |
| `flos_01:thm:01-ring-int` | `chapters/flos_01.tex` |
| `flos_01:thm:01-three-witnesses` | `chapters/flos_01.tex` |
| `flos_01:thm:01-trace-as-Z` | `chapters/flos_01.tex` |
| `flos_01:thm:01-universal-anchor` | `chapters/flos_01.tex` |
| `flos_02:abstract` | `chapters/flos_02.tex` |
| `flos_02:discussion` | `chapters/flos_02.tex` |
| `flos_02:early-symbolicconnectionist-hybrids` | `chapters/flos_02.tex` |
| `flos_02:fibonacci-and-lucas-lattices-as-basis-sets` | `chapters/flos_02.tex` |
| `flos_02:gap-in-prior-art` | `chapters/flos_02.tex` |
| `flos_02:introduction` | `chapters/flos_02.tex` |
| `flos_02:logic-tensor-networks-and-differentiable-reasoning` | `chapters/flos_02.tex` |
| `flos_02:qed-assertions` | `chapters/flos_02.tex` |
| `flos_02:references` | `chapters/flos_02.tex` |
| `flos_02:representational-bottleneck-and-the-ux3c6-structural-prior` | `chapters/flos_02.tex` |
| `flos_02:results-evidence` | `chapters/flos_02.tex` |
| `flos_02:sealed-seeds` | `chapters/flos_02.tex` |
| `flos_02:sparse-and-ternary-neural-computation` | `chapters/flos_02.tex` |
| `flos_02:taxonomy-of-neuro-symbolic-paradigms` | `chapters/flos_02.tex` |
| `flos_02:the-normalisation-problem` | `chapters/flos_02.tex` |
| `flos_02:vector-symbolic-architectures` | `chapters/flos_02.tex` |
| `flos_03:abstract` | `chapters/flos_03.tex` |
| `flos_03:coq-mechanisation-and-sac-0-invariant` | `chapters/flos_03.tex` |
| `flos_03:derivation-of-the-anchor-identity` | `chapters/flos_03.tex` |
| `flos_03:discussion` | `chapters/flos_03.tex` |
| `flos_03:introduction` | `chapters/flos_03.tex` |
| `flos_03:invariant-sac-0` | `chapters/flos_03.tex` |
| `flos_03:minimal-polynomial-and-basic-consequences` | `chapters/flos_03.tex` |
| `flos_03:power-survey` | `chapters/flos_03.tex` |
| `flos_03:proof-architecture` | `chapters/flos_03.tex` |
| `flos_03:qed-assertions` | `chapters/flos_03.tex` |
| `flos_03:references` | `chapters/flos_03.tex` |
| `flos_03:relation-to-fibonacci-arithmetic` | `chapters/flos_03.tex` |
| `flos_03:results-evidence` | `chapters/flos_03.tex` |
| `flos_03:sealed-seeds` | `chapters/flos_03.tex` |
| `flos_03:the-integer-3-coincidence` | `chapters/flos_03.tex` |
| `flos_04:abstract` | `chapters/flos_04.tex` |
| `flos_04:derivation-of-the-closed-form` | `chapters/flos_04.tex` |
| `flos_04:discussion` | `chapters/flos_04.tex` |
| `flos_04:introduction` | `chapters/flos_04.tex` |
| `flos_04:multiplicative-identity-and-kernel-integration` | `chapters/flos_04.tex` |
| `flos_04:qed-assertions` | `chapters/flos_04.tex` |
| `flos_04:references` | `chapters/flos_04.tex` |
| `flos_04:results-evidence` | `chapters/flos_04.tex` |
| `flos_04:sealed-seeds` | `chapters/flos_04.tex` |
| `flos_05:ch:golden-bridge` | `chapters/flos_05.tex` |
| `flos_05:cor:05-anchor-via-genfn` | `chapters/flos_05.tex` |
| `flos_05:cor:05-matrix-cassini` | `chapters/flos_05.tex` |
| `flos_05:def:05-egf` | `chapters/flos_05.tex` |
| `flos_05:def:05-fib` | `chapters/flos_05.tex` |
| `flos_05:def:05-hankel` | `chapters/flos_05.tex` |
| `flos_05:def:05-luc` | `chapters/flos_05.tex` |
| `flos_05:lem:05-D-fib` | `chapters/flos_05.tex` |
| `flos_05:lem:05-D-luc` | `chapters/flos_05.tex` |
| `flos_05:lem:05-coupling-check` | `chapters/flos_05.tex` |
| `flos_05:lem:05-degree` | `chapters/flos_05.tex` |
| `flos_05:lem:05-degree-Q-phi` | `chapters/flos_05.tex` |
| `flos_05:lem:05-fib-egf` | `chapters/flos_05.tex` |
| `flos_05:lem:05-fib-hankel` | `chapters/flos_05.tex` |
| `flos_05:lem:05-fib-vals` | `chapters/flos_05.tex` |
| `flos_05:lem:05-luc-egf` | `chapters/flos_05.tex` |
| `flos_05:lem:05-luc-vals` | `chapters/flos_05.tex` |
| `flos_05:lem:05-pole-locations` | `chapters/flos_05.tex` |
| `flos_05:lem:05-pole-residue` | `chapters/flos_05.tex` |
| `flos_05:lem:05-product-coefs` | `chapters/flos_05.tex` |
| `flos_05:lem:05-rational` | `chapters/flos_05.tex` |
| `flos_05:lem:05-riordan-small` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-A` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-AA` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-AB` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-AC` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-AD` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-AE` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-AF` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-AG` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-AH` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-B` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-C` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-D` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-E` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-G` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-H` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-I` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-J` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-L` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-M` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-N` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-O` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-P` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-Q` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-R` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-S` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-T` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-U` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-V` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-W` | `chapters/flos_05.tex` |
| `flos_05:sec:05-app-Z` | `chapters/flos_05.tex` |
| `flos_05:sec:05-closing` | `chapters/flos_05.tex` |
| `flos_05:sec:05-intro` | `chapters/flos_05.tex` |
| `flos_05:sec:05-library` | `chapters/flos_05.tex` |
| `flos_05:thm:05-bridge-to-l4` | `chapters/flos_05.tex` |
| `flos_05:thm:05-bridge-to-l6` | `chapters/flos_05.tex` |
| `flos_05:thm:05-cassini-triangle` | `chapters/flos_05.tex` |
| `flos_05:thm:05-gf-product` | `chapters/flos_05.tex` |
| `flos_06:abstract` | `chapters/flos_06.tex` |
| `flos_06:coq-encoding` | `chapters/flos_06.tex` |
| `flos_06:discussion` | `chapters/flos_06.tex` |
| `flos_06:goldenfloat-format-definitions` | `chapters/flos_06.tex` |
| `flos_06:introduction` | `chapters/flos_06.tex` |
| `flos_06:key-theorems-and-proof-sketches` | `chapters/flos_06.tex` |
| `flos_06:lucas-closure-on-gf16` | `chapters/flos_06.tex` |
| `flos_06:preliminaries` | `chapters/flos_06.tex` |
| `flos_06:qed-assertions` | `chapters/flos_06.tex` |
| `flos_06:references` | `chapters/flos_06.tex` |
| `flos_06:results-evidence` | `chapters/flos_06.tex` |
| `flos_06:sealed-seeds` | `chapters/flos_06.tex` |
| `flos_07:abstract` | `chapters/flos_07.tex` |
| `flos_07:discussion` | `chapters/flos_07.tex` |
| `flos_07:from-the-trinity-identity-to-the-golden-angle` | `chapters/flos_07.tex` |
| `flos_07:h4-root-system-e8-lattice-and-the-varphi-scaled-block-decomposition` | `chapters/flos_07.tex` |
| `flos_07:introduction` | `chapters/flos_07.tex` |
| `flos_07:qed-assertions` | `chapters/flos_07.tex` |
| `flos_07:references` | `chapters/flos_07.tex` |
| `flos_07:results-evidence` | `chapters/flos_07.tex` |
| `flos_07:sealed-seeds` | `chapters/flos_07.tex` |
| `flos_08:abstract` | `chapters/flos_08.tex` |
| `flos_08:discussion` | `chapters/flos_08.tex` |
| `flos_08:gain-admissibility` | `chapters/flos_08.tex` |
| `flos_08:hybrid-qk-gain-invariant-inv-6` | `chapters/flos_08.tex` |
| `flos_08:introduction` | `chapters/flos_08.tex` |
| `flos_08:proof-sketch-for-admit_phi_sq` | `chapters/flos_08.tex` |
| `flos_08:qed-assertions` | `chapters/flos_08.tex` |
| `flos_08:references` | `chapters/flos_08.tex` |
| `flos_08:results-evidence` | `chapters/flos_08.tex` |
| `flos_08:sealed-seeds` | `chapters/flos_08.tex` |
| `flos_08:sec:falsification:ch08` | `chapters/flos_08.tex` |
| `flos_08:tf3-and-tf9-algebraic-structure` | `chapters/flos_08.tex` |
| `flos_08:tf9-product-encoding` | `chapters/flos_08.tex` |
| `flos_08:trit-encoding` | `chapters/flos_08.tex` |
| `flos_08:ux3c6-normalisation` | `chapters/flos_08.tex` |
| `flos_09:ablation-matrix-tier-abc-m1m6` | `chapters/flos_09.tex` |
| `flos_09:abstract` | `chapters/flos_09.tex` |
| `flos_09:competitor-format-summaries` | `chapters/flos_09.tex` |
| `flos_09:discussion` | `chapters/flos_09.tex` |
| `flos_09:gf16-format-specification` | `chapters/flos_09.tex` |
| `flos_09:gf16-phi_bias60-and-the-inv-3-safe-domain` | `chapters/flos_09.tex` |
| `flos_09:introduction` | `chapters/flos_09.tex` |
| `flos_09:inv-3-nine-coq-precision-bounds` | `chapters/flos_09.tex` |
| `flos_09:qed-assertions` | `chapters/flos_09.tex` |
| `flos_09:references` | `chapters/flos_09.tex` |
| `flos_09:results-evidence` | `chapters/flos_09.tex` |
| `flos_09:sealed-seeds` | `chapters/flos_09.tex` |
| `flos_10:abstract` | `chapters/flos_10.tex` |
| `flos_10:discussion` | `chapters/flos_10.tex` |
| `flos_10:gf16-range-and-precision-formalisation` | `chapters/flos_10.tex` |
| `flos_10:introduction` | `chapters/flos_10.tex` |
| `flos_10:qed-assertions` | `chapters/flos_10.tex` |
| `flos_10:references` | `chapters/flos_10.tex` |
| `flos_10:results-evidence` | `chapters/flos_10.tex` |
| `flos_10:sealed-seeds` | `chapters/flos_10.tex` |
| `flos_10:the-pareto-frontier-and-conjecture-c1` | `chapters/flos_10.tex` |
| `flos_11:abstract` | `chapters/flos_11.tex` |
| `flos_11:discussion` | `chapters/flos_11.tex` |
| `flos_11:hypothesis-formalisation-and-registration-protocol` | `chapters/flos_11.tex` |
| `flos_11:introduction` | `chapters/flos_11.tex` |
| `flos_11:inv-7-invariant-and-coq-formalisation` | `chapters/flos_11.tex` |
| `flos_11:qed-assertions` | `chapters/flos_11.tex` |
| `flos_11:references` | `chapters/flos_11.tex` |
| `flos_11:results-evidence` | `chapters/flos_11.tex` |
| `flos_11:sealed-seeds` | `chapters/flos_11.tex` |
| `flos_12:abstract` | `chapters/flos_12.tex` |
| `flos_12:bridge-architecture-and-interface-contracts` | `chapters/flos_12.tex` |
| `flos_12:clock-domain-analysis-and-timing` | `chapters/flos_12.tex` |
| `flos_12:discussion` | `chapters/flos_12.tex` |
| `flos_12:error-handling-protocol` | `chapters/flos_12.tex` |
| `flos_12:frequency-ratios-and-the-golden-ratio` | `chapters/flos_12.tex` |
| `flos_12:introduction` | `chapters/flos_12.tex` |
| `flos_12:logical-structure` | `chapters/flos_12.tex` |
| `flos_12:power-accounting` | `chapters/flos_12.tex` |
| `flos_12:qed-assertions` | `chapters/flos_12.tex` |
| `flos_12:references` | `chapters/flos_12.tex` |
| `flos_12:results-evidence` | `chapters/flos_12.tex` |
| `flos_12:sealed-seeds` | `chapters/flos_12.tex` |
| `flos_12:signal-naming-convention` | `chapters/flos_12.tex` |
| `flos_12:throughput-budget` | `chapters/flos_12.tex` |
| `flos_13:ch:13-metatron` | `chapters/flos_13.tex` |
| `flos_13:sec:13-appD` | `chapters/flos_13.tex` |
| `flos_13:sec:13-appE` | `chapters/flos_13.tex` |
| `flos_13:sec:13-appF` | `chapters/flos_13.tex` |
| `flos_13:sec:13-appG` | `chapters/flos_13.tex` |
| `flos_13:sec:13-appI` | `chapters/flos_13.tex` |
| `flos_13:sec:13-appJ` | `chapters/flos_13.tex` |
| `flos_13:sec:13-appK` | `chapters/flos_13.tex` |
| `flos_13:sec:13-appL` | `chapters/flos_13.tex` |
| `flos_13:sec:13-appM` | `chapters/flos_13.tex` |
| `flos_13:sec:13-arch` | `chapters/flos_13.tex` |
| `flos_13:sec:13-arch-summary` | `chapters/flos_13.tex` |
| `flos_13:sec:13-cartesian-rim` | `chapters/flos_13.tex` |
| `flos_13:sec:13-conn-23` | `chapters/flos_13.tex` |
| `flos_13:sec:13-connection-to-17` | `chapters/flos_13.tex` |
| `flos_13:sec:13-coords-bookkeeping` | `chapters/flos_13.tex` |
| `flos_13:sec:13-coq-map` | `chapters/flos_13.tex` |
| `flos_13:sec:13-counting-arch` | `chapters/flos_13.tex` |
| `flos_13:sec:13-cube-vs-spiral` | `chapters/flos_13.tex` |
| `flos_13:sec:13-diagram` | `chapters/flos_13.tex` |
| `flos_13:sec:13-disc-est` | `chapters/flos_13.tex` |
| `flos_13:sec:13-disc-not` | `chapters/flos_13.tex` |
| `flos_13:sec:13-disc-open` | `chapters/flos_13.tex` |
| `flos_13:sec:13-disc-summary` | `chapters/flos_13.tex` |
| `flos_13:sec:13-discussion` | `chapters/flos_13.tex` |
| `flos_13:sec:13-edge-counts` | `chapters/flos_13.tex` |
| `flos_13:sec:13-emp-26` | `chapters/flos_13.tex` |
| `flos_13:sec:13-filt-def` | `chapters/flos_13.tex` |
| `flos_13:sec:13-filt-quotients` | `chapters/flos_13.tex` |
| `flos_13:sec:13-filt-why` | `chapters/flos_13.tex` |
| `flos_13:sec:13-five-platonic` | `chapters/flos_13.tex` |
| `flos_13:sec:13-gf16` | `chapters/flos_13.tex` |
| `flos_13:sec:13-identities` | `chapters/flos_13.tex` |
| `flos_13:sec:13-layer-distances` | `chapters/flos_13.tex` |
| `flos_13:sec:13-lucas-12-orbit` | `chapters/flos_13.tex` |
| `flos_13:sec:13-lucas-ring-coords` | `chapters/flos_13.tex` |
| `flos_13:sec:13-notation` | `chapters/flos_13.tex` |
| `flos_13:sec:13-origin` | `chapters/flos_13.tex` |
| `flos_13:sec:13-polar` | `chapters/flos_13.tex` |
| `flos_13:sec:13-projection` | `chapters/flos_13.tex` |
| `flos_13:sec:13-strand-I` | `chapters/flos_13.tex` |
| `flos_13:sec:13-strand-I-takeaway` | `chapters/flos_13.tex` |
| `flos_13:sec:13-strand-II` | `chapters/flos_13.tex` |
| `flos_13:sec:13-strand-II-wrap` | `chapters/flos_13.tex` |
| `flos_13:sec:13-strand-III` | `chapters/flos_13.tex` |
| `flos_13:sec:13-strand-III-wrap` | `chapters/flos_13.tex` |
| `flos_13:sec:13-thirteen` | `chapters/flos_13.tex` |
| `flos_13:sec:13-three-cubes` | `chapters/flos_13.tex` |
| `flos_13:sec:13-three-strands` | `chapters/flos_13.tex` |
| `flos_13:sec:13-trinity-plane` | `chapters/flos_13.tex` |
| `flos_14:abstract` | `chapters/flos_14.tex` |
| `flos_14:bpb-definition-and-algebraic-properties` | `chapters/flos_14.tex` |
| `flos_14:byte-level-normalisation` | `chapters/flos_14.tex` |
| `flos_14:cross-entropy-and-perplexity` | `chapters/flos_14.tex` |
| `flos_14:discussion` | `chapters/flos_14.tex` |
| `flos_14:gate-2-bpb-1.85` | `chapters/flos_14.tex` |
| `flos_14:gate-3-bpb-1.50` | `chapters/flos_14.tex` |
| `flos_14:gate-thresholds-and-their-derivation` | `chapters/flos_14.tex` |
| `flos_14:introduction` | `chapters/flos_14.tex` |
| `flos_14:qed-assertions` | `chapters/flos_14.tex` |
| `flos_14:references` | `chapters/flos_14.tex` |
| `flos_14:relationship-to-the-darpa-energy-goal` | `chapters/flos_14.tex` |
| `flos_14:results-evidence` | `chapters/flos_14.tex` |
| `flos_14:sealed-seeds` | `chapters/flos_14.tex` |
| `flos_14:ux3c6-weighted-bpb` | `chapters/flos_14.tex` |
| `flos_15:abstract` | `chapters/flos_15.tex` |
| `flos_15:bpb-protocol-and-monotone-backward-invariant-inv-1` | `chapters/flos_15.tex` |
| `flos_15:database-schema` | `chapters/flos_15.tex` |
| `flos_15:discussion` | `chapters/flos_15.tex` |
| `flos_15:evaluation-protocol` | `chapters/flos_15.tex` |
| `flos_15:gate-evaluation` | `chapters/flos_15.tex` |
| `flos_15:introduction` | `chapters/flos_15.tex` |
| `flos_15:inv-1-bpb-monotone-backward` | `chapters/flos_15.tex` |
| `flos_15:qed-assertions` | `chapters/flos_15.tex` |
| `flos_15:railway-write-back-architecture` | `chapters/flos_15.tex` |
| `flos_15:references` | `chapters/flos_15.tex` |
| `flos_15:results-evidence` | `chapters/flos_15.tex` |
| `flos_15:sealed-seeds` | `chapters/flos_15.tex` |
| `flos_15:warmup-gate` | `chapters/flos_15.tex` |
| `flos_15:write-back-protocol` | `chapters/flos_15.tex` |
| `flos_16:abstract` | `chapters/flos_16.tex` |
| `flos_16:discussion` | `chapters/flos_16.tex` |
| `flos_16:grid-construction-and-sparsity-analysis` | `chapters/flos_16.tex` |
| `flos_16:introduction` | `chapters/flos_16.tex` |
| `flos_16:qed-assertions` | `chapters/flos_16.tex` |
| `flos_16:references` | `chapters/flos_16.tex` |
| `flos_16:results-evidence` | `chapters/flos_16.tex` |
| `flos_16:sealed-seeds` | `chapters/flos_16.tex` |
| `flos_16:the-phi-distance-function` | `chapters/flos_16.tex` |
| `flos_17:abstract` | `chapters/flos_17.tex` |
| `flos_17:analysis-of-effects-and-golden-ratio-structure` | `chapters/flos_17.tex` |
| `flos_17:discussion` | `chapters/flos_17.tex` |
| `flos_17:factor-definitions-and-experimental-design` | `chapters/flos_17.tex` |
| `flos_17:introduction` | `chapters/flos_17.tex` |
| `flos_17:qed-assertions` | `chapters/flos_17.tex` |
| `flos_17:references` | `chapters/flos_17.tex` |
| `flos_17:results-evidence` | `chapters/flos_17.tex` |
| `flos_17:sealed-seeds` | `chapters/flos_17.tex` |
| `flos_18:abstract` | `chapters/flos_18.tex` |
| `flos_18:coq.interval-upgrade-lane` | `chapters/flos_18.tex` |
| `flos_18:discussion` | `chapters/flos_18.tex` |
| `flos_18:hardware-and-runtime-limitations` | `chapters/flos_18.tex` |
| `flos_18:introduction` | `chapters/flos_18.tex` |
| `flos_18:qed-assertions` | `chapters/flos_18.tex` |
| `flos_18:references` | `chapters/flos_18.tex` |
| `flos_18:sealed-seeds` | `chapters/flos_18.tex` |
| `flos_18:sec:falsification:ch18` | `chapters/flos_18.tex` |
| `flos_18:state-of-the-art-comparison-clara-soa-snapshot` | `chapters/flos_18.tex` |
| `flos_19:abstract` | `chapters/flos_19.tex` |
| `flos_19:discussion` | `chapters/flos_19.tex` |
| `flos_19:introduction` | `chapters/flos_19.tex` |
| `flos_19:qed-assertions` | `chapters/flos_19.tex` |
| `flos_19:references` | `chapters/flos_19.tex` |
| `flos_19:results-evidence` | `chapters/flos_19.tex` |
| `flos_19:sealed-seeds` | `chapters/flos_19.tex` |
| `flos_19:test-design-and-hypotheses` | `chapters/flos_19.tex` |
| `flos_19:welch-t-statistic-and-degrees-of-freedom` | `chapters/flos_19.tex` |
| `flos_20:ch:20` | `chapters/flos_20.tex` |
| `flos_20:def:alpha` | `chapters/flos_20.tex` |
| `flos_20:def:ckm` | `chapters/flos_20.tex` |
| `flos_20:def:gauge-boson` | `chapters/flos_20.tex` |
| `flos_20:def:higgs` | `chapters/flos_20.tex` |
| `flos_20:def:koide` | `chapters/flos_20.tex` |
| `flos_20:def:lepton` | `chapters/flos_20.tex` |
| `flos_20:def:pmns` | `chapters/flos_20.tex` |
| `flos_20:def:quark` | `chapters/flos_20.tex` |
| `flos_20:def:su2` | `chapters/flos_20.tex` |
| `flos_20:def:su3` | `chapters/flos_20.tex` |
| `flos_20:def:u1` | `chapters/flos_20.tex` |
| `flos_20:prop:ckm-golden` | `chapters/flos_20.tex` |
| `flos_20:prop:golden-alpha` | `chapters/flos_20.tex` |
| `flos_20:prop:golden-koide` | `chapters/flos_20.tex` |
| `flos_20:prop:higgs-mass` | `chapters/flos_20.tex` |
| `flos_20:prop:pmns-golden` | `chapters/flos_20.tex` |
| `flos_20:prop:su3-dim` | `chapters/flos_20.tex` |
| `flos_20:prop:u1-charge` | `chapters/flos_20.tex` |
| `flos_20:sec:20-falsify` | `chapters/flos_20.tex` |
| `flos_20:thm:pauli` | `chapters/flos_20.tex` |
| `flos_20:thm:sm-symmetry` | `chapters/flos_20.tex` |
| `flos_20:thm:strong-golden` | `chapters/flos_20.tex` |
| `flos_20:thm:weak-golden` | `chapters/flos_20.tex` |
| `flos_21:ch:21` | `chapters/flos_21.tex` |
| `flos_21:def:dim-reg` | `chapters/flos_21.tex` |
| `flos_21:def:eft` | `chapters/flos_21.tex` |
| `flos_21:def:field-ops` | `chapters/flos_21.tex` |
| `flos_21:def:fock` | `chapters/flos_21.tex` |
| `flos_21:def:higgs-pot` | `chapters/flos_21.tex` |
| `flos_21:def:kg` | `chapters/flos_21.tex` |
| `flos_21:def:lagrangian` | `chapters/flos_21.tex` |
| `flos_21:def:path-integral` | `chapters/flos_21.tex` |
| `flos_21:def:qed` | `chapters/flos_21.tex` |
| `flos_21:def:yang-mills` | `chapters/flos_21.tex` |
| `flos_21:prop:beta-golden` | `chapters/flos_21.tex` |
| `flos_21:prop:feynman` | `chapters/flos_21.tex` |
| `flos_21:prop:goldstone` | `chapters/flos_21.tex` |
| `flos_21:prop:kg-eq` | `chapters/flos_21.tex` |
| `flos_21:prop:non-abelian` | `chapters/flos_21.tex` |
| `flos_21:sec:21-falsify` | `chapters/flos_21.tex` |
| `flos_21:thm:mode-expansion` | `chapters/flos_21.tex` |
| `flos_21:thm:n-point` | `chapters/flos_21.tex` |
| `flos_21:thm:rg` | `chapters/flos_21.tex` |
| `flos_21:thm:ssb` | `chapters/flos_21.tex` |
| `flos_21:thm:weinberg` | `chapters/flos_21.tex` |
| `flos_22:abstract` | `chapters/flos_22.tex` |
| `flos_22:discussion` | `chapters/flos_22.tex` |
| `flos_22:introduction` | `chapters/flos_22.tex` |
| `flos_22:qed-assertions` | `chapters/flos_22.tex` |
| `flos_22:references` | `chapters/flos_22.tex` |
| `flos_22:results-evidence` | `chapters/flos_22.tex` |
| `flos_22:satisfaction-witness-and-victory-predicate` | `chapters/flos_22.tex` |
| `flos_22:sealed-seeds` | `chapters/flos_22.tex` |
| `flos_22:worker-pool-invariants-and-falsification-witnesses` | `chapters/flos_22.tex` |
| `flos_23:abstract` | `chapters/flos_23.tex` |
| `flos_23:discussion` | `chapters/flos_23.tex` |
| `flos_23:introduction` | `chapters/flos_23.tex` |
| `flos_23:mcp-adapter-layer-architecture` | `chapters/flos_23.tex` |
| `flos_23:protocol-implementation-and-latency-analysis` | `chapters/flos_23.tex` |
| `flos_23:qed-assertions` | `chapters/flos_23.tex` |
| `flos_23:references` | `chapters/flos_23.tex` |
| `flos_23:results-evidence` | `chapters/flos_23.tex` |
| `flos_23:sealed-seeds` | `chapters/flos_23.tex` |
| `flos_24:abstract` | `chapters/flos_24.tex` |
| `flos_24:agent-model` | `chapters/flos_24.tex` |
| `flos_24:coq-encoding` | `chapters/flos_24.tex` |
| `flos_24:discussion` | `chapters/flos_24.tex` |
| `flos_24:formal-model-of-the-period-locked-monitor` | `chapters/flos_24.tex` |
| `flos_24:implementation-and-hardware-interface` | `chapters/flos_24.tex` |
| `flos_24:interrupt-interface-with-the-hardware-bridge` | `chapters/flos_24.tex` |
| `flos_24:introduction` | `chapters/flos_24.tex` |
| `flos_24:period-ratio-and-non-resonance` | `chapters/flos_24.tex` |
| `flos_24:priority-queue-and-phi-weighted-scheduling` | `chapters/flos_24.tex` |
| `flos_24:qed-assertions` | `chapters/flos_24.tex` |
| `flos_24:references` | `chapters/flos_24.tex` |
| `flos_24:results-evidence` | `chapters/flos_24.tex` |
| `flos_24:rtl-implementation` | `chapters/flos_24.tex` |
| `flos_24:sealed-seeds` | `chapters/flos_24.tex` |
| `flos_24:sec:falsification:ch24` | `chapters/flos_24.tex` |
| `flos_25:abstract` | `chapters/flos_25.tex` |
| `flos_25:cycle-classification-and-attention-periodicity` | `chapters/flos_25.tex` |
| `flos_25:discussion` | `chapters/flos_25.tex` |
| `flos_25:introduction` | `chapters/flos_25.tex` |
| `flos_25:qed-assertions` | `chapters/flos_25.tex` |
| `flos_25:references` | `chapters/flos_25.tex` |
| `flos_25:results-evidence` | `chapters/flos_25.tex` |
| `flos_25:sealed-seeds` | `chapters/flos_25.tex` |
| `flos_25:sec:falsification:ch25` | `chapters/flos_25.tex` |
| `flos_25:varphi-lattice-structure-and-the-cycle-map` | `chapters/flos_25.tex` |
| `flos_26:abstract` | `chapters/flos_26.tex` |
| `flos_26:discussion` | `chapters/flos_26.tex` |
| `flos_26:gf16_quant-galois-field-16-quantisation` | `chapters/flos_26.tex` |
| `flos_26:instruction-encoding` | `chapters/flos_26.tex` |
| `flos_26:introduction` | `chapters/flos_26.tex` |
| `flos_26:isa-register-file-and-encoding` | `chapters/flos_26.tex` |
| `flos_26:opcode-specifications` | `chapters/flos_26.tex` |
| `flos_26:phi_rope-ux3c6-rotary-position-encoding` | `chapters/flos_26.tex` |
| `flos_26:qed-assertions` | `chapters/flos_26.tex` |
| `flos_26:references` | `chapters/flos_26.tex` |
| `flos_26:register-file` | `chapters/flos_26.tex` |
| `flos_26:results-evidence` | `chapters/flos_26.tex` |
| `flos_26:sealed-seeds` | `chapters/flos_26.tex` |
| `flos_26:tf3_add-ternary-addition` | `chapters/flos_26.tex` |
| `flos_26:tf3_mul-ternary-multiplication` | `chapters/flos_26.tex` |
| `flos_26:vsa_bind-hyperdimensional-binding` | `chapters/flos_26.tex` |
| `flos_26:vsa_bundle-hyperdimensional-bundling` | `chapters/flos_26.tex` |
| `flos_26:vsa_unbind-hyperdimensional-unbinding` | `chapters/flos_26.tex` |
| `flos_27:abstract` | `chapters/flos_27.tex` |
| `flos_27:abstract-syntax` | `chapters/flos_27.tex` |
| `flos_27:discussion` | `chapters/flos_27.tex` |
| `flos_27:environments-and-evaluation` | `chapters/flos_27.tex` |
| `flos_27:introduction` | `chapters/flos_27.tex` |
| `flos_27:mechanised-proofs-determinism-and-exhaustiveness` | `chapters/flos_27.tex` |
| `flos_27:qed-assertions` | `chapters/flos_27.tex` |
| `flos_27:references` | `chapters/flos_27.tex` |
| `flos_27:relation-to-gf16-and-varphi-arithmetic` | `chapters/flos_27.tex` |
| `flos_27:results-evidence` | `chapters/flos_27.tex` |
| `flos_27:sealed-seeds` | `chapters/flos_27.tex` |
| `flos_27:ternary-arithmetic` | `chapters/flos_27.tex` |
| `flos_27:theorem-eval_det-determinism` | `chapters/flos_27.tex` |
| `flos_27:theorem-trit_exhaustive-exhaustiveness` | `chapters/flos_27.tex` |
| `flos_27:tri27-syntax-and-denotational-semantics` | `chapters/flos_27.tex` |
| `flos_28:abstract` | `chapters/flos_28.tex` |
| `flos_28:architecture-zero-dsp-ternary-datapath` | `chapters/flos_28.tex` |
| `flos_28:discussion` | `chapters/flos_28.tex` |
| `flos_28:introduction` | `chapters/flos_28.tex` |
| `flos_28:qed-assertions` | `chapters/flos_28.tex` |
| `flos_28:references` | `chapters/flos_28.tex` |
| `flos_28:resource-utilisation-and-timing-closure` | `chapters/flos_28.tex` |
| `flos_28:results-evidence` | `chapters/flos_28.tex` |
| `flos_28:sealed-seeds` | `chapters/flos_28.tex` |
| `flos_29:ch:29` | `chapters/flos_29.tex` |
| `flos_29:def:lucas` | `chapters/flos_29.tex` |
| `flos_29:def:lucas-primes` | `chapters/flos_29.tex` |
| `flos_29:def:lucas-spiral` | `chapters/flos_29.tex` |
| `flos_29:def:lucas-tiling` | `chapters/flos_29.tex` |
| `flos_29:def:lucas-trinity` | `chapters/flos_29.tex` |
| `flos_29:prop:golden-lucas-mixing` | `chapters/flos_29.tex` |
| `flos_29:prop:lucas-golden` | `chapters/flos_29.tex` |
| `flos_29:prop:lucas-mod` | `chapters/flos_29.tex` |
| `flos_29:prop:lucas-tiling` | `chapters/flos_29.tex` |
| `flos_29:sec:29-falsify` | `chapters/flos_29.tex` |
| `flos_29:thm:cassini` | `chapters/flos_29.tex` |
| `flos_29:thm:lucas-div` | `chapters/flos_29.tex` |
| `flos_29:thm:lucas-fibo` | `chapters/flos_29.tex` |
| `flos_29:thm:lucas-prime-density` | `chapters/flos_29.tex` |
| `flos_29:thm:lucas-spiral` | `chapters/flos_29.tex` |
| `flos_29:thm:neutrino-lucas` | `chapters/flos_29.tex` |
| `flos_29:thm:product` | `chapters/flos_29.tex` |
| `flos_30:abstract` | `chapters/flos_30.tex` |
| `flos_30:associative-recall-memory` | `chapters/flos_30.tex` |
| `flos_30:discussion` | `chapters/flos_30.tex` |
| `flos_30:goldenfloat-encoding-of-hypervectors` | `chapters/flos_30.tex` |
| `flos_30:hypervector-definition` | `chapters/flos_30.tex` |
| `flos_30:introduction` | `chapters/flos_30.tex` |
| `flos_30:phi-rotary-position-encoding-phi-rope-in-vsa-context` | `chapters/flos_30.tex` |
| `flos_30:qed-assertions` | `chapters/flos_30.tex` |
| `flos_30:references` | `chapters/flos_30.tex` |
| `flos_30:results-evidence` | `chapters/flos_30.tex` |
| `flos_30:sealed-seeds` | `chapters/flos_30.tex` |
| `flos_30:ternary-vsa-over-the-goldenfloat-substrate` | `chapters/flos_30.tex` |
| `flos_31:ch:31` | `chapters/flos_31.tex` |
| `flos_31:def:antirealism` | `chapters/flos_31.tex` |
| `flos_31:def:apriori` | `chapters/flos_31.tex` |
| `flos_31:def:beauty` | `chapters/flos_31.tex` |
| `flos_31:def:constants` | `chapters/flos_31.tex` |
| `flos_31:def:empiricism` | `chapters/flos_31.tex` |
| `flos_31:def:muh` | `chapters/flos_31.tex` |
| `flos_31:def:platonism` | `chapters/flos_31.tex` |
| `flos_31:def:pythagorean` | `chapters/flos_31.tex` |
| `flos_31:def:realism` | `chapters/flos_31.tex` |
| `flos_31:def:structuralism` | `chapters/flos_31.tex` |
| `flos_31:prop:empirical-golden` | `chapters/flos_31.tex` |
| `flos_31:prop:golden-anthropic` | `chapters/flos_31.tex` |
| `flos_31:prop:golden-cat` | `chapters/flos_31.tex` |
| `flos_31:prop:golden-effective` | `chapters/flos_31.tex` |
| `flos_31:prop:golden-muh` | `chapters/flos_31.tex` |
| `flos_31:prop:golden-platonism` | `chapters/flos_31.tex` |
| `flos_31:prop:instrumental-golden` | `chapters/flos_31.tex` |
| `flos_31:thm:apriori-golden` | `chapters/flos_31.tex` |
| `flos_31:thm:golden-beauty` | `chapters/flos_31.tex` |
| `flos_31:thm:golden-constants` | `chapters/flos_31.tex` |
| `flos_31:thm:golden-struct` | `chapters/flos_31.tex` |
| `flos_31:thm:platonic-golden` | `chapters/flos_31.tex` |
| `flos_31:thm:pythagorean-golden` | `chapters/flos_31.tex` |
| `flos_31:thm:realist-golden` | `chapters/flos_31.tex` |
| `flos_32:prop:golden-opt` | `chapters/flos_32.tex` |
| `flos_32:thm:alpha-summary` | `chapters/flos_32.tex` |
| `flos_32:thm:e8-summary` | `chapters/flos_32.tex` |
| `flos_32:thm:golden-entropy` | `chapters/flos_32.tex` |
| `flos_32:thm:golden-unif` | `chapters/flos_32.tex` |
| `flos_32:thm:trinity-summary` | `chapters/flos_32.tex` |
| `flos_33:abstract` | `chapters/flos_33.tex` |
| `flos_33:diagnosis-and-root-cause` | `chapters/flos_33.tex` |
| `flos_33:discussion` | `chapters/flos_33.tex` |
| `flos_33:flash_no_sudo.sh` | `chapters/flos_33.tex` |
| `flos_33:fxload-cross-compilation` | `chapters/flos_33.tex` |
| `flos_33:introduction` | `chapters/flos_33.tex` |
| `flos_33:qed-assertions` | `chapters/flos_33.tex` |
| `flos_33:references` | `chapters/flos_33.tex` |
| `flos_33:results-evidence` | `chapters/flos_33.tex` |
| `flos_33:sealed-seeds` | `chapters/flos_33.tex` |
| `flos_33:usb-enumeration-on-macos-arm` | `chapters/flos_33.tex` |
| `flos_33:verified-hardware-configuration-post-blk-001` | `chapters/flos_33.tex` |
| `fig:<slug>-<n>` | `frontmatter/list-of-figures.tex` |
| `lem:01-best-rational` | `chapters/flos_01.tex` |
| `lem:01-cf-rec` | `chapters/flos_01.tex` |
| `lem:05-coef-limit` | `chapters/flos_05.tex` |
| `lem:05-luc-hankel` | `chapters/flos_05.tex` |
| `lem:05-matrix-power` | `chapters/flos_05.tex` |
| `lem:13-galois` | `chapters/flos_13.tex` |
| `lem:13-gf16-floor` | `chapters/flos_13.tex` |
| `lem:13-primary` | `chapters/flos_13.tex` |
| `lem:13-secondary` | `chapters/flos_13.tex` |
| `lem:13-tertiary` | `chapters/flos_13.tex` |
| `lem:13-trinity` | `chapters/flos_13.tex` |
| `sec:05-anchor-coeff` | `chapters/flos_05.tex` |
| `sec:05-app-F` | `chapters/flos_05.tex` |
| `sec:05-app-K` | `chapters/flos_05.tex` |
| `sec:05-app-X` | `chapters/flos_05.tex` |
| `sec:05-app-Y` | `chapters/flos_05.tex` |
| `sec:05-closed-form` | `chapters/flos_05.tex` |
| `sec:05-coupling` | `chapters/flos_05.tex` |
| `sec:05-falsification` | `chapters/flos_05.tex` |
| `sec:05-partial-frac` | `chapters/flos_05.tex` |
| `sec:05-prelim` | `chapters/flos_05.tex` |
| `sec:05-radius` | `chapters/flos_05.tex` |
| `sec:05-strand-i` | `chapters/flos_05.tex` |
| `sec:05-strand-ii` | `chapters/flos_05.tex` |
| `sec:05-strand-iii` | `chapters/flos_05.tex` |
| `sec:13-appA` | `chapters/flos_13.tex` |
| `sec:13-appB` | `chapters/flos_13.tex` |
| `sec:13-appC` | `chapters/flos_13.tex` |
| `sec:13-appH` | `chapters/flos_13.tex` |
| `sec:13-arch-scaffold` | `chapters/flos_13.tex` |
| `sec:13-filt-coq` | `chapters/flos_13.tex` |
| `sec:13-filtration` | `chapters/flos_13.tex` |
| `sec:13-seventy-eight` | `chapters/flos_13.tex` |
| `sec:13-symmetry-group` | `chapters/flos_13.tex` |
| `sec:13-trinity-bookkeeping` | `chapters/flos_13.tex` |
| `sec:ckm` | `chapters/flos_20.tex` |
| `sec:mass` | `chapters/flos_20.tex` |
| `sec:mesh-roadmap` | `chapters/flos_69.tex` |
| `sec:xvc-bridge` | `appendix/F-fpga-bitstream.tex` |
| `tab:<slug>-<n>` | `frontmatter/list-of-tables.tex` |
| `tab:ch0-fits` | `chapters/flos_34.tex` |
| `tab:power` | `chapters/flos_69.tex` |
| `thm:01-anchor` | `chapters/flos_01.tex` |
| `thm:01-convergent-fib` | `chapters/flos_01.tex` |
| `thm:01-fixed` | `chapters/flos_01.tex` |
| `thm:01-pentagon` | `chapters/flos_01.tex` |
| `thm:01-pentagon-alg` | `chapters/flos_01.tex` |
| `thm:01-quadratic` | `chapters/flos_01.tex` |
| `thm:01-vesica-lens` | `chapters/flos_01.tex` |
| `thm:05-anchor-as-coeff` | `chapters/flos_05.tex` |
| `thm:05-asymptotic` | `chapters/flos_05.tex` |
| `thm:05-bridge` | `chapters/flos_05.tex` |
| `thm:05-cassini-fib` | `chapters/flos_05.tex` |
| `thm:05-cassini-luc` | `chapters/flos_05.tex` |
| `thm:05-coupling` | `chapters/flos_05.tex` |
| `thm:05-fl-conv` | `chapters/flos_05.tex` |
| `thm:05-genfn-closed` | `chapters/flos_05.tex` |
| `thm:05-partial-frac` | `chapters/flos_05.tex` |
| `thm:05-radius` | `chapters/flos_05.tex` |
| `thm:13-projection` | `chapters/flos_13.tex` |
| `thm:13-total-edges` | `chapters/flos_13.tex` |
| `thm:D:1` | `appendix/D-golden-mirror.tex` |
| `thm:ch1-trinity-identity` | `chapters/flos_35.tex` |
| `thm:ch3-trinity-canonical` | `chapters/flos_37.tex` |
| `thm:euler-lagrange` | `chapters/flos_21.tex` |
| `thm:lucas-binet` | `chapters/flos_29.tex` |
| `thm:lucas-trinity` | `chapters/flos_29.tex` |

</details>

## Referenced keys (119)

These keys are consumed by at least one `\ref`/`\autoref`/`\eqref`/`\Cref`/`\pageref` and were preserved in their bare form:

<details><summary>Click to expand</summary>

| Key | Defined in | Referenced from |
|---|---|---|
| `ch:1` | `chapters/flos_01.tex` | `chapters/flos_34.tex` |
| `ch:11` | `chapters/flos_11.tex` | `chapters/flos_34.tex` |
| `ch:13` | `chapters/flos_13.tex` | `appendix/B-falsification.tex`, `appendix/J-troubleshooting.tex` |
| `ch:15` | `chapters/flos_15.tex` | `appendix/B-falsification.tex`, `appendix/G-data-availability.tex` |
| `ch:17-spiral` | `chapters/flos_17.tex` | `chapters/flos_13.tex` |
| `ch:18` | `chapters/flos_18.tex` | `appendix/B-falsification.tex`, `appendix/G-data-availability.tex` |
| `ch:19` | `chapters/flos_19.tex` | `chapters/flos_34.tex` |
| `ch:21-experiments-jepa` | `chapters/flos_21.tex` | `chapters/flos_13.tex` |
| `ch:23-gf16-algebra` | `chapters/flos_23.tex` | `chapters/flos_13.tex` |
| `ch:24` | `chapters/flos_24.tex` | `appendix/B-falsification.tex` |
| `ch:24-igla-arch` | `chapters/flos_24.tex` | `chapters/flos_13.tex` |
| `ch:25` | `chapters/flos_25.tex` | `appendix/B-falsification.tex` |
| `ch:25-benchmarks` | `chapters/flos_25.tex` | `chapters/flos_13.tex` |
| `ch:26-data-analysis` | `chapters/flos_26.tex` | `chapters/flos_13.tex` |
| `ch:28` | `chapters/flos_28.tex` | `appendix/F-fpga-bitstream.tex` |
| `ch:28-momentum-algebra` | `chapters/flos_28.tex` | `chapters/flos_13.tex` |
| `ch:32` | `chapters/flos_32.tex` | `appendix/I-xdc-pin-map.tex` |
| `ch:33` | `chapters/flos_33.tex` | `appendix/F-fpga-bitstream.tex` |
| `ch:34` | `chapters/flos_33.tex` | `appendix/B-falsification.tex`, `appendix/F-fpga-bitstream.tex` |
| `ch:6` | `chapters/flos_06.tex` | `appendix/C-golden-benchmark.tex` |
| `ch:9` | `chapters/flos_09.tex` | `appendix/B-falsification.tex`, `appendix/C-golden-benchmark.tex` |
| `ch:benchmarks` | `chapters/flos_25.tex` | `chapters/flos_00.tex` |
| `ch:data-analysis` | `chapters/flos_26.tex` | `chapters/flos_00.tex` |
| `ch:e8-symmetry` | `chapters/flos_22.tex` | `chapters/flos_00.tex` |
| `ch:energy` | `chapters/flos_28.tex` | `frontmatter/notation.tex` |
| `ch:experiments-asha` | `chapters/flos_21.tex` | `frontmatter/notation.tex` |
| `ch:experiments-bpb` | `chapters/flos_21.tex` | `frontmatter/notation.tex` |
| `ch:experiments-gf16` | `chapters/flos_23.tex` | `frontmatter/notation.tex` |
| `ch:fibonacci` | `chapters/flos_07.tex` | `frontmatter/notation.tex` |
| `ch:fibonacci-tesselation` | `chapters/flos_07.tex` | `chapters/flos_00.tex` |
| `ch:gf16-algebra` | `chapters/flos_23.tex` | `chapters/flos_00.tex` |
| `ch:golden-egg` | `chapters/flos_01.tex` | `frontmatter/notation.tex` |
| `ch:golden-seed` | `chapters/flos_01.tex` | `frontmatter/notation.tex` |
| `ch:igla-architecture` | `chapters/flos_24.tex` | `chapters/flos_00.tex` |
| `ch:igla-race` | `chapters/flos_24.tex` | `frontmatter/notation.tex` |
| `ch:jepa` | `chapters/flos_21.tex` | `frontmatter/notation.tex` |
| `ch:lucas-closure` | `chapters/flos_29.tex` | `chapters/flos_00.tex` |
| `ch:lucas-ladder` | `chapters/flos_29.tex` | `chapters/flos_05.tex` |
| `ch:lucas-ring` | `chapters/flos_27.tex` | `chapters/flos_00.tex`, `chapters/flos_05.tex`, `frontmatter/notation.tex` |
| `ch:monad` | `chapters/flos_00.tex` | `chapters/flos_00.tex` |
| `ch:nca` | `chapters/flos_29.tex` | `frontmatter/notation.tex` |
| `ch:plm` | `chapters/flos_24.tex` | `appendix/F-fpga-bitstream.tex` |
| `ch:standard-model` | `chapters/flos_20.tex` | `chapters/flos_00.tex` |
| `ch:three-strands` | `chapters/flos_27.tex` | `chapters/flos_00.tex`, `frontmatter/notation.tex` |
| `ch:trinity-identity` | `chapters/flos_27.tex` | `chapters/flos_00.tex` |
| `ch:vesica-piscis` | `chapters/flos_11.tex` | `chapters/flos_00.tex` |
| `ch:vsa` | `chapters/flos_29.tex` | `frontmatter/notation.tex` |
| `cor:01-l2-three` | `chapters/flos_01.tex` | `chapters/flos_01.tex` |
| `cor:01-lucas-as-trace` | `chapters/flos_01.tex` | `chapters/flos_01.tex` |
| `cor:01-reciprocal` | `chapters/flos_01.tex` | `chapters/flos_01.tex` |
| `cor:05-asymptotic-rate` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `cor:05-binet` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `def:13-lucas-12` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `eq:ch0-fit` | `chapters/flos_34.tex` | `chapters/flos_34.tex` |
| `lem:01-best-rational` | `chapters/flos_01.tex` | `chapters/flos_01.tex` |
| `lem:01-cf-rec` | `chapters/flos_01.tex` | `chapters/flos_01.tex` |
| `lem:05-coef-limit` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `lem:05-luc-hankel` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `lem:05-matrix-power` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `lem:13-galois` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `lem:13-gf16-floor` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `lem:13-primary` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `lem:13-secondary` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `lem:13-tertiary` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `lem:13-trinity` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `sec:05-anchor-coeff` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:05-app-F` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:05-app-K` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:05-app-X` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:05-app-Y` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:05-closed-form` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:05-coupling` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:05-falsification` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:05-partial-frac` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:05-prelim` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:05-radius` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:05-strand-i` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:05-strand-ii` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:05-strand-iii` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `sec:13-appA` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `sec:13-appB` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `sec:13-appC` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `sec:13-appH` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `sec:13-arch-scaffold` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `sec:13-filt-coq` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `sec:13-filtration` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `sec:13-seventy-eight` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `sec:13-symmetry-group` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `sec:13-trinity-bookkeeping` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `sec:ckm` | `chapters/flos_20.tex` | `chapters/flos_20.tex` |
| `sec:mass` | `chapters/flos_20.tex` | `chapters/flos_20.tex` |
| `sec:mesh-roadmap` | `chapters/flos_69.tex` | `chapters/flos_69.tex` |
| `sec:xvc-bridge` | `appendix/F-fpga-bitstream.tex` | `appendix/F-fpga-bitstream.tex` |
| `tab:ch0-fits` | `chapters/flos_34.tex` | `chapters/flos_34.tex` |
| `tab:power` | `chapters/flos_69.tex` | `chapters/flos_69.tex` |
| `thm:01-anchor` | `chapters/flos_01.tex` | `chapters/flos_01.tex` |
| `thm:01-convergent-fib` | `chapters/flos_01.tex` | `chapters/flos_01.tex` |
| `thm:01-fixed` | `chapters/flos_01.tex` | `chapters/flos_01.tex` |
| `thm:01-pentagon` | `chapters/flos_01.tex` | `chapters/flos_01.tex` |
| `thm:01-pentagon-alg` | `chapters/flos_01.tex` | `chapters/flos_01.tex` |
| `thm:01-quadratic` | `chapters/flos_01.tex` | `chapters/flos_01.tex` |
| `thm:01-vesica-lens` | `chapters/flos_01.tex` | `chapters/flos_01.tex` |
| `thm:05-anchor-as-coeff` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `thm:05-asymptotic` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `thm:05-bridge` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `thm:05-cassini-fib` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `thm:05-cassini-luc` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `thm:05-coupling` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `thm:05-fl-conv` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `thm:05-genfn-closed` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `thm:05-partial-frac` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `thm:05-radius` | `chapters/flos_05.tex` | `chapters/flos_05.tex` |
| `thm:13-projection` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `thm:13-total-edges` | `chapters/flos_13.tex` | `chapters/flos_13.tex` |
| `thm:ch1-trinity-identity` | `chapters/flos_35.tex` | `chapters/flos_35.tex` |
| `thm:ch3-trinity-canonical` | `chapters/flos_37.tex` | `chapters/flos_37.tex` |
| `thm:euler-lagrange` | `chapters/flos_21.tex` | `chapters/flos_21.tex` |
| `thm:lucas-binet` | `chapters/flos_29.tex` | `chapters/flos_05.tex` |
| `thm:lucas-trinity` | `chapters/flos_29.tex` | `chapters/flos_29.tex` |

</details>

## Skill provenance

Authored under skills `phd-chapter-author v1.1` + `phd-monograph-auditor v1.2`.
Per R5 (honesty): all renames are mechanical, none flip Admitted↔Proven; no `.py`/`.sh` were committed (R1).
