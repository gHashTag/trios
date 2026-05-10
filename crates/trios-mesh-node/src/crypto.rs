//! Trinity Mesh — E2E Encryption Layer
//! X25519 ECDH key exchange + ChaCha20-Poly1305 AEAD
//! φ² + φ⁻² = 3
//!
//! KDF: SHA256(dh_shared || min(pk_a, pk_b) || max(pk_a, pk_b))
//! Sorting pubkeys makes KDF commutative: encrypt(A→B) == decrypt(B←A)

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

/// Node keypair — deterministic from seed (dev) or random (prod)
pub struct MeshKeypair {
    pub secret: StaticSecret,
    pub public: PublicKey,
    /// dest_hash = SHA256(pubkey)[..16]
    pub dest_hash: [u8; 16],
}

impl MeshKeypair {
    pub fn from_seed(seed: u8) -> Self {
        let mut seed_bytes = [0u8; 32];
        seed_bytes[0] = seed;
        seed_bytes[1] = 0xA3;
        let stretched = Sha256::digest(&seed_bytes);
        let mut key = [0u8; 32];
        key.copy_from_slice(&stretched);
        let secret = StaticSecret::from(key);
        let public = PublicKey::from(&secret);
        let hash = Sha256::digest(public.as_bytes());
        let mut dest_hash = [0u8; 16];
        dest_hash.copy_from_slice(&hash[..16]);
        Self { secret, public, dest_hash }
    }

    pub fn random() -> Self {
        use rand::RngCore;
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        let secret = StaticSecret::from(key);
        let public = PublicKey::from(&secret);
        let hash = Sha256::digest(public.as_bytes());
        let mut dest_hash = [0u8; 16];
        dest_hash.copy_from_slice(&hash[..16]);
        Self { secret, public, dest_hash }
    }

    /// Symmetric ECDH shared secret.
    /// KDF = SHA256(dh_output || min(pk_local, pk_remote) || max(pk_local, pk_remote))
    /// Sorting ensures KDF is commutative: A.shared(B) == B.shared(A)
    pub fn shared_secret(&self, remote_pubkey_bytes: &[u8; 32]) -> [u8; 32] {
        let remote = PublicKey::from(*remote_pubkey_bytes);
        let dh = self.secret.diffie_hellman(&remote);
        let local_bytes = self.public.as_bytes();
        let (pk_lo, pk_hi) = if local_bytes <= remote_pubkey_bytes {
            (local_bytes.as_slice(), remote_pubkey_bytes.as_slice())
        } else {
            (remote_pubkey_bytes.as_slice(), local_bytes.as_slice())
        };
        let mut kdf = Vec::with_capacity(96);
        kdf.extend_from_slice(dh.as_bytes());
        kdf.extend_from_slice(pk_lo);
        kdf.extend_from_slice(pk_hi);
        let derived = Sha256::digest(&kdf);
        let mut out = [0u8; 32];
        out.copy_from_slice(&derived);
        out
    }

    pub fn pubkey_hex(&self) -> String { hex::encode(self.public.as_bytes()) }
    pub fn pubkey_bytes(&self) -> [u8; 32] { *self.public.as_bytes() }
}

/// Encrypt plaintext for recipient. Returns base64(nonce || ciphertext)
pub fn encrypt_for(payload: &[u8], keypair: &MeshKeypair, recipient_pubkey: &[u8; 32]) -> Result<String> {
    let shared = keypair.shared_secret(recipient_pubkey);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&shared));
    use rand::RngCore;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher.encrypt(nonce, payload).map_err(|e| anyhow!("encrypt error: {e}"))?;
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(B64.encode(&out))
}

/// Decrypt base64(nonce || ciphertext) from sender
pub fn decrypt_from(b64: &str, keypair: &MeshKeypair, sender_pubkey: &[u8; 32]) -> Result<Vec<u8>> {
    let raw = B64.decode(b64).map_err(|e| anyhow!("base64: {e}"))?;
    if raw.len() < 12 { return Err(anyhow!("payload too short")); }
    let (nonce_bytes, ct) = raw.split_at(12);
    let shared = keypair.shared_secret(sender_pubkey);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&shared));
    cipher.decrypt(Nonce::from_slice(nonce_bytes), ct)
        .map_err(|e| anyhow!("decrypt error: {e} — wrong key or tampered payload"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_a_to_b() {
        let alice = MeshKeypair::from_seed(0);
        let bob   = MeshKeypair::from_seed(1);
        let enc = encrypt_for(b"phi^2 + phi^-2 = 3", &alice, &bob.pubkey_bytes()).unwrap();
        let dec = decrypt_from(&enc, &bob, &alice.pubkey_bytes()).unwrap();
        assert_eq!(dec, b"phi^2 + phi^-2 = 3");
    }

    #[test]
    fn roundtrip_b_to_a() {
        let alice = MeshKeypair::from_seed(0);
        let bob   = MeshKeypair::from_seed(1);
        let enc = encrypt_for(b"reverse ok", &bob, &alice.pubkey_bytes()).unwrap();
        let dec = decrypt_from(&enc, &alice, &bob.pubkey_bytes()).unwrap();
        assert_eq!(dec, b"reverse ok");
    }

    #[test]
    fn kdf_symmetric() {
        let a = MeshKeypair::from_seed(0);
        let b = MeshKeypair::from_seed(1);
        assert_eq!(a.shared_secret(&b.pubkey_bytes()), b.shared_secret(&a.pubkey_bytes()));
    }

    #[test]
    fn wrong_key_fails() {
        let alice = MeshKeypair::from_seed(0);
        let bob   = MeshKeypair::from_seed(1);
        let eve   = MeshKeypair::from_seed(2);
        let enc = encrypt_for(b"secret", &alice, &bob.pubkey_bytes()).unwrap();
        assert!(decrypt_from(&enc, &eve, &alice.pubkey_bytes()).is_err());
    }

    #[test]
    fn dest_hash_from_pubkey() {
        let kp = MeshKeypair::from_seed(0);
        assert_eq!(&kp.dest_hash, &Sha256::digest(kp.public.as_bytes())[..16]);
    }
}
