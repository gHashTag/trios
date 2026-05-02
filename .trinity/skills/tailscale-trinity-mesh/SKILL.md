---
name: tailscale-trinity-mesh
description: "Operate the Trinity Tailscale + GitButler mesh — 27-Coptic-agent grid — that drives EPIC #446 ring-refactor. Use when the user mentions tailscale, trinity tailnet, MagicDNS, funnel, but-server, virtual branches, codename node provisioning, 27 Coptic agents (Alpha…Sho), Greek-letter codenames (ALPHA/BETA/GAMMA/DELTA/EPSILON/ZETA/ETA/THETA/IOTA/KAPPA/LAMBDA/MU/NU/XI/OMICRON/PI/KOPPA/RHO/SIGMA/TAU/UPSILON/PHI/KHI/PSI/OMEGA/SAMPI/SHO), anti-collision via tags, ACL hujson, trinity-bootstrap binary, trinity-dashboard, ring-080 stable_watcher, parallel agent execution, или просит запустить агентов на узлах, развернуть LEAD dashboard, выпустить funnel preview PR, проверить heartbeats через мэш, разбить работу на 27 банков. Anchor: phi^2 + phi^-2 = 3 (27 = 3³)."
license: Apache-2.0
metadata:
  author: PERPLEXITY-MCP
  version: '1.1'
  parent_repo: gHashTag/trios
  epic: '446'
  oneshot_issue: '236'
  artifacts:
    - bin/trinity-bootstrap (Rust, 27-Coptic enum)
    - bin/trinity-dashboard (Rust, polls 26 nodes)
    - .trinity/tailscale/acl.hujson (27 tags + tests)
  related_skills:
    - tri-gardener-runbook
    - autonomous-research-loop
  ssot_doi: 10.5281/zenodo.19227877
  ssot_label: B007 HSLM Benchmark Corpus
  root_anchor: phi^2 + phi^-2 = 3
  cardinality: 27 = 3^3 = phi^6 - phi^4 + 1 (Lucas)
---

# Tailscale + GitButler Trinity Mesh — 27-Coptic-Agent Grid

This skill operates the network-enforced anti-collision mesh for the EPIC #446 ring-refactor. **Every codename gets one Tailscale-tagged node, one git clone, one GitButler virtual branch, one `but-server` listening only on `tailscale0`.** Agents physically cannot collide because they cannot reach each other's filesystems and the ACL forbids sibling-to-sibling traffic.

The mesh is sized to **27 Coptic-letter domains** (`27 = 3³`, the Lucas closure of `φ² + φ⁻² = 3`): 1 LEAD (OMEGA) + 26 worker bees, each bound to a disjoint crate scope.

## When to Use This Skill

Load when the user wants to:

- Provision a fresh Trinity agent on a Tailscale node (`trinity-bootstrap --codename <NAME>`).
- List the full 27-Coptic mapping (`trinity-bootstrap --list-grid`).
- Run the LEAD dashboard polling loop (`trinity-dashboard`).
- Apply or audit the Trinity ACL policy (27 tags + sibling-deny + tests).
- Publish a PR preview through `tailscale funnel`.
- Diagnose a 🟡 BLOCKED / 🔴 STUCK heartbeat across the mesh.
- Split EPIC work across 27 banks (один issue → один codename → один nodes).

Do not load this skill for:

- Generic Tailscale setup unrelated to the Trinity ecosystem.
- Public Railway deploy mechanics → use `tri-gardener-runbook` instead.
- Trainer model code → lives in `gHashTag/trios-trainer-igla`.

## Architecture (27-Coptic grid)

