# Skill: Competitive Landscape Assessment

## When to use

At the start of every wave loop to assess whether the competitive landscape has changed significantly and whether Trinity's differentiation is still valid.

## Why

- The geometric-unification and formal-verification space moves fast (95+ competitors)
- Missing a new competitor for 1–2 weeks can mean missing a critical threat
- Systematic assessment prevents reactive panic and enables proactive differentiation

## Steps

1. **Count total competitors:**
   - Check `docs/COMPETITIVE_POSITIONING.md` for current count
   - Verify last sweep date

2. **Run competitive sweep:**
   - See [[competitor-intel-sweep]] for search queries
   - Focus on: arXiv hep-th/math-ph, Zenodo, viXra, GitHub

3. **Categorize new entrants:**
   | Level | Criteria |
   |-------|----------|
   | **EXTREME** | Lean 4/Coq formalization + SM predictions + zero free inputs + published |
   | **HIGH** | Lean 4/Coq formalization + physics content; OR broad-scope Python with many predictions |
   | **MEDIUM** | Finite geometry / NCG / flavor physics overlap; no formal proofs |
   | **LOW** | Orthogonal paradigm; OR tooling-only (no physics content); OR foundational math (enabler) |

4. **Assess differentiation:**
   - Does the new competitor have formal proofs? (Trinity: 166+ Coq theorems)
   - Does it have hardware instantiation? (Trinity: CORDIC RTL + sacred opcodes)
   - Does it claim zero free inputs? (Trinity: φ, π, e only)
   - Does it have numerical predictions with error bars? (Trinity: certified tolerances)
   - Does it have an arXiv presence? (Trinity: preparing submission)

5. **Identify highest threat:**
   - Which competitor challenges Trinity's strongest differentiator?
   - Is there a new threat axis (e.g., autoformalization pipeline)?
   - Does any competitor have a testable prediction where Trinity has a gap?

6. **Update positioning:**
   - Add new competitors to `COMPETITIVE_POSITIONING.md`
   - Update total count
   - Update key insights / strategic assessment
   - Update date

7. **Recommend response:**
   - No new entrants → focus on execution (close gaps, submit arXiv)
   - MEDIUM threat → document differentiation, monitor for 2 weeks
   - HIGH/EXTREME threat → prioritize defensive move (close gap, accelerate proof)

## Key Insight from W90

**Maturation plateau:** No new EXTREME or HIGH threats in late June 2026. The most dangerous competitors remain the Lean 4 formalization axis (Washburn, GIFT, Douglas et al., Meadows et al.). Incremental additions (MEDIUM/LOW) do not require immediate response but should be tracked.

## Related

- [[competitor-intel-sweep]] — Finding new competitors
- [[autoformalization-threat]] — Assessing infrastructure competitors
- [[trinity-competitor-tracking]] — Long-term memory of tracked competitors
