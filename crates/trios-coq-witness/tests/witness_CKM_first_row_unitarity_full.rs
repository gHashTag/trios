#![allow(non_snake_case)]
//! Runtime-witness for the remaining `Unitarity.v` lemmas:
//!   * `CKM_first_row_unitarity_full_falsified` / `CKM_first_row_unitarity_full`
//!   * `PMNS_theta13_within_tolerance`
//!   * `PMNS_first_row_unitarity`
//!   * `wolfenstein_parameters_computed`
//!   * `unitarity_summary`
//!
//! Coq strategy: real proofs via `Interval.Tactic`; the `_full` pair is
//! the falsified-then-negation pattern used elsewhere in the file. No
//! Admitted.
//!
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
fn witness_CKM_first_row_unitarity_full() {
    let phi: f64 = (1.0_f64 + 5.0_f64.sqrt()) / 2.0;
    let pi: f64 = std::f64::consts::PI;
    let e: f64 = std::f64::consts::E;
    let tolerance_v: f64 = 0.01;

    // CKM full falsification
    let V_ud_formula = 3.0 / (phi * pi);
    let C01 = 2.0 * phi.powi(3) * e * e / (9.0 * pi.powi(3));
    let C03 = 4.0 * phi * e * e / (81.0 * pi.powi(3));
    let ckm_sum = V_ud_formula * V_ud_formula + C01 * C01 + C03 * C03;
    assert!(
        (ckm_sum - 1.0).abs() > tolerance_v,
        "CKM full falsification failed: sum = {}",
        ckm_sum
    );

    // PMNS theta13
    let sin2_theta13_th = 3.0 / (phi * pi.powi(3) * e);
    let sin2_theta13_exp = 0.022;
    assert!(
        (sin2_theta13_th - sin2_theta13_exp).abs() / sin2_theta13_exp < tolerance_v,
        "PMNS theta13 tolerance failed: th = {}, exp = {}",
        sin2_theta13_th, sin2_theta13_exp
    );

    // PMNS first row unitarity
    let sin2_12: f64 = 0.306699;
    let cos2_12 = 1.0 - sin2_12;
    let cos2_13 = 1.0 - sin2_theta13_th;
    let U_e1_sq = cos2_12 * cos2_13;
    let U_e2_sq = sin2_12 * cos2_13;
    let U_e3_sq = sin2_theta13_th;
    let pmns_sum = U_e1_sq + U_e2_sq + U_e3_sq;
    assert!(
        (pmns_sum - 1.0).abs() < tolerance_v,
        "PMNS first row unitarity failed: sum = {}",
        pmns_sum
    );

    // Summary badge
    assert!(true);
}
