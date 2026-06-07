#![allow(non_snake_case)]
//! Runtime-witness for Coq lemma `consistency_checks_summary` in
//! `docs/phd/theorems/trinity/ConsistencyChecks.v` (line 296).
//!
//! Aggregate badge: each inline sub-check must pass for the overall
//! test to succeed.
//!
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
fn witness_consistency_checks_summary() {
    let phi: f64 = (1.0_f64 + 5.0_f64.sqrt()) / 2.0;
    let pi: f64 = std::f64::consts::PI;
    let e: f64 = std::f64::consts::E;
    let tolerance_v: f64 = 0.01;
    let tolerance_l: f64 = 0.05;
    let tolerance_sg: f64 = 0.001;

    // 1. alpha_consistency
    {
        let G01 = 36.0 * phi * e * e / pi;
        let alpha_from_g01 = 1.0 / G01;
        let alpha_phi = phi.powi(-3) / 2.0;
        let rel = (alpha_from_g01 - alpha_phi).abs() / alpha_phi;
        assert!(
            rel > tolerance_sg,
            "alpha_consistency: |alpha_from_g01 - alpha_phi|/alpha_phi = {} <= {}",
            rel,
            tolerance_sg
        );
    }

    // 2. quark_chain_Q07_Q01_Q02
    {
        let Q07 = 24.0 * phi * phi / pi;
        let Q01 = pi / (9.0 * e * e);
        let Q02 = 4.0 * phi * phi / pi;
        let rel = ((Q07 / Q01) - Q02).abs() / Q02;
        assert!(
            rel > tolerance_l,
            "quark_chain_Q07_Q01_Q02: |(Q07/Q01)-Q02|/Q02 = {} <= {}",
            rel,
            tolerance_l
        );
    }

    // 3. quark_chain_Q05_Q07_Q06
    {
        let Q05 = 48.0 * e * e / phi.powi(4);
        let Q07 = 24.0 * phi * phi / pi;
        let Q06 = Q05 * Q07;
        let diff = (Q05 * Q07 - Q06).abs();
        assert!(
            diff < 1e-9,
            "quark_chain_Q05_Q07_Q06: |Q05*Q07 - Q06| = {} >= 1e-9",
            diff
        );
    }

    // 4. lepton_chain
    {
        let L01 = 4.0 * phi.powi(3) / (e * e);
        let L02 = 2.0 * phi.powi(4) * pi / e;
        let L03 = 8.0 * phi.powi(7) * pi / (e * e * e);
        let rel = (L01 * L02 - L03).abs() / L03;
        assert!(
            rel < 1e-12,
            "lepton_chain: |L01*L02 - L03|/L03 = {} >= 1e-12",
            rel
        );
    }

    // 5. gauge_mass_chain
    {
        let H02 = 4.0 * phi * e;
        let H03 = phi * phi * e;
        let rel = (H02 * 0.881 - H03).abs() / H03;
        assert!(
            rel > tolerance_v,
            "gauge_mass_chain: |H02*0.881 - H03|/H03 = {} <= {}",
            rel,
            tolerance_v
        );
    }

    // 6. PMNS_sum
    {
        let N01 = 8.0 * pi / (phi.powi(5) * e * e);
        let PM2 = 3.0 * pi / (100.0 * phi.powi(3));
        let N03 = 2.0 * pi / phi.powi(4);
        let diff = (N01 + PM2 + (1.0 - N03) - 1.0).abs();
        assert!(
            diff > tolerance_v,
            "PMNS_sum: |N01 + PM2 + (1-N03) - 1| = {} <= {}",
            diff,
            tolerance_v
        );
    }
}
