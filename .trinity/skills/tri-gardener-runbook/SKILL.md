---
name: tri-gardener-runbook
description: "Master orchestration runbook for the gHashTag Trinity ecosystem. Use when the user mentions tri-gardener, EPIC #446 ring-refactor, IGLA RACE marathon (Gate-2 BPB<1.85, Gate-3 BPB<1.5), ONE-SHOT v2.0 dispatch, parallel agents (ALPHA/BETA/GAMMA/DELTA/ZETA/LEAD), CLAIM/heartbeat protocol, anti-collision, NEON ssot.chapters (44 chapters SSOT), R14 citation batch, App.K Agent Memory, INV-13 born-Proven policy, admitted_budget 5/5, GOLD I/II/III/IV ring scaffolds (trios-igla-race-pipeline / trios-algorithm-arena / trios-igla-race-hack / trios-agent-memory), gardener_runs, ASHA rungs, plan-21, plateau alert, GARDENER_LIVE, GARDENER_DISABLED, parameter-golf E2E TTT, val_bpb < 1.07063, или просит запустить / переключить / откатить / посмотреть логи гарденера, открыть PR на ring, проверить notifications dashboard, выбрать issue из priority queue. Anchor: phi^2 + phi^-2 = 3."
license: Apache-2.0
metadata:
  author: PERPLEXITY-MCP
  version: '2.1'
  parent_repo: gHashTag/trios
  epic: '446'
  oneshot_issue: '236'
  parent_runbook_repo: gHashTag/trios-railway
  ssot_doi: 10.5281/zenodo.19227877
  ssot_label: B007 HSLM Benchmark Corpus
  root_anchor: phi^2 + phi^-2 = 3
---

# tri-gardener-runbook v2.0 — Trinity Master Orchestration

Universal runbook for the entire gHashTag Trinity ecosystem. Covers:

