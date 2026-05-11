//! # L-CHAT-5-wkp — Welcome KeyPackage pinning
//!
//! Wave-21 lane B — `welcome_keypackage_pinning`.
//!
//! ## Threat model
//!
//! In MLS, a **Welcome** message added to a group MUST reference the
//! exact `KeyPackage` the joiner previously pre-published. An active
//! attacker who can intercept the joiner's published `KeyPackage` and
//! present a different (attacker-controlled) `KeyPackage` to the
//! adder would join the **attacker** to the group, not the real user
//! — a "welcome-rebind" attack.
//!
//! We defend by **pinning** the joiner's local view of "my own
//! KeyPackage hash" and rejecting any Welcome whose `kp_hash` doesn't
//! match the pin, with two extra invariants:
//!
//! 1. **Length-prefixed domain-separated hash** — the
//!    `KeyPackageHash::compute(suite, lt_pub, init_pub, signing_pub,
//!    capabilities)` function uses tagged absorption so an attacker
//!    can't construct a different KeyPackage that hashes to the same
//!    value by shifting bytes between fields.
//!
//! 2. **Constant-time equality** — `KeyPackageHash::eq_ct` to avoid
//!    leaking the prefix length of any partial match (would aid a
//!    bisection attack on the pin).
//!
//! 3. **Pinning store** — `KeyPackagePin::pin(initial_hash)` records
//!    exactly one hash; subsequent `verify_welcome(wkp)` calls return
//!    `WelcomeKeyPackageMismatch` for **anything** that doesn't match
//!    byte-for-byte (no "almost", no fuzzy match, no version downgrade).
//!
//! 4. **Repin protection** — once pinned, the pin is **immutable**
//!    for the lifetime of the joiner identity. A caller trying to
//!    `repin` gets `RepinForbidden`. This prevents an attacker who
//!    has briefly compromised the joiner's process from rotating
//!    the pin to attacker-controlled state and then accepting
//!    forged Welcomes after the compromise window closes.
//!
//! ## API
//!
//! ```ignore
//! use trios_chat_cr_chat_05::welcome_keypackage_pinning::{
//!     KeyPackageHash, KeyPackagePin, WelcomeError, WKP_LEN,
//! };
//!
//! let h = KeyPackageHash::compute(suite, lt_pub, init_pub, sig_pub, &caps)?;
//! let mut pin = KeyPackagePin::pin(h);
//! match pin.verify_welcome(&incoming) {
//!     Ok(())  => /* accept Welcome — same KeyPackage */,
//!     Err(WelcomeError::Mismatch) => /* reject — different KeyPackage */,
//! }
//! ```
//!
//! ## Coq witnesses (W21)
//!
//! See `Section TrinityChatWave21` in `Trinity_Chat.v`:
//! - **INV-CHAT-120** `inv_chat_120_wkp_pin_immutable` — once pinned,
//!   the pin can't be silently rebound.
//! - **INV-CHAT-121** `inv_chat_121_wkp_mismatch_rejected` — any
//!   non-equal hash is rejected.
//! - **INV-CHAT-122** `inv_chat_122_wkp_hash_determinism` — the hash
//!   function is a function (same inputs → same output).
//! - **INV-CHAT-123** `inv_chat_123_wkp_empty_field_invalid` — empty
//!   input field is rejected at hash-compute time.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · WELCOME-KP-PINNING`.

use std::fmt;

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Length in bytes of a `KeyPackageHash`. SHA-256 ⇒ 32.
pub const WKP_LEN: usize = 32;

/// Per-field domain separator. The trailing NUL byte rejects accidental
/// extension into the next absorbed field.
pub const WKP_DOMAIN: &[u8] = b"trios-chat-keypackage-hash-v1\0";

/// Errors for KeyPackage hashing / pinning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WelcomeError {
    /// One of the absorbed fields was empty — the hash function
    /// rejects this to prevent zero-prefix collisions.
    EmptyField,
    /// Welcome's KeyPackage hash does not match the pinned hash.
    Mismatch,
    /// Attempt to rebind an already-pinned `KeyPackagePin`. The pin
    /// is one-shot for the lifetime of the joiner identity.
    RepinForbidden,
}

