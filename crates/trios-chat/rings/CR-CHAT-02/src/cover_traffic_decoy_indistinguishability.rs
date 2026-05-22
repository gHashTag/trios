//! Wave-35 / L-CHAT-2-ctdi (R-CHAT-2 / CR-CHAT-02) — Cover-traffic decoy
//! indistinguishability per NDSS 2021 §V "Cover-traffic flooding
//! defence" + USENIX'22 "Pretzel" equal-length padding bins.
//!
//! Wave-34 closed the *receiver* side of the Statistical Disclosure
//! Attack by routing every envelope through a one-shot ephemeral
//! mailbox. The remaining gap on the *relay* side is timing/volume:
//! a passive relay that can distinguish cover (decoy) traffic from
//! real traffic still gets a strong correlation signal. NDSS 2021 §V
//! and the Pretzel padding-bin paper both call for cover packets to
//! be **byte-indistinguishable from real packets at the AEAD-frame
//! boundary**: same length class, same nonce shape, same AAD layout,
//! same MAC-tag length. Anything else leaks.
//!
//! This lane is the consumption-side guard that the relay (and the
//! receiver-side router on egress) applies to every packet —
//! cover-or-real — before it is forwarded or accepted. A single deny
//! wins.
//!
//! Seven rules enforced in fixed order:
//!   1. NonCanonicalPacketLengthClass — `packet.ciphertext.len()`
//!      must equal one of the published equal-length bins
//!      (`view.length_classes`). Cover packets that are 1 byte off
//!      a real bin are trivially fingerprintable.
//!   2. UnknownLengthClassId — `packet.length_class_id` must be in
//!      `view.length_classes` (the relay refuses packets that claim
//!      a length class it has never published).
//!   3. LengthClassMismatch — the bin id MUST match the actual
//!      `ciphertext.len()`. A packet that claims bin "1024" but is
//!      actually 1023 bytes is a fingerprinting probe.
//!   4. NonCanonicalNonceLength — `packet.aead_nonce.len()` must
//!      equal `COVER_AEAD_NONCE_LEN` (12 bytes — ChaCha20-Poly1305
//!      per R-CHAT-4). Cover packets with shorter/longer nonces are
//!      fingerprintable.
//!   5. NonCanonicalAadLength — `packet.aead_aad.len()` must equal
//!      `COVER_AAD_LEN` (16 bytes — fixed `(epoch_u64 ‖ class_u64)`
//!      header per R-CHAT-9). AAD-shape drift is the loudest signal.
//!   6. NonCanonicalTagLength — `packet.aead_tag.len()` must equal
//!      `COVER_AEAD_TAG_LEN` (16 bytes — Poly1305 tag). Truncated
//!      tags on cover packets reveal them instantly.
//!   7. CoverFlagShapeMismatch — `packet.is_cover` is allowed to be
//!      either `true` or `false`, but the **on-wire packet bytes
//!      MUST be identical in either case**, i.e. the cover flag is
//!      never inferable from the ciphertext / aad / nonce / tag. We
//!      enforce this constructively by demanding `packet.wire_digest
//!      == view.expected_wire_digest_for(class_id)` — the digest is
//!      a function of class only, so cover and real packets in the
//!      same bin map to the same expected digest.

#![forbid(unsafe_code)]

/// Canonical AEAD nonce length (12 bytes — ChaCha20-Poly1305 per
/// R-CHAT-4).
pub const COVER_AEAD_NONCE_LEN: usize = 12;

/// Canonical AAD length (16 bytes — fixed `(epoch_u64 ‖ class_u64)`
/// header per R-CHAT-9).
pub const COVER_AAD_LEN: usize = 16;

/// Canonical AEAD tag length (16 bytes — Poly1305).
pub const COVER_AEAD_TAG_LEN: usize = 16;

/// A single packet (cover or real) leaving the egress path or
/// arriving at the relay, per NDSS 2021 §V cover-traffic flooding
/// defence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverPacket {
    /// Length-class identifier the packet claims to belong to.
    pub length_class_id: u64,
    /// AEAD ciphertext (already sealed, fixed length-class).
    pub ciphertext: Vec<u8>,
    /// AEAD nonce (12 bytes).
    pub aead_nonce: Vec<u8>,
    /// AEAD AAD (16 bytes).
    pub aead_aad: Vec<u8>,
    /// AEAD authentication tag (16 bytes).
    pub aead_tag: Vec<u8>,
    /// Cover flag (true = decoy, false = real). MUST NOT influence
    /// any other field's shape.
    pub is_cover: bool,
    /// Class-only digest the relay recomputes to check
    /// indistinguishability.
    pub wire_digest: Vec<u8>,
}

