# SR-ALG-03 — e2e-ttt (spec + verifier ring) ⭐ WIN lane

**Soul-name:** `Bit Per Byte Hunter` · **Codename:** `LEAD` · **Tier:** 🥈 Silver · **Kingdom:** Cross-kingdom

> Closes #457 (spec + verifier portion) · Part of #446
> Anchor: `φ² + φ⁻² = 3` · `α_φ = φ⁻³ / 2` (Coq PROVEN)
> WIN target: 3-seed mean `val_bpb < 1.07063`

## Honest scope (R5)

This ring ships the **Rust spec + verifier**. It does NOT ship the GPU run. The 3-seed sweep on F₁₇/F₁₈/F₁₉ that produces `val_bpb < 1.07063` is a *follow-up* PR through SR-02 `TrainerRunner` + a BR-OUTPUT GPU harness. Everything that makes that follow-up painless is here:

- pinned `ENTRY_PATH` (lives OUTSIDE `crates/`, R-L6-PURE-007)
- `TARGET_VAL_BPB = 1.07063` constant
- `FIBONACCI_SEEDS = [1597, 2584, 4181]`
- `COQ_THEOREM_ID = "alpha_phi_phi_cubed"` for `bpb_samples` rows
- `EMBARGO_DAYS = 14`
- `E2eTttEnv { inner_steps, lr_inner, chunk_len }` matching paper defaults
- `Verifier::preflight()` — hash + path + env-sanity checks, called by SR-02 before spawn
- `evaluate_gate([f64; 3]) -> GateVerdict::{Win, HonestNotYet}` — the strict-inequality gate

## Wraps

[arXiv:2512.23675 — *End-to-End Test-Time Training for Long Context*](https://arxiv.org/abs/2512.23675).

The Python entry script `parameter-golf/records/track_10min_16mb/e2e-ttt/train_gpt.py` is *referenced* — never executed by this ring, never edited, never copied into `crates/`. SR-02 `TrainerRunner` calls `Verifier::preflight` against the actual file bytes before spawning.

## API

```rust
pub const TARGET_VAL_BPB: f64 = 1.07063;
pub const FIBONACCI_SEEDS: [i64; 3] = [1597, 2584, 4181];
pub const ENTRY_PATH: &str = "parameter-golf/records/track_10min_16mb/e2e-ttt/train_gpt.py";
pub const EMBARGO_DAYS: u32 = 14;
pub const COQ_THEOREM_ID: &str = "alpha_phi_phi_cubed";

pub struct E2eTttEnv { inner_steps, lr_inner, chunk_len }
impl E2eTttEnv {
    pub fn default() -> Self;                         // paper defaults
    pub fn baseline_equivalence() -> Self;             // inner_steps=0
    pub fn to_env_vars(&self) -> Vec<(EnvVar, EnvValue)>;
}

pub fn build_spec(entry_hash: EntryHash, env: &E2eTttEnv) -> AlgorithmSpec;
pub fn sha256_bytes(bytes: &[u8]) -> [u8; 32];

pub struct Verifier;
impl Verifier {
    pub fn preflight(spec: &AlgorithmSpec, env: &E2eTttEnv, actual: &[u8]) -> VerifierOutcome;
}

pub enum VerifierOutcome { Ok, HashMismatch{...}, PathUnderCrates{...}, InvalidEnv{...} }

pub fn evaluate_gate(seed_bpbs: [f64; 3]) -> GateVerdict;
pub enum GateVerdict { Win{mean_val_bpb, margin}, HonestNotYet{mean_val_bpb, gap} }
```

## Tests (19/19 GREEN)

| Group | Tests |
|---|---|
| Constants | `target_val_bpb_matches_issue`, `fibonacci_seeds_match_spec` (Fibonacci identity), `entry_path_outside_crates` (R-L6-PURE-007), `embargo_window_is_14_days` |
| Env | `env_default_matches_paper`, `env_to_env_vars_preserves_order`, `env_baseline_equivalence_zero_inner_steps` |
| Verifier | `verifier_ok_path`, `verifier_rejects_hash_mismatch`, `verifier_rejects_path_under_crates`, `verifier_rejects_inner_steps_too_high`, `verifier_rejects_non_power_of_two_chunk`, `verifier_rejects_zero_chunk_len`, `verifier_rejects_non_finite_lr` |
| Spec | `build_spec_attaches_theorem_id`, `build_spec_entry_path_is_pinned_constant` |
| Gate | `gate_win_below_target`, `gate_honest_not_yet_at_or_above_target`, `gate_honest_not_yet_exact_target` (strict inequality), `gate_verdict_serialises_tagged_kind` |

## What this ring does NOT do (follow-up PR)

| Out of scope here | Lives in |
|---|---|
| GPU spawn of `train_gpt.py` | SR-02 `TrainerRunner` (#454) — not yet shipped |
| 3-seed sweep on F₁₇/F₁₈/F₁₉ | BR-OUTPUT GPU harness — separate PR |
| Writing `bpb_samples` rows with `algorithm = "e2e-ttt"`, `entry_hash`, `coq_theorem` | SR-03 `BpbWriter` already in main; the BR-OUTPUT harness uses it |
| Setting `bpb_samples.embargo_until` | BR-OUTPUT writer policy |
| `make verify`-style smoke (CPU, 1 step, < 30 s) | Out-of-tree harness — not a Rust unit test |

## Dependencies

- `trios-algorithm-arena-sr-alg-00` (path) — `AlgorithmSpec`, `EntryHash`, env types
- `serde`, `sha2`, `hex`
- `serde_json` (dev only)

R-RING-DEP-002 — no `tokio`, no `sqlx`, no `reqwest`.

## Build & test

```bash
cargo build  -p trios-algorithm-arena-sr-alg-03
cargo clippy -p trios-algorithm-arena-sr-alg-03 --all-targets -- -D warnings
cargo test   -p trios-algorithm-arena-sr-alg-03
```

## Laws

- L1 ✓ no `.sh`
- L3 ✓ clippy clean
- L4 ✓ tests before merge
- L6 ✓ no I/O, no async
- L11 ✓ soul-name `Bit Per Byte Hunter` claimed before any future `train_gpt.py` invocation
- L13 ✓ I-SCOPE: this ring only
- L14 ✓ `Agent: Bit-Per-Byte-Hunter` trailer
- R-RING-DEP-002 ✓ strict dep list above
- R-L6-PURE-007 ✓ no `.py` in this crate; `ENTRY_PATH` is a *reference* outside `crates/`