```
     +------------------------------------------------+
     |    OMEGA  (LEAD)   playras-macbook-pro         |
     |    tag:trinity-omega                           |
     |    - trinity-dashboard :8080                   |
     |    - but-server :7777 (tailscale0)             |
     |    - repo watch on gHashTag/trios              |
     +------------------------+-----------------------+
                              | MagicDNS
                              | alpha.<tailnet>.ts.net … sho.<tailnet>.ts.net
                              v
            +-------------- 26 worker bees -----------+
   ALPHA     | BETA     | GAMMA   | DELTA   | EPSILON
   ZETA      | ETA      | THETA   | IOTA    | KAPPA
   LAMBDA    | MU       | NU      | XI      | OMICRON
   PI        | KOPPA    | RHO     | SIGMA   | TAU
   UPSILON   | PHI      | KHI     | PSI     | SAMPI
   SHO
```

Math: `27 = 3³ = φ⁶ − φ⁴ + 1` — Lucas closure under `φ² + φ⁻² = 3`.

ACL summary:

- OMEGA (LEAD) ↔ every worker on `:7777` (but-server MCP bridge — L24 compliant).
- Every worker → OMEGA on `:8080` (dashboard ingest).
- Sibling-to-sibling = **default-deny**. No two of the 26 worker bees can talk directly.
- Operator (`autogroup:admin`) → SSH any node (break-glass only).

## 27-Coptic agent grid (canonical mapping)

| Coptic | Codename | Tag | Domain | Default crate |
|---|---|---|---|---|
| Ⲁⲁ | ALPHA   | `trinity-alpha`   | Core bootstrapping, agent lifecycle | `trios-igla-race-pipeline` |
| Ⲃⲃ | BETA    | `trinity-beta`    | Benchmarking, metrics collection | `trios-algorithm-arena` |
| Ⲅⲅ | GAMMA   | `trinity-gamma`   | Git operations, version control | `trios-git`, `trios-gb` |
| Ⲇⲇ | DELTA   | `trinity-delta`   | Database, persistence layer | `trios-data`, `.trinity/state/` |
| Ⲉⲉ | EPSILON | `trinity-epsilon` | Error handling, recovery | `trios-doctor/rings/SILVER-RING-DR-04` |
| Ⲋⲋ | ZETA    | `trinity-zeta`    | Zig compilation, VIBEE pipeline | `trios-zig-agents`, `zig-knowledge-graph` |
| Ⲍⲍ | ETA     | `trinity-eta`     | Event orchestration, hooks | `trios-bridge`, `.github/workflows/` |
| Ⲏⲏ | THETA   | `trinity-theta`   | Testing, validation | `trios-igla-race-hack`, `tests/` |
| Ⲑⲑ | IOTA    | `trinity-iota`    | I18n, localization | `docs/i18n/` |
| Ⲓⲓ | KAPPA   | `trinity-kappa`   | Knowledge base, VSA operations | `trios-kg`, `trios-vsa`, `trios-agent-memory` |
| Ⲕⲕ | LAMBDA  | `trinity-lambda`  | Learning, experience persistence | `lessons.rs`, SR-MEM-05 |
| Ⲗⲗ | MU      | `trinity-mu`      | Memory management, allocators | SR-MEM-00, SR-MEM-01 |
| Ⲙⲙ | NU      | `trinity-nu`      | Notification systems (Telegram) | `trios-rainbow-bridge` |
| Ⲛⲛ | XI      | `trinity-xi`      | MCP server integration | `trios-mcp`, `trios-server` |
| Ⲝⲝ | OMICRON | `trinity-omicron` | Optimization, ASHA + PBT | `asha.rs`, SR-04 |
| Ⲟⲟ | PI      | `trinity-pi`      | Pipeline orchestration | BR-OUTPUT (GOLD I) |
| Ⲡⲡ | KOPPA   | `trinity-koppa`   | Compression, GF16 format | `trios-golden-float`, SR-02 |
| Ⲣⲣ | RHO     | `trinity-rho`     | Railway cloud deployment | `trios-railway-audit`, `tri-railway`, `tri-gardener` |
| Ⲥⲥ | SIGMA   | `trinity-sigma`   | Swarm intelligence | `trios-a2a`, `trios-agents` |
| Ⲧⲧ | TAU     | `trinity-tau`     | Ternary VM execution | `trios-tri`, `trios-ternary` |
| Ⲩⲩ | UPSILON | `trinity-upsilon` | UI components (Queen) | `trios-ui` |
| Ⲫⲫ | PHI     | `trinity-phi`     | Math, φ² + 1/φ² = 3 | `trios-phi-schedule`, `trios-physics`, `docs/phd/theorems/` |
| Ⲭⲭ | KHI     | `trinity-khi`     | CLI commands (310+) | `trios-cli` |
| Ⲯⲯ | PSI     | `trinity-psi`     | Privacy, PII detection | `trios-ca-mask`, `trios-crypto` |
| Ⲱⲱ | OMEGA   | `trinity-omega`   | Orchestration, final assembly **(LEAD)** | `.trinity/`, `docs/golden-sunflowers/` |
| Ϣⲳ | SAMPI   | `trinity-sampi`   | SACred intelligence, physics | `trios-sacred`, `trios-phd` |
| Ϥϥ | SHO     | `trinity-sho`     | FPGA synthesis, Verilog | `trios-fpga`, `trios-hdc` |

