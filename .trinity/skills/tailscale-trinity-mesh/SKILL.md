---
name: tailscale-trinity-mesh
description: "Operate the Trinity Tailscale + GitButler mesh that drives EPIC #446 ring-refactor. Use when the user mentions tailscale, trinity tailnet, MagicDNS, funnel, but-server, virtual branches, codename node provisioning, ALPHA/BETA/GAMMA/DELTA/ZETA/LEAD nodes, anti-collision via tags, ACL hujson, trinity-bootstrap binary, trinity-dashboard, ring-080 stable_watcher, parallel agent execution, или просит запустить агентов на узлах, развернуть LEAD dashboard, выпустить funnel preview PR, проверить heartbeats через мэш. Anchor: phi^2 + phi^-2 = 3."
license: Apache-2.0
metadata:
  author: PERPLEXITY-MCP
  version: '1.0'
  parent_repo: gHashTag/trios
  epic: '446'
  oneshot_issue: '236'
  artifacts:
    - bin/trinity-bootstrap (Rust)
    - bin/trinity-dashboard (Rust)
    - .trinity/tailscale/acl.hujson
  related_skills:
    - tri-gardener-runbook
    - autonomous-research-loop
  ssot_doi: 10.5281/zenodo.19227877
  ssot_label: B007 HSLM Benchmark Corpus
  root_anchor: phi^2 + phi^-2 = 3
---

# Tailscale + GitButler Trinity Mesh — MVP runbook

This skill operates the network-enforced anti-collision mesh for the EPIC #446 ring-refactor: every codename gets one Tailscale-tagged node, one git clone, one GitButler virtual branch, and one `but-server` listening only on `tailscale0`. Agents physically cannot collide because they cannot reach each other's filesystems and the ACL forbids sibling-to-sibling traffic.

## When to Use This Skill

Load when the user wants to:

- Provision a fresh Trinity agent on a Tailscale node (`trinity-bootstrap`).
- Run the LEAD dashboard polling loop (`trinity-dashboard`).
- Apply or audit the Trinity ACL policy (`acl.hujson`).
- Publish a PR preview through `tailscale funnel`.
- Diagnose a 🟡 BLOCKED / 🔴 STUCK heartbeat across the mesh.
- Add a new codename or retire one.

Do not load this skill for:

- Generic Tailscale setup unrelated to the Trinity ecosystem → use the official Tailscale docs.
- Public Railway deploy mechanics → use `tri-gardener-runbook` instead.
- Trainer model code → lives in `gHashTag/trios-trainer-igla`.

## Architecture (one-pager)

```
                ┌────────────────────────────────────────────┐
                │     LEAD  (playras-macbook-pro)            │
                │     tag:trinity-lead                        │
                │     • trinity-dashboard :8080               │
                │     • but-server :7777 (tailscale0)         │
                │     • repo watch on gHashTag/trios          │
                └────────────────────┬───────────────────────┘
                                     │  MagicDNS
                                     │  (alpha.<tailnet>.ts.net …)
       ┌─────────┬─────────┬─────────┼─────────┬─────────┬─────────┐
       ▼         ▼         ▼         ▼         ▼         ▼         ▼
   ALPHA     BETA      GAMMA     DELTA     EPSILON    ZETA      operator (admin)
 GOLD I    GOLD II   GOLD III   doctor    trios-ext  GOLD IV    SSH break-glass
 #448      #450      #447       #462      —          #449       any node
```

ACL summary:

- LEAD ↔ every agent on `:7777` (but-server MCP bridge — L24 compliant).
- Every agent → LEAD on `:8080` (dashboard ingest).
- Sibling-to-sibling = **default-deny**. No two agents can talk directly.
- Operator (`autogroup:admin`) → SSH any node.

## Five-step copy-paste runbook

### 1. Apply the ACL template

`.trinity/tailscale/acl.hujson` is the canonical policy. Apply it once:

```bash
# In the Tailscale admin console: paste the contents of .trinity/tailscale/acl.hujson
# OR via API:
curl -X POST -H "Authorization: Bearer $TS_API_KEY" \
  -H "Content-Type: application/hujson" \
  --data-binary @.trinity/tailscale/acl.hujson \
  "https://api.tailscale.com/api/v2/tailnet/$TS_TAILNET/acl"
```

Verify the embedded `tests` block passes (LEAD→ALPHA accept, ALPHA→BETA deny, agents→LEAD:8080 accept).

