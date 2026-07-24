# Skill: GitHub Issue Triage for Budget-Gated Roadmap

## When to Use

When you encounter open GitHub issues that are blocked by external dependencies (GPU budget, model training, API access) rather than engineering capacity.

## Steps

1. **List open issues** via `gh issue list --repo owner/repo --state open`
2. **Assess each issue** for:
   - Engineering blockers (can we fix this with code?)
   - Budget blockers (requires GPU/API/training budget?)
   - External blockers (depends on third-party action?)
3. **Tag budget-gated issues** as `blocked-external` or `blocked-budget`
4. **Document in wave report** — honesty about why issues cannot be closed prevents zombie issue accumulation
5. **Set revisit criteria** — e.g., "Revisit when GPU budget > $X/month secured"

## Why It Matters

Zombie issues (open but unactionable) inflate issue count and create false sense of backlog. Honest triage separates "we can't fix yet" from "we haven't fixed yet." This maintains L1 TRACEABILITY integrity.

## Example (W109)

All 5 open issues (#1037–#1041) were IGLA-Coder roadmap items requiring model training budget. Assessment: deferred to W110+ with tag "blocked-budget".
