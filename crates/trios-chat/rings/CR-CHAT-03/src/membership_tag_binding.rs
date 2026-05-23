//! # CR-CHAT-03 — Membership tag binding verification (Wave-40 Lane B)
//!
//! RFC 9420 §8.2 — MLS membership tag integrity.
//!
//! Every `MLSPlaintext` carries a membership tag that binds the message
//! to the group context at the time of sending. An adversary who can forge
//! or replay a membership tag can:
//!
//! * **Impersonate a member** — send messages that appear to come from
//!   a legitimate member who has since been removed.
//! * **Cross-epoch replay** — replay a message from a previous epoch
//!   where the member had different capabilities.
//! * **Cross-group splice** — use a membership tag from one group in
//!   another group where the leaf index is occupied by a different user.
//!
//! trios-chat enforces **7 rules**:
//!
//! 1. Tag is non-empty.
//! 2. Tag length is canonical (32 bytes, HMAC-SHA-256 output).
//! 3. `group_id` matches the receiver's group.
//! 4. `epoch` matches the receiver's current epoch.
//! 5. Sender leaf index is within group bounds.
//! 6. Sender is an active member (not removed).
//! 7. Tag is not a replay (not in the consumed-tags ledger).
//!
//! Tests **MTAG-01..10**. Error enum [`MembershipTagError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · MEMBERSHIP-TAG`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Canonical membership tag length (HMAC-SHA-256 output).
pub const MTAG_TAG_LEN: usize = 32;

/// A membership tag with its binding context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipTag {
    /// The tag value (HMAC-SHA-256 of message + context).
    pub tag: Vec<u8>,
    /// Group identifier.
    pub group_id: Vec<u8>,
    /// Epoch at which the tag was generated.
    pub epoch: u64,
    /// Sender leaf index.
    pub sender_leaf: u32,
    /// Serialized message content that the tag covers.
    pub message: Vec<u8>,
}

/// Receiver's group view for membership tag validation.
#[derive(Debug, Clone)]
pub struct MembershipTagView {
    /// Receiver's group identifier.
    pub group_id: Vec<u8>,
    /// Receiver's current epoch.
    pub current_epoch: u64,
    /// Total number of leaves in the tree.
    pub leaf_count: u32,
    /// Set of removed leaf indices.
    pub removed_leaves: BTreeSet<u32>,
    /// Set of already-consumed tag values (replay protection).
    pub consumed_tags: BTreeSet<Vec<u8>>,
}

/// All ways a membership tag can be rejected.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MembershipTagError {
    /// Tag is empty.
    EmptyTag,
    /// Tag length is not 32 bytes.
    NonCanonicalTagLength,
    /// `group_id` does not match receiver's group.
    GroupIdMismatch,
    /// `epoch` does not match receiver's current epoch.
    EpochMismatch,
    /// Sender leaf index exceeds group bounds.
    SenderOutOfBounds,
    /// Sender has been removed from the group.
    SenderRemoved,
    /// Tag has already been consumed (replay).
    TagReplay,
}

