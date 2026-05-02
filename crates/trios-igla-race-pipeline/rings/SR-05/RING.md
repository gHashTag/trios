# RING — SR-05

| Field | Value |
|---|---|
| Tier | 🥈 Silver |
| Crate | `trios-igla-race-pipeline-sr-05` |
| Path | `crates/trios-igla-race-pipeline/rings/SR-05/` |
| Deps in | `trios-igla-race-pipeline-sr-00..04`, `serde`, `serde_json`, `chrono`, `thiserror`, `tracing` |
| Deps out (path) | none yet — wired into BR-OUTPUT once BR-IO adapter exists |
| I/O | none (Silver-tier; concrete `reqwest` GraphQL client + `trios-railway-audit` git dep live in sibling BR-IO ring) |
| Public verbs | `RailwayDeployer::deploy`, `audit`, `fleet_snapshot` |
| Public traits | `RailwayApi` |
| Public types | `DeployerMode`, `FleetSnapshot`, `RailwayService`, `DeployReceipt`, `DeployAction`, `AuditReport`, `R7Triplet`, `DeployErr` |

## Ring contract

- **In:** desired `FleetSnapshot` from BR-OUTPUT (post-gardener decisions).
- **Process:** snapshot live fleet (`fleet_snapshot()`), diff against desired, emit a `DeployReceipt` of `DeployAction`s. In `Observe` mode each action is logged but the underlying RailwayApi is called read-only; in `Apply` mode mutations flow through the trait.
- **Out:** `DeployReceipt::actions` + `DeployReceipt::triplets` (one R7 audit triplet per action).
- **R7 triplet:** `RAIL=<verb> @ project=<8c> service=<8c> sha=<8c> ts=<rfc3339>` — formatter ships in this ring.

## Sibling BR-IO ring

The concrete `reqwest` GraphQL client (Railway API) and the optional `sqlx` audit-log writer live in
`crates/trios-igla-race-pipeline/rings/BR-IO-SR-05/` (future PR). They implement `RailwayApi` against the live data plane.

🌻 `phi^2 + phi^-2 = 3`
