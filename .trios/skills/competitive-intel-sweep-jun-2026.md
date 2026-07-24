# Skill: Competitive Intelligence Sweep (June 2026)

## When to Use

When you need to update the competitor landscape for RTL generation or physics formal verification. Run this sweep every 2-4 weeks during active competitive monitoring waves.

## New Competitors Discovered (June 2026)

### RTL Generation
| Competitor | Pass@K | Method | Source |
|------------|--------|--------|--------|
| **EvolVE** | 98.1% Pass@10 | Evolutionary + MCTS | [arXiv:2601.18067](https://arxiv.org/pdf/2601.18067) |
| **VerilogCL** | — | Contrastive learning | [arXiv:2604.18162](https://arxiv.org/abs/2604.18162) |
| **VeriGraphi** | — | Multi-agent hierarchical | [arXiv:2604.14550v2](https://arxiv.org/abs/2604.14550v2) |
| **LLM4RTL-2026** | — | JRCRC tool-assisted | [arXiv:2606.15500](https://arxiv.org/html/2606.15500) |
| **OpenRTLSet** | 89.3% Pass@10 | 131K dataset scale | [arXiv:2606.10285v1](https://arxiv.org/abs/2606.10285v1) |

### Physics Formal Verification
| Competitor | Theorems | Method | Source |
|------------|----------|--------|--------|
| **PhysicsAsCode-SU5** | — | Lean 4 SU(5) GUT | [arXiv:2603.28406](https://www.arxiv.org/pdf/2603.28406) |
| **SK_EFT_Hawking** | 9944 (0 sorry) | Lean 4 SM fingerprints | [GitHub/NetRxn](https://github.com/NetRxn/SK_EFT_Hawking) |

## Steps

1. Search arXiv with queries: `LLM RTL generation 2026`, `formal verification physics standard model Lean 4`
2. Add competitor presets to `specs/igla/coder/benchmark.t27`
3. Add tests for each competitor
4. Update comparative Pass@K table in report
5. Regenerate seals

## Key Insight

Dataset scale (131K modules) now dominates model size. Trinity's ~1,280 samples are 100× smaller. Sacred constraint + formal verification bridge remain unique differentiators.