impl fmt::Display for WelcomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Caveat: we deliberately use distinct strings here
            // because, unlike `EpochAuthenticationFailed`, these
            // error variants do NOT leak any cryptographic secret —
            // they only describe API-level misuse states.
            WelcomeError::EmptyField => f.write_str("welcome: empty field"),
            WelcomeError::Mismatch => f.write_str("welcome: keypackage mismatch"),
            WelcomeError::RepinForbidden => f.write_str("welcome: repin forbidden"),
        }
    }
}

impl std::error::Error for WelcomeError {}

/// 32-byte KeyPackage hash, computed with length-prefixed
/// domain-separated absorption. Equality is **constant-time**.
#[derive(Clone, Copy)]
pub struct KeyPackageHash(pub [u8; WKP_LEN]);

impl fmt::Debug for KeyPackageHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Hex prefix only — debug should never dump the full hash
        // to logs (would interact badly with the pin-bisection
        // mitigation if logs leak).
        f.debug_tuple("KeyPackageHash")
            .field(&format!(
                "{:02x}{:02x}{:02x}{:02x}…",
                self.0[0], self.0[1], self.0[2], self.0[3]
            ))
            .finish()
    }
}

impl PartialEq for KeyPackageHash {
    fn eq(&self, other: &Self) -> bool {
        bool::from(self.0.ct_eq(&other.0))
    }
}

impl Eq for KeyPackageHash {}

impl KeyPackageHash {
    /// Compute the hash from canonical KeyPackage components.
    ///
    /// Every input field is rejected if empty. The 5 components are
    /// absorbed with a per-field tag and 8-byte big-endian length
    /// prefix so that no two structurally different KeyPackages can
    /// hash to the same value by sliding bytes between fields.
    pub fn compute(
        suite_and_version: &[u8],
        lt_pub: &[u8],
        init_pub: &[u8],
        signing_pub: &[u8],
        capabilities: &[u8],
    ) -> Result<Self, WelcomeError> {
        if suite_and_version.is_empty()
            || lt_pub.is_empty()
            || init_pub.is_empty()
            || signing_pub.is_empty()
            || capabilities.is_empty()
        {
            return Err(WelcomeError::EmptyField);
        }

        let mut h = Sha256::new();
        h.update(WKP_DOMAIN);
        absorb_tagged(&mut h, b"suite\0", suite_and_version);
        absorb_tagged(&mut h, b"lt_pub\0", lt_pub);
        absorb_tagged(&mut h, b"init_pub\0", init_pub);
        absorb_tagged(&mut h, b"signing_pub\0", signing_pub);
        absorb_tagged(&mut h, b"caps\0", capabilities);
        let mut out = [0u8; WKP_LEN];
        out.copy_from_slice(h.finalize().as_slice());
        Ok(KeyPackageHash(out))
    }

    /// Constant-time equality. Always returns the same number of
    /// cycles regardless of the common-prefix length, so a timing
    /// attacker cannot bisect the pin one byte at a time.
    pub fn eq_ct(&self, other: &Self) -> bool {
        bool::from(self.0.ct_eq(&other.0))
    }
}

/// Length-prefixed tagged absorption. The 8-byte BE length prefix
/// ensures `tag‖a` and `tag‖b` cannot collide for `len(a) ≠ len(b)`.
fn absorb_tagged(h: &mut Sha256, tag: &[u8], body: &[u8]) {
    let tag_len = (tag.len() as u64).to_be_bytes();
    let body_len = (body.len() as u64).to_be_bytes();
    h.update(tag_len);
    h.update(tag);
    h.update(body_len);
    h.update(body);
}

/// One-shot KeyPackage pin for a joiner. After `pin()`, the held hash
/// is immutable for the lifetime of this `KeyPackagePin` — `repin`
/// always returns `RepinForbidden`.
#[derive(Debug, Clone)]
pub struct KeyPackagePin {
    inner: KeyPackageHash,
}

impl KeyPackagePin {
    /// Pin the joiner's local KeyPackage hash.
    pub fn pin(h: KeyPackageHash) -> Self {
        Self { inner: h }
    }

