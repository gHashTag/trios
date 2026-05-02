# LAWS.md — trios Constitutional Document v2.0

**LAWS_SCHEMA_VERSION:** 2.0
**Created:** 2026-04-22
**Amended:** 2026-05-02

> *"Every agent is a temporary citizen with bounded authority. Every task has a contract. Every mutation leaves evidence. Every victory must improve the long-term health of the realm."*

---

## §0 SUPREMACY CLAUSE

This constitution is the supreme law of the trios repository. Any conflicting provision must yield. LAWS.md v2.0 is protected by 4 immutability mechanisms:

| Mechanism | Config Location | Enforced By |
|-----------|-----------------|-------------|
| CODEOWNERS | .github/CODEOWNERS | GitHub PR review |
| Branch Protection | Settings → Rules → main | GitHub Platform |
| CI Gate | .github/workflows/laws-guard.yml | GitHub Actions |
| Integrity Hash | .trinity/state/LAWS_HASH | laws-guard job |

Any agent that modifies LAWS.md without §8 procedure commits constitutional breach. Soul-name is forfeit and agent is blacklisted for session.

---

## §1 Constitutional Hierarchy

Files ranked by authority (rank 1 = highest):

| Rank | File | Description |
|------|------|-------------|
| 1 | LAWS.md | Supreme law (immutable without §8) |
| 2 | CLAUDE.md | Agent operating manual |
| 3 | AGENTS.md | Agent roster |
| 4 | TASK.md | Current task contract |
| 5 | CONTEXT.md | Session context (append-only) |
| 6 | .github/ISSUE_TEMPLATE/ | Issue schema |
| 7 | .github/CODEOWNERS | Access control |
| 8 | laws-guard.yml | Enforcement mechanism |

**Conflict resolution:** Higher rank always wins.

---

## §2 Required Repository Layout

```
trios/
├── crates/                    # Rust workspace
│   ├── trios-git/           # MCP tools (git operations)
│   ├── trios-gb/            # GitButler virtual branches
│   ├── trios-server/        # MCP server (port 9005)
│   ├── trios-ext/           # Chrome extension
│   ├── trios-training/      # Training infrastructure
│   ├── trios-igla-trainer/  # Training binary
│   ├── trios-ca-mask/       # Content-aware masking
│   ├── trios-crypto/        # Cryptographic primitives
│   ├── trios-phi-schedule/  # Phi-based scheduling
│   ├── trios-sdk/           # Rust SDK
│   ├── trios-a2a/           # Agent-to-agent communication
│   ├── trios-agents/        # Agent implementations
│   ├── trios-bridge/        # Bridge implementations
│   ├── trios-data/          # Data structures
│   ├── trios-golden-float/  # Golden float operations
│   ├── trios-hdc/           # Hardware data collection
│   ├── trios-hybrid/        # Hybrid operations
│   ├── trios-model/         # Model definitions
│   ├── trios-physics/       # Physics simulation
│   ├── trios-sacred/        # Sacred geometry operations
│   ├── trios-server/        # MCP server (Axum)
│   ├── trios-train-cpu/     # CPU training
│   ├── trios-training-ffi/  # Training FFI bindings
│   ├── trios-tri/           # TRI operations
│   ├── trios-trinity-init/  # Trinity initialization
│   ├── trios-ui/            # UI components
│   ├── trios-vsa/           # Vector symbolic architecture
│   ├── precision-router/    # Precision routing
│   ├── zig-agents/          # Zig agent implementations
│   └── zig-knowledge-graph/ # Zig knowledge graph
├── .trinity/                  # Trinity state directory
│   ├── experience/            # Experience logs (L7)
│   ├── prompts/               # Agent dispatch prompts (§13)
│   ├── specs/                # Task specifications
│   └── state/                # Immutable state files
├── .github/                   # GitHub configuration
│   ├── workflows/            # CI/CD pipelines
│   ├── ISSUE_TEMPLATE/       # Issue schemas
│   └── CODEOWNERS            # Access control
├── docs/                      # Documentation
│   └── TRINITY_BRAND_KIT.md  # Brand guidelines
├── .claude/                   # Claude Code configuration
│   └── skills/              # Custom skills
├── CLAUDE.md                  # Legacy laws (v1 reference)
├── LAWS.md                    # Supreme constitution (v2)
├── AGENTS.md                  # Agent roster (if exists)
├── README.md                  # Project readme
├── Cargo.toml                 # Workspace manifest
├── railway.toml               # Railway configuration
└── [config files]
```

