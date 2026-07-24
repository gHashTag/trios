# Skill: Neutrino Numerical Prediction in Coq

## Description
Add a validated numerical prediction for neutrino masses in Trinity's Coq framework, using the Type-II seesaw mechanism with generation-dependent splitting.

## When to Use
- The neutrino mass gap needs to be closed with an absolute numerical prediction
- A competitor has published a neutrino mass prediction that needs a response
- The Coq proof base needs a cosmology-competitive numerical theorem

## Prerequisites
- `NeutrinoMasses.v` exists with Type-II seesaw framework
- `lra` tactic works with floating-point bounds
- `SpectralAction600Cell.v` and `CorePhi.v` are compiled and available

## Steps

1. **Verify existing framework**
   ```bash
   cd proofs/trinity && coqc -R . Trinity NeutrinoMasses.v
   ```
   Ensure the file compiles with all existing lemmas.

2. **Identify the numerical pathway**
   - Type-I seesaw: m_ν ≈ m_D² / M_R — often gives wrong scale
   - Type-II seesaw: m_ν ≈ f_II · v_EW² / M_Delta — more flexible
   - Generation-dependent splitting: multiply by (3 · g_i / g_sum_φ)

3. **Compute expected values in Python first**
   ```python
   f_II = 0.01
   v_EW = 246.0
   M_Delta = 1e14
   g_e, g_mu, g_tau = 1.0, phi, phi**2
   g_sum = g_e + g_mu + g_tau
   base = f_II * v_EW**2 / M_Delta * 1e9  # eV
   masses = [base * (3*g/g_sum) for g in [g_e, g_mu, g_tau]]
   sum_m = sum(masses)
   print(f"Sum = {sum_m:.6f} eV")
   ```

4. **Add the Coq lemma**
   Use `rewrite Sum_m_nu_typeII_split_equal` to reduce to generation-independent form, then `unfold` + `lra`.

5. **Verify compilation**
   ```bash
   coqc -R . Trinity NeutrinoMasses.v
   ```

6. **Update competitive positioning**
   Add the numerical prediction to `docs/COMPETITIVE_POSITIONING.md` with comparison to competitors.

## Example

```coq
Theorem Sum_m_nu_typeII_split_exact_estimate :
  Sum_m_nu_typeII_split > 0.018 /\ Sum_m_nu_typeII_split < 0.019.
Proof.
  rewrite Sum_m_nu_typeII_split_equal.
  unfold Sum_m_nu_typeII.
  unfold m_nu_electron_typeII_eV, m_nu_muon_typeII_eV, m_nu_tau_typeII_eV.
  unfold m_nu_electron_typeII, m_nu_muon_typeII, m_nu_tau_typeII.
  unfold f_II, v_EW, M_Delta.
  split; lra.
Qed.
```

## Common Pitfalls
- `lra` may fail if bounds are too tight — try widening by ±0.001
- Always verify Python computation before stating Coq bounds
- Use `rewrite Sum_m_nu_typeII_split_equal` to avoid complex split definitions
- If `lra` fails on split form, compute through the equal form

## Related Skills
- [[trinity-coq-workflow]] — General Coq proof workflow
- [[trinity-competitor-tracking]] — Updating competitive positioning
- [[honest-withdrawal-protocol]] — Handling incorrect predictions