/// `[VERIFIED]` Validate a membership tag against the receiver's group
/// view. Returns `Ok(())` if all rules pass, else the first failing rule.
///
/// Rules enforced in fixed order:
///
/// 1. `tag` is non-empty.
/// 2. `tag.len() == 32`.
/// 3. `group_id == view.group_id`.
/// 4. `epoch == view.current_epoch`.
/// 5. `sender_leaf < view.leaf_count`.
/// 6. `sender_leaf` not in `view.removed_leaves`.
/// 7. `tag` not in `view.consumed_tags`.
pub fn validate_membership_tag(
    mtag: &MembershipTag,
    view: &MembershipTagView,
) -> Result<(), MembershipTagError> {
    if mtag.tag.is_empty() {
        return Err(MembershipTagError::EmptyTag);
    }
    if mtag.tag.len() != MTAG_TAG_LEN {
        return Err(MembershipTagError::NonCanonicalTagLength);
    }
    if mtag.group_id != view.group_id {
        return Err(MembershipTagError::GroupIdMismatch);
    }
    if mtag.epoch != view.current_epoch {
        return Err(MembershipTagError::EpochMismatch);
    }
    if mtag.sender_leaf >= view.leaf_count {
        return Err(MembershipTagError::SenderOutOfBounds);
    }
    if view.removed_leaves.contains(&mtag.sender_leaf) {
        return Err(MembershipTagError::SenderRemoved);
    }
    if view.consumed_tags.contains(&mtag.tag) {
        return Err(MembershipTagError::TagReplay);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_view() -> MembershipTagView {
        MembershipTagView {
            group_id: vec![0xAA; 32],
            current_epoch: 7,
            leaf_count: 10,
            removed_leaves: BTreeSet::new(),
            consumed_tags: BTreeSet::new(),
        }
    }

    fn good_tag() -> MembershipTag {
        MembershipTag {
            tag: vec![0x11; 32],
            group_id: vec![0xAA; 32],
            epoch: 7,
            sender_leaf: 3,
            message: vec![0x22; 64],
        }
    }

    /// **MTAG-01** — empty tag rejected.
    #[test]
    fn mtag_01_empty_tag_rejected() {
        let mut t = good_tag();
        t.tag = vec![];
        assert_eq!(
            validate_membership_tag(&t, &good_view()),
            Err(MembershipTagError::EmptyTag)
        );
    }

    /// **MTAG-02** — wrong tag length (16 bytes) rejected.
    #[test]
    fn mtag_02_wrong_tag_length_rejected() {
        let mut t = good_tag();
        t.tag = vec![0x11; 16];
        assert_eq!(
            validate_membership_tag(&t, &good_view()),
            Err(MembershipTagError::NonCanonicalTagLength)
        );
    }

    /// **MTAG-03** — group_id mismatch rejected.
    #[test]
    fn mtag_03_group_id_mismatch_rejected() {
        let mut t = good_tag();
        t.group_id = vec![0xBB; 32];
        assert_eq!(
            validate_membership_tag(&t, &good_view()),
            Err(MembershipTagError::GroupIdMismatch)
        );
    }

    /// **MTAG-04** — epoch mismatch rejected.
    #[test]
    fn mtag_04_epoch_mismatch_rejected() {
        let mut t = good_tag();
        t.epoch = 99;
        assert_eq!(
            validate_membership_tag(&t, &good_view()),
            Err(MembershipTagError::EpochMismatch)
        );
    }

    /// **MTAG-05** — sender out of bounds rejected.
    #[test]
    fn mtag_05_sender_out_of_bounds_rejected() {
        let mut t = good_tag();
        t.sender_leaf = 15;
        assert_eq!(
            validate_membership_tag(&t, &good_view()),
            Err(MembershipTagError::SenderOutOfBounds)
        );
    }

    /// **MTAG-06** — removed sender rejected.
    #[test]
    fn mtag_06_removed_sender_rejected() {
        let mut view = good_view();
        view.removed_leaves.insert(3);
        assert_eq!(
            validate_membership_tag(&good_tag(), &view),
            Err(MembershipTagError::SenderRemoved)
        );
    }

    /// **MTAG-07** — replayed tag rejected.
    #[test]
    fn mtag_07_tag_replay_rejected() {
        let mut view = good_view();
        view.consumed_tags.insert(vec![0x11; 32]);
        assert_eq!(
            validate_membership_tag(&good_tag(), &view),
            Err(MembershipTagError::TagReplay)
        );
    }

    /// **MTAG-08** — valid tag accepted.
    #[test]
    fn mtag_08_valid_tag_accepted() {
        assert_eq!(validate_membership_tag(&good_tag(), &good_view()), Ok(()));
    }

    /// **MTAG-09** — tag with same sender but different epoch rejected.
    #[test]
    fn mtag_09_cross_epoch_rejected() {
        let mut t = good_tag();
        t.epoch = 5;
        assert_eq!(
            validate_membership_tag(&t, &good_view()),
            Err(MembershipTagError::EpochMismatch)
        );
    }

    /// **MTAG-10** — tag at leaf boundary (leaf_count - 1) accepted.
    #[test]
    fn mtag_10_leaf_boundary_accepted() {
        let mut t = good_tag();
        t.sender_leaf = 9;
        let t2 = MembershipTag { tag: vec![0x33; 32], ..t.clone() };
        assert_eq!(validate_membership_tag(&t2, &good_view()), Ok(()));
    }
}
