//! CY-02 — signature verification
//!
//! Stub verifier. Real verification migrates from FFI.

use trios_crypto_cy00::PublicKey;
use trios_crypto_cy01::Signature;

/// Stub verifier — accepts any signature whose first bytes match the pubkey.
/// NOT cryptographically secure; placeholder for migration.
pub fn verify_stub(pub_key: &PublicKey, _message: &[u8], sig: &Signature) -> bool {
    sig.0.starts_with(&pub_key.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_stub_matches_prefix() {
        let pub_key = PublicKey(vec![1, 2, 3]);
        let good = Signature(vec![1, 2, 3, 99]);
        let bad = Signature(vec![9, 9, 9]);
        assert!(verify_stub(&pub_key, b"msg", &good));
        assert!(!verify_stub(&pub_key, b"msg", &bad));
    }
}
