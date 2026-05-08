//! trios-coq-witness — runtime-witness placeholders for Coq lemmas.
//!
//! Each `tests/witness_<lemma>.rs` is a `#[test] #[ignore]` placeholder anchored
//! to the umbrella tracking issue `trios#587`. Removing the `#[ignore]` and
//! asserting a meaningful invariant graduates a placeholder into a real
//! runtime witness.
//!
//! Anchor: phi^2 + phi^-2 = 3 (DOI 10.5281/zenodo.19227877).

/// Sentinel that exists only so the crate has a non-trivial `lib` target.
/// Tests under `tests/` import this name to keep the crate hooked into
/// `cargo test --workspace`.
pub fn anchor_value() -> f64 {
    // phi = (1 + sqrt(5)) / 2; phi^2 + phi^-2 = 3 exactly.
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
    phi.powi(2) + phi.powi(-2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_phi_identity_holds() {
        let v = anchor_value();
        assert!((v - 3.0).abs() < 1e-12, "phi^2 + phi^-2 must equal 3 (got {v})");
    }
}