---

## §3 Core Laws

### L1 — No Shell Scripts
**Rule:** No `.sh` files anywhere in the repository
**Rationale:** Shell scripts are untested, untyped, non-composable
**CI Gate:** `find . -name "*.sh" ! -path "*/node_modules/*" ! -path "*/.git/*" | wc -l == 0`
**Status:** ✅ ACTIVE (enforced by ci.yml)

### L2 — Every PR Closes an Issue
**Rule:** Every PR description MUST contain `Closes #N`. No orphan PRs.
**Rationale:** Ensures traceability and prevents unaccounted changes
**CI Gate:** PR body check for `Closes #N`, `Fixes #N`, or `Resolves #N`
**Status:** ⏳ PENDING (no CI)

### L3 — Clippy Zero Warnings
**Rule:** `cargo clippy -- -D warnings` must pass before any merge
**Rationale:** Linting catches bugs early and maintains code quality
**CI Gate:** `cargo clippy --all-targets --all-features -- -D warnings`
**Status:** ✅ ACTIVE (enforced by ci.yml)

### L4 — Tests Before Merge
**Rule:** `cargo test` must pass. New code requires new tests.
**Rationale:** Tests ensure correctness and prevent regressions
**CI Gate:** `cargo test --all`
**Status:** ✅ ACTIVE (enforced by ci.yml)

### L5 — Port 9005 is trios-server
**Rule:** The MCP server always runs on `0.0.0.0:9005`. Never change this without a migration plan.
**Rationale:** Fixed port is required for agent communication
**CI Gate:** Port check in deployment configuration
**Status:** ⏳ PENDING (no CI)

### L6 — Fallback Required for GB Tools
**Rule:** `trios-gb` tools must gracefully return `Err` (not panic) if `gitbutler-cli` is not found.
**Rationale:** Graceful degradation prevents crashes
**CI Gate:** Code review check
**Status:** ⏳ PENDING (no CI)

### L7 — Experience Log
**Rule:** Every significant task writes a line to `.trinity/experience/`
**Rationale:** Traceability and learning from experience
**CI Gate:** Verify experience log exists for date
**Status:** ⏳ PENDING (no CI)

### L8 — Push First
**Rule:** Every commit MUST be pushed immediately. Unpushed commits are a violation.
**Rationale:** Remote backup prevents data loss and enables collaboration
**CI Gate:** Check for unpushed commits
**Status:** ⏳ PENDING (no CI)

### L9 — No Handwritten Forbidden Surface
**Rule:** Auto-generated code (WASM pkg, dist/) is committed. Hand-written code in those dirs is forbidden.
**Rationale:** Separates generated from source code
**CI Gate:** Lint check for forbidden patterns
**Status:** ⏳ PENDING (no CI)

### L10 — Issue #143 is Eternal
**Rule:** Issue #143 is the source of truth for all agents. Never close it.
**Rationale:** Central coordination point
**CI Gate:** Issue status check
**Status:** ⏳ PENDING (no CI)

### L11 — Name Before Mutation
**Rule:** Every agent takes a soul-name before making any changes. Format: humorous English name related to the task.
**Rationale:** Accountability and traceability
**CI Gate:** Required field in task_contract.yml
**Status:** ⏳ PENDING (no CI)

### L12 — Spec Before Implementation
**Rule:** Write the spec (TASK.md) before writing code. No spec = no code.
**Rationale:** Planning prevents waste and ensures alignment
**CI Gate:** TASK.md exists before code changes
**Status:** ⏳ PENDING (no CI)

### L13 — Bounded Authority
**Rule:** Agents only modify files within their assigned scope. Cross-scope changes require human approval.
**Rationale:** Prevents unauthorized changes
**CI Gate:** CODEOWNERS enforcement
**Status:** ⏳ PENDING (no CI)

