# SR-05 — Railway Deployer (fleet integration contract + R7 audit triplet)

**Soul-name:** `Loop-Locksmith` · **Codename:** `LEAD` · **Tier:** 🥈 Silver

> Closes #458 · Part of #446 · Anchor: `φ² + φ⁻² = 3`

## Honest scope (R5)

This ring ships the **RailwayApi trait + RailwayDeployer state machine + R7 audit triplet formatter**, not the live `reqwest` GraphQL client. The trade-off mirrors SR-MEM-01 / SR-03 / SR-04 / SR-MEM-05:

- Issue AC asks for `trios-railway-audit = { git = ..., rev = <pinned-sha> } + reqwest`. Pulling reqwest with `rustls-tls` + a git-fetch dep into a Silver ring would inherit ~50 transitive crates and break offline cargo builds — that violates `R-RING-DEP-002`.
- Instead, SR-05 takes a `RailwayApi` trait. The concrete `reqwest`-backed adapter (against the Railway GraphQL endpoint) ships in a sibling BR-IO ring; SR-05 stays mock-testable without Railway credentials.
- The trios-railway-audit triplet contract (`RAIL=<verb> @ project=<8c> service=<8c> sha=<8c> ts=<rfc3339>`, see `trios-railway/AGENTS.md`) is honoured by `R7Triplet::format` — same wire format, computed from `DeployReceipt` fields with no transitive dep.
- Issue AC's `DEPLOYER_MODE=observe` default and `DEPLOYER_MODE=apply` opt-in are honoured exactly; the writer side panics in tests if Apply is reached without explicit consent.

## API

```rust
pub struct RailwayDeployer<A: RailwayApi>;

impl<A: RailwayApi> RailwayDeployer<A> {
    pub fn new(api: A, mode: DeployerMode) -> Self;

    /// Apply (or observe) the desired fleet snapshot. Returns the
    /// receipt + R7 audit triplet for every action taken.
    pub async fn deploy(&self, desired: &FleetSnapshot) -> Result<DeployReceipt, DeployErr>;

    /// Read-only audit pass. Computes drift between desired and live.
    pub async fn audit(&self) -> Result<AuditReport, DeployErr>;

    /// Snapshot the live Railway fleet (project / service / SHA / status).
    pub async fn fleet_snapshot(&self) -> Result<FleetSnapshot, DeployErr>;
}

pub trait RailwayApi: Send + Sync { /* 4 async fns */ }
pub enum DeployerMode { Observe, Apply }
pub struct FleetSnapshot { project_id, services: Vec<RailwayService> }
pub struct RailwayService { project_id, service_id, sha, status }
pub struct DeployReceipt { actions: Vec<DeployAction>, triplets: Vec<R7Triplet> }
pub struct AuditReport { drift_count, missing, extra, sha_mismatch }
pub struct R7Triplet { verb, project_id, service_id, sha, ts }
pub enum DeployErr { Backend, ModeRefused, ShaTooShort }
```

## R7 audit triplet (`AGENTS.md` of `trios-railway`)

`RAIL=<verb> @ project=<8c> service=<8c> sha=<8c> ts=<rfc3339>` — every deploy / audit action emits one triplet, captured in [`DeployReceipt::triplets`]. The formatter trims project/service/sha to 8 chars and refuses anything shorter than 8.

## Dependencies (R-RING-DEP-002)

```
serde, serde_json, chrono, thiserror, tracing
+ trios-igla-race-pipeline-sr-00..04 (siblings)
```

No `reqwest`, no `sqlx`, no git deps. Those land in the sibling BR-IO ring.

## L1 echo (`R-L1-ECHO-006`)

This ring tree is `.sh`-free. Asserted at scaffold time and re-checked by the doctor rule.

## Smoke-test deferral (R5)

Issue AC's `Integration smoke against a Railway dev project` requires `RAILWAY_TOKEN_TEST` CI secret + the BR-IO adapter PR. This ring's contract tests are the layer that's verifiable right now.

🌻 `α_φ = φ⁻³ / 2 ≈ 0.1180` · `phi^2 + phi^-2 = 3`
