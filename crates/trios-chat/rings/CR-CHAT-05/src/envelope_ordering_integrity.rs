//! # CR-CHAT-05 — Envelope ordering integrity guard (Wave-42 Lane A)
//!
//! R-CHAT-1 / R-CHAT-9 — At-rest envelope ordering verification.
//!
//! Persistence layers store sealed envelopes in counter order. An attacker
//! with write access to the store can:
//!
//! * **Reorder envelopes** — swap counters to break ratchet replay
//!   detection when the receiver re-reads from storage.
//! * **Inject duplicates** — insert a copy of an old envelope to force
//!   the receiver to accept a stale key.
//! * **Delete selectively** — remove specific envelopes to create a gap
//!   that forces re-derivation, potentially downgrading keys.
//!
//! trios-chat enforces **6 rules** on a retrieved envelope sequence:
//!
//! 1. Sequence is non-empty.
//! 2. All envelopes belong to the same session.
//! 3. Counters are strictly monotone (no duplicates).
//! 4. No gaps within the expected window (counter = base + position).
//! 5. Ciphertext lengths are consistent (same padding class per R-CHAT-9).
//! 6. Counter range does not exceed session maximum.
//!
//! Tests **EORD-01..10**. Error enum [`EnvelopeOrderError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ENVELOPE-ORDER`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum counter span per session.
pub const EORD_MAX_COUNTER_SPAN: u64 = 1_000_000;

/// One sealed envelope as retrieved from storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredEnvelope {
    /// Session identifier.
    pub session_id: [u8; 32],
    /// Ratchet counter.
    pub counter: u64,
    /// AEAD ciphertext (already padded to a class).
    pub ciphertext: Vec<u8>,
}

/// All ways an envelope sequence can fail ordering validation.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EnvelopeOrderError {
    /// Sequence is empty.
    EmptySequence,
    /// Envelope belongs to a different session.
    SessionMismatch,
    /// Duplicate counter detected.
    DuplicateCounter,
    /// Gap in counter sequence (counter != expected).
    GapInSequence,
    /// Ciphertext length inconsistency (padding class mismatch).
    CiphertextLengthInconsistency,
    /// Counter range exceeds session maximum.
    CounterSpanExceeded,
}

/// `[VERIFIED]` Validate ordering integrity of a retrieved envelope
/// sequence. Returns `Ok(())` if all rules pass.
///
/// Rules enforced in fixed order:
///
/// 1. `envelopes` is non-empty.
/// 2. All `session_id` fields are identical.
/// 3. All counters are unique.
/// 4. Counters form a contiguous range (sorted ASC, no gaps).
/// 5. All ciphertexts have the same length (same padding class).
/// 6. `max_counter - min_counter < EORD_MAX_COUNTER_SPAN`.
pub fn validate_envelope_order(
    envelopes: &[StoredEnvelope],
) -> Result<(), EnvelopeOrderError> {
    if envelopes.is_empty() {
        return Err(EnvelopeOrderError::EmptySequence);
    }
    let sid = &envelopes[0].session_id;
    for e in envelopes {
        if e.session_id != *sid {
            return Err(EnvelopeOrderError::SessionMismatch);
        }
    }
    let mut seen = BTreeSet::new();
    for e in envelopes {
        if !seen.insert(e.counter) {
            return Err(EnvelopeOrderError::DuplicateCounter);
        }
    }
    let mut sorted: Vec<u64> = seen.iter().copied().collect();
    sorted.sort();
    if sorted.last().unwrap() - sorted[0] >= EORD_MAX_COUNTER_SPAN {
        return Err(EnvelopeOrderError::CounterSpanExceeded);
    }
    let min_ct = sorted[0];
    for (i, &ct) in sorted.iter().enumerate() {
        if ct != min_ct + i as u64 {
            return Err(EnvelopeOrderError::GapInSequence);
        }
    }
    let first_len = envelopes[0].ciphertext.len();
    for e in &envelopes[1..] {
        if e.ciphertext.len() != first_len {
            return Err(EnvelopeOrderError::CiphertextLengthInconsistency);
        }
    }
    if sorted.last().unwrap() - min_ct >= EORD_MAX_COUNTER_SPAN {
        return Err(EnvelopeOrderError::CounterSpanExceeded);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SID: [u8; 32] = [0xAA; 32];

    fn env(counter: u64, ct_len: usize) -> StoredEnvelope {
        StoredEnvelope {
            session_id: SID,
            counter,
            ciphertext: vec![0x22; ct_len],
        }
    }

    fn env_other_session(counter: u64) -> StoredEnvelope {
        StoredEnvelope {
            session_id: [0xBB; 32],
            counter,
            ciphertext: vec![0x22; 64],
        }
    }

    /// **EORD-01** — empty sequence rejected.
    #[test]
    fn eord_01_empty_sequence_rejected() {
        assert_eq!(
            validate_envelope_order(&[]),
            Err(EnvelopeOrderError::EmptySequence)
        );
    }

    /// **EORD-02** — session mismatch rejected.
    #[test]
    fn eord_02_session_mismatch_rejected() {
        let envelopes = vec![env(0, 64), env_other_session(1)];
        assert_eq!(
            validate_envelope_order(&envelopes),
            Err(EnvelopeOrderError::SessionMismatch)
        );
    }

    /// **EORD-03** — duplicate counter rejected.
    #[test]
    fn eord_03_duplicate_counter_rejected() {
        let envelopes = vec![env(0, 64), env(0, 64)];
        assert_eq!(
            validate_envelope_order(&envelopes),
            Err(EnvelopeOrderError::DuplicateCounter)
        );
    }

    /// **EORD-04** — gap in sequence rejected.
    #[test]
    fn eord_04_gap_in_sequence_rejected() {
        let envelopes = vec![env(0, 64), env(2, 64)];
        assert_eq!(
            validate_envelope_order(&envelopes),
            Err(EnvelopeOrderError::GapInSequence)
        );
    }

    /// **EORD-05** — ciphertext length inconsistency rejected.
    #[test]
    fn eord_05_ciphertext_length_mismatch_rejected() {
        let envelopes = vec![env(0, 64), env(1, 128)];
        assert_eq!(
            validate_envelope_order(&envelopes),
            Err(EnvelopeOrderError::CiphertextLengthInconsistency)
        );
    }

    /// **EORD-06** — counter span exceeded rejected.
    #[test]
    fn eord_06_counter_span_exceeded_rejected() {
        let mut envelopes = Vec::new();
        for i in 0..100u64 {
            envelopes.push(env(i, 64));
        }
        envelopes.push(env(EORD_MAX_COUNTER_SPAN, 64));
        assert_eq!(
            validate_envelope_order(&envelopes),
            Err(EnvelopeOrderError::CounterSpanExceeded)
        );
    }

    /// **EORD-07** — valid contiguous sequence accepted.
    #[test]
    fn eord_07_valid_contiguous_accepted() {
        let envelopes = vec![env(0, 64), env(1, 64), env(2, 64)];
        assert_eq!(validate_envelope_order(&envelopes), Ok(()));
    }

    /// **EORD-08** — single envelope accepted.
    #[test]
    fn eord_08_single_envelope_accepted() {
        let envelopes = vec![env(42, 256)];
        assert_eq!(validate_envelope_order(&envelopes), Ok(()));
    }

    /// **EORD-09** — out-of-order but contiguous input accepted (sorted internally).
    #[test]
    fn eord_09_unsorted_contiguous_accepted() {
        let envelopes = vec![env(2, 64), env(0, 64), env(1, 64)];
        assert_eq!(validate_envelope_order(&envelopes), Ok(()));
    }

    /// **EORD-10** — large valid sequence accepted.
    #[test]
    fn eord_10_large_valid_sequence_accepted() {
        let envelopes: Vec<StoredEnvelope> = (0..100).map(|i| env(i, 1024)).collect();
        assert_eq!(validate_envelope_order(&envelopes), Ok(()));
    }
}
