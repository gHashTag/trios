# SR-05 — TASK

**Closes:** #458 · **Part of:** #446 · **Soul:** Loop-Locksmith · **Tier:** 🥈 Silver

## Goal

Integrate the operator surface from `gHashTag/trios-railway` (`bin/tri-railway`,
`crates/trios-railway-audit`, `bin/tri-gardener`, `bin/seed-agent`) — but at the
contract layer. Reaffirm L1 (no `.sh`) inside this ring tree.

## Acceptance criteria (issue AC ↔ this ring)

- [x] `rings/SR-05/` with I5 trinity (README, TASK, AGENTS, RING, Cargo, lib).
- [x] Deps: SR-00..04 + serde/thiserror/tracing. **`trios-railway-audit` git
      dep + `reqwest` deferred to sibling BR-IO ring** (R-RING-DEP-002 +
      offline-build hygiene; R5-honest in README).
- [x] Public API: `RailwayDeployer::deploy(&FleetSnapshot)`, `audit()`,
      `fleet_snapshot()`.
- [x] Observe-only mode by default (`DeployerMode::Observe`); Apply mode
      requires explicit constructor opt-in (env-var-driven check ships at
      the BR-IO adapter boundary that does the actual mutation, mirroring
      `DEPLOYER_MODE=apply`).
- [x] No `.sh` shipped; ring tree audited at scaffold time.
- [x] Doctor rule `R-L1-ECHO-006` re-runs L1 check inside SR-05 tree —
      contract present, tree is `.sh`-free.
- [x] R7 audit triplet `RAIL=<verb> @ project=<8c> service=<8c> sha=<8c>
      ts=<rfc3339>` honoured by `R7Triplet::format`.
- [ ] Integration smoke against a Railway dev project — **deferred to
      BR-IO adapter PR** (sibling ring; needs RAILWAY_TOKEN_TEST secret).
- [x] PR closes this issue, `Agent: Loop-Locksmith` trailer.

🌻 `phi^2 + phi^-2 = 3`
