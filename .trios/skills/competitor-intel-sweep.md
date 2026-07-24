# Skill: Competitor Intelligence Sweep

## When to use

When you need to discover new scientific competitors for the Trinity S³AI project. Run this skill at the start of every Wave Loop.

## Why

- The geometric-unification and formal-verification space is moving fast (91+ competitors)
- Missing a competitor for 1–2 weeks can mean missing a critical threat (e.g., Washburn, GIFT)
- Systematic sweeps prevent reliance on accidental discovery

## Steps

1. **Define the search window:** Last sweep date → today. Default: 2 weeks.
2. **Run parallel searches:**
   - `site:arxiv.org "Lean 4" "physics" after:YYYY-MM-DD`
   - `site:arxiv.org "Coq" "physics" after:YYYY-MM-DD`
   - `site:arxiv.org "golden ratio" "fermion" after:YYYY-MM-DD`
   - `site:arxiv.org "spectral action" "noncommutative" after:YYYY-MM-DD`
   - `site:zenodo.org "neutrino mass" after:YYYY-MM-DD`
   - `site:vixra.org "fermion" after:YYYY-MM-DD`
   - `site:github.com "Lean 4" "physics" pushed:>YYYY-MM-DD`
3. **For each hit, evaluate:**
   - **Title + authors + date + source**
   - **Threat level:** LOW / MEDIUM / HIGH / EXTREME
   - **Differentiation:** Why is Trinity better (or worse)?
   - **AI angle:** Does it explicitly mention AI assistants? (This validates Trinity's methodology.)
   - **Missing pillar:** Does it lack formal proofs, hardware, zero free inputs, or numerical predictions?
4. **Update `docs/COMPETITIVE_POSITIONING.md`:**
   - Append new competitors with sequential IDs
   - Update total count
   - Update date
5. **Report:**
   - New competitors added: N
   - Updated total: M
   - Highest new threat: [name + level]
   - Key insight: 1 sentence

## Threat Level Criteria

| Level | Criteria |
|-------|----------|
| **EXTREME** | Lean 4/Coq formalization + SM predictions + zero free inputs + published |
| **HIGH** | Lean 4/Coq formalization + physics content; OR broad-scope Python with many predictions |
| **MEDIUM** | Finite geometry / NCG / flavor physics overlap; no formal proofs |
| **LOW** | Orthogonal paradigm; OR tooling-only (no physics content) |

## Common Pitfalls

- **Don't over-count:** A paper about Lean 4 tactics with no physics is LOW, not HIGH
- **Don't under-count:** A viXra paper with testable predictions is MEDIUM even if source is low-credibility
- **Track missed competitors:** If you find a paper from March/April that was missed, add it retroactively with a note

## Related

- [[trinity-competitor-tracking]] — Long-term memory of tracked competitors
- [[autoformalization-threat]] — Infrastructure competitors (Meadows et al.)