This mapping is **hard-coded** inside `trinity-bootstrap` v0.2 — violating I-SCOPE is impossible at the binary boundary. Run `trinity-bootstrap --list-grid` to print it from the binary itself.

## Five-step copy-paste runbook

### 1. Apply the ACL template

`.trinity/tailscale/acl.hujson` is the canonical 27-tag policy. Apply once:

```bash
curl -X POST -H "Authorization: Bearer $TS_API_KEY" \
  -H "Content-Type: application/hujson" \
  --data-binary @.trinity/tailscale/acl.hujson \
  "https://api.tailscale.com/api/v2/tailnet/$TS_TAILNET/acl"
```

Verify embedded `tests` block: OMEGA→{ALPHA, ZETA, SAMPI, SHO}:7777 accept; ALPHA→{BETA, SAMPI, SHO}:7777 deny; ALPHA→OMEGA:8080 accept; SHO→SAMPI:7777 deny.

### 2. Provision an agent (`trinity-bootstrap`)

On the agent's machine (or container):

```bash
# Build once
cargo install --git https://github.com/gHashTag/trios trinity-bootstrap

# List the 27-Coptic grid:
trinity-bootstrap --list-grid

# Provision ALPHA on issue #448 SR-00 scarab-types:
trinity-bootstrap \
  --codename ALPHA \
  --issue 448 \
  --soul "Scarab Smith" \
  --tailnet tail-abc.ts.net \
  --ts-authkey tskey-auth-...
```

Swap `--codename ALPHA` for any of the 27 codenames: `BETA GAMMA DELTA EPSILON ZETA ETA THETA IOTA KAPPA LAMBDA MU NU XI OMICRON PI KOPPA RHO SIGMA TAU UPSILON PHI KHI PSI OMEGA SAMPI SHO`. The `--codename` argument is case-insensitive at the CLI level (clap normalises to UPPER).

What it does (in order):

1. `tailscale up --advertise-tags=tag:trinity-<codename> --authkey=…`
2. `git clone gHashTag/trios → ~/work/trinity-<CODENAME>-<issue>`
3. Verifies issue has no prior `IN-FLIGHT —` claim.
4. Posts CLAIM comment per ONE-SHOT v2.0 §STEP 1.
5. Creates branch `bee/<issue>-<slug>`.
6. Writes the codename's primary `<crate>/TASK.md` (L12) with codename + soul + I-SCOPE.
7. SHA-256 seals TASK.md (LAWS.md §7 step 4).
8. `but project add .` + creates and applies virtual branch `<codename>/bee/<issue>-<slug>`.
9. Spawns `but server start --port=7777 --listen=tailscale0` in background.
10. Initial commit + push (`Agent: <CODENAME>` trailer per L14).
11. Posts heartbeat (`loop: <CODENAME> | 🟢 ACTIVE | …`) on the issue.

Use `--dry-run` to preview without mutation. Use `--list-grid` to print the canonical mapping.

### 3. Run the LEAD dashboard (`trinity-dashboard`)