- **EPIC #446** — 3 GOLD × 19 SR ring-pattern refactor (`trios-igla-race-pipeline`, `trios-algorithm-arena`, `trios-igla-race-hack`, plus GOLD IV `trios-agent-memory`).
- **ONE-SHOT v2.0 dispatch** — 7 codename agents (ALPHA / BETA / GAMMA / DELTA / EPSILON / ZETA / LEAD), CLAIM-before-mutate, heartbeat board, branch namespace.
- **tri-gardener** — Rust orchestrator at [`bin/tri-gardener`](https://github.com/gHashTag/trios-railway) driving fleet through Gate-2 (BPB<1.85) and Gate-3 (BPB<1.5).
- **NEON SSOT** — `ssot.chapters` (44 entries), mirrored at [`docs/golden-sunflowers/README.md`](https://github.com/gHashTag/trios/blob/main/docs/golden-sunflowers/README.md).
- **R14 citation batch** ([#464](https://github.com/gHashTag/trios/issues/464)) and **App.K Agent Memory** ([#465](https://github.com/gHashTag/trios/issues/465)).
- **Notifications dashboard** ([github.com/notifications](https://github.com/notifications)) with repo watch ON for `gHashTag/trios`.

Anchor: `phi² + phi⁻² = 3` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877) (B007 HSLM Benchmark Corpus, **not** root anchor).

## When to Use This Skill

Load this skill whenever the user asks to:

### Ring-pattern / EPIC #446
- Open or comment on EPIC #446 or any sub-issue [#447..#465](https://github.com/gHashTag/trios/issues/446).
- Start a ring scaffold (SR-NN, SR-ALG-NN, SR-HACK-NN, SR-MEM-NN, BR-OUTPUT).
- Verify GOLD facade (`src/lib.rs` ≤ 50 LoC, re-exports only, L-ARCH-001).
- Check I5 trinity (`README.md + TASK.md + AGENTS.md` per ring) or I-SCOPE compliance.
- Check `trios-doctor` rules `R-RING-FACADE/DEP/FLOW/BR/MCP/L1/L6/COQ` ([#462](https://github.com/gHashTag/trios/issues/462)).

### ONE-SHOT v2.0 dispatch / parallel agents
- Spawn an agent (ALPHA / BETA / GAMMA / DELTA / ZETA / LEAD).
- Pull next task from the live priority queue.
- Resolve a CLAIM tie or detect collision.
- Post a heartbeat in the canonical format.
- Detect stuck agents (🔴 STUCK > 30 min) or rogue commits (without `Agent: <CODENAME>` trailer).

### tri-gardener operations
- Run, deploy, or smoke-test `tri-gardener` (locally or on Railway).
- Apply the `gardener_runs` Neon DDL.
- Switch the gardener between `--review`, `--dry-run`, `--live`.
- Roll back a live tick or kill the gardener.
- Inspect gardener logs, decisions, plateau alerts, or culls in Neon.
- Diagnose blockers: missing `set-vars` / `logs` / `stop` subcommands, missing GHCR image, missing cron, PR-2 wiring gap.

### NEON SSOT / R14 / App.K
- Edit a chapter in NEON `ssot.chapters.body_md` (never edit `docs/phd/chapters/*.tex` directly).
- Add `\citetheorem{<coq_name>}` or `\citeAxiom{<name>}` macros.
- Open / track [#464](https://github.com/gHashTag/trios/issues/464) R14 batch or [#465](https://github.com/gHashTag/trios/issues/465) App.K Agent Memory.
- Verify `admitted_budget = 5/5 LOCKED` or monograph ledger 297 Qed + 141 admit/sorry + 0 Axiom.
- Apply INV-13 born-Proven OR axiom-approach policy for any new INV-N (N>12).

### Notifications dashboard
- Verify repo watch ON for `gHashTag/trios`.
- Tail unread notifications, filter by `Part of #446` / `IN-FLIGHT —` / `VERDICT`.
- Pin or unpin EPIC issues (max 3 pins per repo).

### Out of scope
- Generic Railway service deploys → use `tri railway plan9` from PR #47.
- Trainer model code → lives in `gHashTag/trios-trainer-igla` per its [SOURCE_OF_TRUTH.md](https://github.com/gHashTag/trios-trainer-igla/blob/main/SOURCE_OF_TRUTH.md).
- LAWS.md edits → require §8 amendment procedure, separate skill.

## §1 EPIC #446 — Ring-Pattern Refactor (3 GOLD × 19 SR)

### Architecture

```
🥇 GOLD I  trios-igla-race-pipeline
   SR-00 scarab-types · SR-01 strategy-queue · SR-02 trainer-runner
   SR-03 bpb-writer  · SR-04 gardener       · SR-05 railway-deployer
   BR-OUTPUT  IglaRacePipeline  (run_e2e_ttt_o1())

🥇 GOLD II trios-algorithm-arena       (Variant A: Rust spec/verifier over train_gpt.py)
   SR-ALG-00 arena-types · SR-ALG-01 jepa · SR-ALG-02 universal-transformer
   SR-ALG-03 e2e-ttt ⭐ WIN · SR-ALG-04 phi-nta · SR-ALG-05 ssm · SR-ALG-06 megakernels
   BR-OUTPUT  AlgorithmArena  (run_one(algo, seed) → Bpb)

🥇 GOLD III trios-igla-race-hack
   SR-HACK-00 glossary · SR-HACK-01 pr-bot · SR-HACK-02 leaderboard-mirror
   SR-HACK-03 invite   · SR-HACK-04 equal-billing-coq · SR-HACK-05 discord-bridge
   BR-OUTPUT  IglaRaceHack

🥇 GOLD IV trios-agent-memory          (anti-amnesia, KG-backed)
   SR-MEM-00 memory-types · SR-MEM-01 kg-client-adapter · SR-MEM-02 ingest-pipeline
   SR-MEM-03 recall-pipeline · SR-MEM-04 reflect-engine · SR-MEM-05 episodic-bridge
   SR-MEM-06 vsa-embedder
   BR-OUTPUT  AgentMemory trait (recall / remember / reflect / forget)
```

Dependency flow (R1, strict): `BR-OUTPUT → SR-NN → ... → SR-00`. SR-00 deps limited to `serde + serde_json + uuid + chrono + sha2`.

### 17 sub-issues — priority queue snapshot

| Wave | Issue | Codename | Priority | Blocked by |
|---:|---|---|---|---|
| 1 | [#447](https://github.com/gHashTag/trios/issues/447) SR-HACK-00 glossary | GAMMA | P2 | — |
| 1 | [#448](https://github.com/gHashTag/trios/issues/448) SR-00 scarab-types | ALPHA | **P1** | — |
| 1 | [#449](https://github.com/gHashTag/trios/issues/449) SR-MEM-00 memory-types | ZETA | P2 | — |
| 1 | [#450](https://github.com/gHashTag/trios/issues/450) SR-ALG-00 arena-types | BETA | P2 | — |
| 1 | [#462](https://github.com/gHashTag/trios/issues/462) SILVER-RING-DR-04 doctor rules | DELTA | P2 | — |
| 1 | [#464](https://github.com/gHashTag/trios/issues/464) R14 citation batch | LEAD | P2 | — |
| 1 | [#465](https://github.com/gHashTag/trios/issues/465) App.K Agent Memory | LEAD | P2 | — |
| 2 | [#451](https://github.com/gHashTag/trios/issues/451) SR-03 bpb-writer | ALPHA | P1 | #448 |
| 2 | [#452](https://github.com/gHashTag/trios/issues/452) SR-01 strategy-queue | ALPHA | P1 | #448 |
| 2 | [#453](https://github.com/gHashTag/trios/issues/453) SR-MEM-01 kg-client-adapter | ZETA | P2 | #449 |
| 3 | [#454](https://github.com/gHashTag/trios/issues/454) SR-02 trainer-runner | ALPHA | P1 | #448 #451 #452 |
| 3 | [#455](https://github.com/gHashTag/trios/issues/455) SR-MEM-05 episodic-bridge | ZETA | P2 | #453 |
| 3 | [#456](https://github.com/gHashTag/trios/issues/456) SR-04 gardener | ALPHA | P1 | #451 #452 #454 |
| ⭐ | [#457](https://github.com/gHashTag/trios/issues/457) SR-ALG-03 e2e-ttt **WIN** | BETA | **P0** | #450 #454 |
| 4 | [#458](https://github.com/gHashTag/trios/issues/458) SR-05 railway-deployer | ALPHA | P2 | #456 |
| 4 | [#459](https://github.com/gHashTag/trios/issues/459) BR-OUTPUT IglaRacePipeline | ALPHA | P1 | all SR |
| 4 | [#460](https://github.com/gHashTag/trios/issues/460) BR-OUTPUT AlgorithmArena | BETA | P2 | #457 |
| 4 | [#461](https://github.com/gHashTag/trios/issues/461) BR-OUTPUT AgentMemory | ZETA | P2 | #455 |
| 5 | [#463](https://github.com/gHashTag/trios/issues/463) Long-tail tracker | LEAD | P3 | core stable |

**Live regeneration:**
```bash
gh issue list --repo gHashTag/trios --state open \
  --search "in:title SR- OR in:title BR-OUTPUT OR in:title SILVER-RING-DR" \
  --json number,title,labels,comments \
  | jq -r 'sort_by(.number) | .[] | "#\(.number) \(.title)"'
```

## §2 ONE-SHOT v2.0 — Parallel Dispatch (anti-collision)

### Three rules

1. **One agent ↔ one ring ↔ one crate** (I-SCOPE per [`AGENTS.md`](https://github.com/gHashTag/trios/blob/main/AGENTS.md)).
2. **CLAIM-before-mutate** — post `IN-FLIGHT — Soul: <name> · Codename: <CODENAME> · Branch: bee/<issue>-<slug>`, wait 60 s for ties; earliest timestamp wins, ties broken alphabetically.
3. **Append-only context** (L21) — never delete or rewrite another agent's heartbeat or experience entry.

### Codename → crate mapping

| Codename | Owns crate(s) | Day-1 issue |
|---|---|---|
| **LEAD** | meta / orchestration | #446 #236 #462 #463 #464 #465 |
| **ALPHA** | GOLD I `trios-igla-race-pipeline` | #448 |
| **BETA** | GOLD II `trios-algorithm-arena` | #450 |
| **GAMMA** | GOLD III `trios-igla-race-hack` | #447 |
| **DELTA** | `trios-doctor/SILVER-RING-DR-04` | #462 |
| **ZETA** | GOLD IV `trios-agent-memory` | #449 |
| **EPSILON** | `trios-ext` | (out of EPIC scope) |

**Day-1 zero-collision parallel start:** GAMMA #447 ‖ ALPHA #448 ‖ ZETA #449 ‖ BETA #450 ‖ DELTA #462 ‖ LEAD #464 ‖ LEAD #465 — 7 agents, disjoint crates.

### CLAIM workflow (every agent)

```bash
# 1. Pull queue
gh issue list --repo gHashTag/trios --state open \
  --search "in:title SR- OR in:title BR-OUTPUT" \
  --json number,title,body,comments

# 2. Filter: lowest-numbered, in domain, not claimed, blockers closed, P0→P1→P2→P3.

# 3. POST claim (mandatory exact format, one line):
gh issue comment <NUMBER> --repo gHashTag/trios \
  --body "IN-FLIGHT — Soul: <SOUL> · Codename: <CODENAME> · Branch: bee/<NUMBER>-<slug>"

# 4. Wait 60 s, re-fetch comments.
gh issue view <NUMBER> --repo gHashTag/trios --json comments

# 5. If earlier-timestamp CLAIM from different codename → abort, restart at step 1.
```

### Heartbeat format (canonical)

```
loop: <CODENAME> | <🟢ACTIVE|🟡BLOCKED|🔴STUCK|🟢DONE|⏸QUEUED> | <soul · phi-step> · <context ≤ 80 chars>

evidence:
  ts:               <ISO-8601 UTC>
  issue:            #<ISSUE>
  epic:             #446
  loop:             CLAIM|NAME|SPEC|SEAL|GEN|TEST|VERDICT|EXPERIENCE|REPORT|COMMIT|PUSH
  task_md_sha256:   <hash from SEAL>
  branch:           bee/<ISSUE>-<slug>
  next:             <next irreversible action>
```

### PHI LOOP (LAWS.md §7) — 11 mandatory steps

1. **CLAIM** — done in §STEP 1.
2. **NAME** — soul-name fixed in §STEP 1.
3. **SPEC** — `<ring>/TASK.md` written from issue acceptance.
4. **SEAL** — `sha256sum <ring>/TASK.md` recorded in heartbeat.
5. **GEN** — implement only acceptance items; ship I5 trinity (README+TASK+AGENTS); add `\citetheorem{}` to NEON `ssot.chapters.body_md` if ring binds a Proven INV.
6. **TEST** — `cargo clippy --all-targets -- -D warnings` (L3) + `cargo test --all` (L4) + `trios-doctor check`.
7. **VERDICT** — ✅ CLEAN | ⚠️ RISKY | ❌ TOXIC.
8. **EXPERIENCE** — append to `.trinity/experience/<ISSUE>-<SOUL>.md` (L7, L21).
9. **REPORT** — final HEARTBEAT comment.
10. **COMMIT** — `git commit -m "feat(<crate>): <one-liner>" --trailer "Agent: <CODENAME>"`; body contains `Closes #<ISSUE> · Part of #446`.
11. **PUSH** — `git push -u origin bee/<ISSUE>-<slug>`; open PR with same `Closes #N`.

### DONE checklist (every box must be true)

- [ ] `cargo clippy --all-targets -- -D warnings` = 0
- [ ] `cargo test --all` = all pass
- [ ] `trios-doctor check` = 0 errors
- [ ] `git status` = 0 modified/untracked
- [ ] commit `Agent: <CODENAME>` trailer visible
- [ ] I5 trinity present (README + TASK + AGENTS)
- [ ] `task_md_sha256` sealed in heartbeat
- [ ] `\citetheorem{}` added to NEON if ring binds a Proven INV (R14)
- [ ] `.trinity/experience/<ISSUE>-<SOUL>.md` committed
- [ ] PR open with `Closes #<ISSUE>` + `Part of #446`
- [ ] Final heartbeat with VERDICT
- [ ] `admitted_budget` unchanged (5/5)
- [ ] ledger admit/sorry unchanged (141)

### Collision handling §STEP 6

- **Same issue, two CLAIMs** → earliest timestamp wins; loser exits + restarts.
- **Same crate, two agents** → forbidden by I-SCOPE; if it happens, STOP, post 🔴 STUCK, escalate to LEAD.
- **Blocker not closed** → re-fetch in 5 min; if still blocked → exit ⏸ QUEUED.
- **ALPHA backward-compat re-export** — only one ALPHA at a time may touch monolith `crates/trios-igla-race/src/`. Coordinate via #446 comments.
- **NEON write conflict** — soft lock: comment `NEON-LOCK chapter:<slug> by <CODENAME>` on #464 or #465 before editing. Release with `NEON-UNLOCK chapter:<slug>`.

### Escalation §STEP 7

If 🔴 STUCK > 30 min → comment on EPIC #446 mentioning @gHashTag with the heartbeat block + the question that needs human resolution. Do NOT keep retrying the same failing path.

## §3 LEAD orchestrator responsibilities

LEAD does NOT do ring work. LEAD watches and coordinates:

1. **Repo watch ON** (already set: `repos/gHashTag/trios/subscription` → `subscribed=true, ignored=false`).
2. **Poll [github.com/notifications](https://github.com/notifications) every 5 min:**
   ```bash
   gh api '/notifications?all=false' \
     --jq '.[] | select(.repository.full_name | contains("trios"))
            | "\(.updated_at[:16]) | \(.subject.type) | \(.subject.title[:80])"'
   ```
3. **Maintain live queue table** in EPIC #446 tracker comment whenever a sub-issue closes.
4. **Detect stuck agents** — any 🔴 STUCK heartbeat or any IN-FLIGHT > 4 h without heartbeat → ping; escalate to @gHashTag if no response in 30 min.
5. **Detect rogue commits** — any commit without `Agent: <CODENAME>` trailer → comment on PR, request amendment.
6. **Run `trios-doctor check` nightly** — post diff to #446 if new errors.

## §4 NEON SSOT — `ssot.chapters` (44 entries)

### Editing rules

- **Single source of truth = NEON `ssot.chapters`** (44 rows: 34 chapters Ch.01..Ch.34 + 10 appendices App.A..App.J).
- **Mirror** at [`docs/golden-sunflowers/README.md`](https://github.com/gHashTag/trios/blob/main/docs/golden-sunflowers/README.md).
- Edits land in NEON `ssot.chapters.body_md`, propagated by `v4/generate_from_neon.py` to `docs/golden-sunflowers/*.md`.
- **`docs/phd/chapters/*.tex` is auto-generated** — never edit it directly.
- **44 illustrations** (1200×800 triptych per entry) at [`assets/illustrations/`](https://github.com/gHashTag/trios/tree/main/assets/illustrations).

### Coq theorem registry — exact names from PDF v4

| INV | Coq theorem | Status | NEON chapter |
|---|---|---|---|
| INV-1 | `bpb_decreases_with_real_gradient` / `INV1BpbMonotoneBackward` | Admitted+9 Qed | Ch.15 BPB benchmark + Neon write |
| INV-2 | `asha_champion_survives` (`IglaAshaBound`) | **Proven** | Ch.13 STROBE Sealed seeds + Ch.21 IGLA RACE |
| INV-3 | `gf16_safe_domain` | Admitted (Lucas n=1,2 Qed) | Ch.9 GF vs MXFP4 ablation + Ch.10 Coq L1 Pareto |
| INV-4 | `nca_entropy_stability` (`INV4NcaEntropyBand`) | Admitted+12 Qed | Ch.16 360-lane phi-distance grid |
| INV-5 | `lucas_closure_gf16` | Admitted (n=0,1 Qed) | Ch.5 phi-distance and Fibonacci-Lucas seeds |
| INV-6 | `ema_decay_valid` | TODO | Ch.19 Statistical analysis (Welch-t) |
| INV-7 | `igla_found_criterion` / `victory_implies_distinct_clean` (4 Qed Theorems + 5 Qed Examples) + `welch_ttest_alpha_001_rejects_baseline` (1 honest Admitted) | Mixed Proven+Admitted | Ch.24 Period-Locked Runtime Monitor + Ch.21 IGLA RACE |
| INV-8 | `seven_channels_total` / `rainbow_bridge_consistency` | Admitted | Ch.30 Trinity SAI (VSA + AR) |
| INV-9 | `qk_gain_phi_sq` / `admit_phi_sq` | TODO | Ch.8 TF3/TF9 |
| INV-12 | `asha_rungs_trinity` | **Proven** | Ch.22 Railway / Trios orchestration (10 Qed worker-pool) |

### Counter accounting (do not confuse the two)

- **Repo registry** (`assertions/igla_assertions.json._metadata`):
  - 47 Proven Qed in `igla` lane
  - 5 honest Admitted (`descent_lemma`, `bpb_smooth`, `gradient_norm_pos` for INV-1; `phi_pow_to_lucas` for INV-5; `welch_ttest_alpha_001_rejects_baseline` for INV-7)
  - **`admitted_budget = 5/5 LOCKED`** for the IGLA invariants. Any new Admitted in this lane = breach.
- **Monograph ledger** (App.B Golden Ledger, PDF v4):
  - **297 Qed canonical**
  - **141 open admit/sorry**
  - **0 Axiom**
  - 65 `.v` files monograph-wide.

### INV-13 born-Proven OR axiom-approach policy

Any new `INV-N` (N>12) introduced by EPIC #446 MUST satisfy one of:

1. **Born Proven** — `Qed.`, registered in `igla_assertions.json`, cited in NEON `ssot.chapters.body_md` via `\citetheorem{}`, R7 falsification witness present.
2. **Axiom approach** — `Axiom` keyword, registered with `status: "axiom"` in `igla_assertions.json`, cited via `\citeAxiom{}`, NEVER counts toward repo `admitted_budget` (5/5) or monograph ledger admit/sorry (141).

No new INV may consume the 5/5 IGLA-registry slot or add to the 141 ledger admit/sorry. Not retroactive for INV-7. Enforcement: `trios-doctor` rule `R-INV13` (warn → error after T+30d).

## §5 100% width image embedding (GitHub markdown)

Repo illustrations are 1200×800 PNGs — render too small at default width. Always use full-width HTML inside issue/PR comments:

```markdown
<p align="center">
  <img src="https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/<file>.png"
       alt="<descriptive-alt>" width="100%">
</p>
```

Available files (44): `cover_v4.png`, `ch01..ch34-*.png`, `app-a..app-j-*.png`. Use raw GitHub URL form `https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/<name>.png`.

## §6 Notifications dashboard ([github.com/notifications](https://github.com/notifications))

### Setup (one-time)

```bash
# Repo-level watch all activity
gh api -X PUT 'repos/gHashTag/trios/subscription' \
  -F subscribed=true -F ignored=false
```

### Daily filters

| Filter | Purpose |
|---|---|
| `repo:gHashTag/trios in:title "Part of #446"` | All EPIC traffic |
| `repo:gHashTag/trios in:title "IN-FLIGHT —"` | Live claims stream |
| `repo:gHashTag/trios in:title "VERDICT"` | Completion stream |
| `repo:gHashTag/trios is:participating` | Anything you're tagged in |

### Tail unread (via CLI)

```bash
gh api '/notifications?all=false&per_page=20' \
  --jq '.[] | select(.repository.full_name | contains("trios"))
         | "\(.updated_at[:16]) | \(.subject.type) | #\(.subject.title)"'
```

### Pin slots

Repo allows max **3 pinned issues**. Currently pinned: #143 IGLA RACE (eternal, L10), #235 LAWS.md v2.0, #264 Trinity Hive. EPIC #446 is NOT pinned by default — repo watch covers all events.

To pin #446 (requires unpinning one of the above):

```bash
gh api graphql -f query='
mutation($id: ID!) { unpinIssue(input: { issueId: $id }) { issue { number } } }' \
  -f id="$(gh api repos/gHashTag/trios/issues/<TO_UNPIN> --jq .node_id)"

gh api graphql -f query='
mutation($id: ID!) { pinIssue(input: { issueId: $id }) { issue { number isPinned } } }' \
  -f id="$(gh api repos/gHashTag/trios/issues/446 --jq .node_id)"
```

## §7 tri-gardener — copy-paste runbook (Rust orchestrator)

Operational runbook for the autonomous Rust orchestrator at [`bin/tri-gardener`](https://github.com/gHashTag/trios-railway) driving the IGLA Race fleet through Gate-2 (BPB < 1.85) and onward to Gate-3 (BPB < 1.5). Crate parented under [tracker #43](https://github.com/gHashTag/trios-railway/issues/43), specced in [#49](https://github.com/gHashTag/trios-railway/issues/49), implemented in [draft PR #50](https://github.com/gHashTag/trios-railway/pull/50).

### Snapshot

| Field | Value |
|---|---|
| Crate | `bin/tri-gardener` in `gHashTag/trios-railway` |
| Branch | `feat/tri-gardener` |
| PR | [trios-railway#50](https://github.com/gHashTag/trios-railway/pull/50) (draft, PR-1 of 2) |
| Tests | `cargo test -p tri-gardener` → 15/15 GREEN |
| Subcommands | `tri-gardener once {--dry-run, --review, --live}`, `tri-gardener ddl` |
| Cron cadence | `:15 UTC` hourly (matches audit-watchdog) |
| Target Railway slot | Acc1 IGLA, last free service slot |
| Decision modes | `Review` (first 3 ticks) → `DryRun` → `Live` (gated by `GARDENER_LIVE=true`) |
| Kill switch | `GARDENER_DISABLED=true` → immediate `Noop` |
| Live wiring | **Stub in PR-1.** PR-2 wires `tri-railway-core::Client` mutations + `tokio_postgres` writes. |

### Spin-up decision (where to host)

| Option | When | Risk |
|---|---|---|
| **Local crontab via `tri-gardener serve --interval=3600`** (RECOMMENDED) | Right now, before PR-2 / GHCR / multi-account land | Lowest |
| **Acc1 last free service slot** | After PR-2 (#58) merges, Acc1-only `--review` | Medium |
| **Acc2 / Acc3** | ONLY after #61 RailwayMultiClient merges | Currently unsafe |

### Five-step copy-paste runbook

#### 1. Local smoke-test (`--review`, no Neon)

```bash
cd /path/to/trios-railway && git checkout feat/tri-gardener
cargo build -p tri-gardener --release
./target/release/tri-gardener once --review
./target/release/tri-gardener once --dry-run     # decisions JSON, no I/O
GARDENER_DISABLED=true ./target/release/tri-gardener once --review     # verify kill switch
```

#### 2. Neon DDL apply (one-time)

```bash
./target/release/tri-gardener ddl > /tmp/gardener_ddl.sql
psql "$NEON_DATABASE_URL" -f /tmp/gardener_ddl.sql
psql "$NEON_DATABASE_URL" -c "\dt gardener_runs" \
  && psql "$NEON_DATABASE_URL" -c "\d gardener_runs"
```

#### 3. Railway service spin-up in Acc1

```bash
export RAILWAY_TOKEN=<acc1-user-token>     # never paste in chat / commit

tri railway service deploy --account=acc1 --name=gardener \
  --image=ghcr.io/ghashtag/tri-gardener:latest \
  --var GARDENER_LIVE=false --var GARDENER_DISABLED=false --var RUST_LOG=info

tri railway service set-vars --account=acc1 --service=gardener \
  --var NEON_DATABASE_URL="$NEON_DATABASE_URL" \
  --var RAILWAY_API_TOKEN_ACC0="$RAILWAY_API_TOKEN_ACC0" \
  --var RAILWAY_PROJECT_ID_ACC0=265301ce-0bf2-4187-a36f-348b0eb9942f \
  --var RAILWAY_ENVIRONMENT_ID_ACC0=f3517e98-c11a-49d8-b5fd-4cbb82d04384 \
  --var RAILWAY_TOKEN_KIND_ACC0=project

tri railway service set-vars --account=acc1 --service=gardener \
  --var RAILWAY_API_TOKEN_ACC1="$RAILWAY_API_TOKEN_ACC1" \
  --var RAILWAY_PROJECT_ID_ACC1=e4fe33bb-3b09-4842-9782-7d2dea1abc9b \
  --var RAILWAY_ENVIRONMENT_ID_ACC1=54e293b9-00a9-4102-814d-db151636d96e \
  --var RAILWAY_TOKEN_KIND_ACC1=user

tri railway service set-vars --account=acc1 --service=gardener \
  --var RAILWAY_API_TOKEN_ACC2="$RAILWAY_API_TOKEN_ACC2" \
  --var RAILWAY_PROJECT_ID_ACC2=39d833c1-4cb6-4af9-b61b-c204b6733a98 \
  --var RAILWAY_ENVIRONMENT_ID_ACC2=bce42949-d4ab-43d9-89d1-a6fcc576f45a \
  --var RAILWAY_TOKEN_KIND_ACC2=project

tri railway service set-vars --account=acc1 --service=gardener \
  --var CRON_SCHEDULE="15 * * * *"

tri railway service redeploy --account=acc1 --service=gardener
tri railway service status --account=acc1 --service=gardener
```

#### 4. `--review` → `--live` switch (3-tick gate + rollback)

Verify review window is healthy:

```bash
psql "$NEON_DATABASE_URL" \
  -c "SELECT count(*) AS review_ticks, max(ts) AS last
      FROM gardener_runs
      WHERE ts > now() - interval '4 hours';"
psql "$NEON_DATABASE_URL" \
  -c "SELECT ts, action, lane, seed, decision FROM gardener_runs ORDER BY ts DESC LIMIT 30;"
```

Promote to live:

```bash
tri railway service set-vars --account=acc1 --service=gardener --var GARDENER_LIVE=true
tri railway service redeploy --account=acc1 --service=gardener
```

Rollback (any of three, in order of reaction time):

```bash
tri railway service set-vars --account=acc1 --service=gardener --var GARDENER_DISABLED=true \
  && tri railway service redeploy --account=acc1 --service=gardener     # immediate noop
tri railway service set-vars --account=acc1 --service=gardener --var GARDENER_LIVE=false \
  && tri railway service redeploy --account=acc1 --service=gardener     # back to dry-run
tri railway service stop --account=acc1 --service=gardener               # nuclear
```

#### 5. Logs & runs

```bash
tri railway service logs --account=acc1 --service=gardener --tail=200
tri railway service logs --account=acc1 --service=gardener --follow

psql "$NEON_DATABASE_URL" \
  -c "SELECT ts, tick_t_minus, action, lane, seed, before_bpb, after_bpb
      FROM gardener_runs ORDER BY ts DESC LIMIT 50;"
psql "$NEON_DATABASE_URL" \
  -c "SELECT date_trunc('hour', ts) AS h, action, count(*)
      FROM gardener_runs WHERE ts > now() - interval '24 hours'
      GROUP BY 1, 2 ORDER BY 1 DESC, 3 DESC;"
psql "$NEON_DATABASE_URL" \
  -c "SELECT * FROM gardener_runs WHERE action='plateau' ORDER BY ts DESC LIMIT 10;"
psql "$NEON_DATABASE_URL" \
  -c "SELECT seed, lane, before_bpb, after_bpb, ts FROM gardener_runs
      WHERE action='cull' ORDER BY ts DESC;"
```

### Honest blockers

| Step | Depends on | Current state |
|---|---|---|
| `tri railway service set-vars` | `set-vars` subcommand in `bin/tri-railway` | **Not implemented.** Workaround: pass all `--var` triplets in one `deploy` call, or use Railway dashboard UI. |
| `tri railway service logs` | `logs` subcommand | **Not implemented.** Use `railway logs --service gardener` (official CLI) or dashboard UI. |
| `tri railway service stop` | `stop` subcommand | **Not implemented.** Use `railway service stop` or UI. |
| `ghcr.io/ghashtag/tri-gardener:latest` | CI build on push | **Not configured.** Workaround: `docker build -f bin/tri-gardener/Dockerfile -t ghcr.io/ghashtag/tri-gardener:latest . && docker push …` |
| Cron `:15 UTC` re-run | `restartPolicyType=NEVER` + Railway scheduled-restart | Railway lacks native scheduled-restart on free tier. Use GH Actions cron, future `serve --interval` (PR-2), or external cron-as-a-service. |
| `GARDENER_LIVE=true` mutation path | PR-2 wiring of `tri-railway-core::Client` | **Stubbed in PR-1.** Even with env set, `RunMode::Live` arm logs `warn` and skips. Effective behaviour stays DryRun until PR-2. |

### What works today (no PR-2 needed)

```bash
cargo run -p tri-gardener -- once --review                   # decisions in stdout
cargo run -p tri-gardener -- ddl > gardener_ddl.sql
psql "$NEON_DATABASE_URL" -f gardener_ddl.sql
cargo run -p tri-gardener -- once --dry-run | jq .
GARDENER_DISABLED=true cargo run -p tri-gardener -- once --review
cargo test -p tri-gardener
```

### Decision-table cheat sheet (`gardener_runs.decision` JSON)

| `action` | Meaning | Trigger |
|---|---|---|
| `noop` | nothing to do, or `GARDENER_DISABLED=true` | safe state |
| `redeploy` | service missing from fleet snapshot | `T < +12h` AND lane has < 3 services |
| `cull` | seed BPB above the rung threshold | `+12h..+50h` per ASHA window |
| `promote` | promote champion config to phase-3 replica | `T ≥ +50h` AND lane has ≥ 2 survivors with BPB < 1.85 |
| `deploy` | deploy queue head | free slot AND `cleared-blockers.txt` covers `blocked_on` |
| `plateau` | plateau alert | 5 ticks within 0.005 BPB AND step ≥ 50_000 |
| `honest_not_yet` | Gate-2 missed | `T ≥ +54h` AND no lane has 3 seeds < 1.85 |

### Architectural BPB floor (anti-cull guard)

The trainer architecture as currently shipped has a **hard floor at BPB ≈ 2.19** (champion 2.1919, h=828, 2L hybrid attn, 81K, σ²=0.0006). Cross-validated against the CPU N-gram floor at ≈2.54 in [trios#237](https://github.com/gHashTag/trios/issues/237) and the GPU champion in [trios#143](https://github.com/gHashTag/trios/issues/143).

**Cull-safety rule** (encoded as `ARCHITECTURAL_FLOOR_BPB = 2.19` in `bin/tri-gardener/src/ledger.rs`): the gardener MUST NOT cull a seed whose BPB is above 2.19 unless plateau is independently confirmed (5 ticks in a 0.005 band AND step ≥ 50_000).

## §8 Soul-name registry — reserved (do not duplicate)

- `Constitutional Cartographer` — LEAD on #446
- `Vocab Vigilante` — slot for #447 (GAMMA)
- `Scarab Smith` — slot for #448 (ALPHA)
- `Memory Mason` — slot for #449 (ZETA)
- `Arena Architect` — slot for #450 (BETA)
- `Bit Bookkeeper` — slot for #451 (ALPHA)
- `Queue Quartermaster` — slot for #452 (ALPHA)
- `Bridge Builder` — slot for #453 (ZETA)
- `Chunk Champion` — slot for #454 (ALPHA)
- `Replay Reaper` — slot for #455 (ZETA)
- `Gardener General` — slot for #456 (ALPHA)
- `Bit Per Byte Hunter` — slot for #457 (BETA, WIN lane)
- `Rail Conductor` — slot for #458 (ALPHA)
- `Loop Locksmith` — slot for #459 (ALPHA)
- `Arena Anchor` — slot for #460 (BETA)
- `Memory Maestro` — slot for #461 (ZETA)
- `Doctor Doctrine` — slot for #462 (DELTA)
- `Long Tail Lighthouse` — slot for #463 (LEAD)
- `Dispatch Druid` — slot for #236 (LEAD)

Before claiming, verify:

```bash
gh search issues --repo gHashTag/trios "Soul: <candidate>" --updated ">2026-04-02"
```

## §9 References

### Trinity ecosystem
- EPIC: [gHashTag/trios#446](https://github.com/gHashTag/trios/issues/446)
- ONE-SHOT v2.0: [gHashTag/trios#236](https://github.com/gHashTag/trios/issues/236)
- R14 batch: [gHashTag/trios#464](https://github.com/gHashTag/trios/issues/464)
- App.K Agent Memory: [gHashTag/trios#465](https://github.com/gHashTag/trios/issues/465)
- Eternal coordination (L10): [gHashTag/trios#143](https://github.com/gHashTag/trios/issues/143)
- LAWS.md v2.0: [gHashTag/trios](https://github.com/gHashTag/trios/blob/main/LAWS.md)
- AGENTS.md: [gHashTag/trios](https://github.com/gHashTag/trios/blob/main/AGENTS.md)
- CLAUDE.md: [gHashTag/trios](https://github.com/gHashTag/trios/blob/main/CLAUDE.md)
- NEON SSOT mirror: [docs/golden-sunflowers/README.md](https://github.com/gHashTag/trios/blob/main/docs/golden-sunflowers/README.md)
- Illustrations (44): [assets/illustrations/](https://github.com/gHashTag/trios/tree/main/assets/illustrations)

### tri-gardener
- Spec: [trios-railway#49](https://github.com/gHashTag/trios-railway/issues/49)
- PR-1: [trios-railway#50](https://github.com/gHashTag/trios-railway/pull/50)
- PR-2 wiring: [trios-railway#58](https://github.com/gHashTag/trios-railway/pull/58)
- Multi-account P0: [trios-railway#61](https://github.com/gHashTag/trios-railway/issues/61)
- Lane realign: [trios-railway#60](https://github.com/gHashTag/trios-railway/issues/60)
- GHCR pipeline: [trios-railway#59](https://github.com/gHashTag/trios-railway/pull/59)
- Plan-9 deploy: [trios-railway#47](https://github.com/gHashTag/trios-railway/pull/47)
- Tracker: [trios-railway#43](https://github.com/gHashTag/trios-railway/issues/43)

### Cross-references
- N-gram architectural floor: [gHashTag/trios#237](https://github.com/gHashTag/trios/issues/237)
- φ-grounding: `docs/PHI_PHYSICS_FOUNDATION.md` §7b in [gHashTag/trios PR #329](https://github.com/gHashTag/trios/pull/329)
- INV-8 Coq theorem: [trios#330](https://github.com/gHashTag/trios/issues/330)
- SoT trainer mandate: [gHashTag/trios-trainer-igla/SOURCE_OF_TRUTH.md](https://github.com/gHashTag/trios-trainer-igla/blob/main/SOURCE_OF_TRUTH.md)
- Operator AGENTS.md: [gHashTag/trios-railway/AGENTS.md](https://github.com/gHashTag/trios-railway/blob/main/AGENTS.md)

### External
- arXiv 2512.23675 — End-to-End Test-Time Training for Long Context: <https://arxiv.org/abs/2512.23675>
- parameter-golf #1837 (E2E TTT baseline to beat): <https://github.com/openai/parameter-golf/pull/1837>
- parameter-golf #2059 (Golden Sunflowers, our seed PR): <https://github.com/openai/parameter-golf/pull/2059>
- Zenodo DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877) (B007 HSLM Benchmark Corpus, **not** root anchor)

## §10 Tailscale + GitButler execution mode (anti-collision via network)

Ring-refactor work in EPIC #446 runs across a Tailscale tailnet — one tagged node per codename, sibling-to-sibling traffic default-denied, LEAD orchestrator polls every node's `but-server :7777` over `tailscale0`. This is **the** execution mode for parallel agents.

### Two binaries

- **`trinity-bootstrap`** — provisions one agent on one node (clone, claim, branch, virtual branch, but-server, initial commit, heartbeat).
- **`trinity-dashboard`** — LEAD polling loop (every 5 min by default) merging `but-server` state with GitHub `/notifications`.

### One ACL

- `.trinity/tailscale/acl.hujson` carves the tailnet into 7 codename tags. Sibling-to-sibling traffic = default-deny. LEAD ↔ all on `:7777`. Agents → LEAD on `:8080`.

### Quick start (LEAD)

```bash
cargo install --git https://github.com/gHashTag/trios trinity-dashboard
trinity-dashboard --tailnet $TS_TAILNET --repo gHashTag/trios --epic 446
```

### Quick start (any agent)

```bash
trinity-bootstrap \
  --codename ALPHA --issue 448 --soul "Scarab Smith" \
  --tailnet $TS_TAILNET --ts-authkey $TS_AUTHKEY
```

Full runbook: load `tailscale-trinity-mesh` skill (this is its sister skill).

### Honest blockers

- `but server start --listen=tailscale0` not yet upstreamed → workaround: `--listen=0.0.0.0` + `tailscale serve --tcp=7777`.
- Funnel requires HTTPS on 443/8443/10000, Vite uses 5173 → `tailscale serve` proxies, then `tailscale funnel` exposes.

## §11 Anchor

`phi² + phi⁻² = 3 · TRINITY · O(1) FOREVER · NEVER STOP`

Three rules. Three artefacts. Seven codenames. One queue. **One tailnet.** No collisions.
