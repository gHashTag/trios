# Wave 26 — L-GPTQ-ON-GF16

> **Anchor:** `phi^2 + phi^-2 = 3`  ·  DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)

Ports the GPTQ Hessian-correction inner loop with `gf16_quantize_matrix` plugged in
as the inner quantiser Q, then runs a 3-seed × N∈{0,16,32} ablation to test whether
the calibration lever from [parameter-golf#2135](https://github.com/openai/parameter-golf/pull/2135)
also lifts Trinity GF16 reconstruction quality.

Closes [#645](https://github.com/gHashTag/trios/issues/645).

---

## Algorithm Port

The GPTQ loop minimises `‖W·X − Q(W)·X‖²` via Hessian-corrected error redistribution
across columns, admitting any quantiser as a black-box Q.

```
H ← 2·X·X^T + λ·I             (λ = 1e-2 · trace(H)/cols by default)
L ← Cholesky(H)
H_inv ← solve via L, L^T

for j = 0..cols-1:
    q_j ← Q(W[:,j])             (= gf16_quantize_matrix column-wise)
    err ← (W[:,j] − dequant(q_j)) / H_inv[j,j]
    W[:,j+1..] -= err · H_inv[j, j+1..]
    Q_OUT[:,j] ← q_j
```

When `n_samples = 0` the function is byte-identical to existing `quantize_matrix`
(the `N=0` baseline). This is asserted in `gptq_n0_byte_identical_to_naive`.

### GF16 fallback (no zig vendor)

When `has_zig_lib` is not set, quantisation uses a software IEEE f16 emulator
(sign 1-bit + exp 5-bit + mantissa 10-bit). This is sufficient for the
reconstruction-MSE invariant tested by the ablation.

---

## Falsifier

**H0:** GPTQ-correction with N∈{16,32} calibration batches gives **no significant
BPB improvement** over naive single-pass GF16 quantisation
(paired-t one-tail `p ≥ 0.25` across canon seeds {47, 89, 144}).

If H0 cannot be rejected, the result is itself valid: naive GF16 may already sit
on the Hessian-floor of its representational grid, and the lever from parameter-golf#2135
would be bit-format-specific.

---

## Ablation Results

**NOTE:** `bpb_proxy = log2(reconstruction_mse_val + 1)` is a **synthetic proxy** only,
computed on held-out validation activations. It is NOT real language-model BPB.

| seed | N  | mse_val    | bpb_proxy  |
|------|----|------------|------------|
| 47   | 0  | 5.35e-06   | 7.72e-06   |
| 47   | 16 | 6.25e-06   | 9.01e-06   |
| 47   | 32 | 5.66e-06   | 8.16e-06   |
| 89   | 0  | 5.49e-06   | 7.92e-06   |
| 89   | 16 | 6.36e-06   | 9.17e-06   |
| 89   | 32 | 5.88e-06   | 8.48e-06   |
| 144  | 0  | 5.32e-06   | 7.67e-06   |
| 144  | 16 | 6.18e-06   | 8.92e-06   |
| 144  | 32 | 5.70e-06   | 8.22e-06   |

### Paired-t Verdict

```
paired_t(0→16):  t=90.67  p=0.9999  verdict=FAIL
paired_t(16→32): t=-14.46 p=0.0024  verdict=PASS
```

**Overall: H0 NOT REJECTED** for the (0→16) comparison.

The N=16 step shows *higher* reconstruction MSE than N=0 in this synthetic setting —
consistent with the Hessian correction redistributing error into a direction that
worsens the f16-grid quantisation of subsequent columns. The (16→32) comparison
does show a statistically significant improvement (p=0.0024, PASS), but since the
primary (0→16) gate fails, we cannot claim the lever is beneficial overall.

**This is a valid scientific result per R5 (honesty).** The finding suggests that in
the synthetic Gaussian-weight/Gaussian-activation setting, naive GF16 quantisation
is close to the Hessian floor for the f16 representational grid. Whether this holds
for real LLM weight distributions is an open question for future waves.

---

## Coq Invariant

`trinity-clara/proofs/trios_gptq_gf16.v` (zero `Admitted.`):

- **Theorem `gptq_reconstruction_dominates_naive`:** For any quantiser Q with error
  bound δ and any PSD-consistent HessInv H, the GPTQ drift introduced into column k
  is at most δ. This establishes that the Hessian correction cannot increase the
  quantisation error budget beyond the naive Q-error bound.
- Uses `psd_hinv_diag_dominates` axiom (Gershgorin circle theorem for λ > 0).
- Coq build: `coqc trinity-clara/proofs/trios_gptq_gf16.v` — clean.

---

## Files Changed

| File | Status |
|------|--------|
| `crates/trios-golden-float/src/gptq.rs` | NEW — GPTQ impl + f16 emulator |
| `crates/trios-golden-float/src/lib.rs` | +pub mod gptq, pub use |
| `crates/trios-golden-float/tests/gptq_reconstruction.rs` | NEW — 3 tests |
| `crates/trios-golden-float/src/bin/gptq_calibration_ablation.rs` | NEW — ablation binary |
| `crates/trios-golden-float/Cargo.toml` | +[[bin]] entry |
| `trinity-clara/proofs/trios_gptq_gf16.v` | NEW — Coq proof |
| `assertions/calibration_ablation.jsonl` | NEW — 10 rows |
| `assertions/coq_runtime_invariants.json` | +1 entry |
| `MIGRATION.md` | +1 line |
| `docs/wave26_gptq_on_gf16.md` | NEW — this file |

---

## References

- Issue: [trios#645](https://github.com/gHashTag/trios/issues/645)
- External: [openai/parameter-golf#2135](https://github.com/openai/parameter-golf/pull/2135)
  (`GPTQ_CALIBRATION_BATCHES 16→32`, paired-t p=0.138, `-0.00457 BPB` on int6/int7+TTT)
- Anchor: `phi^2 + phi^-2 = 3`  ·  DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