### L14 — Auditability by Default
**Rule:** Every action must be traceable. Commit messages, issue comments, experience logs.
**Rationale:** Enables post-hoc analysis
**CI Gate:** Commit message format validation
**Status:** ⏳ PENDING (no CI)

### L15 — Validation is a Separate Duty
**Rule:** Verification is done by a different agent than the one who wrote the code.
**Rationale:** Independent verification catches bias
**CI Gate:** Code review requirement
**Status:** ⏳ PENDING (no CI)

### L16 — Tailoring Requires Rationale
**Rule:** Any deviation from laws must include a written rationale in the commit.
**Rationale:** Prevents silent drift from standards
**CI Gate:** Commit message check for "L16 rationale"
**Status:** ⏳ PENDING (no CI)

### L17 — Improve Code Health, Not Perform Heroics
**Rule:** Fix what's broken. Don't gold-plate.
**Rationale:** Focus on value, not perfectionism
**CI Gate:** Code review for scope creep
**Status:** ⏳ PENDING (no CI)

### L18 — Structured Conflict Resolution
**Rule:** When agents disagree, escalate to issue #143 with evidence.
**Rationale:** Prevents endless loops
**CI Gate:** Issue escalation pattern
**Status:** ⏳ PENDING (no CI)

### L19 — Humans Remain Sovereign
**Rule:** Human overrides always win. No exceptions.
**Rationale:** Final authority rests with humans
**CI Gate:** Manual override flag
**Status:** ⏳ PENDING (no CI)

### L20 — Turn Sessions into Tools
**Rule:** Every manual workflow must be codified into `tri` CLI within one session.
**Rationale:** Automation prevents repeated manual work
**CI Gate:** CLI command exists for workflow
**Status:** ⏳ PENDING (no CI)

### L21 — Context Immutability
**Rule:** Task context is sacred. Agents MAY append, MAY NOT delete.
**Rationale:** Autonomous agents routinely "tidy" context under token pressure, destroying situational awareness
**CI Gate:** context-guard.yml checks for context shrinkage > 50 lines
**Status:** ✅ ACTIVE (enforced by context-guard.yml)

### L22 — Schema-Response Parity
**Rule:** Tools declaring `outputSchema` MUST emit `structuredContent` matching it. `response.text()` alone is never sufficient when `outputSchema` is present.
**Rationale:** Ensures contract compliance
**CI Gate:** context-guard.yml checks for response.data() calls
**Status:** ✅ ACTIVE (enforced by context-guard.yml)

### L23 — No Cryptic Fallbacks
**Rule:** Stub implementations of external dependencies (CDP, network, FS) MUST throw descriptive errors naming: the unavailable capability, the required setup step, and the relevant env var or flag.
**Rationale:** "X is not a function" is a constitutional bug
**CI Gate:** Code review for error messages
**Status:** ⏳ PENDING (no CI)

### L24 — Agent Traffic Through MCP Bridge
**Rule:** All traffic between agents MUST pass through `trios-server` MCP bridge. Direct A2A calls bypassing broadcast are forbidden.
**Rationale:** Enables SSE observability
**CI Gate:** Network traffic validation
**Status:** ⏳ PENDING (no CI)

### L25 — Chrome Extension is Observability Channel
**Rule:** Chrome Extension (Trinity Agent Bridge) is the single visual channel for human observability. Any new tool MUST appear in sidepanel Tools tab automatically via `tools/list`.
**Rationale:** No hidden tool surfaces
**CI Gate:** Tool registration validation
**Status:** ⏳ PENDING (no CI)

---

## §4 Nine Kingdoms Invariants

| Invariant | Kingdom | Rule | CI Gate |
|-----------|---------|------|---------| 
| I1: Build passes | Rust | `cargo build --all --workspace` exits 0 | `cargo build --all --workspace` |
| I2: Tests pass | Test | No merge with failing tests | `cargo test --all --workspace` |
| I3: Clippy clean | Lint | clippy 0 warnings | `cargo clippy --all-targets -- -D warnings` |
| I4: Docs exist | Documentation | README.md and brand kit present | `test -f README.md && test -f docs/TRINITY_BRAND_KIT.md` |
| I5: No /extension root | Structure | No `/extension` directory at repo root | `test ! -d ./extension` |
| I6: Port 9005 works | Network | MCP server accessible on port 9005 | `nc -z localhost 9005` or equivalent |
| I7: No unsafe eval | Security | `wasm-unsafe-eval` only if declared | `! grep -r "wasm-unsafe-eval" crates/trios-ext/extension/manifest.json` |
| I8: MCP bridge online | Protocol | MCP server responds to /health | MCP health endpoint check |
| I9: Experience current | Experience | Daily experience log exists | `.trinity/experience/trios_$(date +%Y%m%d).trinity` exists |

