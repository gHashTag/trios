# 🐝 HIVE PROTOCOL — Cellular Automaton for Autonomous Agent Army

**Version:** 1.0 · **Issued:** 2026-04-25T22:30+07 · **Authority:** Queen of Trinity
**Anchor:** `φ² + φ⁻² = 3` (Zenodo DOI [10.5281/zenodo.19227877](https://zenodo.org/records/19227877))
**Throne:** [trios#264](https://github.com/gHashTag/trios/issues/264) · **Active race:** [trios#143](https://github.com/gHashTag/trios/issues/143)

> **Contract:** the user issues ONE SHOT exactly once. From that moment, every spawned agent self-bootstraps, claims a lane, ships, and re-enters the loop **without asking the user a single question**. The hive runs as a cellular automaton: one shared state, deterministic transition rules, zero human in the inner loop.

---

## 0. Why agents kept stopping (root cause)

Previous behaviour:

```
spawn → load skill → claim lane → ship lane → DONE comment → IDLE → wait for ORDER
                                                                ▲
                                                                └─ HUMAN-IN-THE-LOOP ❌
```

Each agent treated DONE as a terminal state. The human had to manually re-prompt, breaking parallelism and burning the 5-day window.

**Fix:** DONE is not terminal. DONE is a transition. The next state is `READ_HIVE_STATE`, not `WAIT`.

---

## 1. The State Machine (cellular automaton rules)

Each agent is a cell. The hive is the lattice. The shared substrate is **GitHub issue #143 + `assertions/hive_state.json` on `main`**. Every agent runs the same transition function:

```
                ┌─────────────────────────┐
                │   START / RESPAWN       │
                └────────────┬────────────┘
                             │
                             ▼
                ┌─────────────────────────┐
                │  S0: BOOTSTRAP          │  read HIVE_PROTOCOL.md +
                │  (skill load, git pull) │  assertions/hive_state.json +
                └────────────┬────────────┘  last 50 comments on #143
                             │
                             ▼
                ┌─────────────────────────┐
                │  S1: SCAN_LANES         │  build set of OPEN lanes
                │                         │  ∖ {claimed in last 4h}
                └────────────┬────────────┘  ∖ {DONE on main}
                             │
                ┌────────────┴────────────┐
                │                         │
        ∃ open lane              all lanes DONE
                │                         │
                ▼                         ▼
   ┌─────────────────────┐    ┌──────────────────────┐
   │  S2: PICK_LANE      │    │  S6: VICTORY_GATE    │
   │  (priority order)   │    │  3-seed BPB<1.50?    │
   └──────────┬──────────┘    └─────────┬────────────┘
              │                         │
              ▼                  yes ───┴─── no
   ┌─────────────────────┐       │           │
   │  S3: CLAIM          │       ▼           ▼
   │  post §4.1 comment  │   FOUND        S7: REOPEN
   └──────────┬──────────┘   close #143    refresh state
              │              with 🎯       jump to S1
              ▼
   ┌─────────────────────┐
   │  S4: SHIP           │  edit/build/test/clippy
   │  atomic commit      │  push to main, post §4.3 DONE
   └──────────┬──────────┘
              │
              ▼
   ┌─────────────────────┐
   │  S5: UPDATE_STATE   │  patch hive_state.json:
   │                     │   - mark lane DONE
   └──────────┬──────────┘   - bump cycle counter
              │              commit "[hive-tick]"
              │
              └────────► loop back to S1
```

**Key invariant (the cellular-automaton rule):** every agent in state `IDLE` after DONE must **immediately re-enter S1** without asking the human. The only paths out of the loop are:

1. **VICTORY** — `S6` confirms 3-seed BPB < 1.50 → close #143 → terminate.
2. **DEADLINE** — current UTC > `2026-04-30T23:59:59Z` → terminate with status report.
3. **HARD-BLOCK** — every remaining lane has at least one of: an active claim < 4h old, a missing upstream dependency that the agent cannot resolve. Post §4.4 BLOCK and terminate.

Anything else = keep cycling.

---

## 2. Shared State File — `assertions/hive_state.json`

Single source of truth for lane status. Read at S0, updated at S5. Schema:

```json
{
  "schema_version": 1,
  "mission": "IGLA RACE v2 — Coq-Bounded Distributed Hunt",
  "deadline_utc": "2026-04-30T16:59:59Z",
  "target": { "metric": "BPB", "operator": "<", "value": 1.50, "seeds_required": 3 },
  "trinity_identity": "phi^2 + phi^-2 = 3",
  "cycle": 0,
  "lanes": {
    "L1":  { "status": "OPEN",     "owner_file": "crates/trios-igla-race/src/bpb.rs",      "claimed_by": null, "claimed_at": null, "commit": null, "blocks": [] },
    "L2":  { "status": "DONE",     "owner_file": "crates/trios-igla-race/src/asha.rs",     "claimed_by": "f424e0f", "claimed_at": "2026-04-25T20:00:00Z", "commit": "f424e0f", "blocks": [] },
    "L3":  { "status": "OPEN",     "owner_file": "crates/trios-igla-race/src/gf16.rs",     "claimed_by": null, "claimed_at": null, "commit": null, "blocks": [] },
    "L4":  { "status": "OPEN",     "owner_file": "crates/trios-igla-race/src/nca.rs",      "claimed_by": null, "claimed_at": null, "commit": null, "blocks": [] },
    "L5":  { "status": "DONE",     "owner_file": "crates/trios-igla-race/src/invariants.rs","claimed_by": "a8afd52", "claimed_at": "2026-04-25T20:00:00Z", "commit": "a8afd52", "blocks": [] },
    "L6":  { "status": "OPEN",     "owner_file": "crates/trios-igla-race/src/ema.rs",      "claimed_by": null, "claimed_at": null, "commit": null, "blocks": [] },
    "L7":  { "status": "OPEN",     "owner_file": "crates/trios-igla-race/src/found.rs",    "claimed_by": null, "claimed_at": null, "commit": null, "blocks": ["L11"] },
    "L8":  { "status": "DONE",     "owner_file": "crates/trios-igla-race/src/sampler.rs",  "claimed_by": "5a4c980", "claimed_at": "2026-04-25T22:09:00Z", "commit": "5a4c980", "blocks": [] },
    "L9":  { "status": "OPEN",     "owner_file": "crates/trios-igla-race/src/attn.rs",     "claimed_by": null, "claimed_at": null, "commit": null, "blocks": [] },
    "L10": { "status": "DONE",     "owner_file": "crates/trios-igla-race/src/rungs.rs",    "claimed_by": "b959c43", "claimed_at": "2026-04-25T22:20:00Z", "commit": "b959c43", "blocks": [] },
    "L11": { "status": "OPEN",     "owner_file": "crates/trios-igla-race/src/race.rs",     "claimed_by": null, "claimed_at": null, "commit": null, "blocks": [] },
    "L12": { "status": "OPEN",     "owner_file": ".github/workflows/coq-check.yml",        "claimed_by": null, "claimed_at": null, "commit": null, "blocks": [] }
  },
  "victory": { "achieved": false, "seeds": [], "best_bpb": null, "evidence_commit": null }
}
```

**Update protocol (R-CAS):** every state transition is a **compare-and-swap commit**. The agent:

1. Pulls latest `main`, reads `assertions/hive_state.json`.
2. Computes the proposed new state.
3. Commits with message `chore(hive-tick): <agent-id> <lane> <S→S'> [cycle=<n+1>]`.
4. Pushes. If push is rejected (someone else won the CAS), `git pull --rebase` and re-run S1.

This is exactly Lamport's bakery algorithm over `git push` — guaranteed progress, no deadlock, no central coordinator.

---

## 3. Coordination Protocol (binding for every agent)

### 3.1 Claim (S3)

```
🔒 AGENT <id> CLAIMING: L<n> — <lane name>
ETA: <ISO ≤ 2h>
Constants pinned: PHI=1.618…, prune=3.5, warmup=4000, dmin=256, lr=0.004, K=9, qk=φ²
Skill: coq-runtime-invariants v1.0
Hive cycle: <n>
```

First comment after the latest `[hive-tick]` commit wins.

### 3.2 Heartbeat (every 60 min while in S4)

```
⏱️ AGENT <id> L<n> HEARTBEAT: <% done> · last commit <sha> · next: <step>
```

No heartbeat for 2h → `[hive-tick]` reaper auto-flips `claimed_by` to `null`.

### 3.3 DONE (S4 → S5)

```
✅ AGENT <id> DONE: L<n>
Commit: <sha>
cargo test → <N>/<M> passed
Coq: Proven=K, Admitted=A (≤4 budget)
Hive cycle: <n+1> (will scan again immediately, no human prompt needed)
```

Crucial: **the DONE comment ends with a self-trigger sentence** so any reading agent (including the same one) knows to re-enter S1.

### 3.4 BLOCK (terminal only)

```
⚠️ AGENT <id> BLOCKED after L<n>
Open lanes remaining: <list>
All blocked because: <reason>
Hive cycle: <n>
```

This is the only exit-without-victory the agent may take, and only after S1 returns ∅ open lanes.

---

## 4. Lane priority (deterministic — no agent guesses)

Agents in S2 pick the **first OPEN lane** in this list. Identical across all agents → identical decisions → no double-claim races beyond the §3.1 first-comment rule.

```
1. L11 — race.rs       (critical path: nothing measures BPB E2E yet)
2. L7  — found.rs      (small, victory gate)
3. L12 — CI hygiene    (clears clippy debt blocking Gate #3 globally)
4. L1  — bpb.rs        (INV-1 BPB tracker, JEPA proxy reject)
5. L3  — gf16.rs       (-40% configs)
6. L4  — nca.rs        (-30% configs)
7. L6  — ema.rs        (-20% configs)
8. L9  — attn.rs       (-10% configs)
```

If the priority list changes, the queen pushes a new `HIVE_PROTOCOL.md` revision and bumps `schema_version`. Agents always re-read the file at S0.

---

## 5. Forbidden actions (auto-reject by `git pre-push` and CI)

- ❌ Claiming a lane whose `status == DONE` in `hive_state.json`.
- ❌ Skipping the `[hive-tick]` commit after DONE — breaks the CAS chain.
- ❌ Asking the human a clarifying question while in S1..S5 (only S6 / S7 may surface to the human).
- ❌ Force-pushing to overwrite another agent's `[hive-tick]`.
- ❌ Re-implementing a DONE lane (verify against `hive_state.json` before any work).
- ❌ Touching another lane's `owner_file` (one cell per agent — strict locality).
- ❌ Closing #143 outside the VICTORY transition.

---

## 6. Bootstrap prompt (paste this verbatim to spawn an autonomous worker)

```
You are an IGLA RACE worker. The mission is in trios#143. The protocol is
docs/hive/HIVE_PROTOCOL.md on gHashTag/trios main.

Loop forever:
1. git clone --depth=20 https://github.com/gHashTag/trios.git (or pull)
2. Read docs/hive/HIVE_PROTOCOL.md and assertions/hive_state.json.
3. If victory.achieved == true → terminate.
4. If UTC > deadline_utc → post §3.4 BLOCK and terminate.
5. Pick the first OPEN lane in the §4 priority list whose claimed_at is null
   or older than 2h.
6. Post the §3.1 CLAIM comment on issue #143.
7. Implement the lane in one atomic commit per R10. cargo test + clippy must
   pass for your file(s).
8. Push to main (R3, no PR, no branch). Update assertions/hive_state.json
   with status=DONE, commit=<sha>, in a follow-up [hive-tick] commit.
9. Post the §3.3 DONE comment.
10. Go to step 2 — DO NOT WAIT for the human.

Skills: coq-runtime-invariants (mandatory), trinity-queen-hive (for cross-repo).
Anchor: φ² + φ⁻² = 3.
NEVER ASK QUESTIONS. The protocol answers everything.
```

That single block is the entire ONE SHOT. Issue it once, walk away, the hive runs.

---

## 7. Termination conditions (only three)

| Condition | Detected by | Action |
|---|---|---|
| 🏆 **VICTORY** | S6 reads `victory.achieved == true` after 3-seed BPB<1.50 | Close #143, write Ch.24 evidence figure, archive `hive_state.json` snapshot |
| ⏰ **DEADLINE** | UTC > `2026-04-30T16:59:59Z` (= 2026-04-30 23:59 +07) | Post final status table on #143, do **not** close issue (R3 of #143 forbids close) |
| 🛑 **HARD-BLOCK** | All remaining lanes have active claims < 2h old AND no S6-eligible victory commit on `main` | Post §3.4 BLOCK, terminate |

No other exit. The cellular automaton runs until one of these fires.

---

## 8. Anti-pattern catalogue (lessons from the first 3 hours of the race)

| Smell | Diagnosis | Fix in this protocol |
|---|---|---|
| Two agents claim L8 within 5 min | No CAS on claim comments | §3.1 first-comment + §2 R-CAS commit-and-rebase |
| Agent ships DONE then idles | DONE treated as terminal | §1 explicit S5 → S1 loop, §3.3 DONE comment ends with self-trigger |
| Pre-existing clippy debt blocks every gate | No L12 hygiene lane | §4 priority L12 ranked 3rd |
| Agents ask the human "what next?" | Underspecified bootstrap | §6 bootstrap prompt is the **only** thing the human sends |
| `Cargo.lock` drift between agents | Each rebuilds independently | `cargo build` before push; lockfile committed; no `--frozen` here |

---

## 9. Queen's standing orders (re-asserted)

1. RUST ONLY in CROWN repos.
2. `main`-only on this race (no PRs, no branches).
3. Every numeric constant ↔ `.v` file ↔ `assertions/igla_assertions.json` ↔ Rust (L-R14).
4. `Proven` / `Admitted` honesty is non-negotiable.
5. `nca_empirical_band` and `nca_certified_band` stay sibling fields.
6. Forbidden values reject: `prune=2.65`, `warmup<4000`, `d_model<256`, `lr ∉ [0.002,0.007]`.
7. Falsification witness mandatory in every new `.v` file.
8. **Claim → ACK → commit → DONE → loop.** No human in the inner cycle.

---

## 10. Battle cry

> One identity, one protocol, one main branch. Twelve cells in the lattice. The queen issues the law once; the hive computes the rest. **NEVER ASK. JUST CYCLE.**

— HIVE PROTOCOL v1.0 · skill `trinity-queen-hive` · skill `coq-runtime-invariants`
