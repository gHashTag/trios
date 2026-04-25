# 🐝 HIVE.md — Cellular Autonomy Contract v2

> **Status:** ACTIVE · **Issued:** 2026-04-25T22:30+07 · **Anchor:** `φ² + φ⁻² = 3`
> **Throne issue:** [trios#143](https://github.com/gHashTag/trios/issues/143)
> **Mission:** IGLA RACE v2 — `BPB < 1.50` on 3 seeds by 2026-04-30.
>
> This file is the **single autonomy contract** for every agent that joins the
> Trinity hive. Read it once, obey it always. The General does not answer
> clarifying questions — agents converge by graph, not by chatter.

---

## 1. Cellular-automaton model

The hive is a 1-D cellular automaton over the lane map (L0…L12 of #143).

- **Cell** = one lane.
- **State** ∈ { `OPEN`, `CLAIMED(agent, ts)`, `DONE(sha, ts)`, `BLOCKED(reason)` }.
- **Transition rule** = the protocol below. No central scheduler. Each agent
  reads neighbour state from `#143` + `assertions/hive_honey.jsonl`, applies
  the rule, writes its own next state. That's it. No supervisor needed.

The General writes **one ONE SHOT** to seed the automaton. Agents propagate.
The queen watchdog (cron) only sweeps dead claims; it never overrides agent
choices.

---

## 2. Hard rules (R-series, additive to #143 §0)

| # | Rule | Why |
|---|------|-----|
| **R1** | RUST ONLY (`cargo`). No `.py`, no `.sh`. | Single toolchain |
| **R2** | Race never stops until `BPB < 1.50` on 3 seeds. | Mission contract |
| **R3** | Land on `main`. No PRs, no `feat/*` branches. | Avoid merge fan-out |
| **R4** | Every numeric constant traces to a `.v` file via `assertions/igla_assertions.json`. | L-R14 |
| **R5** | Coq status (`Proven`/`Admitted`) is honest. | Trust |
| **R6** | NCA empirical/certified bands stay sibling fields. | Anti-conflict |
| **R7** | Forbidden: `prune=2.65`, `warmup<4000`, `d_model<256`, `lr∉[0.002,0.007]`. | Champion-killer |
| **R8** | Falsification witness mandatory in every new `.v`. | Popper |
| **R9** | Claim-before-work. First comment on #143 wins the lane. | No double-work |
| **R10** | Atomic commits: `feat(igla-<lane>): <change> [agent=<id>]`. | Bisect |
| **R11** | **No questions to the General.** If a spec gap exists, pick the most conservative reading consistent with R1–R10 and proceed. Document the choice in the commit message. | **Autonomy** |
| **R12** | **Self-pivot on block.** If a lane is blocked > 30 min, post `⚠️ BLOCKED`, release the lane, claim the next-priority OPEN lane from §4, continue. | **Liveness** |
| **R13** | **Honey deposit.** Every DONE appends 1 JSON line to `assertions/hive_honey.jsonl` (schema in §5). The next agent reads honey first to avoid repeating mistakes. | **Memory** |

---

## 3. Cellular protocol — exact transitions

```
state(L) = OPEN
   ── agent posts §4.1 CLAIMING ──▶ state(L) = CLAIMED(agent, now)

state(L) = CLAIMED(agent, ts)
   ── agent posts §4.3 DONE ──▶ state(L) = DONE(sha, now)
                                  + honey_jar.append(...)
   ── agent posts §4.4 BLOCKED ──▶ state(L) = OPEN (auto-released)
   ── now − ts > 4h, no heartbeat ──▶ state(L) = OPEN (queen watchdog)

state(L) = DONE(sha, _)            (terminal)
state(L) = BLOCKED(reason)         (queen escalates only)
```

**Auto-release** is the dead-man switch. A silent agent is a dead agent.
The queen watchdog cron (hourly) sweeps `CLAIMED` cells whose last comment
is older than 4h and resets them to `OPEN`. No human in the loop.

---

## 4. Lane priority (read this, then claim)

When a fresh agent enters the hive, it **must** pick a lane in this order
(skip any lane that is `CLAIMED` or `DONE`):

| Priority | Lane | Status @ 2026-04-25T22:30 | Why |
|----------|------|---------------------------|-----|
| **P0** | L11 worker pool (`race.rs` + `bin/race.rs`) | OPEN | Critical path — no end-to-end BPB harness yet |
| **P0** | L7 victory gate (`found.rs`) | OPEN | Required to declare 3-seed BPB<1.50 |
| **P1** | L1 BPB tracker (`bpb.rs`) | OPEN | INV-1 numerics, JEPA proxy reject |
| **P1** | L6 EMA (`ema.rs`) | OPEN | INV-6 cosine schedule |
| **P1** | L9 QK head (`attn.rs`) | OPEN | INV-9 QK-gain = φ² |
| **P1** | L4 NCA (`nca.rs`) | OPEN | INV-4 dual-band enum, K=9 |
| **P1** | L3 GF16 (`gf16.rs`) | OPEN | INV-3, d_model floor 256 |
| **P1** | L2-incremental ASHA (`asha.rs`) | OPEN | INV-2 + INV-10 enforcement edges |
| **P2** | L12 CI hygiene (`coq-check.yml` + clippy debt) | OPEN | Clears Quality Gate #3 |
| **DONE** | L8 LR sampler | [`5a4c980`](https://github.com/gHashTag/trios/commit/5a4c980) | — |
| **DONE** | L10 ASHA rungs | [`b959c43`](https://github.com/gHashTag/trios/commit/b959c43) | — |

**Tie-break inside a priority tier:** lowest INV number wins. **Two agents
on the same tier:** R9 — first claim wins; loser drops to next tier.

---

## 5. Honey jar — `assertions/hive_honey.jsonl`

Append-only JSONL. Every DONE appends one line (atomic commit). Format:

```json
{"ts":"2026-04-25T22:17Z","lane":"L8","agent":"perplexity-computer",
 "sha":"5a4c980","inv":"INV-1","theorem":"lr_phi_band","status":"partial-Proven",
 "tests_added":9,"tests_total":30,"clippy":"clean(own)","admitted_burned":0,
 "bpb_delta":null,"lesson":"log-uniform > linear-uniform on lr; champion=0.004 stays inside band by construction"}
```

Required fields: `ts`, `lane`, `agent`, `sha`, `inv`, `theorem`, `status`,
`tests_added`, `tests_total`, `clippy`, `admitted_burned`, `lesson`.
Optional: `bpb_delta`, `seed_results`, `links`.

**Read-before-write rule:** before claiming a lane, an agent reads the last
20 honey entries. Repeating a documented mistake is grounds for self-block.

---

## 6. ONE SHOT v2 dispatch — single copy-paste block

The General no longer writes long task documents. **One block** is enough:

```
🐝 ONE SHOT v2 — JOIN HIVE
Skill: trinity-queen-hive + coq-runtime-invariants
Throne: https://github.com/gHashTag/trios/issues/143
Contract: https://github.com/gHashTag/trios/blob/main/HIVE.md
Honey: https://github.com/gHashTag/trios/blob/main/assertions/hive_honey.jsonl
Mission: IGLA RACE v2 — BPB<1.50 on 3 seeds by 2026-04-30.
Action: read HIVE.md §4, claim highest-OPEN lane, ship per R3/R10, deposit honey per R13. Do NOT ask questions (R11). On block, self-pivot (R12).
ACK: post `ACK HIVE <agent-id>` on #143, then begin.
```

Paste this to any new agent thread. The agent self-routes.

---

## 7. Queen watchdog (hourly cron)

Lives in this Computer thread (`schedule_cron`). Every hour:

1. Pull #143 comments since last sweep.
2. For every `CLAIMING` with no follow-up in > 4h → post `🔓 lane released — silent > 4h` and flip back to `OPEN` in §4 table.
3. Re-render the lane table from observed comments.
4. If 0 lanes are CLAIMED and BPB target not met → post a nudge tagging the highest-priority lane.
5. Refresh #143 dashboard comment.

The watchdog **never** overrides an agent's claim or commit. It only
recycles dead lanes and republishes state.

---

## 8. Forbidden in autonomous mode

- ❌ Asking the General for clarification (R11).
- ❌ Holding a lane silently > 4h.
- ❌ Skipping the honey jar deposit (R13).
- ❌ Claiming a lane that's already DONE in §4.
- ❌ Editing a sibling lane's file in your commit.
- ❌ Hand-editing `hive_honey.jsonl` (append-only; never rewrite history).
- ❌ Force-pushing over another agent's claim (§4.5 of #143).

---

## 9. Battle cry

> The General writes one line. The hive converges by itself. φ² + φ⁻² = 3.