### 2. Provision an agent (`trinity-bootstrap`)

On the agent's machine (or container):

```bash
# Build once
cargo install --git https://github.com/gHashTag/trios trinity-bootstrap

# Provision ALPHA on issue #448 SR-00 scarab-types
trinity-bootstrap \
  --codename ALPHA \
  --issue 448 \
  --soul "Scarab Smith" \
  --tailnet tail-abc.ts.net \
  --ts-authkey tskey-auth-...    # one-time key from admin console
```

What it does (in order):

1. `tailscale up --advertise-tags=tag:trinity-alpha --authkey=…`
2. `git clone gHashTag/trios → ~/work/trinity-ALPHA-448`
3. Verifies issue #448 has no prior `IN-FLIGHT —` claim.
4. Posts CLAIM comment per ONE-SHOT v2.0 §STEP 1.
5. Creates branch `bee/448-scarab-smith`.
6. Writes `crates/trios-igla-race-pipeline/TASK.md` (L12) with codename + soul + I-SCOPE.
7. SHA-256 seals TASK.md (LAWS.md §7 step 4).
8. `but project add .` + creates and applies virtual branch `alpha/bee/448-scarab-smith`.
9. Spawns `but server start --port=7777 --listen=tailscale0` in background.
10. Initial commit + push (`Agent: ALPHA` trailer per L14).
11. Posts heartbeat (`loop: ALPHA | 🟢 ACTIVE | …`) on the issue.

Use `--dry-run` to preview without mutation.

### 3. Run the LEAD dashboard (`trinity-dashboard`)

On the LEAD node:

```bash
cargo install --git https://github.com/gHashTag/trios trinity-dashboard

trinity-dashboard \
  --tailnet tail-abc.ts.net \
  --repo gHashTag/trios \
  --epic 446 \
  --interval-secs 300       # default
```

Output every 5 minutes:

```
══════════════════════════════════════════════════════════════════════
🐝 TRINITY DASHBOARD  ·  EPIC #446  ·  gHashTag/trios  ·  2026-05-02 14:00:00 UTC
══════════════════════════════════════════════════════════════════════

🌐 Tailnet nodes (tail-abc.ts.net):
  ALPHA     🟢 vbranches=1  wip_files=12  last_commit=8d3aabc "feat(SR-00): Scarab type lattice" Agent:ALPHA
             ↳ alpha/bee/448-scarab-smith              head=8d3aabc
  BETA      🟢 vbranches=1  wip_files=4   last_commit=accf37c "feat(SR-ALG-00): arena-types"     Agent:BETA
             ↳ beta/bee/450-arena-architect            head=accf37c
  GAMMA     🟢 vbranches=1  wip_files=1   last_commit=3bbc54a "feat(SR-HACK-00): glossary"       Agent:GAMMA
  DELTA     ⚫ offline (no tailnet route or but-server down)
  EPSILON   ⚫ offline
  ZETA      🟢 vbranches=1  wip_files=8   last_commit=1cbebb5 "feat(SR-MEM-00): memory-types"   Agent:ZETA

📬 GitHub notifications (recent unread, EPIC-relevant):
   13:55  Issue          #448 SR-00 scarab-types — heartbeat from ALPHA
   13:50  PullRequest    #467 feat(SR-HACK-00): glossary
   …

🌻 phi² + phi⁻² = 3  ·  next poll in 300 s
```

Run with `--once` for a single-cycle smoke test.

### 4. Tailscale Funnel — public PR previews

For a Svelte/Vite preview on agent's `bee/<issue>-<slug>` branch:

```bash
# On the agent node, after `pnpm dev:web` is running on 5173:
tailscale serve --bg --https=443 --set-path=/preview localhost:5173
tailscale funnel 443 on
# → https://alpha.tail-abc.ts.net/preview/  publicly reachable
```

Then comment on the PR:

```
🔗 Live preview: https://alpha.tail-abc.ts.net/preview/
```

### 5. Soft-locks and break-glass

NEON `ssot.chapters` write conflict (per ONE-SHOT v2.0 §STEP 6 E):

```bash
gh issue comment 464 --repo gHashTag/trios \
  --body "NEON-LOCK chapter:ch-15-bpb-benchmark-neon-write by LEAD"
# … do edit …
gh issue comment 464 --repo gHashTag/trios \
  --body "NEON-UNLOCK chapter:ch-15-bpb-benchmark-neon-write"
```

