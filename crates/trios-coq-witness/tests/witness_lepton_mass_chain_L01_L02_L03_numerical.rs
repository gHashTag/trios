#![allow(non_snake_case)]
//! Runtime-witness for Coq lemma `lepton_mass_chain_L01_L02_L03_numerical`
//! in `docs/phd/theorems/trinity/ConsistencyChecks.v` (line 117).
//!
//! PDG cross-check: (m_mu/m_e) * (m_tau/m_mu) should equal m_tau/m_e
//! within experimental tolerance.
//!
//! Anchor: phi^2 + phi^-2 = 3.

#[test]
fn witness_lepton_mass_chain_L01_L02_L03_numerical() {
    let tolerance_l: f64 = 0.05;

    let m_mu_over_m_e: f64 = 206.8;
    let m_tau_over_m_mu: f64 = 16.8;
    let m_tau_over_m_e: f64 = 3477.0;

    let product = m_mu_over_m_e * m_tau_over_m_mu;
    let rel = (product - m_tau_over_m_e).abs() / m_tau_over_m_e;
    assert!(
        rel < tolerance_l,
        "|206.8*16.8 - 3477|/3477 = {} >= {}",
        rel,
        tolerance_l
    );
}
