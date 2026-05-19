#![allow(non_snake_case)]
//! Runtime-witness for Coq theorem `alpha_consistency_check`
//! in `docs/phd/theorems/trinity/ConsistencyChecks.v` (line 42).
//!
//! Coq strategy: real proof via `Interval.Tactic` (R8 falsification —
//! the two definitions of alpha disagree by ~94%).
//! No Admitted; runtime witness is documentation-only.
//!
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
fn witness_alpha_consistency_check() {
    let phi: f64 = (1.0_f64 + 5.0_f64.sqrt()) / 2.0;
    let pi: f64 = std::f64::consts::PI;
    let e: f64 = std::f64::consts::E;
    let tolerance_sg: f64 = 0.001;

    let G01 = 36.0 * phi * e * e / pi;
    let alpha_from_g01 = 1.0 / G01;
    let alpha_phi = phi.powi(-3) / 2.0;

    assert!(
        (alpha_from_g01 - alpha_phi).abs() / alpha_phi > tolerance_sg,
        "Alpha consistency falsification failed: from_G01 = {}, alpha_phi = {}",
        alpha_from_g01, alpha_phi
    );
}