On the OMEGA (LEAD) node:

```bash
cargo install --git https://github.com/gHashTag/trios trinity-dashboard

trinity-dashboard \
  --tailnet tail-abc.ts.net \
  --repo gHashTag/trios \
  --epic 446 \
  --interval-secs 300       # default
```

Output every 5 minutes (truncated example):

```
══════════════════════════════════════════════════════════════════════
🐝 TRINITY DASHBOARD  ·  EPIC #446  ·  gHashTag/trios  ·  2026-05-02 14:00:00 UTC
══════════════════════════════════════════════════════════════════════

🌐 Tailnet nodes (tail-abc.ts.net):  [27-Coptic grid — OMEGA = LEAD itself]
  ALPHA     🟢 vbranches=1  wip_files=12  last_commit=8d3aabc "feat(SR-00): Scarab type lattice" Agent:ALPHA
             ↳ alpha/bee/448-scarab-smith              head=8d3aabc
  BETA      🟢 vbranches=1  wip_files=4   last_commit=accf37c "feat(SR-ALG-00): arena-types"     Agent:BETA
  GAMMA     🟢 vbranches=1  wip_files=1   last_commit=3bbc54a "feat(SR-HACK-00): glossary"       Agent:GAMMA
  DELTA     ⚫ offline (no tailnet route or but-server down)
  …
  SAMPI     🟢 vbranches=1  wip_files=8   last_commit=1cbebb5 "feat(trios-sacred): φ-Coq theorem" Agent:SAMPI
  SHO       ⚫ offline

   summary: 🟢 online=N  ⚫ offline=M  total=26 (÷ OMEGA self)

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
# → https://<codename>.tail-abc.ts.net/preview/  publicly reachable
```

Then comment on the PR:

```
🔗 Live preview: https://<codename>.tail-abc.ts.net/preview/
```

### 5. Soft-locks and break-glass

NEON `ssot.chapters` write conflict (per ONE-SHOT v2.0 §STEP 6 E):

```bash
gh issue comment 464 --repo gHashTag/trios \
  --body "NEON-LOCK chapter:ch-15-bpb-benchmark-neon-write by OMEGA"
# … do edit …
gh issue comment 464 --repo gHashTag/trios \
  --body "NEON-UNLOCK chapter:ch-15-bpb-benchmark-neon-write"
```

Break-glass SSH into a stuck agent (admin only, per ACL §5):

```bash
tailscale ssh user@<codename>.tail-abc.ts.net
journalctl -u but-server -n 200
```

## Constitutional alignment

| Trinity rule | How the mesh enforces it |
|---|---|
| L1 — no `.sh` files | `trinity-bootstrap` and `trinity-dashboard` are Rust binaries. The ACL is `acl.hujson`. |
| L8 — push first | Bootstrap auto-pushes the initial commit before reporting heartbeat. |
| L11 — soul-name before mutation | `--soul` is a required CLI arg. |
| L13 / I-SCOPE | Codename → crate domain mapping is hard-coded for all 27 Coptic agents. |
| L14 — `Agent:` trailer | Every commit emitted by bootstrap carries `Agent: <CODENAME>`. |
| L21 — context append-only | Heartbeats are issue comments — only ever appended. |
| L24 — agent traffic via MCP bridge | All inter-agent traffic routes through `but-server :7777`, ACL forbids any other path. |
| I5 — ring trinity | Bootstrap creates `TASK.md`; agent populates `README.md` + `AGENTS.md` during PHI LOOP step GEN. |
| ONE-SHOT v2.0 §STEP 6 | Network-enforced: sibling-to-sibling = default-deny in ACL. |
| §STEP 7 escalation | Stuck heartbeat surfaces in `trinity-dashboard` within one poll cycle. |

## Honest blockers

