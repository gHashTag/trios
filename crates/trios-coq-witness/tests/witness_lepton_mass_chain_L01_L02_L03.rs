#![allow(non_snake_case)]
//! Runtime-witness for Coq lemma `lepton_mass_chain_L01_L02_L03`
//! in `docs/phd/theorems/trinity/ConsistencyChecks.v` (line 109).
//!
//! Exact algebraic identity: L01 * L02 = L03, i.e.
//! (4 phi^3 / e^2) * (2 phi^4 pi / e) = 8 phi^7 pi / e^3.
//!
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
fn witness_lepton_mass_chain_L01_L02_L03() {
    let phi: f64 = (1.0_f64 + 5.0_f64.sqrt()) / 2.0;
    let e: f64 = std::f64::consts::E;

    let L01 = 4.0 * phi.powi(3) / (e * e);
    let L02 = 2.0 * phi.powi(4) * std::f64::consts::PI / e;
    let L03 = 8.0 * phi.powi(7) * std::f64::consts::PI / (e * e * e);

    let rel = (L01 * L02 - L03).abs() / L03;
    assert!(rel < 1e-12, "|L01*L02 - L03|/L03 = {} >= 1e-12", rel);
}
