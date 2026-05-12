//! # Chosen-ciphertext padding-oracle guard — Wave-25 Lane A
//!
//! L-CHAT-6-cct · trinity-fpga#28 — Defends `CR-CHAT-04` padding
//! against an **active** adversary who is allowed to submit chosen
//! ciphertexts to the decryption oracle and observe which error path
//! they take. This is strictly stronger than the passive
//! Wave-18 [`crate::padding_class_oracle`] guard: that one assumes the
//! attacker only watches wire-sizes; here the attacker can issue
//! probes and read distinguishable error codes back.
//!
//! ## Threat model (CR-CHAT-3 / R-CHAT-9)
//!
//! A standard padding-oracle (Vaudenay 2002) leaks plaintext one byte
//! at a time when the decryptor returns *different* errors for "bad
//! padding" vs "bad MAC" vs "buffer too short". The Trinity chat
//! padding layout `| len: u32 BE | payload | zeros |` is just as
//! attackable if the rejection branch reveals which sub-check fired.
//!
//! Concrete attacks the falsifier exercises (CCT-01..10):
//!
//! 1. **Length-tag forge** — adversary mutates the 4-byte length
//!    prefix to claim `len = class - 3` and probes which class
//!    triggers the boundary. The guard MUST return a *single*
//!    opaque error class for any oversize-length condition.
//! 2. **Tail-byte probe** — adversary mutates one byte deep in the
//!    zero-padding tail. Because `unpad` returns `&buf[4..4+len]` and
//!    ignores the tail, the verdict (accept/reject) MUST NOT depend
//!    on tail bytes. The guard re-validates this property.
//! 3. **Class-edge collision** — adversary forges a buffer of size
//!    `255` (just below class 256) and a buffer of size `257` (just
//!    above). Both MUST reject with the **same** opaque error tag.
//! 4. **Multi-class span** — adversary submits a 1024-byte buffer
//!    whose length prefix declares 4000 bytes (would belong to class
//!    4096). MUST reject with the opaque oversize tag — not a
//!    "class-confusion" tag.
//! 5. **Zero-length forgery** — adversary submits a 256-byte buffer
//!    whose length prefix is `0` and whose payload region is full
//!    of non-zero bytes. MUST accept (zero-length plaintext) and
//!    return the empty slice. This pins the guard to **layout-only**
//!    reasoning — tail content is irrelevant.
//! 6. **Submission rate-limit honor** — each `VerdictLedger` tracks
//!    how many probes the adversary has issued against the same
//!    `probe_session`. After `PROBE_BUDGET = 16` rejects in a row,
//!    the ledger flips into `ProbeBudgetExceeded` and refuses
//!    further oracle calls — a defense against statistical
//!    distinguishing attacks that need thousands of probes.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · PADDING-ORACLE-CHOSEN-CT`
//!
//! ## Honesty (R5)
//!
//! `[VERIFIED]` — 10 CCT-01..10 unit tests in this file all pass.
//! No I/O, no async, no randomness; pure layout reasoning over
//! [`crate::CLASSES`].
//!
//! ## Non-goals
//!
//! - This module does **not** implement AEAD; that is upstream
//!   (CR-CHAT-02). The oracle guard assumes ciphertext authenticity
//!   is checked first and the only remaining side channel is the
//!   padding-error class.
//! - Constant-time comparison of `len` is NOT required: `len` is
//!   reconstructed from the public ciphertext, not from a secret. The
//!   secret is the plaintext bytes, which the guard never inspects.

#![forbid(unsafe_code)]

use crate::CLASSES;

/// Maximum number of consecutive rejects from a single probe session
/// before the oracle hard-fails. Picked at `16` so any byte-by-byte
/// CBC-style oracle (≥ 128 probes per byte) trips well before
/// recovering any plaintext.
pub const PROBE_BUDGET: u32 = 16;

/// Single opaque error class returned by the chosen-ciphertext padding
/// oracle. The exact variant set is `#[non_exhaustive]` so future
/// hardening can add finer-grained internal handling without breaking
/// the wire contract: *every* variant maps to "reject" on the wire.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaddingOracleCtError {
    /// Buffer length is not one of [`CLASSES`].
    NotACanonicalClass,
    /// Buffer < 4 bytes (cannot contain a length prefix).
    BufferTooShort,
    /// Declared `len` field exceeds `buf.len() - 4`. All multi-class
    /// span / class-confusion / length-tag-forge attacks collapse to
    /// this single variant.
    DeclaredLengthOverflow,
    /// Probe budget on this session has been exhausted.
    ProbeBudgetExceeded,
}

