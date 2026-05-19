#![allow(non_snake_case)]
//! Runtime-witness for Coq theorem `gauge_mass_chain_check`
//! in `docs/phd/theorems/trinity/ConsistencyChecks.v` (line 146).
//!
//! Coq strategy: real proof via `Interval.Tactic` (R8 falsified — Higgs
//! to W to Z chain breaks by ~118%). No Admitted.
//!
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
fn witness_gauge_mass_chain_check() {
    let phi: f64 = (1.0_f64 + 5.0_f64.sqrt()) / 2.0;
    let e: f64 = std::f64::consts::E;
    let tolerance_v: f64 = 0.01;

    let H02 = 4.0 * phi * e;
    let H03 = phi * phi * e;

    assert!(
        (H02 * 0.881 - H03).abs() / H03 > tolerance_v,
        "Gauge mass chain falsification failed: H02*0.881 = {}, H03 = {}",
        H02 * 0.881, H03
    );
}
