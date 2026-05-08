#![allow(non_snake_case)]
//! Runtime-witness placeholder for the V_ud lemmas in
//! `docs/phd/theorems/trinity/Unitarity.v`:
//!   * `CKM_first_row_unitarity`
//!   * `V_ud_formula_falsified_by_PDG`
//!   * `V_ud_within_tolerance`
//!
//! Coq strategy: real proofs via `Interval.Tactic` and algebraic
//! manipulation. No Admitted.
//!
//! TODO ticket: trios#587.
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
#[ignore = "TODO: numerical witness for V_ud / CKM-row-1 lemmas. Tracker: trios#587."]
fn witness_V_ud_PDG() {
    // TODO: assert sqrt(1 - V_us^2 - V_ub^2) ≈ 0.974 within tolerance_V.
}