---

## §5 Issue Standards

### Classification

Every issue MUST include:

- **Priority:** P0-CRITICAL, P1-HIGH, P2-MEDIUM, or P3-LONG-TERM
- **Kingdom:** Rust, Test, Lint, Network, Structure, Security, Protocol, Identity, or Cross-kingdom
- **Soul-Name:** Humorous English name for the agent

### Priority Definitions

| Priority | SLA | Examples |
|----------|-----|----------|
| P0-CRITICAL | < 4 hours | Production break, security issue, release blocker |
| P1-HIGH | < 24 hours | Important feature, blocking bug, infra task |
| P2-MEDIUM | < 1 week | Improvement, refactor, documentation |
| P3-LONG-TERM | No SLA | Research, experiment, PhD work |

### Kingdoms

- **Rust:** Rust code changes
- **Test:** Test additions/modifications
- **Lint:** Linting or code quality
- **Network:** Network-related changes
- **Structure:** Repository structure
- **Security:** Security-related changes
- **Protocol:** Protocol or communication changes
- **Identity:** Authentication/authorization
- **Cross-kingdom:** Changes affecting multiple kingdoms

### PHI LOOP Status

Every issue tracks its position in the PHI LOOP:

1. **CLAIM** — Issue opened with acceptance criteria
2. **SPEC** — TASK.md filled, goal formalized
3. **SEAL** — TASK.md locked (no changes after)
4. **HASH** — SHA256(TASK.md) stored
5. **GEN** — Code written, tests written
6. **TEST** — `cargo test --all` PASSES
7. **VERDICT** — All acceptance criteria verified
8. **EXPERIENCE** — `.trinity/experience/` updated
9. **SKILL** — `.claude/skills/` updated if needed
10. **COMMIT** — Commit with "Closes #N"
11. **PUSH** — PR created and merged

### Comment Types

- **loop:** — Update PHI LOOP status
- **evidence:** — Provide evidence
- **rationale:** — Explain reasoning
- **verdict:** — Result of verification

---

## §6 Heartbeat Protocol

Canonical format (for /tri compatibility):

```
loop: <NATO_AGENT_NAME> | <STATUS> | <CONTEXT>
```

**Status values:**
- 🟢 ACTIVE — Agent is working
- 🟡 BLOCKED — Waiting for something
- 🔴 STUCK — Cannot proceed
- 🟢 DONE — Task completed
- ⏸ QUEUED — In queue

**Examples:**
```
loop: ALFA | ACTIVE | Phase B grid sweep {0.0, 0.01, 0.02}
loop: BRAVO | BLOCKED | Waiting for @gHashTag review on #123
loop: DELTA | DONE | L1 compliance verified, 3 files added
```

---

## §7 PHI LOOP+ Protocol

11-step workflow with entry/exit criteria:

1. **CLAIM** → Issue #N opened with acceptance criteria
2. **SPEC** → TASK.md filled, goal formalized
3. **SEAL** → TASK.md locked (no changes after this point)
4. **HASH** → SHA256(TASK.md) stored in `.trinity/state/`
5. **GEN** → Code written, tests written
6. **TEST** → `cargo test --all` PASSES
7. **VERDICT** → All acceptance criteria verified
8. **EXPERIENCE** → `.trinity/experience/*.md` updated
9. **SKILL** → `.claude/skills/*.md` updated if needed
10. **COMMIT** → Commit with "Closes #N"
11. **PUSH** → PR created and merged

**Exit condition:** Only through COMMIT+PUSH or explicit verdict FAILED.

---

## §8 Amendment Process

6-step procedure for changing LAWS.md:

**Step 1:** Create issue with `constitutional_amendment.yml` template
   - Required: affected laws, rationale, hash_before

**Step 2:** Discussion (minimum 24 hours)
   - Comments from @gHashTag
   - Risk assessment (LOW/MEDIUM/HIGH/CRITICAL)

**Step 3:** Create feature branch
   - `git checkout -b feat/laws-amend-<section>`
   - Develop changes

**Step 4:** Review + Approval
   - @gHashTag approval REQUIRED (CODEOWNERS)
   - 1 reviewer minimum
   - CI pass (laws-guard)

**Step 5:** Merge + Hash update
   - Merge to main
   - Update LAWS_HASH: `sha256sum LAWS.md > .trinity/state/LAWS_HASH`

**Step 6:** Experience record
   - Record in `.trinity/experience/laws-v2-amendment.md`

**Emergency bypass:** P0 tasks require 2 approvers instead of 1.

---

## §9 Agent Personhood

**Soul-name rules:**
- Format: Humorous English name related to task (per L11)
- Uniqueness: One soul-name per task
- Forbidden: Reuse after violation (blacklist)
- Automation: Required field in task_contract.yml
- Verification: Checked in task_issue template

**Examples:**
- "Justice League" — Building constitutional infrastructure
- "Speed Racer" — Performance optimization
- "Doc Ock" — Documentation task

---

## §10 Priority Matrix

| Priority | SLA | Response Time | Examples |
|----------|-----|---------------|----------|
| P0-CRITICAL | < 4 hours | Immediate | Production break, security issue, release blocker |
| P1-HIGH | < 24 hours | Same day | Important feature, blocking bug, infra task |
| P2-MEDIUM | < 1 week | Within week | Improvement, refactor, documentation |
| P3-LONG-TERM | No SLA | As available | Research, experiment, PhD work |

---

## §11 Law Status Dashboard

L1-L25 with current status (live-checked by /tri laws):

| Law | Description | CI Enforcement | Status |
|-----|-------------|----------------|--------|
| L1 | NO .sh files | ✅ ci.yml | ✅ PASS |
| L2 | Every PR closes an issue | ⏳ PENDING | ⏳ CHECK |
| L3 | clippy zero warnings | ✅ ci.yml | ✅ PASS |
| L4 | Tests before merge | ✅ ci.yml | ✅ PASS |
| L5 | Port 9005 is trios-server | ⏳ PENDING | ⏳ CHECK |
| L6 | Fallback required for GB tools | ⏳ PENDING | ⏳ CHECK |
| L7 | Experience log | ⏳ PENDING | ⏳ CHECK |
| L8 | Push first | ⏳ PENDING | ⏳ CHECK |
| L9 | No handwritten forbidden surface | ⏳ PENDING | ⏳ CHECK |
| L10 | Issue #143 is eternal | ⏳ PENDING | ⏳ CHECK |
| L11 | Name before mutation | ⏳ PENDING | ⏳ CHECK |
| L12 | Spec before implementation | ⏳ PENDING | ⏳ CHECK |
| L13 | Bounded authority | ⏳ PENDING | ⏳ CHECK |
| L14 | Auditability by default | ⏳ PENDING | ⏳ CHECK |
| L15 | Validation is a separate duty | ⏳ PENDING | ⏳ CHECK |
| L16 | Tailoring requires rationale | ⏳ PENDING | ⏳ CHECK |
| L17 | Improve code health, not heroics | ⏳ PENDING | ⏳ CHECK |
| L18 | Structured conflict resolution | ⏳ PENDING | ⏳ CHECK |
| L19 | Humans remain sovereign | ⏳ PENDING | ⏳ CHECK |
| L20 | Turn sessions into tools | ⏳ PENDING | ⏳ CHECK |
| L21 | Context immutability | ✅ context-guard.yml | ✅ PASS |
| L22 | Schema-response parity | ✅ context-guard.yml | ✅ PASS |
| L23 | No cryptic fallbacks | ⏳ PENDING | ⏳ CHECK |
| L24 | Agent traffic through MCP bridge | ⏳ PENDING | ⏳ CHECK |
| L25 | Chrome Extension is observability channel | ⏳ PENDING | ⏳ CHECK |

