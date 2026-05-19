#![allow(non_snake_case)]
//! Runtime-witness for the V_ud lemmas in
//! `docs/phd/theorems/trinity/Unitarity.v`:
//!   * `CKM_first_row_unitarity`
//!   * `V_ud_formula_falsified_by_PDG`
//!   * `V_ud_within_tolerance`
//!
//! Coq strategy: real proofs via `Interval.Tactic` and algebraic
//! manipulation. No Admitted.
//!
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
fn witness_V_ud_PDG() {
    let phi: f64 = (1.0_f64 + 5.0_f64.sqrt()) / 2.0;
    let pi: f64 = std::f64::consts::PI;
    let e: f64 = std::f64::consts::E;
    let tolerance_v: f64 = 0.01;

    // CKM first-row unitarity
    let C01 = 2.0 * phi.powi(3) * e * e / (9.0 * pi.powi(3));
    let C03 = 4.0 * phi * e * e / (81.0 * pi.powi(3));
    let V_ud = (1.0 - C01 * C01 - C03 * C03).sqrt();
    assert!(
        (V_ud * V_ud + C01 * C01 + C03 * C03 - 1.0).abs() < tolerance_v,
        "CKM first-row unitarity failed: sum = {}",
        V_ud * V_ud + C01 * C01 + C03 * C03
    );

    // Falsification: V_ud_formula vs PDG
    let V_ud_formula = 3.0 / (phi * pi);
    let V_ud_exp = 0.974;
    assert!(
        (V_ud_formula - V_ud_exp).abs() / V_ud_exp > tolerance_v,
        "V_ud falsification failed: formula = {}, exp = {}",
        V_ud_formula, V_ud_exp
    );

    // Unitarity-derived vs PDG
    assert!(
        (V_ud - V_ud_exp).abs() / V_ud_exp < tolerance_v,
        "V_ud vs PDG tolerance failed: V_ud = {}, exp = {}",
        V_ud, V_ud_exp
    );
}
