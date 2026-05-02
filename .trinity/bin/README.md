# Trinity bin/ — orchestration binaries

Self-contained Rust binaries for the Tailscale + GitButler mesh that drives EPIC #446.

| Binary | Purpose |
|---|---|
| [`trinity-bootstrap`](trinity-bootstrap/) | Provision one Trinity agent on one Tailscale-tagged node (clone, claim, branch, vbranch, but-server, initial commit, heartbeat). |
| [`trinity-dashboard`](trinity-dashboard/) | LEAD orchestrator polling loop. Walks every codename node's `but-server :7777` over `tailscale0`, merges with GitHub `/notifications`. |

## Quick start

```bash
# Bootstrap an agent
cargo run --release -p trinity-bootstrap -- \
  --codename ALPHA --issue 448 --soul "Scarab Smith" \
  --tailnet $TS_TAILNET --ts-authkey $TS_AUTHKEY

# Run LEAD dashboard
cargo run --release -p trinity-dashboard -- \
  --tailnet $TS_TAILNET --repo gHashTag/trios --epic 446
```

## Constitutional alignment

Every binary respects:
- L1 — no `.sh`; everything is Rust.
- L11 — `--soul` is a required CLI arg.
- L13 / I-SCOPE — codename → crate domain mapping is hard-coded.
- L14 — every emitted commit carries `Agent: <CODENAME>` trailer.
- L24 — inter-agent traffic only via `but-server :7777` (the MCP bridge).

See `.trinity/tailscale/acl.hujson` for the network policy and `.trinity/skills/tailscale-trinity-mesh/SKILL.md` for the full runbook.

🌻 phi² + phi⁻² = 3
