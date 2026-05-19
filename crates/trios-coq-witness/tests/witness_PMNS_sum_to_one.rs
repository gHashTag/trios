#![allow(non_snake_case)]
//! Runtime-witness for Coq theorem `PMNS_sum_to_one`
//! in `docs/phd/theorems/trinity/ConsistencyChecks.v` (line 227).
//!
//! Coq strategy: real proof via `Interval.Tactic` (R8 falsified —
//! PMNS row-sum identity does NOT hold under Chimera v1.0 N03).
//! No Admitted.
//!
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
fn witness_PMNS_sum_to_one() {
    let phi: f64 = (1.0_f64 + 5.0_f64.sqrt()) / 2.0;
    let pi: f64 = std::f64::consts::PI;
    let e: f64 = std::f64::consts::E;
    let tolerance_v: f64 = 0.01;

    let N01 = 8.0 * pi / (phi.powi(5) * e * e);
    let PM2 = 3.0 * pi / (100.0 * phi.powi(3));
    let N03 = 2.0 * pi / phi.powi(4);
    let sum = N01 + PM2 + (1.0 - N03);

    assert!(
        (sum - 1.0).abs() > tolerance_v,
        "PMNS falsification failed: sum = {}",
        sum
    );
}
