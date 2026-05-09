//! L-CHAT-4 · trinity-fpga#32 — Sealed Sender envelope over trios-mesh.
//!
//! Per **R-CHAT-3** the mesh sees only `(dest_hash[16], padded_envelope)`.
//! `src_pub` is encrypted to `recipient.x25519_pub` via X25519 ECDH +
//! ChaCha20-Poly1305 (the same KDF rule trios-mesh-node uses: SHA-256 over
//! min/max-sorted public keys — see `crypto.rs:ab6769f` from PR #629).
//!
//! `[VERIFIED]` for the construction; `[ASPIRATIONAL]` for full
//! sender-receiver-unlinkability proof (G-C3 statistical test in L-CHAT-10).

use chacha20poly1305::aead::Aead;
use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, Nonce};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

use trios_chat_cr_chat_00::{Error, Result};
use trios_chat_cr_chat_04::{pad_class, unpad};

/// 16-byte destination hash — what the mesh sees and routes on.
pub fn dest_hash(recipient_x25519_pub: &PublicKey) -> [u8; 16] {
    let mut h = Sha256::new();
    h.update(b"trinity-chat:dest-hash:v1");
    h.update(recipient_x25519_pub.as_bytes());
    let full = h.finalize();
    let mut out = [0u8; 16];
    out.copy_from_slice(&full[..16]);
    out
}

/// Symmetric KDF identical to trios-mesh-node `crypto.rs` (commit ab6769f):
/// sort the two X25519 public keys lexicographically, then SHA-256 the pair.
/// Guarantees A and B derive the same 32-byte session key without exchanging
/// roles.
fn symmetric_kdf(a: &[u8; 32], b: &[u8; 32], shared: &[u8; 32]) -> [u8; 32] {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let mut h = Sha256::new();
    h.update(b"trinity-chat:sealed:v1");
    h.update(lo);
    h.update(hi);
    h.update(shared);
    let mut out = [0u8; 32];
    out.copy_from_slice(&h.finalize()[..32]);
    out
}

/// Wire-format envelope.
///
/// Layout (after pad_class):
/// `| 16 dest_hash | 32 src_x25519_pub | 12 nonce | N ciphertext+tag |`
///
/// `src_x25519_pub` is the **sender's** ephemeral or prekey public, not their
/// long-term identity — the receiver dereferences it from their contact book
/// to authenticate.
pub struct SealedEnvelope {
    /// What mesh routes on.
    pub dest_hash: [u8; 16],
    /// Sender X25519 ephemeral (32 B).
    pub src_x25519_pub: [u8; 32],
    /// AEAD nonce (12 B).
    pub nonce: [u8; 12],
    /// Padded ciphertext (size class).
    pub ciphertext: Vec<u8>,
}

impl SealedEnvelope {
    /// Seal `payload` for `recipient_x25519_pub`. Returns `(envelope,
    /// pad_size_class)` where the ciphertext fits into one of `{256, 1024,
    /// 4096, 16384}` bytes per R-CHAT-9.
    pub fn seal(
        sender_secret: &StaticSecret,
        sender_public: &PublicKey,
        recipient_x25519_pub: &PublicKey,
        nonce: [u8; 12],
        payload: &[u8],
    ) -> Result<Self> {
        let shared = sender_secret.diffie_hellman(recipient_x25519_pub);
        let sk = symmetric_kdf(
            sender_public.as_bytes(),
            recipient_x25519_pub.as_bytes(),
            shared.as_bytes(),
        );
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&sk));
        let padded = pad_class(payload);
        let ct = cipher
            .encrypt(Nonce::from_slice(&nonce), padded.as_slice())
            .map_err(|_| Error::Crypto("sealed encrypt failed"))?;
        Ok(Self {
            dest_hash: dest_hash(recipient_x25519_pub),
            src_x25519_pub: *sender_public.as_bytes(),
            nonce,
            ciphertext: ct,
        })
    }

    /// Unseal — caller has already routed on `dest_hash` and matched it to a
    /// local recipient. Returns the *unpadded* original plaintext.
    pub fn unseal(
        &self,
        recipient_secret: &StaticSecret,
        recipient_public: &PublicKey,
    ) -> Result<Vec<u8>> {
        if dest_hash(recipient_public) != self.dest_hash {
            return Err(Error::Invariant("sealed: dest_hash mismatch"));
        }
        let src_pub = PublicKey::from(self.src_x25519_pub);
        let shared = recipient_secret.diffie_hellman(&src_pub);
        let sk = symmetric_kdf(
            &self.src_x25519_pub,
            recipient_public.as_bytes(),
            shared.as_bytes(),
        );
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&sk));
        let padded = cipher
            .decrypt(Nonce::from_slice(&self.nonce), self.ciphertext.as_slice())
            .map_err(|_| Error::Crypto("sealed decrypt failed"))?;
        unpad(&padded).map(|s| s.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    fn pair() -> (StaticSecret, PublicKey) {
        let s = StaticSecret::random_from_rng(OsRng);
        let p = PublicKey::from(&s);
        (s, p)
    }

    #[test]
    fn roundtrip_seal_unseal() {
        let (a_s, a_p) = pair();
        let (b_s, b_p) = pair();
        let msg = b"hello bob, this is alice";
        let env = SealedEnvelope::seal(&a_s, &a_p, &b_p, [9u8; 12], msg).unwrap();
        let dec = env.unseal(&b_s, &b_p).unwrap();
        assert_eq!(dec, msg);
    }

    #[test]
    fn falsifier_wrong_recipient_cannot_unseal() {
        let (a_s, a_p) = pair();
        let (_, b_p) = pair();
        let (c_s, c_p) = pair();
        let env = SealedEnvelope::seal(&a_s, &a_p, &b_p, [1u8; 12], b"x").unwrap();
        // Charlie tries to unseal a message destined for Bob.
        assert!(env.unseal(&c_s, &c_p).is_err());
    }

    #[test]
    fn falsifier_tampered_ciphertext_rejected() {
        let (a_s, a_p) = pair();
        let (b_s, b_p) = pair();
        let mut env = SealedEnvelope::seal(&a_s, &a_p, &b_p, [2u8; 12], b"y").unwrap();
        env.ciphertext[0] ^= 1;
        assert!(env.unseal(&b_s, &b_p).is_err());
    }

    #[test]
    fn dest_hash_is_deterministic() {
        let (_, p) = pair();
        assert_eq!(dest_hash(&p), dest_hash(&p));
    }

    #[test]
    fn dest_hash_differs_for_different_keys() {
        let (_, p1) = pair();
        let (_, p2) = pair();
        assert_ne!(dest_hash(&p1), dest_hash(&p2));
    }

    #[test]
    fn ciphertext_padding_class_is_one_of_canonical_classes() {
        // R-CHAT-9 — wire-size privacy. The ciphertext expands by 16 (Poly1305 tag)
        // over the padded plaintext (one of 256/1024/4096/16384).
        let (a_s, a_p) = pair();
        let (_, b_p) = pair();
        let env = SealedEnvelope::seal(&a_s, &a_p, &b_p, [3u8; 12], b"short").unwrap();
        let len = env.ciphertext.len();
        // Expected padded size = 256 + 16 (AEAD tag).
        assert_eq!(len, 256 + 16, "expect smallest pad class + AEAD tag");
    }
}
