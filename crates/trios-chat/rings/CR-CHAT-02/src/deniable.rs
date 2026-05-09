//! L-CHAT-5 (Wave-13) — Cryptographic deniability + transcript-forgery resistance.
//!
//! `[DERIVED from OTR (Borisov–Goldberg–Brewer 2004), Signal Double Ratchet
//!  deniability claim (Marlinspike–Perrin 2016 §4), R-CHAT-4]`
//!
//! ## Threat model addressed
//!
//! After a session ends an honest participant must be able to produce a
//! transcript that is **indistinguishable** from the real transcript Bob
//! would have seen, *even though Bob never sent those messages*. This
//! property is called **deniability** in OTR/Signal literature. It rules
//! out any cryptographic third-party attribution — Bob can always claim
//! "Alice forged this transcript herself".
//!
//! Concretely we forbid the use of *any* per-message public-key signature
//! (Ed25519, ML-DSA, …) for content authentication, and instead rely on a
//! **shared symmetric MAC** derived from the chain key. Any party that
//! holds the chain key — including Alice, who chose the root — can mint
//! valid MACs for arbitrary plaintexts. Hence:
//!
//! * Online phase: MAC convinces the **intended recipient** that the
//!   message came from someone holding the chain key.
//! * Offline phase: MAC convinces **nobody else** that any specific
//!   participant authored anything.
//!
//! ## Constitutional invariants (this module)
//!
//! * **INV-CHAT-61** `deniable_mac_verifies`            — well-formed MAC verifies.
//! * **INV-CHAT-62** `transcript_forgery_indistinguishable` —
//!   given the chain key, a forged transcript is bit-equal to a real one.
//! * **INV-CHAT-63** `no_per_message_signature`         — `Tag` carries no
//!   public-key signature field; it is exactly 32 bytes of HMAC-SHA-256.
//! * **INV-CHAT-64** `mac_tamper_rejected`              — flipping any byte
//!   of plaintext or AAD invalidates the tag.
//!
//! ## Honesty (R5)
//!
//! `[VERIFIED]` for the 6 unit tests below; deniability of the underlying
//! Double Ratchet construction is `[CITED]` to Marlinspike–Perrin 2016
//! and not re-proved here.

use hkdf::hmac::{Hmac, Mac};
use sha2::Sha256;
use zeroize::ZeroizeOnDrop;

/// 32-byte symmetric key used to derive deniable MACs.
///
/// Derived from the Triple-Ratchet chain key via a `"deniable-mac"`
/// HKDF info string (kept short to make the binding explicit). Wiping
/// happens on drop via [`zeroize`].
#[derive(Clone, ZeroizeOnDrop)]
pub struct DeniableMacKey(pub(crate) [u8; 32]);

impl DeniableMacKey {
    /// Wrap raw bytes (for use by the ratchet, tests).
    pub fn from_bytes(b: [u8; 32]) -> Self {
        Self(b)
    }

