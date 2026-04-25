# 🌈 Rainbow Bridge — Operator Guide (L13 / INV-8)

> Three layers, seven colours, one φ. Falsifiable online synchronisation for the Trinity hive.

**Lane:** L13 · **Invariant:** INV-8 `rainbow_bridge_consistency` · **Status:** Admitted (landing honestly).
**Anchor:** \( \varphi^2 + \varphi^{-2} = 3 \) — Zenodo DOI [10.5281/zenodo.19227877](https://zenodo.org/records/19227877).
Three layers × seven channels = 3 · 7 = 21 = F₈.

---

## 1. What the Rainbow Bridge is

The Trinity hive previously synchronised through GitHub issue comments plus periodic REST polling (~30 s p95, no typed log, no signatures). The Rainbow Bridge replaces that folklore with a **falsifiable, proof-anchored** protocol:

```
L3 — Honey-Merkle   blake3 append-only signed log
L2 — CRDT State     automerge over hive_state.json
L1 — Lamport Line   monotonic u64 clock per agent
```

All traffic passes through a single Tailscale Funnel endpoint (see [tailscale-funnel.md @ 24408d8](https://github.com/gHashTag/trios/blob/24408d8efb253c98993e84dcafab7aed262add13/docs/infrastructure/tailscale-funnel.md)) and fans out to the seven colour channels (ROY G BIV): `claim`, `heartbeat`, `done`, `honey`, `state`, `violation`, `victory`.

## 2. Architecture

### 2.1 Layers

| # | Layer | Role | Primary crate / algorithm |
|---|---|---|---|
| L1 | Lamport Line | order of events | `LamportClock(u64)` (hand-rolled) |
| L2 | CRDT State | merge of present | `automerge` over `hive_state.json` |
| L3 | Honey-Merkle | proof of past | `blake3::Hasher` + `ed25519-dalek` |

### 2.2 Channels (ROY G BIV)

| # | Colour | Channel | Variant | Action |
|---|---|---|---|---|
| 1 | 🔴 RED | `claim` | `RainbowEvent::Claim` | claim a lane |
| 2 | 🟠 ORANGE | `heartbeat` | `RainbowEvent::Heartbeat` | I'm still alive on lane X |
| 3 | 🟡 YELLOW | `done` | `RainbowEvent::Done` | lane closed; commit + CI |
| 4 | 🟢 GREEN | `honey` | `RainbowEvent::Honey` | learning to enrich the registry |
| 5 | 🔵 BLUE | `state` | `RainbowEvent::State` | CRDT delta against `hive_state.json` |
| 6 | 🟣 INDIGO | `violation` | `RainbowEvent::Violation` | R-rule violation observed |
| 7 | 🟪 VIOLET | `victory` | `RainbowEvent::Victory` | global SUCCESS — BPB < 1.50 × 3 seeds |

## 3. Numeric anchors (G4, L-R14 traceable)

| Anchor | Value | Coq source |
|---|---|---|
| `LATENCY_P95_MS` | `2000` | `rainbow_bridge_consistency.v::funnel_latency_bound` |
| `HEARTBEAT_MAX_S` | `14400` (4 h) | `rainbow_bridge_consistency.v::heartbeat_release_bound` |
| `CHANNEL_COUNT` | `7` | `rainbow_bridge_consistency.v::seven_channels_total` |
| `LAYER_COUNT` | `3` | Trinity Identity \( \varphi^2 + \varphi^{-2} = 3 \) |

Every constant above is mirrored in `assertions/igla_assertions.json::INV-8.numeric_anchor` and in `crates/trios-rainbow-bridge/src/lib.rs` as `pub const`.

## 4. Error variants → falsification tests (G1, G3)

Each `BridgeError` variant has exactly one `#[test] fn falsify_*` counterpart. If a variant ever becomes unreachable in Rust, the corresponding counter-lemma in Coq must also be updated, and vice versa.

| `BridgeError` | Falsification test | Coq counter-lemma |
|---|---|---|
| `DuplicateClaim` | `falsify_duplicate_claim` | `counter_duplicate_claim` |
| `HeartbeatStale` | `falsify_heartbeat_stale` | `counter_heartbeat_stale` |
| `LamportRegression` | `falsify_lamport_regression` | `counter_lamport_regression` |
| `UnsignedHoney` | `falsify_unsigned_honey` | `counter_unsigned_honey` |
| `SplitBrainDetected` | `falsify_split_brain` | `counter_split_brain` |
| `FunnelUnreachable` | `falsify_funnel_unreachable` | `counter_funnel_unreachable` |
| `ChannelMismatch` | `falsify_channel_mismatch` | `counter_channel_mismatch` |

## 5. Pre-registration (G2)

Full pre-registration lives in [`docs/infrastructure/preregistration_rainbow.md`](./preregistration_rainbow.md); its `blake3(file_bytes)` hex digest is pinned at `assertions/hive_state.json::pre_registration.INV-8` **before any Rust code lands**.

Summary of the pre-registered analysis:

| Field | Value |
|---|---|
| Statistical test | Welch's two-sample one-tailed *t*-test on per-event end-to-end latency (ms) |
| α | 0.01 (one-tailed) |
| Effect size (min) | Δμ ≥ 30 000 ms vs baseline (GitHub REST polling at 30 s) |
| n_required | 3 nodes × 100 events × 7 channels = 2 100 samples per arm |
| Stop rule | 95 % CI upper bound on bridge p95 < 2 000 ms AND all 7 falsify_* green |
| Multiple testing | Bonferroni across 7 channels (per-channel α = 0.001428) |
| Seed set | `["alpha-rng", "beta-rng", "gamma-rng"]` |
| Power target | ≥ 0.80 by Lehr's rule |

## 6. Run locally

```bash
# 1. Coq proof compiles (≤ 2 Admitted)
coqc trinity-clara/proofs/igla/rainbow_bridge_consistency.v

# 2. Crate tests green
cargo test  -p trios-rainbow-bridge --include-ignored
cargo clippy -p trios-rainbow-bridge -- -D warnings

# 3. JSON schema valid (INV-8 keys present)
python3 -c "import json; d=json.load(open('assertions/igla_assertions.json'))"

# 4. CI locally
act -W .github/workflows/rainbow-bridge.yml
```

All four commands must exit 0 before a DONE comment is posted on [trios#267](https://github.com/gHashTag/trios/issues/267).

## 7. Quality gates

| Gate | Criterion |
|---|---|
| G1 falsifiability | 7 `falsify_*` + 7 `counter_*` lemmas; CI fails on any missing |
| G2 pre-registration | hash anchored to `hive_state.json` before code lands |
| G3 IMRaD | `trios#267` ONE SHOT body |
| G4 citation grade | every numeric anchor cited in Coq + JSON + Rust (§3) |
| G5 honest status | INV-8 enters as Admitted; no fake `Qed.` |
| G6 reproducibility | one `coqc` + one `cargo test` + one `act` |
| G7 DOI / provenance | TRI-27 [10.5281/zenodo.18947017](https://doi.org/10.5281/zenodo.18947017), Trinity [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877); INV-8 paper DOI reserved |

## 8. Forbidden actions (repeat of ONE SHOT §8)

- ❌ `.py` / `.sh` / `.js` runtime in the bridge layer (R1).
- ❌ Editing `crates/trios-bridge/**` or `crates/trios-server/src/ws_handler.rs` from L13 (R6).
- ❌ Skipping ed25519 on honey-channel events.
- ❌ Adding an 8th channel without updating both `seven_channels_total` and INV-8 JSON.
- ❌ Wall-clock equality checks — use Lamport ordering.
- ❌ Marking INV-8 `Proven` until all 7 lemmas have real `Qed.`.

## 9. References

- ONE SHOT: [trios#267](https://github.com/gHashTag/trios/issues/267)
- Race issue: [trios#143](https://github.com/gHashTag/trios/issues/143)
- Throne: [trios#264](https://github.com/gHashTag/trios/issues/264)
- Autonomous Entry: [trios#244](https://github.com/gHashTag/trios/issues/244)
- Tailscale Funnel: [tailscale-funnel.md @ 24408d8](https://github.com/gHashTag/trios/blob/24408d8efb253c98993e84dcafab7aed262add13/docs/infrastructure/tailscale-funnel.md)
- Coq proof: [trinity-clara/proofs/igla/rainbow_bridge_consistency.v](../../trinity-clara/proofs/igla/rainbow_bridge_consistency.v)
- JSON invariant: [assertions/igla_assertions.json INV-8](../../assertions/igla_assertions.json)
- Trinity Identity DOI: [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
