# Autonomous Loop — Trios Integration

## Context
**Operation**: Military-style parallel deployment
**Agent**: D (Trios Integration rings 5-8: crypto, kg, agents, training)
**Brothers in arms**:
- Agent A: Parameter Golf + GF16 prototype (Issue #21)
- Agent B: Parameter Golf competition + RunPod grant (#19)
- Agent C: Trios Integration rings 1-4 (golden-float, hdc, sacred, physics)
- Agent D: Trios Integration rings 5-8 (crypto, kg, agents, training)

## Pre-flight
1. cd ~/trios
2. git checkout main && git pull
3. cargo check --workspace || fail
4. Check: .trinity/experience/rings.md exists, read current ring number

## Main Loop (run until all 4 rings SEALED)
FOR ring IN [5, 6, 7, 8]:
  IF ring already SEALED in RINGS.md: CONTINUE

  CHECKOUT feat/trios-ring-<N>
  EXECUTE 10 stages from spec:
    - scaffold → ffi → build → unit → integration
    - e2e → example → docs → mcp → seal

  FOR EACH stage:
    - Make changes
    - RUN: cargo build -p <crate>
    - IF FAIL: fix errors, retry up to 3 times
    - RUN: cargo test -p <crate>
    - IF FAIL: fix errors, retry up to 3 times
    - git add <specific_files>
    - git commit -m "<stage_type>(<crate>): <description>"

  FINALIZE:
    - gh pr create --title "feat(trios): ring <N> — <crate> complete"
    - Wait for CI green
    - gh pr merge --squash
    - Update RINGS.md: ring <N> SEALED
    - git commit -m "chore(rings): seal ring <N>"

## Completion Criteria
- All 4 rings SEALED (5, 6, 7, 8)
- cargo build --workspace exits 0
- cargo test --workspace --all-features exits 0
- cargo clippy --workspace -- -D warnings exits 0
- trios-server --list-tools shows ≥ 40 tools (14 existing + 26+ new)

## Reporting Protocol
After each ring completion:
- Write ring-<N>-report.md to plans/
- Include: lines added, tests passed, coverage %, time taken
- Commit report separately

## Failure Protocol
IF any stage fails 3 times:
- Write .agent/blocker-ring-<N>.md with full error trace
- Skip to next ring
- DO NOT silently skip tests
- DO NOT comment out failing code

## Ring Order (Agent D - rings 5-8)

| Ring | Crate | Dependencies | Stages | Status |
|------|-------|-------------|---------|--------|
| 5 | trios-crypto | None | 10 | PENDING |
| 6 | trios-kg | HTTP client | 10 | PENDING |
| 7 | trios-agents | HTTP + MCP proxy | 10 | PENDING |
| 8 | trios-training | HTTP client (Railway) | 10 | PENDING |

## Integration Points (Stage 9)
Each ring's Stage 9 adds MCP tools to trios-server:

### Ring 5: trios-crypto
- crypto_mine_sha256d → tool: `crypto_mine_sha256`
- crypto_mine_keccak256 → tool: `crypto_mine_keccak`

### Ring 6: trios-kg
- kg_insert → tool: `kg_insert`
- kg_query → tool: `kg_query`
- kg_traverse → tool: `kg_traverse`

### Ring 7: trios-agents
- agents_spawn → tool: `agents_spawn`
- agents_status → tool: `agents_status`
- agents_terminate → tool: `agents_terminate`

### Ring 8: trios-training
- training_submit_job → tool: `training_submit_job`
- training_status → tool: `training_status`
- training_cancel → tool: `training_cancel`
