#![allow(non_snake_case)]
//! Runtime-witness for Coq theorem `quark_mass_chain_Q07_Q01_Q02`
//! in `docs/phd/theorems/trinity/ConsistencyChecks.v` (line 67).
//!
//! Falsification check: Q07/Q01 does NOT equal Q02 — the chain
//! relation is broken by a large relative error.
//!
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
fn witness_quark_mass_chain_Q07_Q01_Q02() {
    let phi: f64 = (1.0_f64 + 5.0_f64.sqrt()) / 2.0;
    let pi: f64 = std::f64::consts::PI;
    let e: f64 = std::f64::consts::E;
    let tolerance_l: f64 = 0.05;

    let Q07 = 24.0 * phi * phi / pi;
    let Q01 = pi / (9.0 * e * e);
    let Q02 = 4.0 * phi * phi / pi;

    let rel = ((Q07 / Q01) - Q02).abs() / Q02;
    assert!(
        rel > tolerance_l,
        "|(Q07/Q01) - Q02|/Q02 = {} <= {}",
        rel, tolerance_l
    );
}
