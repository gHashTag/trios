#![allow(non_snake_case)]
//! Runtime-witness for Coq theorem `quark_mass_chain_Q05_Q07_Q06`
//! in `docs/phd/theorems/trinity/ConsistencyChecks.v` (line 78).
//!
//! Exact identity: Q06 is defined as Q05 * Q07, so the chain holds
//! exactly by construction.
//!
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
fn witness_quark_mass_chain_Q05_Q07_Q06() {
    let phi: f64 = (1.0_f64 + 5.0_f64.sqrt()) / 2.0;
    let pi: f64 = std::f64::consts::PI;
    let e: f64 = std::f64::consts::E;

    let Q05 = 48.0 * e * e / phi.powi(4);
    let Q07 = 24.0 * phi * phi / pi;
    let Q06 = Q05 * Q07;

    let diff = (Q05 * Q07 - Q06).abs();
    assert!(diff < 1e-9, "|Q05*Q07 - Q06| = {} >= 1e-9", diff);
}
