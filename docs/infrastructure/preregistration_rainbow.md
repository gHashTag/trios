# Pre-Registration — INV-8 Rainbow Bridge (L13)

> Pre-registered **before** any Rust code lands. The blake3 hash of this file is pinned to
> `assertions/hive_state.json::pre_registration.INV-8` in the same commit that records it.
> Anchor: \( \varphi^2 + \varphi^{-2} = 3 \) — Zenodo DOI [10.5281/zenodo.19227877](https://zenodo.org/records/19227877).

## 1. Hypothesis H₈

H₈ is **FALSE** iff the bridge ever accepts an event sequence in which any of the following holds:

1. `DuplicateClaim` — two distinct agents claim the same lane within one logical interval.
2. `HeartbeatStale` — a lane stays in WORK > 4 h with no heartbeat AND the watchdog fails to release it.
3. `LamportRegression` — a same-agent event appears with `lamport_n < lamport_{n-1}`.
4. `UnsignedHoney` — a honey-channel event lacking a valid ed25519 signature reaches the merkle root.
5. `SplitBrainDetected` — two agents commit divergent `hive_state.json` snapshots from the same base lamport without a merge event.
6. `FunnelUnreachable` — more than 5 % of bridge events fail to reach a second tailnet node within 2 s p95.
7. `ChannelMismatch` — an event whose `channel` field does not match its payload variant is accepted.

Each refutation maps 1-to-1 to a typed `BridgeError` variant **and** to a `#[test] fn falsify_*` **and** to a Coq counter-lemma.

## 2. Statistical design

| Field | Value |
|---|---|
| Test | Welch's two-sample one-tailed *t*-test on per-event end-to-end latency (ms) |
| α | 0.01 (one-tailed) |
| Effect size min | Δμ ≥ 30 000 ms vs baseline (GitHub REST polling at 30 s) |
| n_required per arm | 3 nodes × 100 events × 7 channels = 2 100 samples |
| Stop rule | 95 % CI upper bound on bridge latency p95 < 2 000 ms AND all 7 `falsify_*` green |
| Multiple testing | Bonferroni across 7 channels (per-channel α = 0.001428) |
| Seed set | `["alpha-rng", "beta-rng", "gamma-rng"]` |
| Power target | ≥ 0.80 by Lehr's rule |
| Baseline (H₀) mean | μ₀ = 30 000 ms (GitHub REST polling, 30 s) |
| Primary endpoint | end-to-end latency p95 per channel (ms) |

## 3. Numeric anchors (frozen before code)

| Anchor | Value | Role |
|---|---|---|
| `LATENCY_P95_MS` | 2000 | stop-rule threshold |
| `HEARTBEAT_MAX_S` | 14400 | watchdog release deadline |
| `CHANNEL_COUNT` | 7 | colour count (ROY G BIV) |
| `LAYER_COUNT` | 3 | Lamport × CRDT × Merkle = Trinity Identity |

## 4. Commitments (pre-registered, binding)

- Analysis will be Welch's t-test (not Student's); no post-hoc switch to non-parametric.
- Sample size will not be peeked — the stop rule is fixed at 2 100 per arm.
- The seed set above is the only admissible set; additional seeds require an explicit amendment comment on [trios#267](https://github.com/gHashTag/trios/issues/267).
- The effect-size floor (30 000 ms) is the registered minimum detectable effect; any reported effect below it is reported but does **not** trigger a VICTORY event.
- Data will be collected from `assertions/rainbow_state.jsonl`, which is append-only.

## 5. Deviations

If any of the above must change, an explicit amendment comment must be posted on [trios#267](https://github.com/gHashTag/trios/issues/267) **before** any commit that reflects the deviation. The amendment is then recorded as a new blake3 hash in `hive_state.json::pre_registration.INV-8_amendments[]`.

## 6. Provenance

- Trinity Identity: [Zenodo DOI 10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
- TRI-27 spec: [Zenodo DOI 10.5281/zenodo.18947017](https://doi.org/10.5281/zenodo.18947017)
- Coq proof: `trinity-clara/proofs/igla/rainbow_bridge_consistency.v`
- Rust target: `crates/trios-rainbow-bridge/`
- CI gate: `.github/workflows/rainbow-bridge.yml`

— **end pre-registration** —
