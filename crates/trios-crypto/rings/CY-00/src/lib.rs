//! CY-00 — cryptographic identity types
//!
//! Bottom of the ring graph for trios-crypto.
//! Newtypes for keys and identifiers. No crypto logic yet.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyId(pub String);

impl KeyId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKey(pub Vec<u8>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivateKey(pub Vec<u8>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyid_basic() {
        let k = KeyId::new("abc");
        assert_eq!(k.as_str(), "abc");
    }

    #[test]
    fn keys_distinct() {
        let pub_k = PublicKey(vec![1, 2, 3]);
        let priv_k = PrivateKey(vec![1, 2, 3]);
        assert_eq!(pub_k.0, priv_k.0);
    }
}