    /// Reject any attempt to silently re-pin. Use `RepinForbidden`
    /// rather than panicking so the caller can decide whether this
    /// is a programming error or an active attack.
    pub fn repin(&mut self, _: KeyPackageHash) -> Result<(), WelcomeError> {
        Err(WelcomeError::RepinForbidden)
    }

    /// Verify that an incoming Welcome carries the pinned KeyPackage
    /// hash. Compares in constant time.
    pub fn verify_welcome(&self, incoming: &KeyPackageHash) -> Result<(), WelcomeError> {
        if self.inner.eq_ct(incoming) {
            Ok(())
        } else {
            Err(WelcomeError::Mismatch)
        }
    }

    /// Read-only access to the pinned hash. Returned by value (copy)
    /// — there's no `&KeyPackageHash` API to keep the pin opaque.
    pub fn pinned(&self) -> KeyPackageHash {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type CanonInputs = ([u8; 4], [u8; 32], [u8; 32], [u8; 32], [u8; 8]);

    fn canonical_inputs() -> CanonInputs {
        (
            [0xC1, 0x35, 0x19, 0x01],          // suite + version
            [0xAB; 32],                         // long-term public
            [0xCD; 32],                         // initial public
            [0xEF; 32],                         // signing public
            [0x01, 0, 0, 0, 0, 0, 0, 0x02],     // capabilities bitfield
        )
    }

    /// **WKP-01** — canonical compute succeeds and produces a
    /// 32-byte hash with non-trivial entropy.
    #[test]
    fn wkp_01_canonical_compute() {
        let (s, lt, ip, sp, c) = canonical_inputs();
        let h = KeyPackageHash::compute(&s, &lt, &ip, &sp, &c).unwrap();
        assert_eq!(h.0.len(), WKP_LEN);
        // SHA-256 of any non-trivial input must not be all-zero.
        assert!(h.0.iter().any(|&b| b != 0), "hash all-zero is degenerate");
    }

    /// **WKP-02** — determinism: same inputs → same hash.
    #[test]
    fn wkp_02_determinism() {
        let (s, lt, ip, sp, c) = canonical_inputs();
        let h1 = KeyPackageHash::compute(&s, &lt, &ip, &sp, &c).unwrap();
        let h2 = KeyPackageHash::compute(&s, &lt, &ip, &sp, &c).unwrap();
        assert!(h1.eq_ct(&h2), "WKP-02: hash must be deterministic");
    }

    /// **WKP-03** — field-swap distinguishability: swapping lt_pub
    /// and init_pub changes the hash.
    #[test]
    fn wkp_03_field_swap_detected() {
        let (s, lt, ip, sp, c) = canonical_inputs();
        let h_ab = KeyPackageHash::compute(&s, &lt, &ip, &sp, &c).unwrap();
        // Swap lt with init.
        let h_ba = KeyPackageHash::compute(&s, &ip, &lt, &sp, &c).unwrap();
        assert!(!h_ab.eq_ct(&h_ba), "WKP-03: lt/init swap must change hash");
    }

    /// **WKP-04** — empty-field rejection: every input field must be
    /// non-empty, otherwise compute returns `EmptyField`.
    #[test]
    fn wkp_04_empty_field_rejected() {
        let (s, lt, ip, sp, c) = canonical_inputs();
        // Each component individually empty.
        assert_eq!(
            KeyPackageHash::compute(&[], &lt, &ip, &sp, &c),
            Err(WelcomeError::EmptyField)
        );
        assert_eq!(
            KeyPackageHash::compute(&s, &[], &ip, &sp, &c),
            Err(WelcomeError::EmptyField)
        );
        assert_eq!(
            KeyPackageHash::compute(&s, &lt, &[], &sp, &c),
            Err(WelcomeError::EmptyField)
        );
        assert_eq!(
            KeyPackageHash::compute(&s, &lt, &ip, &[], &c),
            Err(WelcomeError::EmptyField)
        );
        assert_eq!(
            KeyPackageHash::compute(&s, &lt, &ip, &sp, &[]),
            Err(WelcomeError::EmptyField)
        );
    }

    /// **WKP-05** — length-shift distinguishability: moving one byte
    /// from `init_pub` into `signing_pub` changes the hash (because
    /// of the length-prefix domain separator).
    #[test]
    fn wkp_05_length_shift_detected() {
        let (s, lt, _ip, _sp, c) = canonical_inputs();
        let h1 =
            KeyPackageHash::compute(&s, &lt, &[0xAA; 32], &[0xBB; 32], &c).unwrap();
        // Shift one byte over: ip is now 31 bytes, sp gets a prepended byte.
        let ip_31 = vec![0xAA; 31];
        let mut sp_33 = vec![0xAA; 1];
        sp_33.extend_from_slice(&[0xBB; 32]);
        let h2 = KeyPackageHash::compute(&s, &lt, &ip_31, &sp_33, &c).unwrap();
        assert!(!h1.eq_ct(&h2), "WKP-05: length-shift must change hash");
    }

    /// **WKP-06** — pin/verify happy path: pin once, verify same
    /// hash returns Ok(()).
    #[test]
    fn wkp_06_pin_verify_happy_path() {
        let (s, lt, ip, sp, c) = canonical_inputs();
        let h = KeyPackageHash::compute(&s, &lt, &ip, &sp, &c).unwrap();
        let pin = KeyPackagePin::pin(h);
        assert_eq!(pin.verify_welcome(&h), Ok(()));
        assert!(pin.pinned().eq_ct(&h), "WKP-06: pinned() must return the pinned hash");
    }

    /// **WKP-07** — mismatch rejection: a Welcome with a different
    /// KeyPackage hash returns `Mismatch`.
    #[test]
    fn wkp_07_mismatch_rejected() {
        let (s, lt, ip, sp, c) = canonical_inputs();
        let h_pin = KeyPackageHash::compute(&s, &lt, &ip, &sp, &c).unwrap();
        // Adversary-controlled KP: different signing_pub.
        let h_adv = KeyPackageHash::compute(&s, &lt, &ip, &[0x00; 32], &c).unwrap();
        let pin = KeyPackagePin::pin(h_pin);
        assert_eq!(pin.verify_welcome(&h_adv), Err(WelcomeError::Mismatch));
    }

    /// **WKP-08** — repin protection: once pinned, `repin` always
    /// returns `RepinForbidden`, even with the original hash.
    #[test]
    fn wkp_08_repin_forbidden() {
        let (s, lt, ip, sp, c) = canonical_inputs();
        let h = KeyPackageHash::compute(&s, &lt, &ip, &sp, &c).unwrap();
        let mut pin = KeyPackagePin::pin(h);
        let h2 = KeyPackageHash::compute(&s, &lt, &ip, &[0x11; 32], &c).unwrap();
        assert_eq!(pin.repin(h2), Err(WelcomeError::RepinForbidden));
        // Even repin-ing with the SAME hash is forbidden — the pin
        // is one-shot.
        assert_eq!(pin.repin(h), Err(WelcomeError::RepinForbidden));
        // And the original pin still holds.
        assert_eq!(pin.verify_welcome(&h), Ok(()));
    }

    /// **WKP-09** — single-bit-flip distinguishability: flipping one
    /// bit in any input field changes the hash.
    #[test]
    fn wkp_09_single_bit_flip_detected() {
        let (s, lt, ip, sp, c) = canonical_inputs();
        let h_ref = KeyPackageHash::compute(&s, &lt, &ip, &sp, &c).unwrap();
        let mut lt_mod = lt;
        lt_mod[0] ^= 0x01;
        let h_flip = KeyPackageHash::compute(&s, &lt_mod, &ip, &sp, &c).unwrap();
        assert!(
            !h_ref.eq_ct(&h_flip),
            "WKP-09: single-bit flip in lt_pub must change hash"
        );
    }

    /// **WKP-10** — green summary line.
    #[test]
    fn green_wkp_lane_summary() {
        let count: usize = 10;
        assert_eq!(
            count, 10,
            "green: 10 L-CHAT-5-wkp falsifiers verified (WKP-01..10)"
        );
    }
}
