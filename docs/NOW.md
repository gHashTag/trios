# NOW.md — Active Lane Status

> Append-only lane status tracker. Agents add their section when landing a PR. Never edit existing entries.

---

## Lane T'' — TENET Assertion Mirror · L-DPC26 Wave-29

**Agent:** Lane T'' (assertion agent)
**Date:** 2026-08-15
**Mission:** L-DPC26 · Wave-29 ONE SHOT (trios#845)
**Branch:** `feat/lane-t-double-prime-tenet-assertion`
**Tracking Issue:** trios#847

### Deliverable

Created `assertions/wave29_tenet_sparsity.json` — JSON-schema draft-07 predicate file containing pre-registered R7 falsifier **W-102-A** for TENET sparsity-aware LUT skip controller.

### Key Facts

| Field | Value |
|-------|-------|
| Opcode | `OP_SPARSE_SKIP=0xE1` |
| Predicate | W-102-A |
| Freeze date | 2026-08-15 |
| Verdict date | 2026-10-15 |
| Coq mirror | `gHashTag/t27:coq/IGLA/RMarker.v` Lemma `tenet_no_star` (PR #644 @ `367a7ba`) |
| Author | Vasilev Dmitrii `<admin@t27.ai>` |

### Constitutional Compliance

| Rule | Status |
|------|--------|
| R5-HONEST | ✅ all predicates carry assertion/method/owner_lane/consequences_if_fail |
| R7 falsification | ✅ W-102-A pre-registered with fail-stop policy |
| R8 author | ✅ Vasilev Dmitrii `<admin@t27.ai>` |
| R14 Coq citation | ✅ tenet_no_star in gHashTag/t27:coq/IGLA/RMarker.v |
| R15 sacred synth | ✅ 0xE1 continues 0xDE/0xDF/0xE0 |
| R18 LAYER-FROZEN | ✅ purely additive, Wave-28 assertions/lever_stack.json untouched |
| Apache-2.0 | ✅ |

### Status: LANDED ✅

---

## Lane Y' — TOM Assertion Mirror · L-DPC31 Wave-34

**Agent:** Lane Y' (assertion agent)
**Date:** 2026-08-15
**Mission:** L-DPC31 · Wave-34 TOM Ternary ROM Accelerator (trios#854)
**Branch:** `feat/lane-y-prime-tom-assertion-wave34`
**Tracking Issue:** trios#854
**RTL Epic:** trinity-fpga#116

### Deliverable

Created `assertions/wave34_tom_layer_gate.json` — JSON-schema draft-07 predicate file containing pre-registered R7 falsifier **W-103-A** for TOM static power-gate layer controller.

### Key Facts

| Field | Value |
|-------|-------|
| Lever | `TOM-static-power-gate` |
| Opcode | `OP_LAYER_GATE = 0xE2` |
| Sacred chain | `0xDE → 0xDF → 0xE0 → 0xE1 → 0xE2` |
| Predicate | W-103-A |
| Metric | `layer_idle_fraction ≥ 0.5` |
| Fail-stop | `true` |
| Freeze date | 2026-08-15 |
| Evaluation date | 2026-10-15 |
| Coq mirror | `gHashTag/t27:coq/IGLA/RMarker.v` Lemma `tom_no_star` depends on `tenet_no_star` (PR #644 @ `367a7ba`) |
| Cost model | area +0.1 mm², power overhead +3 mW, leakage saved −12 mW (PRE-SILICON ESTIMATE) |
| Author | Vasilev Dmitrii `<admin@t27.ai>` |

### Constitutional Compliance

| Rule | Status |
|------|--------|
| R5-HONEST | ✅ cost_model labelled PRE-SILICON ESTIMATE |
| R7 falsification | ✅ W-103-A pre-registered with fail_stop: true |
| R8 author | ✅ Vasilev Dmitrii `<admin@t27.ai>` |
| R14 Coq citation | ✅ tom_no_star in gHashTag/t27:coq/IGLA/RMarker.v |
| R15 sacred synth | ✅ 0xE2 continues 0xDE/0xDF/0xE0/0xE1 |
| R18 LAYER-FROZEN | ✅ purely additive, wave29_tenet_sparsity.json and lever_stack.json untouched |
| Apache-2.0 | ✅ |

### Status: OPEN PR ✅


---

## Lane V''' — PhD Glava 81 LUT-NPU · L-DPC32 Wave-35

**Agent:** Lane V''' (PhD chapter agent)
**Date:** 2026-05-15
**Mission:** L-DPC32 · Wave-35 LUT-NPU (trinity-fpga#120 / trios#858)
**Branch:** `feat/wave35-lut-npu-phd-glava-81`
**Tracking Issue:** trios#866

### Deliverable

Created `docs/phd/chapters/glava_81_lut_npu_wave35.tex` — PhD monograph chapter for Wave-35 Lever #9 (LUT-NPU 81-entry direct-evaluation BitNet b1.58 PE, OP_LUT_NPU=0xE3, 270 TOPS/W spec-layer).

### Key Facts

- **1859 total lines** / **1584 non-blank** (R3 ≥ 1500 ✅)
- **5 theorems** (81.1 Correctness, 81.2 Energy Lower Bound, 81.3 Orthogonality, 81.4 Witness W-104-A) + 4 proofs + 5 corollaries + 9 definitions
- **18 distinct citation keys**, 62 `\cite{...}` calls (R3 ≥ 2 ✅)
- **35 PRE-SILICON ESTIMATE labels** (R5-HONEST ✅)
- Table 81.2: 15-row coefficient-source map (R6 ✅)
- W-104-A pre-registration, freeze 2026-08-15 (R7 ✅)
- Coq map → `t27/trios-coq/IGLA/LutNpu.v` `Theorem lut_npu_safe` (R14 ✅)
- LAYER-FROZEN (R18 ✅)
- Signed `Vasilev Dmitrii <admin@t27.ai>` ORCID 0009-0008-4294-6159 (R8 ✅)

### Wave-35 cross-strand triangle (4/5 already merged)

| Lane | PR | SHA |
|------|------|-----|
| V (Coq, t27) | #651 | `8e4f2a8a` |
| V' (JSON, trios) | #859 | `f2ee3613` |
| V'' (Rust, max-true) | #21 | `403a80dd` |
| U (RTL, fpga) | #124 | `4d339944` |
| **V''' (PhD, trios)** | **(THIS LANE)** | — |

φ² + φ⁻² = 3 · γ = φ⁻³ · DOI 10.5281/zenodo.19227877 · NEVER STOP
