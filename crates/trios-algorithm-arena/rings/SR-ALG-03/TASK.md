# TASK — SR-ALG-03 (trios-algorithm-arena)

## Status: ⚠ PARTIAL — spec + verifier DONE ✅; 3-seed GPU sweep is follow-up

Closes #457 (spec + verifier ring) · Part of #446

## Completed (Rust ring)

- [x] Ring at `rings/SR-ALG-03/` with I5 (`README.md`, `TASK.md`, `AGENTS.md`, `RING.md`, `Cargo.toml`, `src/lib.rs`)
- [x] `TARGET_VAL_BPB = 1.07063` constant
- [x] `FIBONACCI_SEEDS = [F_17=1597, F_18=2584, F_19=4181]`
- [x] `ENTRY_PATH = "parameter-golf/records/track_10min_16mb/e2e-ttt/train_gpt.py"` — OUTSIDE `crates/` (R-L6-PURE-007)
- [x] `EMBARGO_DAYS = 14`, `COQ_THEOREM_ID = "alpha_phi_phi_cubed"`
- [x] `E2eTttEnv { inner_steps=4, lr_inner=1e-3, chunk_len=512 }` matching arXiv:2512.23675 paper defaults
- [x] `E2eTttEnv::baseline_equivalence()` (inner_steps=0) — pattern from parameter-golf#2059 `baseline_equivalence.py`
- [x] `sha256_bytes()` helper
- [x] `build_spec(entry_hash, env) -> AlgorithmSpec`
- [x] `Verifier::preflight` — rejects hash mismatch, path-under-crates, inner_steps > 16, non-power-of-two chunk, zero chunk, non-finite LR
- [x] `VerifierOutcome` tagged enum (Ok / HashMismatch / PathUnderCrates / InvalidEnv)
- [x] `evaluate_gate([f64; 3]) -> GateVerdict::{Win, HonestNotYet}` — strict inequality
- [x] 19 unit tests
- [x] `cargo clippy --all-targets -- -D warnings` clean
- [x] `Agent: Bit-Per-Byte-Hunter` trailer (L14)

## Open — follow-up PR required (GPU harness)

- [ ] SR-02 `TrainerRunner` (#454) ships — blocked on `tch` / `trios-trainer-igla` git dep integration
- [ ] BR-OUTPUT GPU harness subscribes to `strategy_queue`, claims a scarab, calls `Verifier::preflight` on the actual bytes of `parameter-golf/records/track_10min_16mb/e2e-ttt/train_gpt.py`, spawns the Python
- [ ] 3-seed sweep on `F_17, F_18, F_19`
- [ ] Each seed writes to Neon `bpb_samples` via SR-03 `BpbWriter` with `algorithm = "e2e-ttt"`, `entry_hash = <pinned>`, `coq_theorem = "alpha_phi_phi_cubed"`
- [ ] `bpb_samples.embargo_until = now() + 14 days`
- [ ] Compute `mean_val_bpb`; run `evaluate_gate([a, b, c])`
- [ ] If `Win` — comment on #457 with the exact three numbers + margin
- [ ] `make verify` smoke test (CPU, 1 step, < 30 s)

## Honest disclosure (R5)

Issue #457 expects a `val_bpb < 1.07063` proof. This PR ships the **Rust ring** that pins every constant, validates every input, and emits the gate verdict. It does NOT produce the GPU number — that requires SR-02 to land first (SR-02 has its own blockers: `tch` feature-gated build, `trios-trainer-igla` git dep pinning, CPU/GPU matrix). Shipping the verifier + gate now unblocks the follow-up GPU PR from having to re-implement any of this scaffolding.

## Next ring

BR-OUTPUT GPU harness (new issue) — consumes SR-ALG-03 + SR-ALG-00 + SR-02 + SR-03 + SR-04 and closes #457 definitively.
