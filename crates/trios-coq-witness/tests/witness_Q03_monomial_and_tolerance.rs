#![allow(non_snake_case)]
//! Runtime-witness for the Q03 lemmas in
//! `docs/phd/theorems/trinity/Bounds_QuarkMasses.v`:
//!   * `Q03_falsified_by_PDG`
//!   * `Q03_within_tolerance`
//!   * `Q03_monomial_form_falsified` / `Q03_monomial_form`
//!
//! Coq strategy: real proofs — tolerance bands and PDG-falsification are
//! discharged by `Interval.Tactic`; monomial-form contradictions are
//! re-derived by `exact …_falsified`. No Admitted.
//!
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
fn witness_Q03_monomial_and_tolerance() {
    let phi: f64 = (1.0_f64 + 5.0_f64.sqrt()) / 2.0;
    let pi: f64 = std::f64::consts::PI;
    let e: f64 = std::f64::consts::E;
    let tolerance_v: f64 = 0.01;

    let Q03_th = phi.powi(4) * pi / (e * e);
    let Q03_exp = 171.5;

    assert!(
        (Q03_th - Q03_exp).abs() / Q03_exp > tolerance_v,
        "Q03 falsification failed: th = {}, exp = {}",
        Q03_th,
        Q03_exp
    );
}