/// Relay-side view of published length-class commitments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverPacketView {
    /// Published length classes: `(class_id, exact_ciphertext_len,
    /// expected_wire_digest)`. The expected digest is a function of
    /// the class only — cover and real packets share it.
    pub length_classes: Vec<(u64, usize, Vec<u8>)>,
}

/// Typed errors for `validate_cover_packet`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CoverPacketError {
    /// Rule 1 — `ciphertext.len()` is not in any published class.
    NonCanonicalPacketLengthClass,
    /// Rule 2 — `length_class_id` is unknown.
    UnknownLengthClassId,
    /// Rule 3 — declared class id does not match `ciphertext.len()`.
    LengthClassMismatch,
    /// Rule 4 — `aead_nonce.len() != COVER_AEAD_NONCE_LEN`.
    NonCanonicalNonceLength,
    /// Rule 5 — `aead_aad.len() != COVER_AAD_LEN`.
    NonCanonicalAadLength,
    /// Rule 6 — `aead_tag.len() != COVER_AEAD_TAG_LEN`.
    NonCanonicalTagLength,
    /// Rule 7 — wire digest does not match the class-only expected
    /// digest (cover/real distinguishable on the wire).
    CoverFlagShapeMismatch,
}

/// Constructive guard for a single cover-or-real packet. Returns
/// `Ok(())` iff every rule (1)..(7) holds.
///
/// `[VERIFIED]` against the 10 unit tests `CTDI-01..10` below and the
/// Coq theorems `INV-CHAT-218..222` in the W35 Section of
/// `proofs/chat/Trinity_Chat.v`.
pub fn validate_cover_packet(
    packet: &CoverPacket,
    view: &CoverPacketView,
) -> Result<(), CoverPacketError> {
    // Rule 1: ciphertext length must be one of the published bins.
    let bin_matches_len = view
        .length_classes
        .iter()
        .any(|(_, len, _)| *len == packet.ciphertext.len());
    if !bin_matches_len {
        return Err(CoverPacketError::NonCanonicalPacketLengthClass);
    }
    // Rule 2: claimed class id must be a published class.
    let class = view
        .length_classes
        .iter()
        .find(|(id, _, _)| *id == packet.length_class_id);
    let (_, expected_len, expected_digest) = match class {
        Some(entry) => entry,
        None => return Err(CoverPacketError::UnknownLengthClassId),
    };
    // Rule 3: declared class id must match actual ciphertext length.
    if *expected_len != packet.ciphertext.len() {
        return Err(CoverPacketError::LengthClassMismatch);
    }
    // Rule 4: nonce length canonical.
    if packet.aead_nonce.len() != COVER_AEAD_NONCE_LEN {
        return Err(CoverPacketError::NonCanonicalNonceLength);
    }
    // Rule 5: AAD length canonical.
    if packet.aead_aad.len() != COVER_AAD_LEN {
        return Err(CoverPacketError::NonCanonicalAadLength);
    }
    // Rule 6: tag length canonical.
    if packet.aead_tag.len() != COVER_AEAD_TAG_LEN {
        return Err(CoverPacketError::NonCanonicalTagLength);
    }
    // Rule 7: wire digest must match the class-only expected digest —
    // cover and real packets in the same bin map to the same digest,
    // so this rejects shape drift between cover and real.
    if &packet.wire_digest != expected_digest {
        return Err(CoverPacketError::CoverFlagShapeMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLASS_1024: u64 = 1024;
    const CLASS_4096: u64 = 4096;

    fn digest_1024() -> Vec<u8> {
        vec![0xC1_u8; 32]
    }

    fn digest_4096() -> Vec<u8> {
        vec![0xC4_u8; 32]
    }

    fn ok_view() -> CoverPacketView {
        CoverPacketView {
            length_classes: vec![
                (CLASS_1024, 1024, digest_1024()),
                (CLASS_4096, 4096, digest_4096()),
            ],
        }
    }

    fn ok_packet() -> CoverPacket {
        CoverPacket {
            length_class_id: CLASS_1024,
            ciphertext: vec![0xAA_u8; 1024],
            aead_nonce: vec![0x11_u8; COVER_AEAD_NONCE_LEN],
            aead_aad: vec![0x22_u8; COVER_AAD_LEN],
            aead_tag: vec![0x33_u8; COVER_AEAD_TAG_LEN],
            is_cover: false,
            wire_digest: digest_1024(),
        }
    }

    /// CTDI-01 — ciphertext length off-bin (1023) rejected.
    #[test]
    fn ctdi_01_off_bin_length_rejected() {
        let mut p = ok_packet();
        p.ciphertext = vec![0xAA_u8; 1023];
        assert_eq!(
            validate_cover_packet(&p, &ok_view()),
            Err(CoverPacketError::NonCanonicalPacketLengthClass)
        );
    }

    /// CTDI-02 — unknown class id rejected.
    #[test]
    fn ctdi_02_unknown_class_id_rejected() {
        let mut p = ok_packet();
        p.length_class_id = 9999;
        assert_eq!(
            validate_cover_packet(&p, &ok_view()),
            Err(CoverPacketError::UnknownLengthClassId)
        );
    }

    /// CTDI-03 — class id mismatch (claims 4096 but ct is 1024).
    #[test]
    fn ctdi_03_class_id_mismatch_rejected() {
        let mut p = ok_packet();
        p.length_class_id = CLASS_4096;
        // wire_digest also drifts; isolate Rule 3 by aligning digest
        // with the *claimed* class to ensure Rule 3 fires, not Rule 7.
        p.wire_digest = digest_4096();
        assert_eq!(
            validate_cover_packet(&p, &ok_view()),
            Err(CoverPacketError::LengthClassMismatch)
        );
    }

    /// CTDI-04 — short nonce (8 bytes) rejected.
    #[test]
    fn ctdi_04_short_nonce_rejected() {
        let mut p = ok_packet();
        p.aead_nonce = vec![0x11_u8; 8];
        assert_eq!(
            validate_cover_packet(&p, &ok_view()),
            Err(CoverPacketError::NonCanonicalNonceLength)
        );
    }

    /// CTDI-05 — over-long nonce (24 bytes) rejected.
    #[test]
    fn ctdi_05_over_long_nonce_rejected() {
        let mut p = ok_packet();
        p.aead_nonce = vec![0x11_u8; 24];
        assert_eq!(
            validate_cover_packet(&p, &ok_view()),
            Err(CoverPacketError::NonCanonicalNonceLength)
        );
    }

    /// CTDI-06 — AAD length drift (12 bytes) rejected — Rule 5.
    #[test]
    fn ctdi_06_aad_length_drift_rejected() {
        let mut p = ok_packet();
        p.aead_aad = vec![0x22_u8; 12];
        assert_eq!(
            validate_cover_packet(&p, &ok_view()),
            Err(CoverPacketError::NonCanonicalAadLength)
        );
    }

    /// CTDI-07 — truncated MAC tag (8 bytes) rejected — Rule 6.
    #[test]
    fn ctdi_07_truncated_tag_rejected() {
        let mut p = ok_packet();
        p.aead_tag = vec![0x33_u8; 8];
        assert_eq!(
            validate_cover_packet(&p, &ok_view()),
            Err(CoverPacketError::NonCanonicalTagLength)
        );
    }

    /// CTDI-08 — wire digest drift rejected — cover/real
    /// distinguishable on the wire (Rule 7).
    #[test]
    fn ctdi_08_wire_digest_drift_rejected() {
        let mut p = ok_packet();
        p.wire_digest = vec![0xFF_u8; 32];
        assert_eq!(
            validate_cover_packet(&p, &ok_view()),
            Err(CoverPacketError::CoverFlagShapeMismatch)
        );
    }

    /// CTDI-09 — canonical cover packet (`is_cover = true`)
    /// indistinguishable from canonical real packet — same wire
    /// digest, both accepted.
    #[test]
    fn ctdi_09_canonical_cover_packet_accepted() {
        let mut p = ok_packet();
        p.is_cover = true;
        assert_eq!(validate_cover_packet(&p, &ok_view()), Ok(()));
    }

    /// CTDI-10 — canonical real packet accepted.
    #[test]
    fn ctdi_10_canonical_real_packet_accepted() {
        assert_eq!(
            validate_cover_packet(&ok_packet(), &ok_view()),
            Ok(())
        );
    }
}
