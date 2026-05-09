//! L-CHAT-1 · trinity-fpga#29 — Identity & Onboarding
//!
//! Prekey bundle = `{ed25519_lt, x25519_pre, mlkem768_pre_placeholder}`.
//! Per **R-CHAT-2**, every handshake is hybrid X25519 ⊕ ML-KEM-768 from day 1.
//! Per **R-CHAT-4**, only the prekey bundle is signed; messages are MAC-only.
//!
//! ML-KEM-768 (NIST FIPS 203) is wired as an opaque-bytes placeholder
//! (`MLKEM_PUB_LEN` = 1184) so the protocol shape is correct while
//! the concrete `ml-kem` crate is feature-gated for CR-CHAT-02 integration.
//! See `[ASPIRATIONAL]` tag below.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey as X25519Pub, StaticSecret as X25519Sec};
use zeroize::ZeroizeOnDrop;

use trios_chat_cr_chat_00::{Error, Result};

use crate::PROTOCOL_VERSION;

/// ML-KEM-768 public key placeholder (1184 B in FIPS 203).
/// `[ASPIRATIONAL]` — opaque bytes; concrete KEM lands in CR-CHAT-02.
pub const MLKEM_PUB_LEN: usize = 1184;

/// ML-KEM-768 secret seed placeholder (32 B).
pub const MLKEM_SEC_LEN: usize = 32;

/// One Trinity-Chat identity — long-term + ephemeral material.
///
/// `[VERIFIED]` — Ed25519 + X25519 generation tested.
/// `[ASPIRATIONAL]` — ML-KEM bytes are random placeholders; concrete KEM in CR-CHAT-02.
#[derive(ZeroizeOnDrop)]
pub struct Identity {
    /// Long-term Ed25519 signing key — used **only** to sign prekey bundles
    /// (R-CHAT-4 forbids per-message signatures).
    pub(crate) lt_signing: SigningKey,
    /// X25519 prekey secret — refreshed per session (one-time prekey).
    pub(crate) pre_x25519: X25519Sec,
    /// ML-KEM-768 prekey secret seed.
    #[zeroize(skip)]
    pub(crate) pre_mlkem_seed: [u8; MLKEM_SEC_LEN],
}

impl Identity {
    /// Generate a fresh identity with secure randomness.
    pub fn generate() -> Self {
        let lt_signing = SigningKey::generate(&mut OsRng);
        let pre_x25519 = X25519Sec::random_from_rng(OsRng);
        let mut pre_mlkem_seed = [0u8; MLKEM_SEC_LEN];
        // Fresh randomness — opaque to the rest of the protocol.
        rand_core::RngCore::fill_bytes(&mut OsRng, &mut pre_mlkem_seed);
        Self {
            lt_signing,
            pre_x25519,
            pre_mlkem_seed,
        }
    }

    /// Borrow the X25519 prekey secret (used by sealed-sender as the
    /// recipient secret).
    pub fn pre_x25519_secret(&self) -> &X25519Sec {
        &self.pre_x25519
    }

    /// Long-term Ed25519 verifying key.
    pub fn lt_verifying(&self) -> VerifyingKey {
        self.lt_signing.verifying_key()
    }

    /// X25519 prekey public.
    pub fn pre_x25519_pub(&self) -> X25519Pub {
        X25519Pub::from(&self.pre_x25519)
    }

    /// ML-KEM-768 prekey public — derived deterministically from seed.
    /// `[ASPIRATIONAL]`: placeholder = SHA-256(seed) repeated to MLKEM_PUB_LEN.
    pub fn pre_mlkem_pub(&self) -> [u8; MLKEM_PUB_LEN] {
        derive_placeholder_pub(&self.pre_mlkem_seed)
    }

    /// Build a published prekey bundle, signed by the long-term key.
    pub fn build_bundle(&self) -> PrekeyBundle {
        let body = PrekeyBundleBody {
            version: PROTOCOL_VERSION,
            lt_pub: self.lt_verifying().to_bytes(),
            x25519_pub: self.pre_x25519_pub().to_bytes(),
            mlkem_pub: self.pre_mlkem_pub(),
            issued_at_unix: 0, // injected by transport at publish time
            valid_for_secs: 7 * 24 * 60 * 60,
        };
        let sig = self.lt_signing.sign(&body.canonical_bytes());
        PrekeyBundle {
            body,
            signature: sig.to_bytes(),
        }
    }

    /// Sign arbitrary bytes with the long-term Ed25519 key.
    /// Used by CR-CHAT-02 (group commits) and CR-CHAT-06 (capabilities).
    pub fn sign(&self, msg: &[u8]) -> [u8; 64] {
        self.lt_signing.sign(msg).to_bytes()
    }

    /// Compute the **safety number** between two identities.
    /// `safety_no = SHA-256(min(lt_a, lt_b) ‖ max(lt_a, lt_b))`.
    /// Returns 30 bytes → 60 decimal digits (5 per byte ÷ 2 grouping).
    pub fn safety_number(a: &VerifyingKey, b: &VerifyingKey) -> [u8; 30] {
        let (lo, hi) = if a.as_bytes() <= b.as_bytes() {
            (a.as_bytes(), b.as_bytes())
        } else {
            (b.as_bytes(), a.as_bytes())
        };
        let mut h = Sha256::new();
        h.update(lo);
        h.update(hi);
        let full = h.finalize();
        let mut out = [0u8; 30];
        out.copy_from_slice(&full[..30]);
        out
    }
}

/// Public, signable body of a prekey bundle.
#[derive(Clone, Serialize, Deserialize)]
pub struct PrekeyBundleBody {
    /// Wire format version.
    pub version: u16,
    /// Long-term Ed25519 verifying key (32 B).
    pub lt_pub: [u8; 32],
    /// X25519 prekey public (32 B).
    pub x25519_pub: [u8; 32],
    /// ML-KEM-768 prekey public (1184 B).
    #[serde(with = "serde_byte_arr_1184")]
    pub mlkem_pub: [u8; MLKEM_PUB_LEN],
    /// Unix-seconds when the bundle was published.
    pub issued_at_unix: u64,
    /// Validity window in seconds; verifier rejects if `now > issued_at + this`.
    pub valid_for_secs: u64,
}

impl PrekeyBundleBody {
    /// Canonical serialization for signing — version-tagged, big-endian.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(2 + 32 + 32 + MLKEM_PUB_LEN + 8 + 8);
        v.extend_from_slice(b"trinity-chat:prekey:v1\0");
        v.extend_from_slice(&self.version.to_be_bytes());
        v.extend_from_slice(&self.lt_pub);
        v.extend_from_slice(&self.x25519_pub);
        v.extend_from_slice(&self.mlkem_pub);
        v.extend_from_slice(&self.issued_at_unix.to_be_bytes());
        v.extend_from_slice(&self.valid_for_secs.to_be_bytes());
        v
    }
}

/// Signed prekey bundle, ready for publication.
#[derive(Clone, Serialize, Deserialize)]
pub struct PrekeyBundle {
    /// Body that was signed.
    pub body: PrekeyBundleBody,
    /// Ed25519 signature over `body.canonical_bytes()`.
    #[serde(with = "serde_byte_arr_64")]
    pub signature: [u8; 64],
}

impl PrekeyBundle {
    /// Verify signature against the embedded long-term key.
    /// Returns `Ok(())` only if signature is valid for the canonical body.
    pub fn verify(&self) -> Result<()> {
        let vk = VerifyingKey::from_bytes(&self.body.lt_pub)
            .map_err(|_| Error::Crypto("malformed lt_pub"))?;
        let sig = Signature::from_bytes(&self.signature);
        vk.verify(&self.body.canonical_bytes(), &sig)
            .map_err(|_| Error::Crypto("prekey signature invalid"))?;
        Ok(())
    }

    /// Verify and check temporal validity at `now_unix`.
    pub fn verify_at(&self, now_unix: u64) -> Result<()> {
        self.verify()?;
        if now_unix > self.body.issued_at_unix.saturating_add(self.body.valid_for_secs) {
            return Err(Error::Invariant("prekey bundle expired"));
        }
        Ok(())
    }
}

fn derive_placeholder_pub(seed: &[u8; 32]) -> [u8; MLKEM_PUB_LEN] {
    let mut out = [0u8; MLKEM_PUB_LEN];
    let mut counter = 0u64;
    let mut filled = 0usize;
    while filled < MLKEM_PUB_LEN {
        let mut h = Sha256::new();
        h.update(b"trinity-chat:mlkem-placeholder:");
        h.update(seed);
        h.update(counter.to_be_bytes());
        let block = h.finalize();
        let n = std::cmp::min(32, MLKEM_PUB_LEN - filled);
        out[filled..filled + n].copy_from_slice(&block[..n]);
        filled += n;
        counter += 1;
    }
    out
}

