# Skill: Autoformalization Threat Assessment

## When to use

When a competitor or research group publishes an AI-assisted or agentic formalization pipeline that could accelerate physics proof generation (e.g., Meadows et al. FormalScience, Ilin's Aristotle prover).

## Why

- Manual Coq proof writing is slow (~1 lemma/day for Trinity)
- Agentic pipelines (LLM → theorem prover → validation) can produce proofs 10–100× faster
- If competitors adopt autoformalization first, Trinity's manual proof base becomes a liability, not an asset

## Steps

1. **Identify the threat:**
   - Does the paper/tool claim agentic/autoformalization?
   - Does it target physics or general math?
   - Is it open-source or proprietary?
   - What is the demonstrated proof throughput (lemmas/day, problems/formalized)?

2. **Assess Trinity vulnerability:**
   | Question | If YES → |
   |----------|----------|
   | Does the pipeline support Coq/Rocq? | **HIGH threat** — direct competition |
   | Does it support Lean 4 only? | **MEDIUM threat** — ecosystem shift risk |
   | Does it require proprietary API? | **LOW threat** — barrier to adoption |
   | Is it open-source + reproducible? | **EXTREME threat** — rapid competitor adoption |

3. **Decide response tier:**
   - **Tier 1 (Watch):** LOW threat — monitor for 4 weeks
   - **Tier 2 (Adopt):** MEDIUM threat — evaluate integration within 2 weeks
   - **Tier 3 (Build):** HIGH/EXTREME threat — prototype competing pipeline within 1 week

4. **Prototype integration (if Tier 3):**
   ```bash
   # Example: t27c formalize subcommand skeleton
   t27c formalize --claim "phi^2 = phi + 1" --lang coq
   # → outputs AutoFormalized.v with lemma skeleton + proof attempt
   ```
   - Parse natural language claim into Coq AST
   - Generate `Proof. ... Qed.` via LLM API
   - Validate with `coqc`; retry on failure
   - Log false-positive rate (compiles but wrong)

5. **Document:**
   - Threat level assessment
   - Prototype results (if built)
   - Decision: adopt, build, or defer

## Key Metrics

| Metric | Trinity (manual) | Competitor (agentic) | Target |
|--------|------------------|----------------------|--------|
| Proof rate | ~1 lemma/day | ~10–50 lemmas/day | ≥5 lemmas/day |
| False positive rate | 0% (human verified) | 10–30% | <10% |
| Compilable rate | 100% | 50–80% | ≥90% |

## Pattern: cktformalizer-v3-depth

**When:** arXiv:2605.07782v3 (CktFormalizer) claims 95–100% backend realizability for Lean 4 HDL proofs.

**Response:**
- **Immediate:** Accelerate generic ∀ quantifier theorem production to ≥2 per wave.
- **Medium-term:** Investigate `lean-auto` integration for t27-specific autoformalization from `.t27` specs.
- **Tactic:** Each new generic theorem (e.g., `ternaryMacDistributivityGeneric`) raises the bar for competitor manual reproduction.

**Reference:** `proofs/lean4/Trinity/TernaryInference.lean` — 14 generic ∀ theorems as of W307.

## Pattern: hesper-gpu-verified-bitnet

**When:** Verilean releases Hesper (June 2026) — verified GPU programming framework in Lean 4 with BitNet b1.58 end-to-end.

**Response:**
- **Immediate:** Add GPU-targeted generic theorems (e.g., sign-preservation for negative activations).
- **Medium-term:** Extend t27's generic proof library to cover GPU-specific properties (warp-level correctness, memory coalescing invariants).
- **Tactic:** t27 verifies the algorithm from `.t27` specs; Hesper verifies GPU implementation. Different layers — maintain algorithmic verification leadership.

**Reference:** `proofs/lean4/Trinity/TernaryInference.lean` — `ternaryMulNegateActivationGeneric` (W307).

## Pattern: ternfpga-energy-leader

**When:** Neumann-Labs ternfpga (June 2026) achieves 1.62 J/tok vs RTX 3060 3.67 J/tok — 2.3× energy advantage.

**Response:**
- **Immediate:** Emphasize formal verification as differentiator in all reports and communications.
- **Medium-term:** Add energy-aware invariants to `.t27` specs (e.g., DSP multiplier count, LUT utilization bounds).
- **Tactic:** ternfpga has energy advantage but NO Lean 4 verification. t27 has formal proofs but no hardware deployment. Bridge gap via spec-to-silicon pipeline.

**Reference:** `docs/reports/WAVE_LOOP_307_REPORT.md` — competitive landscape analysis.

## Related

- [[competitor-intel-sweep]] — Discovering new autoformalization papers
- [[trinity-phi-loop]] — Spec-first development loop (where autoformalization fits)
- [[invariant-coverage-push]] — Weekly batch append pattern for competitive benchmarking