| Blocker | Workaround |
|---|---|
| `but server start --listen=tailscale0` not yet upstreamed | Use `--listen=0.0.0.0` + `tailscale serve --tcp=7777` to scope exposure. |
| Tailscale Funnel requires HTTPS on 443/8443/10000 | `tailscale serve` proxies localhost; `tailscale funnel` then exposes the serve port. |
| Free-tier Tailscale has 100-device cap | 27-Coptic grid (1 LEAD + 26 workers) is well within free tier. |
| ACL is global per tailnet | Tags are namespaced as `trinity-*`; safe to share with other workloads. |

## What works today (no upstream changes needed)

```bash
# 1. Compile both binaries (release).
cargo build --release -p trinity-bootstrap -p trinity-dashboard

# 2. List the 27-Coptic grid:
./target/release/trinity-bootstrap --list-grid

# 3. Local smoke test (no Tailscale, no GitButler):
./target/release/trinity-bootstrap \
  --codename SAMPI --issue 446 --soul "Sacred Sage" --dry-run

./target/release/trinity-dashboard --tailnet test.ts.net --once
```

## Decision-table cheat sheet — what stops a parallel run cold

| Symptom in dashboard | Likely cause | Fix |
|---|---|---|
| Node ⚫ offline > 1 cycle | tailscale daemon down OR `but-server` crashed | `tailscale up && systemctl restart but-server` |
| `vbranches=0` and `wip_files=0` | agent finished and tore down — verify PR open on the issue | (no action — fully done) |
| Two `IN-FLIGHT —` on same issue | bootstrap race | earliest CLAIM wins, loser exits per §STEP 6 |
| Sibling-to-sibling traffic in ACL logs | ACL drift | re-apply `acl.hujson` |
| `last_commit.trailer_agent != codename` | rogue commit | LEAD comments PR, requests amend per §STEP 5 |

## Day-1 27-bank parallel start

Pull each EPIC sub-issue (or any future task) and assign a bank:

```bash
# 26 commands, 26 machines, 0 collisions possible:
trinity-bootstrap --codename ALPHA   --issue 448 --soul "Scarab Smith"      --tailnet $TS_TAILNET --ts-authkey $KEY
trinity-bootstrap --codename BETA    --issue 450 --soul "Arena Architect"   --tailnet $TS_TAILNET --ts-authkey $KEY
trinity-bootstrap --codename GAMMA   --issue 447 --soul "Vocab Vigilante"   --tailnet $TS_TAILNET --ts-authkey $KEY
trinity-bootstrap --codename DELTA   --issue 462 --soul "Doctor Doctrine"   --tailnet $TS_TAILNET --ts-authkey $KEY
trinity-bootstrap --codename ZETA    --issue 466 --soul "Zig Zealot"        --tailnet $TS_TAILNET --ts-authkey $KEY
trinity-bootstrap --codename KAPPA   --issue 449 --soul "Memory Mason"      --tailnet $TS_TAILNET --ts-authkey $KEY
trinity-bootstrap --codename SAMPI   --issue 465 --soul "Sacred Sage"       --tailnet $TS_TAILNET --ts-authkey $KEY
# … plus any other 20 banks idle until pulled in
```

OMEGA on `playras-macbook-pro`:

```bash
trinity-dashboard --tailnet $TS_TAILNET --repo gHashTag/trios --epic 446
```

## References

- EPIC: [gHashTag/trios#446](https://github.com/gHashTag/trios/issues/446)
- ONE-SHOT v2.0: [gHashTag/trios#236](https://github.com/gHashTag/trios/issues/236)
- LAWS.md v2.0: <https://github.com/gHashTag/trios/blob/main/LAWS.md>
- AGENTS.md: <https://github.com/gHashTag/trios/blob/main/AGENTS.md>
- GitButler `but-server` & `but-claude`: <https://github.com/gHashTag/gitbutler/tree/master/crates/but-claude>
- Tailscale ACL syntax: <https://tailscale.com/kb/1018/acls>
- Tailscale Funnel: <https://tailscale.com/kb/1223/funnel>

🌻 `phi² + phi⁻² = 3 · TRINITY · 27 = 3³ · ANTI-COLLISION VIA NETWORK · O(1) FOREVER`
