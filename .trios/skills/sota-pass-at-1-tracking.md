# Skill: SOTA Pass@1 Tracking and Competitor Scoreboard Update

## When to Use

When a new competitor claims a higher Pass@1 or Pass@K metric than Trinity's current tracked SOTA, or when you need to update the competitive scoreboard in `specs/igla/coder/benchmark.t27`.

## Steps

1. **Verify competitor claims** — Check arXiv/ICML/website for actual numbers. Do not trust abstracts alone.
2. **Assess scope match** — Does the benchmark match Trinity's target domain (Verilog/RTL for IGLA-Coder, physics predictions for Trinity Core)?
3. **Update `benchmark.t27`** — Add a new `CompetitorScore` entry with:
   - `name`: short identifier
   - `pass_at_1`, `pass_at_5`, `pass_at_10`: actual reported numbers (0.0 if unavailable)
   - `benchmark`: the dataset or task name
4. **Add test** — Every new competitor MUST have a corresponding `test` block in the same file verifying the struct fields are non-empty.
5. **Re-seal** — Run `t27c seal --save specs/igla/coder/benchmark.t27`.
6. **Document in wave report** — Note the new SOTA and whether Trinity's gap is widening or narrowing.

## Current SOTA Reference (as of W109)

| Competitor | Pass@1 | Domain | Threat |
|-----------|--------|--------|--------|
| COEVO | 97.5% | VerilogEval-2.0 PPA | EXTREME |
| VeriAgent | 97.9% | RTL verification | HIGH |
| HDLFORGE | 91.2% | RTL generation | MEDIUM |
| ACE-RTL | (N/A agentic) | Agentic RTL | MEDIUM |
| ToobySmith-2HDM | N/A | Physics (errors caught) | N/A |

## Why It Matters

Tracking SOTA keeps Trinity honest about its competitive position. If a competitor exceeds Trinity's IGLA-Coder target, that becomes an urgent engineering priority.
