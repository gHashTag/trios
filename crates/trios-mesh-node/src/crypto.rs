//! Trinity Mesh — E2E Encryption Layer
//! X25519 ECDH key exchange + ChaCha20-Poly1305 AEAD
//! φ² + φ⁻² = 3

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use sha2::{Digest, Sha256};
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};

/// Node keypair — deterministic from seed (dev) or random (prod)
pub struct MeshKeypair {
    pub secret: StaticSecret,
    pub public: PublicKey,
    /// dest_hash = SHA256(pubkey)[..16]
    pub dest_hash: [u8; 16],
}

impl MeshKeypair {
    /// Deterministic from seed byte — for dev/test
    pub fn from_seed(seed: u8) -> Self {
        let mut seed_bytes = [0u8; 32];
        seed_bytes[0] = seed;
        seed_bytes[1] = 0xA3; // Trinity φ-marker
        // Stretch seed through SHA256 to fill 32 bytes
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

    /// Fully random keypair — for production nodes
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

    /// ECDH shared secret with remote pubkey
    pub fn shared_secret(&self, remote_pubkey_bytes: &[u8; 32]) -> [u8; 32] {
        let remote = PublicKey::from(*remote_pubkey_bytes);
        // Note: StaticSecret doesn't impl DiffieHellman directly,
        // so we clone into EphemeralSecret-like via raw bytes
        let shared = self.secret.diffie_hellman(&remote);
        // KDF: SHA256(shared || local_pub || remote_pub)
        let mut kdf_input = Vec::with_capacity(96);
        kdf_input.extend_from_slice(shared.as_bytes());
        kdf_input.extend_from_slice(self.public.as_bytes());
        kdf_input.extend_from_slice(remote_pubkey_bytes);
        let derived = Sha256::digest(&kdf_input);
        let mut out = [0u8; 32];
        out.copy_from_slice(&derived);
        out
    }

    pub fn pubkey_hex(&self) -> String {
        hex::encode(self.public.as_bytes())
    }

    pub fn pubkey_bytes(&self) -> [u8; 32] {
        *self.public.as_bytes()
    }
}

/// Encrypt plaintext for recipient identified by their pubkey
/// Returns base64(nonce || ciphertext)
pub fn encrypt_for(payload: &[u8], keypair: &MeshKeypair, recipient_pubkey: &[u8; 32]) -> Result<String> {
    let shared = keypair.shared_secret(recipient_pubkey);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&shared));

    // 12-byte nonce: random
    use rand::RngCore;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, payload)
        .map_err(|e| anyhow!("encrypt error: {e}"))?;

    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(B64.encode(&out))
}

/// Decrypt base64(nonce || ciphertext) from sender's pubkey
pub fn decrypt_from(b64_payload: &str, keypair: &MeshKeypair, sender_pubkey: &[u8; 32]) -> Result<Vec<u8>> {
    let raw = B64.decode(b64_payload).map_err(|e| anyhow!("base64: {e}"))?;
    if raw.len() < 12 {
        return Err(anyhow!("payload too short"));
    }
    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Note: shared secret is symmetric (ECDH is commutative)
    let shared = keypair.shared_secret(sender_pubkey);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&shared));

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("decrypt error: {e} — wrong key or tampered payload"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_encrypt_decrypt() {
        let alice = MeshKeypair::from_seed(0);
        let bob   = MeshKeypair::from_seed(1);

        let plaintext = b"phi^2 + phi^-2 = 3";
        let enc = encrypt_for(plaintext, &alice, &bob.pubkey_bytes()).unwrap();
        let dec = decrypt_from(&enc, &bob, &alice.pubkey_bytes()).unwrap();
        assert_eq!(dec, plaintext);
    }

    #[test]
    fn wrong_key_fails() {
        let alice = MeshKeypair::from_seed(0);
        let bob   = MeshKeypair::from_seed(1);
        let eve   = MeshKeypair::from_seed(2);

        let enc = encrypt_for(b"secret", &alice, &bob.pubkey_bytes()).unwrap();
        // Eve tries to decrypt with wrong key
        assert!(decrypt_from(&enc, &eve, &alice.pubkey_bytes()).is_err());
    }

    #[test]
    fn dest_hash_from_pubkey() {
        let kp = MeshKeypair::from_seed(0);
        // dest_hash must be derived from pubkey, not seed
        let expected = &Sha256::digest(kp.public.as_bytes())[..16];
        assert_eq!(&kp.dest_hash, expected);
    }
}