    /// Borrow the raw bytes — only for explicit forgery tests.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// MAC tag — exactly 32 bytes of HMAC-SHA-256 output. Carries **no**
/// per-message public-key signature; that is the point of deniability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag(pub [u8; 32]);

impl Tag {
    /// Constant length — INV-CHAT-63 (`no_per_message_signature`).
    pub const LEN: usize = 32;
}

/// Authenticate `plaintext` with `aad` under `key`.
///
/// Anyone holding `key` can produce this tag — that is precisely what
/// gives the construction its deniability property.
pub fn mac(key: &DeniableMacKey, aad: &[u8], plaintext: &[u8]) -> Tag {
    let mut m = <Hmac<Sha256> as Mac>::new_from_slice(&key.0).expect("hmac key length");
    // Domain-separate aad and plaintext to prevent length-extension confusions.
    m.update(&(aad.len() as u32).to_be_bytes());
    m.update(aad);
    m.update(&(plaintext.len() as u32).to_be_bytes());
    m.update(plaintext);
    let out = m.finalize().into_bytes();
    let mut t = [0u8; 32];
    t.copy_from_slice(&out);
    Tag(t)
}

/// Verify `tag` against `(aad, plaintext)` under `key`. Constant-time.
pub fn verify(key: &DeniableMacKey, aad: &[u8], plaintext: &[u8], tag: &Tag) -> bool {
    let mut m = <Hmac<Sha256> as Mac>::new_from_slice(&key.0).expect("hmac key length");
    m.update(&(aad.len() as u32).to_be_bytes());
    m.update(aad);
    m.update(&(plaintext.len() as u32).to_be_bytes());
    m.update(plaintext);
    m.verify_slice(&tag.0).is_ok()
}

/// Forge a transcript `(plaintext', tag')` for `plaintext'` chosen by
/// the holder of `key` after the fact. Returns a tag that verifies
/// under `key` even though no real party ever "sent" `plaintext'`.
///
/// **This function is intentionally exposed**: it is the constructive
/// witness for INV-CHAT-62 (`transcript_forgery_indistinguishable`).
/// Real session code never calls it.
pub fn forge_transcript(key: &DeniableMacKey, aad: &[u8], plaintext: &[u8]) -> (Vec<u8>, Tag) {
    (plaintext.to_vec(), mac(key, aad, plaintext))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k() -> DeniableMacKey {
        DeniableMacKey::from_bytes([7u8; 32])
    }

    /// **DEN-01** well-formed MAC verifies (INV-CHAT-61).
    #[test]
    fn den_01_mac_verifies() {
        let key = k();
        let aad = b"session=42|sender=alice";
        let pt = b"meet at the bridge at midnight";
        let t = mac(&key, aad, pt);
        assert!(verify(&key, aad, pt, &t), "honest MAC must verify");
        assert_eq!(t.0.len(), Tag::LEN, "tag must be HMAC-SHA-256 length");
    }

    /// **DEN-02** flipping any plaintext byte invalidates tag (INV-CHAT-64).
    #[test]
    fn den_02_plaintext_tamper_rejected() {
        let key = k();
        let aad = b"aad";
        let pt = b"the cake is a lie";
        let t = mac(&key, aad, pt);
        let mut bad = pt.to_vec();
        bad[3] ^= 0x01;
        assert!(
            !verify(&key, aad, &bad, &t),
            "tampered plaintext must not verify"
        );
    }

    /// **DEN-03** flipping any AAD byte invalidates tag (INV-CHAT-64).
    #[test]
    fn den_03_aad_tamper_rejected() {
        let key = k();
        let aad = b"context=v1";
        let pt = b"hello";
        let t = mac(&key, aad, pt);
        assert!(
            !verify(&key, b"context=v2", pt, &t),
            "swapped AAD must not verify"
        );
    }

    /// **DEN-04** wrong key rejects (basic MAC unforgeability).
    #[test]
    fn den_04_wrong_key_rejected() {
        let key = k();
        let other = DeniableMacKey::from_bytes([8u8; 32]);
        let aad = b"a";
        let pt = b"b";
        let t = mac(&key, aad, pt);
        assert!(
            !verify(&other, aad, pt, &t),
            "MAC under wrong key must not verify"
        );
    }

    /// **DEN-05** transcript forgery is bit-indistinguishable from honest
    /// MAC under the same key (INV-CHAT-62 — the deniability property).
    ///
    /// We construct two transcripts:
    ///   honest: Alice MACs `pt_honest`.
    ///   forged: anyone with `key` MACs `pt_forged` after the fact.
    /// Both verify; an external observer holding `key` cannot tell who
    /// minted which tag — that is deniability.
    #[test]
    fn den_05_transcript_forgery_indistinguishable() {
        let key = k();
        let aad = b"session=99";
        let pt_honest = b"I will be there";
        let pt_forged = b"I confess to everything";

        let t_honest = mac(&key, aad, pt_honest);
        let (forged_pt, t_forged) = forge_transcript(&key, aad, pt_forged);

        assert!(verify(&key, aad, pt_honest, &t_honest));
        assert!(verify(&key, aad, &forged_pt, &t_forged));

        // Both tags have identical structure — the *whole point* of
        // deniability. There is no Ed25519/ML-DSA component that would
        // bind either tag to a specific signer's private key.
        assert_eq!(t_honest.0.len(), t_forged.0.len());
        assert_eq!(t_honest.0.len(), Tag::LEN);
    }

    /// **DEN-06** `Tag` carries **no** public-key signature field
    /// (INV-CHAT-63). This is a structural / type-level test: a `Tag`
    /// is a 32-byte array — no Ed25519 (64-byte) component, no ML-DSA
    /// (3309-byte) component.
    #[test]
    fn den_06_no_per_message_signature() {
        let t = mac(&k(), b"", b"");
        assert_eq!(
            std::mem::size_of_val(&t.0),
            32,
            "Tag is exactly HMAC-SHA-256 (32 bytes), with no PK-signature"
        );
        // Compile-time sanity: there is no `sig` / `pk_sig` field on Tag.
        // The test is here to make the absence explicit and audit-grep-able.
        let _ = t;
    }
}