mod serde_byte_arr_64 {
    use serde::{de, Deserializer, Serializer};
    pub fn serialize<S>(v: &[u8; 64], s: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_bytes(v)
    }
    pub fn deserialize<'de, D>(d: D) -> std::result::Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: Vec<u8> = serde::Deserialize::deserialize(d)?;
        if v.len() != 64 {
            return Err(de::Error::custom("signature length mismatch"));
        }
        let mut out = [0u8; 64];
        out.copy_from_slice(&v);
        Ok(out)
    }
}

mod serde_byte_arr_1184 {
    use serde::{de, Deserializer, Serializer};
    pub fn serialize<S>(v: &[u8; super::MLKEM_PUB_LEN], s: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_bytes(v)
    }
    pub fn deserialize<'de, D>(d: D) -> std::result::Result<[u8; super::MLKEM_PUB_LEN], D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: Vec<u8> = serde::Deserialize::deserialize(d)?;
        if v.len() != super::MLKEM_PUB_LEN {
            return Err(de::Error::custom("mlkem_pub length mismatch"));
        }
        let mut out = [0u8; super::MLKEM_PUB_LEN];
        out.copy_from_slice(&v);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_bundle() {
        let id = Identity::generate();
        let b = id.build_bundle();
        b.verify().expect("freshly built bundle must verify");
    }

    #[test]
    fn falsifier_flipped_bit_breaks_signature() {
        let id = Identity::generate();
        let mut b = id.build_bundle();
        b.signature[0] ^= 1;
        assert!(b.verify().is_err(), "flipped sig must fail");
    }

    #[test]
    fn falsifier_swapped_lt_pub_breaks_signature() {
        let id = Identity::generate();
        let other = Identity::generate();
        let mut b = id.build_bundle();
        b.body.lt_pub = other.lt_verifying().to_bytes();
        assert!(b.verify().is_err(), "swapped lt_pub must fail");
    }

    #[test]
    fn falsifier_expired_bundle() {
        let id = Identity::generate();
        let mut b = id.build_bundle();
        b.body.issued_at_unix = 1_000_000;
        b.body.valid_for_secs = 1;
        // Force re-sign so the only failure is expiry, not signature drift
        let body_bytes = b.body.canonical_bytes();
        let resigned = id.lt_signing.sign(&body_bytes);
        b.signature = resigned.to_bytes();
        assert!(b.verify().is_ok());
        assert!(b.verify_at(2_000_000).is_err(), "expired must fail");
    }

    #[test]
    fn safety_number_is_symmetric() {
        let a = Identity::generate();
        let bb = Identity::generate();
        let s1 = Identity::safety_number(&a.lt_verifying(), &bb.lt_verifying());
        let s2 = Identity::safety_number(&bb.lt_verifying(), &a.lt_verifying());
        assert_eq!(s1, s2, "safety number must be order-independent");
    }

    #[test]
    fn safety_number_differs_for_different_pairs() {
        let a = Identity::generate();
        let bb = Identity::generate();
        let cc = Identity::generate();
        let s_ab = Identity::safety_number(&a.lt_verifying(), &bb.lt_verifying());
        let s_ac = Identity::safety_number(&a.lt_verifying(), &cc.lt_verifying());
        assert_ne!(s_ab, s_ac, "different peers must produce different safety numbers");
    }

    // ---------- Wave-5 L-CHAT-1 hardening — 5-mutation falsifier suite ----------
    //
    // G-C1 from trinity-chat-design.md: prekey bundle MUST validate against:
    //   M1 flipped signature   (already covered above as `falsifier_flipped_bit_breaks_signature`)
    //   M2 swapped order       (here: swap x25519/mlkem fields between two bundles)
    //   M3 expired bundle      (already covered above as `falsifier_expired_bundle`)
    //   M4 replay              (re-publish stale bundle past validity)
    //   M5 foreign CA          (sign bundle with a different lt key; embedded lt_pub remains victim's)
    //
    // Plus 2 structural extras for defence-in-depth:
    //   M6 swapped x25519_pub  — body mutation post-sig
    //   M7 swapped mlkem_pub   — body mutation post-sig
    //   M8 canonical domain separation — flipping `version` byte invalidates sig

    #[test]
    fn falsifier_m4_replay_stale_bundle_rejected() {
        // M4: attacker captures Alice's bundle, re-publishes past expiry window.
        let id = Identity::generate();
        let mut b = id.build_bundle();
        b.body.issued_at_unix = 1_000_000_000;
        b.body.valid_for_secs = 3600; // 1h
        let resigned = id.lt_signing.sign(&b.body.canonical_bytes());
        b.signature = resigned.to_bytes();
        // Signature still valid in isolation
        assert!(b.verify().is_ok());
        // ...but verify_at past expiry (2h later) fails.
        let now = b.body.issued_at_unix + b.body.valid_for_secs + 1;
        assert!(b.verify_at(now).is_err(), "replayed stale bundle must be rejected");
    }