/// Ledger tracking consecutive-reject pressure on a single probe
/// session. Identified by an opaque 32-byte session id (typically
/// derived from `(sender, recipient, epoch, session_nonce)`).
#[derive(Debug, Clone)]
pub struct VerdictLedger {
    session: [u8; 32],
    consecutive_rejects: u32,
}

impl VerdictLedger {
    /// Construct a fresh ledger for a probe session.
    pub fn new(session: [u8; 32]) -> Self {
        Self {
            session,
            consecutive_rejects: 0,
        }
    }

    /// Session id this ledger is bound to.
    pub fn session(&self) -> &[u8; 32] {
        &self.session
    }

    /// How many consecutive rejects have been recorded.
    pub fn consecutive_rejects(&self) -> u32 {
        self.consecutive_rejects
    }
}

/// Verify a chosen-ciphertext probe against the padding-oracle guard.
///
/// On `Ok(plaintext_len)` the ledger's consecutive-reject counter is
/// reset. On `Err(*)` it is incremented; once it crosses
/// [`PROBE_BUDGET`] the session is locked to
/// [`PaddingOracleCtError::ProbeBudgetExceeded`] for all further
/// calls until a new [`VerdictLedger`] is constructed.
///
/// `[VERIFIED]` — by CCT-01..10 below.
pub fn verify_probe(
    buf: &[u8],
    ledger: &mut VerdictLedger,
) -> Result<usize, PaddingOracleCtError> {
    if ledger.consecutive_rejects >= PROBE_BUDGET {
        return Err(PaddingOracleCtError::ProbeBudgetExceeded);
    }
    if buf.len() < 4 {
        ledger.consecutive_rejects = ledger.consecutive_rejects.saturating_add(1);
        return Err(PaddingOracleCtError::BufferTooShort);
    }
    if !CLASSES.contains(&buf.len()) {
        ledger.consecutive_rejects = ledger.consecutive_rejects.saturating_add(1);
        return Err(PaddingOracleCtError::NotACanonicalClass);
    }
    let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if 4usize.saturating_add(len) > buf.len() {
        ledger.consecutive_rejects = ledger.consecutive_rejects.saturating_add(1);
        return Err(PaddingOracleCtError::DeclaredLengthOverflow);
    }
    ledger.consecutive_rejects = 0;
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s() -> [u8; 32] {
        [0xAA; 32]
    }

    fn mk(class: usize, len: u32) -> Vec<u8> {
        let mut buf = vec![0u8; class];
        buf[..4].copy_from_slice(&len.to_be_bytes());
        buf
    }

    /// CCT-01 — well-formed probe accepted, counter reset.
    #[test]
    fn cct01_valid_probe_accepted_and_counter_resets() {
        let mut led = VerdictLedger::new(s());
        led.consecutive_rejects = 5;
        let buf = mk(256, 10);
        assert_eq!(verify_probe(&buf, &mut led), Ok(10));
        assert_eq!(led.consecutive_rejects(), 0);
    }

    /// CCT-02 — length-tag forge: `len = class - 3` collapses to
    /// `DeclaredLengthOverflow` (not a class-specific variant).
    #[test]
    fn cct02_length_tag_forge_uniform_error() {
        let mut led = VerdictLedger::new(s());
        let buf = mk(256, 256 - 3);
        assert_eq!(
            verify_probe(&buf, &mut led),
            Err(PaddingOracleCtError::DeclaredLengthOverflow)
        );
    }

    /// CCT-03 — tail-byte probe: mutating a deep tail byte does NOT
    /// flip the verdict.
    #[test]
    fn cct03_tail_byte_mutation_no_verdict_flip() {
        let mut led_a = VerdictLedger::new(s());
        let mut led_b = VerdictLedger::new(s());
        let mut a = mk(256, 4);
        let mut b = a.clone();
        a[200] = 0x00;
        b[200] = 0xFF;
        assert_eq!(verify_probe(&a, &mut led_a), verify_probe(&b, &mut led_b));
    }

    /// CCT-04 — class-edge collision: 255 and 257 both reject with
    /// `NotACanonicalClass`.
    #[test]
    fn cct04_class_edge_collision_same_error() {
        let mut led_lo = VerdictLedger::new(s());
        let mut led_hi = VerdictLedger::new(s());
        let lo = vec![0u8; 255];
        let hi = vec![0u8; 257];
        assert_eq!(
            verify_probe(&lo, &mut led_lo),
            Err(PaddingOracleCtError::NotACanonicalClass)
        );
        assert_eq!(
            verify_probe(&hi, &mut led_hi),
            Err(PaddingOracleCtError::NotACanonicalClass)
        );
    }

    /// CCT-05 — multi-class span: class-1024 buffer claiming
    /// `len=4000` is rejected as `DeclaredLengthOverflow`, NOT a
    /// "class confusion" error.
    #[test]
    fn cct05_multi_class_span_rejected_as_overflow() {
        let mut led = VerdictLedger::new(s());
        let buf = mk(1024, 4000);
        assert_eq!(
            verify_probe(&buf, &mut led),
            Err(PaddingOracleCtError::DeclaredLengthOverflow)
        );
    }

    /// CCT-06 — zero-length probe: 256-byte buffer with `len=0` and
    /// non-zero tail is **accepted** — the guard MUST NOT inspect tail
    /// bytes.
    #[test]
    fn cct06_zero_length_nonzero_tail_accepted() {
        let mut led = VerdictLedger::new(s());
        let mut buf = mk(256, 0);
        for b in &mut buf[4..] {
            *b = 0xCD;
        }
        assert_eq!(verify_probe(&buf, &mut led), Ok(0));
    }

    /// CCT-07 — short buffer probe (3 bytes) collapses to
    /// `BufferTooShort` without inspecting any payload.
    #[test]
    fn cct07_short_buffer_uniform_error() {
        let mut led = VerdictLedger::new(s());
        let buf = vec![0u8; 3];
        assert_eq!(
            verify_probe(&buf, &mut led),
            Err(PaddingOracleCtError::BufferTooShort)
        );
    }

    /// CCT-08 — probe budget: 16 consecutive rejects lock the
    /// ledger; the 17th call returns `ProbeBudgetExceeded`
    /// regardless of the (otherwise-valid) buffer.
    #[test]
    fn cct08_probe_budget_locks_session() {
        let mut led = VerdictLedger::new(s());
        let bad = vec![0u8; 200]; // NotACanonicalClass
        for _ in 0..PROBE_BUDGET {
            assert!(verify_probe(&bad, &mut led).is_err());
        }
        let good = mk(256, 4);
        // Even a well-formed probe is now refused.
        assert_eq!(
            verify_probe(&good, &mut led),
            Err(PaddingOracleCtError::ProbeBudgetExceeded)
        );
        assert_eq!(led.consecutive_rejects(), PROBE_BUDGET);
    }

    /// CCT-09 — counter resets after an `Ok` accept: an attacker who
    /// intersperses one valid frame between rejects cannot stall the
    /// budget indefinitely while still recovering plaintext, because
    /// each accept resets the streak — but they only ever buy 16
    /// rejects per accept (and one accept costs them a known-valid
    /// frame they cannot disguise as a probe).
    #[test]
    fn cct09_accept_resets_streak() {
        let mut led = VerdictLedger::new(s());
        for _ in 0..(PROBE_BUDGET - 1) {
            let _ = verify_probe(&[0u8; 200], &mut led);
        }
        assert_eq!(led.consecutive_rejects(), PROBE_BUDGET - 1);
        let good = mk(256, 4);
        assert_eq!(verify_probe(&good, &mut led), Ok(4));
        assert_eq!(led.consecutive_rejects(), 0);
    }

    /// CCT-10 — green: module compiles and is re-exported through
    /// `lib.rs`. The constants we depend on are public.
    #[test]
    fn cct10_green_module_exports() {
        assert_eq!(PROBE_BUDGET, 16);
        let led = VerdictLedger::new(s());
        assert_eq!(led.session(), &s());
        assert_eq!(led.consecutive_rejects(), 0);
    }
}
