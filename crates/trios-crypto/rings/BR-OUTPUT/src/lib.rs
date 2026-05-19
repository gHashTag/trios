//! BR-OUTPUT — trios-crypto assembly

pub use trios_crypto_cy00::{KeyId, PrivateKey, PublicKey};
pub use trios_crypto_cy01::{sign_stub, Signature};
pub use trios_crypto_cy02::verify_stub;

pub struct Crypto;

impl Crypto {
    pub const fn anchor() -> f64 {
        3.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_link_sign_verify_stub() {
        let pub_k = PublicKey(vec![1, 2, 3]);
        let priv_k = PrivateKey(vec![1, 2, 3]);
        let sig = sign_stub(&priv_k, b"hello");
        assert!(verify_stub(&pub_k, b"hello", &sig));
    }
}
