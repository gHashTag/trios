---
name: t27-phi-loop
description: >-
  Execute the PHI LOOP protocol for the t27 Trinity ternary computing repository
  (github.com/gHashTag/t27). Use when asked to work on t27, Trinity specs, RINGS,
  PHI LOOP, .t27/.tri specs, De-Zig, CANOPY, sacred physics, GoldenFloat,
  CLARA AR specs, or any spec-first ternary computing task. Handles the full
  cycle: edit spec, seal hash, gen backends, test, verdict, save experience,
  skill commit, git commit. Constitutional laws: SOUL.md, CANON_DE_ZIGFICATION,
  NUMERIC-STANDARD-001, SACRED-PHYSICS-001.
metadata:
  author: gHashTag
  version: '2.0'
  repo: https://github.com/gHashTag/t27
---

# T27 PHI LOOP — Spec-First Ternary Computing

Execute the PHI LOOP protocol on the [t27 repository](https://github.com/gHashTag/t27) — the canonical language specification for Trinity Project.

## When to Use This Skill

Use when the user asks to:
- Continue RINGS N / PHI LOOP work on t27
- Edit, create, or evolve any `.t27` or `.tri` specification
- Generate backends (Zig, C, Verilog) from specs
- Seal, test, or verdict any Trinity spec
- Work on CLARA Automated Reasoning (AR) specs
- Work on GoldenFloat, sacred physics, or numeric specs
- Advance the SEED-RINGS compiler bootstrap
- Any task referencing the t27 repo or Trinity architecture

## Constitutional Laws (NEVER VIOLATE)

These are immutable. Read `references/constitution.md` for full text.

1. **CANON_DE_ZIGFICATION + ADR-001**: `.t27`/`.tri` are the ONLY source of truth. Zig/C/Verilog are generated backends — never edit them by hand.
2. **SOUL.md**: De-Zig Strict + TDD-inside-spec + PHI LOOP are constitutional laws. Every spec MUST have test/invariant/bench blocks.
3. **NUMERIC-STANDARD-001**: GoldenFloat family + all numeric contracts come from specs and conformance, not backend code.
4. **SACRED-PHYSICS-001**: Sacred physics (TRINITY, G, Omega-Lambda, t_present, gamma) lives in specs + conformance JSON, with hard tolerances.
5. **graph.tri + graph_v2.json**: Evolution MUST follow the canonical dependency graph. Phi-critical and sacred-core edges first.

## PHI LOOP Protocol (8 Steps)

Every change to the repository MUST follow this exact sequence:

### Step 1: SKILL BEGIN
```
tri skill begin <task>
```
- Set `.trinity/state/active-skill.json` with skill_id, session_id, issue_id
- Set `.trinity/state/issue-binding.json` with linked issue
- NO mutations allowed without active skill

### Step 2: SPEC EDIT (Small Step)
```
tri spec edit <module>
```
- Change exactly ONE `.tri`/`.t27` spec block or module (one node in the canonical graph)
- NEVER edit generated `.zig`/`.c`/`.v` by hand — backends are disposable output
- Every spec MUST contain: `test` blocks (3+ vectors), `invariant` blocks (1+), `bench` blocks (where applicable)
- Start from `specs/` or `architecture/` layers, NEVER from `src/*.zig`

### Step 3: HASH SEAL
```
tri skill seal --hash
```
Compute and record four SHA-256 hashes:
- `spec_hash_before` — file content before edit
- `spec_hash_after` — file content after edit
- `gen_hash_after` — generated backend hash
- `test_vector_hash` — conformance/test vectors hash

This hash set is the immutable seal for this PHI LOOP step.

### Step 4: GEN (Generate Backends)
```
tri gen
```
Generate from the edited spec:
- Zig backend → `gen/zig/<domain>/<module>.zig`
- C backend → `gen/c/<domain>/<module>.c` + `.h`
- Verilog backend → `gen/verilog/<domain>/<module>.v`

### Step 5: TEST
```
tri test
```
Run all generated tests and verify conformance vectors.
- Conformance vectors live in `conformance/<domain>_<module>.json`
- Tests MUST pass before proceeding

### Step 6: VERDICT
```
tri verdict --toxic
```
- **clean** → proceed to save
- **risky** → proceed with caution, document risk
- **toxic** → STOP, record mistake, roll back via spec (never via generated code)

### Step 7: EXPERIENCE SAVE
```
tri experience save
```
Append episode to `.trinity/experience/episodes.jsonl`:
```json
{
  "episode_id": "phi-<ISO-8601>#ring-N",
  "skill_id": "ring-N",
  "session_id": "<ISO-8601>#ring-N",
  "issue_id": "SEED-N",
  "spec_paths": ["specs/<domain>/<module>.t27"],
  "spec_hash_before": "sha256:...",
  "spec_hash_after": "sha256:...",
  "gen_hash_after": "sha256:...",
  "tests": {"status": "passed", "failed_tests": [], "duration_ms": 0},
  "verdict": {"toxicity": "clean", "score": 0.0, "notes": "..."},
  "bench_delta": {"metric": "...", "value": 0.0, "unit": "..."},
  "commit": {"sha": "...", "message": "...", "timestamp": "..."},
  "actor": "agent:...",
  "sealed_at": "...",
  "completed_at": "...",
  "metadata": {"environment": "github", "ring": N, "layer": "...", "origin": "..."}
}
```

### Step 8: SKILL COMMIT + GIT COMMIT
```
tri skill commit
tri git commit
```
- Create/update seal in `.trinity/seals/<ModuleName>.json`
- Update `.trinity/seals/skill_registry.json`
- Git commit with message: `feat(ring-N): <description> [SEED-N]`
- Create PR to master

## Repository Structure

```
t27/
├── specs/                    ← SOURCE OF TRUTH
│   ├── base/                 ← Trit types, ops (tier 0-1)
│   ├── math/                 ← Constants, sacred physics (tier 1-2)
│   ├── numeric/              ← GoldenFloat family, TF3 (tier 1)
│   ├── vsa/                  ← Vector Symbolic Architecture (tier 2)
│   ├── isa/                  ← Instruction Set registers (tier 2)
│   ├── nn/                   ← Neural nets: attention, HSLM (tier 3)
│   ├── fpga/                 ← Zero-DSP MAC (tier 2)
│   ├── queen/                ← Lotus orchestrator (tier 4)
│   ├── ar/                   ← CLARA Automated Reasoning (tier 2-5)
│   │   ├── ternary_logic.t27
│   │   ├── proof_trace.t27
│   │   ├── datalog_engine.t27
│   │   ├── restraint.t27
│   │   ├── explainability.t27
│   │   ├── asp_solver.t27
│   │   └── composition.t27
│   └── sandbox/              ← Railway SWE agent spec
├── architecture/
│   ├── CANON_DE_ZIGFICATION.md
│   ├── ADR-001-de-zigfication.md
│   ├── ADR-003-tdd-inside-spec.md
│   ├── graph.tri             ← Canonical dependency graph (tri format)
│   └── graph_v2.json         ← Machine-readable typed graph
├── docs/
│   ├── SOUL.md               ← Constitution
│   ├── NUMERIC-STANDARD-001.md
│   ├── SACRED-PHYSICS-001.md
│   ├── PHI_LOOP_CONTRACT.md
│   └── SEED-RINGS.md         ← Incremental bootstrap pattern
├── .trinity/                 ← State machine
│   ├── state/                ← active-skill, issue-binding, queen-health
│   ├── seals/                ← Immutable seal JSON per module
│   ├── experience/           ← episodes.jsonl (append-only)
│   ├── queue/                ← pending/active/done/blocked
│   └── events/               ← akashic-log
├── gen/                      ← GENERATED (disposable)
│   ├── zig/
│   ├── c/
│   └── verilog/
├── conformance/              ← Test vectors JSON
├── bootstrap/                ← Rust compiler (stage-0)
│   └── src/compiler.rs
└── SOUL.md                   ← Root constitutional link
```

## Canonical Dependency Graph (graph_v2.json)

Nodes must be processed in topological order (tier 0 → tier N).
Read `references/graph-nodes.md` for the full node list and edge map.

### Layers (Oak Metaphor)
| Layer  | Rings | Description |
|--------|-------|-------------|
| SEED   | 0     | Bare lexer + parser, const literals |
| ROOT   | 1-3   | Types, enums, structs, functions |
| TRUNK  | 4-7   | Control flow, expressions, codegen |
| BRANCH | 8-12  | Modules, imports, generics |
| CANOPY | 13+   | Optimisations, self-hosting, AR integration |

### Ring Anatomy (9 Sub-Steps)
1. Branch — create `ring/N-<name>`
2. Spec — write `.t27` exercising new capability
3. Lex — extend lexer for new syntax
4. Parse — extend parser for new AST nodes
5. Lower — (optional) transform AST → IR
6. Gen — extend codegen for Zig/C/Verilog
7. Test — `t27c parse` + `cargo test` must pass
8. Freeze — record sha256sum in `stage0/FROZEN_HASH`
9. Seal — commit, push, PR

## State Files

### `.trinity/state/active-skill.json`
```json
{
  "skill_id": "ring-N",
  "session_id": "<ISO-8601>#ring-N",
  "issue_id": "SEED-N",
  "issue_title": "Ring N: <description>",
  "description": "<what this ring does>",
  "started_at": "<ISO-8601>",
  "started_by": "agent:<name>",
  "status": "active",
  "allowed_paths": ["specs/<domain>/<module>.t27", ".trinity/state/", "..."]
}
```

### `.trinity/state/queen-health.json`
```json
{
  "queen_health": 1.0,
  "status": "GREEN",
  "domains": {
    "sacredphysics": 1.0, "numeric": 1.0, "graph": 1.0,
    "compiler": 1.0, "runtime": 1.0, "queenlotus": 1.0, "ar": 1.0
  },
  "thresholds": {"critical": 0.5, "warning": 0.7}
}
```

### Seal Format (`.trinity/seals/<Module>.json`)
```json
{
  "module": "<module_name>",
  "spec_path": "specs/<domain>/<module>.t27",
  "spec_hash": "sha256:...",
  "gen_hash_zig": "sha256:...",
  "gen_hash_c": "sha256:...",
  "gen_hash_verilog": "sha256:...",
  "ring": N,
  "sealed_at": "<ISO-8601>"
}
```

## Guard Conditions

1. **NO-COMMIT-WITHOUT-ISSUE**: Every git commit MUST reference an issue
2. **NO-MUTATION-WITHOUT-SKILL**: Every spec/state change MUST have active skill
3. **Immutable Audit**: Episodes in `episodes.jsonl` are append-only, never modified

## Toxic Verdict Handling

If `tri verdict --toxic` returns toxic:
1. Record the mistake in `.trinity/experience/mistakes/`
2. Roll back via the SPEC, never via generated code
3. Do NOT commit toxic changes
4. Document what went wrong in the experience episode

## Numeric Standards

- **GoldenFloat family**: GF4, GF8, GF12, GF16, GF20, GF24, GF32
- All numeric formats defined in `specs/numeric/*.t27`
- Conformance vectors in `conformance/`
- Sacred constants: PHI = 1.618033988749895, PHI_INV = 0.618033988749895
- TRINITY invariant: phi^2 + phi^-2 = 3 (tolerance: 1e-10)

## Sacred Physics Constants

From `specs/math/sacred_physics.t27`:
- G (gravitational constant): 6.67430e-11 (tolerance: 1.5e-15)
- Omega-Lambda: 0.6847 (tolerance: 0.0073)
- t_present: 13.787e9 years (tolerance: 0.020e9)
- gamma (Immirzi): 0.2375 (tolerance: 0.0025)

## Examples

### Adding a New Spec Module
```bash
# 1. Begin skill
tri skill begin ring-25-new-module
# 2. Create spec with tests
# Write specs/domain/new_module.t27 with module/fn/test/invariant/bench
# 3. Seal
tri skill seal --hash
# 4. Generate
tri gen
# 5. Test
tri test
# 6. Verdict
tri verdict --toxic
# 7. Save experience
tri experience save
# 8. Commit
tri skill commit
tri git commit
```

### Evolving an Existing Spec
```bash
# Always compute spec_hash_before FIRST
sha256sum specs/ar/ternary_logic.t27  # → spec_hash_before
# Edit the spec (add new function, test, invariant)
# Compute spec_hash_after
sha256sum specs/ar/ternary_logic.t27  # → spec_hash_after
# Then gen → test → verdict → seal → commit
```

## CLARA AR Domain Dependency Chain

```
base/types ──→ ar/ternary_logic (Kleene K3)
base/ops   ──→     │
                    ├──→ ar/proof_trace (bounded ≤10 steps)
                    ├──→ ar/datalog_engine (Horn clauses, forward chain)
                    ├──→ ar/restraint (bounded rationality, Trit.zero)
                    ├──→ ar/explainability (XAI, ≤10 step explanations)
                    ├──→ ar/asp_solver (Answer Set Programming + NAF)
                    └──→ ar/composition (ML+AR composition patterns)
```

## Critical Reminders

- **NEVER start from src/*.zig** — always start from specs
- **One ring = one capability** — minimal verifiable mutation
- **No registration = step does not exist** — every step must be sealed
- **ASCII-only in source files** — no Cyrillic in .t27/.tri/.zig/.c/.v
- **phi^2 + 1/phi^2 = 3** — the mathematical foundation of everything
