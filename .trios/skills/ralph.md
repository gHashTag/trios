---
name: ralph
description: Project-wide anomaly detector and auto-remediator. Runs all validators, finds problems, creates a decomposed plan, and implements fixes per project rules.
tags: [audit, loop, validators, remediation, monitoring]
---

# RALPH — Recursive Anomaly Loop + Guardian Heuristic

Continuous project health monitor. Runs all CI validators, detects drift/gaps/anomalies, decomposes into actionable tasks, and executes fixes.

## Trigger

```
/ralph
```

Or schedule recurring:
```bash
/loop 15m /ralph
```

## What it does (one cycle)

1. **Harvest** — BPB data from Railway fleet
2. **Validate** — Run all CI gates in sequence
3. **Audit** — Check uncommitted changes, untracked artifacts, stale docs
4. **Plan** — Decompose findings into prioritized tasks
5. **Fix** — Implement non-breaking fixes (safe edits, commits)
6. **Report** — 1-sentence summary + task list update

## Validator sequence (must all pass)

```bash
python3 scripts/anti_numerology_gate.py
python3 scripts/count_admitted_honest.py
python3 scripts/validators/validate_v4.py
python3 scripts/generate_claims.py --check
bash scripts/check_english_only.sh
```

## Coq check (if environment supports)

```bash
cd proofs/trinity
coq_makefile -f _CoqProject -o Makefile.coq
make -f Makefile.coq -j$(nproc)
```

## Rust checks (if repo has Cargo)

```bash
cd games/trinity_fold && cargo test --workspace
cd trinity_rust && cargo test
```

## Anomaly classes detected

| Class | Check | Action |
|-------|-------|--------|
| **Uncommitted** | `git status --short` | Stage + commit if safe |
| **Untracked** | `git status` | Add to `.gitignore` if artifacts |
| **Coq gap** | Build errors | Document in `docs/COQ_*_GAP.md` |
| **BPB drift** | Harvest log | Report trend |
| **Cyrillic leak** | `check_english_only.sh` | Flag for manual fix |
| **Stale claims** | `generate_claims.py --check` | Regenerate if needed |
| **Skill drift** | `.claude/skills/*.md` age | Update if outdated |

## Safety rules (hard constraints)

- **Never** commit `.env`, tokens, passwords, DSNs
- **Never** push to `main` without PR
- **Never** delete `proofs/` or `docs/claims.yaml`
- **Always** run `--dry-run` before writes to Postgres
- **Always** update `.gitignore` before committing build artifacts

## Output format

```
RALPH CYCLE #N | <timestamp>
--------------------------------
[VALIDATORS] 5/5 PASS | 0 FAIL | 1 GAP documented
[HARVEST] 24 services | best=2.4872 (scarab-fp16-seed77)
[ANOMALIES] 2 found
  - Coq: Interval.Tactic missing (docs/COQ_INTERVAL_LIBRARY_GAP.md)
  - Untracked: pdf-build/ → added to .gitignore
[COMMITS] 2 made
  - trios-trainer-igla: φ⁻³ fix + honest findings
  - trios-mcp-rag: PDF pipeline fix
[TASKS] 1 pending
  - Epic #181 phi ablations
--------------------------------
Next cycle: /ralph or /loop 15m /ralph
```

## Integration with other skills

- After `/ralph` finds issues → use `/tri ci-fix <issue>` for CI repair
- After `/ralph` harvest → use `/gardener` for fleet management
- After `/ralph` audit → use `/honesty-check` for claim ledger review

## Example full session

```
User: /ralph

RALPH: Running cycle...
[VALIDATORS] 5/5 PASS
[HARVEST] best=2.4872 (scarab-fp16-seed77)
[ANOMALIES] 1: trios-mcp-rag uncommitted changes
[FIX] Staged pipeline.rs + template.tex
[COMMIT] "fix(pdf): batched asset download + tectonic template fix"

RALPH cycle complete. 1 commit, 0 anomalies remaining.
Next: /ralph or wait for scheduled loop.
```

---

*Last updated: 2026-05-31*  
*Skill version: 1.0.0*
