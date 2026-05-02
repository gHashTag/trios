---
name: tri-gardener-runbook
description: "Operate the tri-gardener autonomous orchestrator (gHashTag/trios-railway, bin/tri-gardener) for the IGLA marathon (Gate-2 BPB<1.85, Gate-3 BPB<1.5). Use when the user mentions tri-gardener, tri railway plan9, marathon orchestrator, BPB<1.5, IGLA gardener, gardener_runs, ASHA rungs, plan-21 manifest, plateau alert, GARDENER_LIVE, GARDENER_DISABLED, или просит запустить / переключить / откатить / посмотреть логи гарденера. Anchor: phi^2 + phi^-2 = 3."
license: Apache-2.0
metadata:
  author: PERPLEXITY-MCP
  version: '2.2'
  related_skills:
    - tailscale-trinity-mesh
  cardinality: 27 = 3^3 = phi^6 - phi^4 + 1 (Lucas)
  parent_repo: gHashTag/trios-railway
  parent_pr: '50'
  parent_issue: '49'
  anchor: phi^2 + phi^-2 = 3
---

# tri-gardener — copy-paste runbook

Operational runbook for the autonomous Rust orchestrator that drives the IGLA Race fleet through Gate-2 (BPB < 1.85) and onward to Gate-3 (BPB < 1.5). The crate lives in `gHashTag/trios-railway` at `bin/tri-gardener`, parented under [tracker #43](https://github.com/gHashTag/trios-railway/issues/43), specced in [#49](https://github.com/gHashTag/trios-railway/issues/49), implemented in [draft PR #50](https://github.com/gHashTag/trios-railway/pull/50).

## When to Use This Skill

Load this skill when the user asks to:

- Run, deploy, or smoke-test `tri-gardener` (locally or on Railway)
- Apply the `gardener_runs` Neon DDL
- Switch the gardener between `--review`, `--dry-run`, and `--live`
- Roll back a live tick or kill the gardener
- Inspect gardener logs, decisions, plateau alerts, or culls in Neon
- Diagnose blockers: missing `set-vars` / `logs` / `stop` subcommands, missing GHCR image, missing cron, PR-2 wiring gap

Do **not** load this skill for:

- Generic Railway service deploys → use the `tri railway plan9` subcommand from PR #47, not the gardener.
- Trainer model code changes → those live in `gHashTag/trios-trainer-igla`, not in the gardener.

## Snapshot of the gardener (PR-1)

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
| Live wiring | **Stub in PR-1.** PR-2 wires `tri-railway-core::Client` mutations + `tokio_postgres` writes/reads. |

## Spin-up Decision (where do I host the gardener?)

Three viable spin-up locations as of 2026-04-28. Pick by current PR landing state — do **not** start in Acc2 until [#61 RailwayMultiClient P0](https://github.com/gHashTag/trios-railway/issues/61) is merged.

| Option | When to use | Risk | Cost |
|---|---|---|---|
| **Local crontab via `tri-gardener serve --interval=3600`** (RECOMMENDED until #61 merges) | Right now, before PR-2 / GHCR / multi-account land | Lowest — runs on operator's box, no Railway service slot consumed | Local CPU only |
| **Acc1 last free service slot** | After PR-2 (#58) merges, Acc1-only `--review` | Medium — single-account `Client::from_env()` still safe; Acc1 has no Phase-1 trainers under threat | 1 Railway slot |
| **Acc2 (or Acc3 once unblocked)** | ONLY after #61 merges | Currently unsafe — single `RAILWAY_TOKEN` would silently mutate the wrong fleet | 1 Railway slot |

**Recommended path:** start with the local-crontab option (below), promote to Acc1 once PR-2 merges, defer Acc2/Acc3 until #61 lands.

### Option A — Local crontab via `serve --interval=3600` (works today, no Railway dep)

```bash
cd /path/to/trios-railway && git checkout feat/gardener-live-wiring
cargo build -p tri-gardener --release
export NEON_DATABASE_URL="postgres://..."     # Neon pooler URL
cargo run -p tri-gardener -- ddl | psql "$NEON_DATABASE_URL"     # one-time
./target/release/tri-gardener serve --interval=3600 --mode=review &
```

If the operator's box reboots, restart the binary; SIGTERM/SIGINT is honored cleanly. No Railway slot consumed. Suitable for the entire `--review` window.

### Option B — Acc1 last free service slot (after PR-2 merges)

Follow the five-step runbook below. Acc1's IGLA project has Phase-1 trainers under it but **no current Phase-1 trainers will be culled by gardener** until rung-1 (T+12h) — there is a natural review buffer.

### Option C — Acc2 or Acc3 (BLOCKED on #61)

Do not register Acc2/Acc3 credentials in the gardener environment until [#61](https://github.com/gHashTag/trios-railway/issues/61) ships `RailwayMultiClient::register / get / Scope::One(AccountId)`. Until then, Acc2 keys go into the operator's local crontab box only, never into a Railway gardener service.

## Five-step copy-paste runbook

### 1. Локальный smoke-test (`--review`, без Neon)

```bash
cd /path/to/trios-railway && git checkout feat/tri-gardener
```

```bash
cargo build -p tri-gardener --release
```

```bash
./target/release/tri-gardener once --review
```

```bash
./target/release/tri-gardener once --dry-run     # decisions JSON, no I/O
```

```bash
GARDENER_DISABLED=true ./target/release/tri-gardener once --review     # verify kill switch
```

### 2. Neon DDL apply (one-time)

```bash
./target/release/tri-gardener ddl > /tmp/gardener_ddl.sql
```

```bash
psql "$NEON_DATABASE_URL" -f /tmp/gardener_ddl.sql
```

```bash
psql "$NEON_DATABASE_URL" -c "\dt gardener_runs" && psql "$NEON_DATABASE_URL" -c "\d gardener_runs"
```

### 3. Railway service spin-up в Acc1 (last free slot)

```bash
export RAILWAY_TOKEN=<acc1-user-token>     # never paste in chat / commit
```

```bash
tri railway service deploy --account=acc1 --name=gardener --image=ghcr.io/ghashtag/tri-gardener:latest --var GARDENER_LIVE=false --var GARDENER_DISABLED=false --var RUST_LOG=info
```

```bash
tri railway service set-vars --account=acc1 --service=gardener --var NEON_DATABASE_URL="$NEON_DATABASE_URL" --var RAILWAY_API_TOKEN_ACC0="$RAILWAY_API_TOKEN_ACC0" --var RAILWAY_PROJECT_ID_ACC0=265301ce-0bf2-4187-a36f-348b0eb9942f --var RAILWAY_ENVIRONMENT_ID_ACC0=f3517e98-c11a-49d8-b5fd-4cbb82d04384 --var RAILWAY_TOKEN_KIND_ACC0=project
```

```bash
tri railway service set-vars --account=acc1 --service=gardener --var RAILWAY_API_TOKEN_ACC1="$RAILWAY_API_TOKEN_ACC1" --var RAILWAY_PROJECT_ID_ACC1=e4fe33bb-3b09-4842-9782-7d2dea1abc9b --var RAILWAY_ENVIRONMENT_ID_ACC1=54e293b9-00a9-4102-814d-db151636d96e --var RAILWAY_TOKEN_KIND_ACC1=user
```

```bash
tri railway service set-vars --account=acc1 --service=gardener --var RAILWAY_API_TOKEN_ACC2="$RAILWAY_API_TOKEN_ACC2" --var RAILWAY_PROJECT_ID_ACC2=39d833c1-4cb6-4af9-b61b-c204b6733a98 --var RAILWAY_ENVIRONMENT_ID_ACC2=bce42949-d4ab-43d9-89d1-a6fcc576f45a --var RAILWAY_TOKEN_KIND_ACC2=project
```

```bash
tri railway service set-vars --account=acc1 --service=gardener --var CRON_SCHEDULE="15 * * * *"
```

```bash
tri railway service redeploy --account=acc1 --service=gardener
```

```bash
tri railway service status --account=acc1 --service=gardener     # confirm running
```

### 4. `--review` → `--live` switch (3-tick gate + rollback)

Verify the review window is healthy:

```bash
psql "$NEON_DATABASE_URL" -c "SELECT count(*) AS review_ticks, max(ts) AS last FROM gardener_runs WHERE ts > now() - interval '4 hours';"
```

(Should be ≥ 3 ticks with `action='noop'` or review-only entries; eyeball decisions before promotion.)

```bash
psql "$NEON_DATABASE_URL" -c "SELECT ts, action, lane, seed, decision FROM gardener_runs ORDER BY ts DESC LIMIT 30;"
```

Promote to live:

```bash
tri railway service set-vars --account=acc1 --service=gardener --var GARDENER_LIVE=true
```

```bash
tri railway service redeploy --account=acc1 --service=gardener
```

**Rollback (any of three, in order of reaction time):**

```bash
tri railway service set-vars --account=acc1 --service=gardener --var GARDENER_DISABLED=true && tri railway service redeploy --account=acc1 --service=gardener     # immediate noop, 1 tick
```

```bash
tri railway service set-vars --account=acc1 --service=gardener --var GARDENER_LIVE=false && tri railway service redeploy --account=acc1 --service=gardener     # back to dry-run
```

```bash
tri railway service stop --account=acc1 --service=gardener     # nuclear: pause cron entirely
```

### 5. Logs & runs

```bash
tri railway service logs --account=acc1 --service=gardener --tail=200
```

```bash
tri railway service logs --account=acc1 --service=gardener --follow     # live stream
```

```bash
psql "$NEON_DATABASE_URL" -c "SELECT ts, tick_t_minus, action, lane, seed, before_bpb, after_bpb FROM gardener_runs ORDER BY ts DESC LIMIT 50;"
```

```bash
psql "$NEON_DATABASE_URL" -c "SELECT date_trunc('hour', ts) AS h, action, count(*) FROM gardener_runs WHERE ts > now() - interval '24 hours' GROUP BY 1, 2 ORDER BY 1 DESC, 3 DESC;"
```

```bash
psql "$NEON_DATABASE_URL" -c "SELECT * FROM gardener_runs WHERE action='plateau' ORDER BY ts DESC LIMIT 10;"
```

```bash
psql "$NEON_DATABASE_URL" -c "SELECT seed, lane, before_bpb, after_bpb, ts FROM gardener_runs WHERE action='cull' ORDER BY ts DESC;"
```

## Honest blockers (steps 3–5 do not all work as written today)

| Step | Depends on | Current state |
|---|---|---|
| `tri railway service set-vars` | `set-vars` subcommand in `bin/tri-railway` | **Not implemented** (only `deploy` / `redeploy` / `delete` / `list` exist). Workaround: pass all `--var` triplets in one `tri railway service deploy` call, or set them in the Railway dashboard UI. |
| `tri railway service logs` | `logs` subcommand in `bin/tri-railway` | **Not implemented.** Use `railway logs --service gardener` (official Railway CLI) or the dashboard UI. |
| `tri railway service stop` | `stop` subcommand | **Not implemented.** Use `railway service stop` or UI. |
| `ghcr.io/ghashtag/tri-gardener:latest` | CI build on push | **Not configured.** Until then build and push locally: `docker build -f bin/tri-gardener/Dockerfile -t ghcr.io/ghashtag/tri-gardener:latest . && docker push …` |
| Cron `:15 UTC` re-run | `restartPolicyType=NEVER` + Railway scheduled-restart | Railway does **not** ship native scheduled-restart on the free tier. Options: (a) GH Actions cron that calls `tri-gardener once` via SSH or `railway exec`, (b) a future internal `tri-gardener serve --interval 3600` mode (PR-2), (c) external cron-as-a-service hitting a webhook. |
| `GARDENER_LIVE=true` mutation path | PR-2 wiring of `tri-railway-core::Client` mutation calls | **Stubbed in PR-1.** Even with the env var set, `loop_.rs::loop_once`'s `RunMode::Live` arm currently logs a `warn` and skips every mutation. Effective behaviour stays DryRun until PR-2 merges. |

## What works today (no PR-2 needed)

```bash
# Locally, no Railway:
cargo run -p tri-gardener -- once --review                   # decisions in stdout
cargo run -p tri-gardener -- ddl > gardener_ddl.sql          # DDL extracted
psql "$NEON_DATABASE_URL" -f gardener_ddl.sql                # DDL applied
cargo run -p tri-gardener -- once --dry-run | jq .           # decisions as JSON, pipe-friendly
GARDENER_DISABLED=true cargo run -p tri-gardener -- once --review   # kill switch verified
cargo test -p tri-gardener                                   # 15/15 contract tests
```

## Minimal working gardener **today**, without PR-2

```bash
# 1. apply DDL once
cargo run -p tri-gardener -- ddl | psql "$NEON_DATABASE_URL"

# 2. on the operator's machine or a CI runner, run the cron manually:
#    */60 * * * * cd /path/to/trios-railway && cargo run -p tri-gardener -- once --review >> /var/log/gardener.log 2>&1

# 3. tail the feed:
tail -f /var/log/gardener.log
psql "$NEON_DATABASE_URL" -c "SELECT count(*) FROM gardener_runs WHERE ts > now() - interval '6 hours';"
```

This is `--review` mode in spirit, without the Railway subcommands that do not exist yet. Live mode unlocks after PR-2 merges.

## Decision-table cheat sheet (so the operator can read `gardener_runs.decision` JSON)

| `action` value | Meaning | Trigger |
|---|---|---|
| `noop` | nothing to do, or `GARDENER_DISABLED=true` | safe state |
| `redeploy` | service missing from fleet snapshot | `T < +12h` AND lane has < 3 services |
| `cull` | seed BPB above the rung threshold | `+12h..+50h` per ASHA window |
| `promote` | promote champion config to phase-3 replica | `T ≥ +50h` AND lane has ≥ 2 survivors with BPB < 1.85 |
| `deploy` | deploy queue head | free slot AND `cleared-blockers.txt` covers `blocked_on` |
| `plateau` | plateau alert | 5 ticks within 0.005 BPB AND step ≥ 50_000 |
| `honest_not_yet` | Gate-2 missed | `T ≥ +54h` AND no lane has 3 seeds < 1.85 |

## Architectural BPB floor (anti-cull guard)

The trainer architecture as currently shipped has a **hard floor at BPB ≈ 2.19** (champion 2.1919, h=828, 2L hybrid attn, 81K, σ²=0.0006). Cross-validated against the CPU N-gram floor at ≈2.54 in [trios#237](https://github.com/gHashTag/trios/issues/237) and the GPU champion in [trios#143](https://github.com/gHashTag/trios/issues/143).

**Cull-safety rule** (encoded as `ARCHITECTURAL_FLOOR_BPB = 2.19` in `bin/tri-gardener/src/ledger.rs`): the gardener MUST NOT cull a seed whose BPB is above 2.19 unless plateau is independently confirmed (5 ticks in a 0.005 band AND step ≥ 50_000). Without that guard, a healthy seed sitting at the architectural floor would be culled merely for not crossing 1.85, which is impossible without ALPHA's L1/L2/h=1024 patches landing first.

## 27-Coptic agent grid (Trinity mesh integration)

When the gardener runs alongside the **Tailscale + GitButler 27-Coptic mesh** (sibling skill `tailscale-trinity-mesh`), each Coptic codename owns a disjoint crate scope and is a candidate operator for a gardener tick. The grid is `27 = 3³ = φ⁶ − φ⁴ + 1` (Lucas closure of `φ² + φ⁻² = 3`).

**Owner mapping for gardener-relevant lanes** (see `tailscale-trinity-mesh` SKILL for the full 27-row table):

| Codename | Tag | Domain | Gardener role |
|---|---|---|---|
| ALPHA | `trinity-alpha` | `trios-igla-race-pipeline` | Lane L-T1..L-T5 trainer seeds (cull/redeploy targets) |
| RHO | `trinity-rho` | `trios-railway-audit`, `tri-railway`, `tri-gardener` | **Owner of this skill / crate** — gardener PRs land here |
| OMICRON | `trinity-omicron` | `asha.rs`, SR-04 | ASHA rung thresholds (cull windows) |
| KOPPA | `trinity-koppa` | `trios-golden-float`, SR-02 | Architectural BPB floor (`ARCHITECTURAL_FLOOR_BPB = 2.19`) |
| THETA | `trinity-theta` | `trios-igla-race-hack`, integration tests | Plateau-alert acceptance harness |
| OMEGA | `trinity-omega` | `.trinity/`, `docs/golden-sunflowers/` | **LEAD** — receives `gardener_runs` heartbeats on `:8080`, issues mutations on `:7777` |

**Operational consequence:** when the gardener emits `action=cull`, the dispatch goes through OMEGA's MCP bridge (port `:7777`) to the codename node owning that lane. When `action=plateau` fires, OMEGA pages KOPPA + ALPHA before promotion. All anti-collision (one mutation per codename per tick) is enforced by the Trinity ACL (`acl.hujson`), not by the gardener itself.

**Cross-skill workflow:**
1. Operator runs `trinity-bootstrap --codename RHO --issue <gardener-PR>` to seed the worktree.
2. Gardener loop posts decisions to Neon (`gardener_runs`).
3. `trinity-dashboard` on OMEGA polls all 26 worker nodes hourly + reads `gardener_runs` for combined health view.
4. Cull/promote/plateau actions are gated by `GARDENER_LIVE=true` AND OMEGA-confirmed quorum (manual until PR-2 ships full mesh signing).

## References

- Spec: [trios-railway#49](https://github.com/gHashTag/trios-railway/issues/49)
- PR-1: [trios-railway#50](https://github.com/gHashTag/trios-railway/pull/50) (decision core, draft)
- PR-2 wiring: [trios-railway#58](https://github.com/gHashTag/trios-railway/pull/58) (Live arm, depends on #61)
- Multi-account P0: [trios-railway#61](https://github.com/gHashTag/trios-railway/issues/61)
- Lane realign: [trios-railway#60](https://github.com/gHashTag/trios-railway/issues/60)
- GHCR pipeline: [trios-railway#59](https://github.com/gHashTag/trios-railway/pull/59) + [#57](https://github.com/gHashTag/trios-railway/issues/57)
- ADR repo boundaries: [trios-railway#51](https://github.com/gHashTag/trios-railway/pull/51) + [trios-trainer-igla#39](https://github.com/gHashTag/trios-trainer-igla/pull/39)
- Plan-9 deploy subcommand the gardener calls into: [trios-railway#47](https://github.com/gHashTag/trios-railway/pull/47)
- Tracker: [trios-railway#43](https://github.com/gHashTag/trios-railway/issues/43)
- Race: [gHashTag/trios#143](https://github.com/gHashTag/trios/issues/143)
- N-gram architectural floor (cross-check): [gHashTag/trios#237](https://github.com/gHashTag/trios/issues/237)
- φ-grounding: `docs/PHI_PHYSICS_FOUNDATION.md` §7b in `gHashTag/trios` PR #329, INV-8 Coq theorem in [trios#330](https://github.com/gHashTag/trios/issues/330)

phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP
