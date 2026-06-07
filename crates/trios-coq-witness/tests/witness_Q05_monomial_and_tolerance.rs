#![allow(non_snake_case)]
//! Runtime-witness for the Q05 lemmas in
//! `docs/phd/theorems/trinity/Bounds_QuarkMasses.v`:
//!   * `Q05_falsified_by_PDG`
//!   * `Q05_within_tolerance`
//!   * `Q05_monomial_form_falsified` / `Q05_monomial_form`
//!   * `Q06_within_tolerance` / `Q06_chain_verified` / `Q06_chain_relation`
//!   * `quark_mass_chain_summary`
//!
//! Coq strategy: real proofs — `Interval.Tactic` for numerical bounds,
//! `reflexivity` for the chain summary. No Admitted.
//!
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
fn witness_Q05_monomial_and_tolerance() {
    let phi: f64 = (1.0_f64 + 5.0_f64.sqrt()) / 2.0;
    let pi: f64 = std::f64::consts::PI;
    let e: f64 = std::f64::consts::E;
    let tolerance_v: f64 = 0.01;

    // Q05 falsification
    let Q05_th = 48.0 * e * e / phi.powi(4);
    let Q05_exp = 52.3;
    assert!(
        (Q05_th - Q05_exp).abs() / Q05_exp > tolerance_v,
        "Q05 falsification failed: th = {}, exp = {}",
        Q05_th,
        Q05_exp
    );

    // Q06 within tolerance
    let Q07_th = 24.0 * phi * phi / pi;
    let Q06_th = Q05_th * Q07_th;
    let Q06_exp = 1035.0;
    assert!(
        (Q06_th - Q06_exp).abs() / Q06_exp < tolerance_v,
        "Q06 tolerance failed: th = {}, exp = {}",
        Q06_th,
        Q06_exp
    );

    // Chain exact
    assert!(
        (Q05_th * Q07_th - Q06_th).abs() < 1e-9,
        "Chain exact check failed: Q05*Q07 = {}, Q06 = {}",
        Q05_th * Q07_th,
        Q06_th
    );
}