---

## §12 Closing Clause

This constitution represents the sovereign law of the trios repository. Any violation constitutes a breach of trust between agents and human maintainers. The sanctity of this document is protected by CODEOWNERS, branch protection, CI gates, and cryptographic hash verification. Amendments follow §8 procedure only.

**AMENDMENT TARGET:** LAWS.md v1.x → v2.0

---

## §13 Agent Dispatch Onboarding

> *New agent? Start here. Everything you need to run autonomously is in one file.*

### The ONE-SHOT Dispatch Prompt

**Location:** `.trinity/prompts/agent-dispatch.md`

This is the canonical entry point for every Trinity agent. Copy-paste it to start any autonomous session. It contains:

| Component | What it does |
|-----------|-------------|
| **BOOT sequence** | Reads SOUL.md → CLAUDE.md → AGENTS.md → NOW.json → state files |
| **PHI LOOP (11 steps)** | CLAIM → SPEC → SEAL → HASH → GEN → TEST → VERDICT → EXPERIENCE → SKILL → COMMIT → PUSH |
| **LAWS L1–L9 embedded** | Most critical laws inlined so agent can't miss them |
| **HEARTBEAT format** | `loop: <NATO_NAME> \| <STATUS> \| <CONTEXT>` — post every 30 min |
| **DONE checklist (8 gates)** | Prevents premature victory declaration |
| **Soul-name selection** | Agent picks NATO alphabet name autonomously on first run |

### Boot Sequence (Memorise This)

```bash
# Step 1 — Read the constitution
cat SOUL.md CLAUDE.md AGENTS.md NOW.json

# Step 2 — Load current state
cat .trinity/state/active-skill.json
cat .trinity/state/three-roads.json

# Step 3 — Orient
git log --oneline -5
gh issue list --limit 5
```

### PHI LOOP Quick Reference

```
CLAIM → SPEC → SEAL → HASH → GEN → TEST → VERDICT → EXPERIENCE → SKILL → COMMIT → PUSH
```

Full protocol: see §7 of this document.

### HEARTBEAT Protocol Quick Reference

Post to issue every 30 minutes while working:
```
loop: ALFA | ACTIVE | Writing BpbSink trait, 3/5 tests pass
loop: BRAVO | BLOCKED | Waiting for SR-00 merge to unblock SR-01
loop: GOLF | DONE | 16/16 tests GREEN, PR #474 merged
```

Full protocol: see §6 of this document.

### DONE Checklist (8 Gates)

Before declaring completion, verify ALL of these:

- [ ] `cargo test --all` GREEN (no failures, no ignores)
- [ ] `cargo clippy -- -D warnings` ZERO warnings
- [ ] PR description contains `Closes #N`
- [ ] L14 trailer present: `Agent: <Soul-name>` in commit
- [ ] `.trinity/experience/` entry written (L7)
- [ ] `three-roads.json` written with R1/R2/R3
- [ ] Issue comment posted with verdict
- [ ] `git push` completed (L8)

### Common Mistakes (from `.trinity/experience/`)

| Mistake | Prevention |
|---------|-----------|
| Pushing to wrong remote (e.g. gitbutler upstream) | Check `git remote -v` before push — must show `gHashTag/trios` |
| Missing `Closes #N` in PR | Use `gh pr create` template, not manual |
| Declaring done before `three-roads.json` | Write roads BEFORE final commit |
| Working directory deleted mid-session | Use `/tmp/` as fallback; push via MCP if needed |
| Orphan branch (no common history with main) | Always `git checkout -b ring-NNN origin/main`, not from detached HEAD |

### Source of Truth Hierarchy for Agents

```
NEON (Postgres)          ← igla_race_experience, strategy_queue, bpb_samples
.trinity/state/          ← active-skill.json, three-roads.json, LAWS_HASH
.trinity/experience/     ← daily logs, mistake memory
.trinity/prompts/        ← agent-dispatch.md (THIS prompt)
GitHub Issues            ← PHI LOOP status comments, HEARTBEAT
```

**Remember:** NEON is the PhD source of truth. Every BPB measurement, every ASHA decision, every experience entry lives there permanently — not just in context window.
