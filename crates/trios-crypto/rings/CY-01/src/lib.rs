//! CY-01 — signing
//!
//! Stub signature type. Real signing logic will migrate from FFI.

use trios_crypto_cy00::PrivateKey;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature(pub Vec<u8>);

/// Stub signer — produces a deterministic signature from key bytes + message.
/// NOT cryptographically secure; placeholder for migration.
pub fn sign_stub(key: &PrivateKey, message: &[u8]) -> Signature {
    let mut out = key.0.clone();
    out.extend_from_slice(message);
    Signature(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_stub_is_deterministic() {
        let k = PrivateKey(vec![1, 2, 3]);
        let s1 = sign_stub(&k, b"hello");
        let s2 = sign_stub(&k, b"hello");
        assert_eq!(s1, s2);
    }
}
