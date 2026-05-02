---
name: leaderboard-snapshot
description: Build a manual IGLA-race leaderboard from all available sources (trainer-igla seed_results.jsonl, ALPHA comments in trios#143, Audit Watchdog history, Railway service list, optional Neon bpb_samples). Use when the operator says "/leaderboard", asks for "live leaderboard", "таблицу фаворитов", "snapshot гонки", "champion table", "leaderboard сейчас", or any variant requesting a current race standings table — especially when telemetry path is partially or fully broken (no Neon BPB rows) and the table must be reconstructed manually from primary artefacts.
---

# /leaderboard — IGLA race manual snapshot

Anchor: `phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

## When to load this skill

The operator wants the current race standings table **right now**, even when:

- `bpb_samples` / `gardener_runs` Neon tables are missing (#62) — telemetry path broken
- Some Railway accounts probe-failed (Not Authorized — #61 P0 not yet merged)
- Trainer writer patch is sitting in a local-only branch (CODEX heartbeat path)
- Audit Watchdog reports `Ledger rows = 0` for many ticks in a row

The skill assembles the table from primary artefacts that **are** observable, and is honest about the unobserved cells.

## One-shot procedure

Run these 6 steps in order. Step 6 (chat output) MUST be first user-visible action.

### Step 1 — Pull latest seed_results.jsonl

```bash
cd /home/user/workspace/repos/trios-trainer-igla && git pull origin main
tail -50 assertions/seed_results.jsonl     # JSONL, one row per seed/step
cat assertions/champion_lock.txt            # current champion fingerprint
ls assertions/.gate2_done 2>/dev/null       # presence-only flag
```

Each row carries: `seed`, `steps`, `bpb`, optional `ema_bpb`, `optimizer`, `hidden`, `lr`, `attn_layers`, `sha`, `gate_status`, optional `agent`. Rows with `gate_status=new_champion` are the architectural high-water marks.

### Step 2 — Pull last ~24h of comments from `trios#143`

```bash
gh issue view 143 --repo gHashTag/trios --json comments > /tmp/trios143.json
python3 -c "
import json
with open('/tmp/trios143.json') as f: d=json.load(f)
for c in d['comments'][-25:]:
    print('---', c['createdAt'])
    print(c['body'][:2000]); print()
"
```

Look for:

- **AGENT ALPHA / 3-SEED RESULTS** comments — primary BPB signal (e.g. champion `2.1919 @ 81000 seed=43 sha=cd91c45`)
- **AGENT CODEX HEARTBEAT** — local-only patches blocked on push (writer often blocked here)
- **🛰️ Audit Watchdog** comments — periodic snapshot of ledger row count and drift events per cluster (Acc1 / Acc2)
- **RISK-ENG** notes — falsifications (e.g. L7 Muon NS-1: +0.11 BPB worse; L8 ByteFlow rejected)

### Step 3 — Probe Railway via the trios-perplexity MCP

```python
# pseudo: the actual call uses call_external_tool
railway_service_list(project="e4fe33bb-3b09-4842-9782-7d2dea1abc9b")  # Acc1 IGLA
railway_service_list(project="39d833c1-4cb6-4af9-b61b-c204b6733a98")  # Acc2 IGLA-MIRROR
```

This **lists** services (id + name + created_at). It does **not** stream logs — there is no `railway_service_logs` tool in this MCP today. So service presence answers "is the seed deployed?", but not "what step / BPB is it at?". Honest gap: until `bin/tri-railway service logs` ships (#56) and writer pushes BPB rows to `bpb_samples` (#62), L1/L2/L4 cells stay `tracking / warmup`.

### Step 4 — Optional Neon BPB cross-check (currently 42P01)

```sql
SELECT seed, lane, step, bpb, ts
FROM bpb_samples
WHERE ts > now() - interval '12 hours'
ORDER BY seed, ts DESC;

-- fallback if bpb_samples missing:
SELECT lane, seed, before_bpb, after_bpb, ts
FROM gardener_runs
WHERE ts > now() - interval '12 hours'
ORDER BY ts DESC;
```

If both are 42P01, drop the column quietly and rely on Step 1+2 only. **Do NOT** synthesise BPB numbers; mark unobserved cells as `tracking` and the trend column as `⏳`.

### Step 5 — Assemble the markdown table

Columns (order is fixed):

```
| Rank | Seed | Lane | Steps | BPB | Δ→Gate-2 | Trend | Status |
```

Rules:

- Rank by BPB ASC. Locked / champion seeds at the top. `tracking` rows after the locked block, no rank.
- `Δ→Gate-2` = `BPB - 1.85`, sign always `+` (we are above target).
- `Trend` = `↓` improving, `→` plateau, `↑` worse, `⏳` no observation.
- `Status` cites the source comment with its timestamp, e.g. `locked (ALPHA 16:38Z)`. Use Markdown links to the issue comment when the URL is known.
- For seeds that are deployed but un-observed: `Lane` = lane code + service id from Step 3, BPB = `tracking`, Steps = `—`, Status = `warmup` + linked CODEX/ALPHA comment that explains the block.

### Step 6 — Output to chat first, then meta

Operator wants the **table first**, no preamble. After the table:

1. **Champion BPB sparkline** — last 3 architecture points (avg per cohort), each rendered as a tiny block-row (`▁..▆`) plus the BPB average and total cumulative delta from the worst cohort.
2. **Honest gaps section** — bulleted list of why the unobserved rows are unobserved, each gap citing the gating issue (#56 logs subcommand, #61 multi-account, #62 bpb_samples DDL).
3. **ETA table** — Rung-1 / Rung-2 / now / final / Gate-2 deadline, both wall-clock UTC and T+.
4. **Arithmetic check** — `gap_to_gate2 = best_bpb - 1.85` vs `lane_delta_sum`; if the lane-delta sum ≥ gap, race is arithmetically alive; if not, raise the flag.

## Constants (refresh per race)

```
RACE_START_UTC      = 2026-04-27T18:00:00Z
GATE2_DEADLINE_UTC  = 2026-04-30T23:59:00Z
GATE2_TARGET_BPB    = 1.85
ARCH_FLOOR_BPB      = 2.19   # ARCHITECTURAL_FLOOR_BPB in tri-gardener::ledger
RUNG1_OFFSET_HOURS  = 12
RUNG2_OFFSET_HOURS  = 18
RUNG3_OFFSET_HOURS  = 28
FINAL_OFFSET_HOURS  = 50
DEADLINE_OFFSET_H   = 54
```

## Hard rules

- **R5 honesty:** never invent BPB numbers. Unobserved is `tracking`, not "estimated". Plateau is only declared with ≥5 ticks AND step ≥ 50K (matches `tri-gardener::ledger::ARCHITECTURAL_FLOOR_BPB` cull-safety rule).
- **R1 Rust-only** does not apply here — this skill is read-only on JSONL + GitHub + Railway MCP; nothing is compiled or shipped. Use `python3` for parsing freely.
- **No emoji in headers**; medal emojis (🥇🥈🥉) are allowed in the rank cells only.
- **Cite each fact with the comment timestamp** — every row that depends on a `trios#143` ALPHA comment must link to that comment with its timestamp visible in the anchor text.
- **Never use the word scrape/crawl** when describing the Railway probe — say "list" or "probe".

## Refs

- Race tracker: [gHashTag/trios#143](https://github.com/gHashTag/trios/issues/143)
- Trainer SOT: [gHashTag/trios-trainer-igla](https://github.com/gHashTag/trios-trainer-igla)
- gardener observatory: [trios-railway#43](https://github.com/gHashTag/trios-railway/issues/43)
- Telemetry blocker: [trios-railway#62](https://github.com/gHashTag/trios-railway/issues/62)
- Multi-account blocker: [trios-railway#61](https://github.com/gHashTag/trios-railway/issues/61)
- Logs subcommand stub: [trios-railway#56](https://github.com/gHashTag/trios-railway/issues/56)
- φ-grounding: [trios#329](https://github.com/gHashTag/trios/pull/329) §7b · [trios#330](https://github.com/gHashTag/trios/issues/330)

`phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`