Break-glass SSH into a stuck agent (admin only, per ACL §5):

```bash
tailscale ssh user@alpha.tail-abc.ts.net
journalctl -u but-server -n 200
```

## Constitutional alignment

| Trinity rule | How the mesh enforces it |
|---|---|
| L1 — no `.sh` files | `trinity-bootstrap` and `trinity-dashboard` are Rust binaries. The ACL is `acl.hujson`. |
| L8 — push first | Bootstrap auto-pushes the initial commit before reporting heartbeat. |
| L11 — soul-name before mutation | `--soul` is a required CLI arg. |
| L13 / I-SCOPE | Codename → crate domain mapping is hard-coded in the Rust binary. |
| L14 — `Agent:` trailer | Every commit emitted by bootstrap carries `Agent: <CODENAME>`. |
| L21 — context append-only | Heartbeats are issue comments — only ever appended. |
| L24 — agent traffic via MCP bridge | All inter-agent traffic routes through `but-server :7777` (the MCP bridge), and the ACL forbids any other path. |
| I5 — ring trinity | Bootstrap creates `TASK.md`; the agent populates `README.md` + `AGENTS.md` during PHI LOOP step GEN. |
| ONE-SHOT v2.0 §STEP 6 | Network-enforced: sibling-to-sibling = default-deny in ACL. |
| §STEP 7 escalation | Stuck heartbeat surfaces in `trinity-dashboard` within one poll cycle. |

## Honest blockers

| Blocker | Workaround until fixed |
|---|---|
| `but server start --listen=tailscale0` not yet shipped upstream in `gitbutlerapp/gitbutler` | Use `--listen=0.0.0.0` then add a `tailscale serve --tcp=7777` rule to scope the exposure to tailnet only. |
| Tailscale Funnel requires HTTPS on 443/8443/10000 — Vite default 5173 is not directly funnellable | `tailscale serve` proxies to localhost; `tailscale funnel` then exposes the serve port. |
| Free-tier Tailscale has 100-device cap | EPIC #446 needs 7 codenames + LEAD = 8 nodes — well within free tier. |
| ACL is global per tailnet | If you share the tailnet with other workloads, namespace the tags as `tag:trinity-*`. |

## What works today (no upstream changes needed)

```bash
# 1. Compile both binaries:
cargo build --release -p trinity-bootstrap -p trinity-dashboard

# 2. Local smoke test (no Tailscale, no GitButler):
./target/release/trinity-bootstrap \
  --codename ALPHA --issue 448 --soul "Scarab Smith" --dry-run

./target/release/trinity-dashboard \
  --tailnet tail-abc.ts.net --once
```

## Decision-table cheat sheet — what stops a parallel run cold

| Symptom in dashboard | Likely cause | Fix |
|---|---|---|
| Node ⚫ offline > 1 cycle | tailscale daemon down OR `but-server` crashed | `tailscale up && systemctl restart but-server` |
| `vbranches=0` and `wip_files=0` | agent finished and tore down — verify PR open on the issue | (no action — fully done) |
| Two `IN-FLIGHT —` on same issue | bootstrap race | earliest CLAIM wins, loser exits per §STEP 6 |
| Sibling-to-sibling traffic in ACL logs | ACL drift | re-apply `acl.hujson` |
| `last_commit.trailer_agent != codename` | rogue commit | LEAD comments PR, requests amend per §STEP 5 |

## References

- EPIC: [gHashTag/trios#446](https://github.com/gHashTag/trios/issues/446)
- ONE-SHOT v2.0: [gHashTag/trios#236](https://github.com/gHashTag/trios/issues/236)
- LAWS.md v2.0: <https://github.com/gHashTag/trios/blob/main/LAWS.md>
- AGENTS.md: <https://github.com/gHashTag/trios/blob/main/AGENTS.md>
- GitButler `but-server` & `but-claude`: <https://github.com/gHashTag/gitbutler/tree/master/crates/but-claude>
- Tailscale ACL syntax: <https://tailscale.com/kb/1018/acls>
- Tailscale Funnel: <https://tailscale.com/kb/1223/funnel>

🌻 `phi² + phi⁻² = 3 · TRINITY · ANTI-COLLISION VIA NETWORK · O(1) FOREVER`