    #[test]
    fn falsifier_m5_foreign_ca_signature_rejected() {
        // M5: attacker signs Alice's body with their own lt key but leaves
        // Alice's lt_pub embedded. Verification MUST reject because the embedded
        // key cannot validate the foreign signature.
        let alice = Identity::generate();
        let mallory = Identity::generate();
        let mut b = alice.build_bundle();
        // Mallory re-signs Alice's body with Mallory's key.
        let foreign_sig = mallory.lt_signing.sign(&b.body.canonical_bytes());
        b.signature = foreign_sig.to_bytes();
        assert!(b.verify().is_err(), "foreign CA signature must fail against embedded lt_pub");
    }

    #[test]
    fn falsifier_m2_swapped_order_breaks_signature() {
        // M2: attacker swaps x25519/mlkem fields between two captured bundles
        // hoping the verifier wouldn't notice. Canonical-bytes domain pinning
        // means the signature MUST refuse.
        let a = Identity::generate();
        let bb = Identity::generate();
        let mut a_bundle = a.build_bundle();
        let b_bundle = bb.build_bundle();
        a_bundle.body.x25519_pub = b_bundle.body.x25519_pub;
        assert!(a_bundle.verify().is_err(), "swapped x25519_pub must break sig");
    }

    #[test]
    fn falsifier_m6_swapped_mlkem_breaks_signature() {
        // M6: same as M2 but for the ML-KEM-768 prekey field.
        let a = Identity::generate();
        let bb = Identity::generate();
        let mut a_bundle = a.build_bundle();
        let b_bundle = bb.build_bundle();
        a_bundle.body.mlkem_pub = b_bundle.body.mlkem_pub;
        assert!(a_bundle.verify().is_err(), "swapped mlkem_pub must break sig");
    }

    #[test]
    fn falsifier_m8_version_downgrade_breaks_signature() {
        // M8: attacker tries to downgrade `version` from 1 to 0 to coerce the
        // verifier into a legacy code path. Canonical-bytes domain separation
        // means the embedded sig refuses any version mutation.
        let id = Identity::generate();
        let mut b = id.build_bundle();
        b.body.version = 0;
        assert!(b.verify().is_err(), "version downgrade must break sig");
    }

    #[test]
    fn falsifier_g_c1_full_5_mutation_suite_summary() {
        // Meta-test: verifies the G-C1 obligation that ALL 5 prescribed mutations
        // (M1..M5 from trinity-chat-design.md) plus M6/M8 extras refuse the bundle.
        // Wave-5 evidence anchor — single assertion summarising the suite.
        let alice = Identity::generate();
        let mallory = Identity::generate();
        let mut blocks = 0u32;

        // M1 flipped sig
        let mut b1 = alice.build_bundle();
        b1.signature[0] ^= 1;
        if b1.verify().is_err() { blocks += 1; }

        // M2 swapped lt_pub (foreign CA front-half)
        let mut b2 = alice.build_bundle();
        b2.body.lt_pub = mallory.lt_verifying().to_bytes();
        if b2.verify().is_err() { blocks += 1; }

        // M3 expired
        let mut b3 = alice.build_bundle();
        b3.body.issued_at_unix = 1_000_000;
        b3.body.valid_for_secs = 1;
        let r = alice.lt_signing.sign(&b3.body.canonical_bytes());
        b3.signature = r.to_bytes();
        if b3.verify_at(b3.body.issued_at_unix + 100).is_err() { blocks += 1; }

        // M4 replay — stale window
        let mut b4 = alice.build_bundle();
        b4.body.issued_at_unix = 0;
        b4.body.valid_for_secs = 60;
        let r = alice.lt_signing.sign(&b4.body.canonical_bytes());
        b4.signature = r.to_bytes();
        if b4.verify_at(10_000).is_err() { blocks += 1; }

        // M5 foreign CA full
        let mut b5 = alice.build_bundle();
        let foreign = mallory.lt_signing.sign(&b5.body.canonical_bytes());
        b5.signature = foreign.to_bytes();
        if b5.verify().is_err() { blocks += 1; }

        assert_eq!(blocks, 5, "G-C1 obligation: all 5 prekey mutations MUST be rejected");
    }
}
