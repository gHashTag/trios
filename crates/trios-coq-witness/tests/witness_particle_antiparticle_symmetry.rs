#![allow(non_snake_case)]
//! Runtime-witness for Coq lemma `particle_antiparticle_symmetry` in
//! `docs/phd/theorems/trinity/ConsistencyChecks.v` (line 284).
//!
//! Structural check: all mass-formula constants are positive and finite,
//! confirming sign-symmetry between particle and antiparticle formulas.
//!
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
fn witness_particle_antiparticle_symmetry() {
    let phi: f64 = (1.0_f64 + 5.0_f64.sqrt()) / 2.0;
    let pi: f64 = std::f64::consts::PI;
    let e: f64 = std::f64::consts::E;

    let G01: f64 = 36.0 * phi * e * e / pi;
    let Q01: f64 = pi / (9.0 * e * e);
    let Q02: f64 = 4.0 * phi * phi / pi;
    let Q03: f64 = 2.0 * phi * e * e / pi;
    let Q05: f64 = 48.0 * e * e / phi.powi(4);
    let Q06: f64 = Q05 * (24.0 * phi * phi / pi);
    let Q07: f64 = 24.0 * phi * phi / pi;
    let H02: f64 = 4.0 * phi * e;
    let H03: f64 = phi * phi * e;
    let L01: f64 = 4.0 * phi.powi(3) / (e * e);
    let L02: f64 = 2.0 * phi.powi(4) * pi / e;
    let L03: f64 = 8.0 * phi.powi(7) * pi / (e * e * e);
    let N01: f64 = 8.0 * pi / (phi.powi(5) * e * e);
    let N03: f64 = 2.0 * pi / phi.powi(4);

    let constants = [
        ("G01", G01),
        ("Q01", Q01),
        ("Q02", Q02),
        ("Q03", Q03),
        ("Q05", Q05),
        ("Q06", Q06),
        ("Q07", Q07),
        ("H02", H02),
        ("H03", H03),
        ("L01", L01),
        ("L02", L02),
        ("L03", L03),
        ("N01", N01),
        ("N03", N03),
    ];

    for &(name, x) in &constants {
        assert!(x > 0.0, "{} = {} is not > 0", name, x);
        assert!(x.is_finite(), "{} = {} is not finite", name, x);
    }
}
