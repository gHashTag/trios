//! # CR-CHAT-05 · L-CHAT-5-grpext — MLS GroupContext extensions consistency
//!
//! `[VERIFIED]` Wave-29 lane B — Defends against a class of attacks
//! where an adversary mutates the `extensions` field inside a
//! `GroupContext` between successive epochs, or where a member silently
//! drops a previously-required extension (RFC 9420 §8.1 — Group Context,
//! §11 — GroupContextExtensions proposal, §12.1 — Required Capabilities):
//!
//! * **Required-extension drop** — a `RequiredCapabilities` extension
//!   listed in the receiver's `required_extensions` mask is missing
//!   from the inbound `GroupContext.extensions` (RFC 9420 §12.1.1:
//!   every member MUST advertise every required extension).
//! * **Forbidden-extension injection** — the inbound `extensions`
//!   contains an extension ID that the receiver has explicitly placed
//!   on its `forbidden_extensions` deny-list (operator policy; e.g.
//!   the receiver refuses the `external_pub` extension because
//!   external-init is disabled for this group).
//! * **Cross-group extension splice** — the inbound `group_id` does
//!   not match the receiver's `local_group_id`; even if the extension
//!   set is identical, the binding is wrong.
//! * **Stale-epoch extension snapshot** — the inbound `epoch` is
//!   strictly less than the receiver's `current_epoch`; an attacker
//!   could otherwise resurrect a past extension set after the group
//!   has moved on (e.g. re-enable a removed capability).
//! * **Reserved extension-ID forge** — an extension whose `id` falls
//!   inside the IANA reserved range (RFC 9420 §17.4 — IDs 0x0000 and
//!   0xF000..0xFFFF are reserved) is present in the inbound payload.
//! * **Duplicate extension ID** — the inbound `extensions` contains
//!   two entries with the same `id`; RFC 9420 §6.1 (Extensions
//!   structure) demands a unique-ID set so the receiver cannot be
//!   confused by which copy is authoritative.
//!
//! See RFC 9420 §8.1 (GroupContext structure), §11 (the
//! `GroupContextExtensions` proposal), §12.1 (Required Capabilities
//! semantics), §17.4 (IANA reserved IDs). The six rules below are
//! enforced in fixed order; any attempt to weaken or skip them
//! produces a `GroupContextExtensionsError`.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · MLS-GROUP-CONTEXT-EXTENSIONS`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Lowest extension ID reserved by IANA (RFC 9420 §17.4 — ID 0x0000
/// is reserved as the "unallocated" sentinel).
pub const RESERVED_EXTENSION_ID_LOW: u16 = 0x0000;

/// Highest extension ID reserved by IANA (RFC 9420 §17.4 — IDs from
/// 0xF000..=0xFFFF are reserved for Private Use; the canonical guard
/// refuses ANY use of those IDs over the wire).
pub const RESERVED_EXTENSION_ID_HIGH_START: u16 = 0xF000;

/// One extension entry as it appears inside a `GroupContext`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExtensionEntry {
    /// IANA-assigned extension ID (RFC 9420 §17.4).
    pub id: u16,
    /// Opaque extension payload (we do not parse it here — the guard
    /// is structural).
    pub payload: Vec<u8>,
}

/// One inbound `GroupContext` snapshot the receiver must validate
/// before accepting any Commit that depends on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupContextSnapshot {
    /// Local group identifier the snapshot is bound to.
    pub group_id: Vec<u8>,
    /// Epoch the snapshot claims to be valid for.
    pub epoch: u64,
    /// Extension set (RFC 9420 §8.1: `extensions` field of
    /// `GroupContext`).
    pub extensions: Vec<ExtensionEntry>,
}

/// Receiving-side view used to validate a `GroupContextSnapshot`.
#[derive(Debug, Clone)]
pub struct GroupContextExtensionsView {
    /// `group_id` of the local group.
    pub local_group_id: Vec<u8>,
    /// Current epoch the receiver has accepted.
    pub current_epoch: u64,
    /// Extension IDs the receiver REQUIRES every inbound snapshot to
    /// carry (RFC 9420 §12.1.1 `RequiredCapabilities`).
    pub required_extensions: BTreeSet<u16>,
    /// Extension IDs the receiver has explicitly DENIED (operator
    /// policy / threat-model lock).
    pub forbidden_extensions: BTreeSet<u16>,
}

/// Why the GroupContext-extensions guard rejected a snapshot.
/// Variants are in the same fixed order as the rules in
/// [`validate_group_context_extensions`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupContextExtensionsError {
    /// `snapshot.group_id != view.local_group_id`.
    CrossGroupSplice,
    /// `snapshot.epoch < view.current_epoch`.
    StaleEpochSnapshot,
    /// An extension ID falls inside the IANA reserved range.
    ReservedExtensionIdForge,
    /// Two entries in `snapshot.extensions` share the same `id`.
    DuplicateExtensionId,
    /// A required extension ID is missing from `snapshot.extensions`.
    RequiredExtensionDropped(u16),
    /// A forbidden extension ID is present in `snapshot.extensions`.
    ForbiddenExtensionInjected(u16),
}

/// Validate the extensions field of an inbound `GroupContext`
/// snapshot. Enforces the six rules from RFC 9420 §8.1 + §12.1 in
/// fixed order.
pub fn validate_group_context_extensions(
    snapshot: &GroupContextSnapshot,
    view: &GroupContextExtensionsView,
) -> Result<(), GroupContextExtensionsError> {
    // Rule 1 — group_id binding.
    // Coq: Trinity_Chat.v::gcx_cross_group_splice
    if snapshot.group_id != view.local_group_id {
        return Err(GroupContextExtensionsError::CrossGroupSplice);
    }
    // Rule 2 — epoch must NOT be strictly less than current.
    // Coq: Trinity_Chat.v::gcx_stale_epoch_snapshot
    if snapshot.epoch < view.current_epoch {
        return Err(GroupContextExtensionsError::StaleEpochSnapshot);
    }
    // Rule 3 — no reserved IDs.
    // Coq: Trinity_Chat.v::gcx_reserved_extension_id_forge
    for ext in &snapshot.extensions {
        if ext.id == RESERVED_EXTENSION_ID_LOW
            || ext.id >= RESERVED_EXTENSION_ID_HIGH_START
        {
            return Err(GroupContextExtensionsError::ReservedExtensionIdForge);
        }
    }
    // Rule 4 — duplicate ID detection.
    // Coq: Trinity_Chat.v::gcx_duplicate_extension_id
    let mut seen: BTreeSet<u16> = BTreeSet::new();
    for ext in &snapshot.extensions {
        if !seen.insert(ext.id) {
            return Err(GroupContextExtensionsError::DuplicateExtensionId);
        }
    }
    // Rule 5 — every required ID must appear.
    // Coq: Trinity_Chat.v::gcx_required_extension_dropped
    for required_id in &view.required_extensions {
        if !seen.contains(required_id) {
            return Err(GroupContextExtensionsError::RequiredExtensionDropped(
                *required_id,
            ));
        }
    }
    // Rule 6 — no forbidden ID may appear.
    // Coq: Trinity_Chat.v::gcx_forbidden_extension_injected (operational).
    for ext in &snapshot.extensions {
        if view.forbidden_extensions.contains(&ext.id) {
            return Err(
                GroupContextExtensionsError::ForbiddenExtensionInjected(ext.id),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ext(id: u16) -> ExtensionEntry {
        ExtensionEntry {
            id,
            payload: vec![0xAA, 0xBB, 0xCC],
        }
    }

    fn good_snapshot() -> GroupContextSnapshot {
        GroupContextSnapshot {
            group_id: b"trinity-chat-room".to_vec(),
            epoch: 7,
            // 0x0001 = required_capabilities, 0x0002 = ratchet_tree,
            // 0x0003 = application_id (canonical MLS set).
            extensions: vec![ext(0x0001), ext(0x0002), ext(0x0003)],
        }
    }

    fn good_view() -> GroupContextExtensionsView {
        GroupContextExtensionsView {
            local_group_id: b"trinity-chat-room".to_vec(),
            current_epoch: 7,
            required_extensions: [0x0001u16, 0x0002].into_iter().collect(),
            forbidden_extensions: BTreeSet::new(),
        }
    }

    /// **GCX-01** — cross-group splice rejected.
    #[test]
    fn gcx_01_cross_group_splice_rejected() {
        let mut s = good_snapshot();
        s.group_id = b"other-room".to_vec();
        assert_eq!(
            validate_group_context_extensions(&s, &good_view()),
            Err(GroupContextExtensionsError::CrossGroupSplice)
        );
    }

    /// **GCX-02** — stale-epoch snapshot rejected.
    #[test]
    fn gcx_02_stale_epoch_snapshot_rejected() {
        let mut s = good_snapshot();
        s.epoch = 3;
        assert_eq!(
            validate_group_context_extensions(&s, &good_view()),
            Err(GroupContextExtensionsError::StaleEpochSnapshot)
        );
    }

    /// **GCX-03** — low-reserved extension ID (0x0000) rejected.
    #[test]
    fn gcx_03_low_reserved_id_rejected() {
        let mut s = good_snapshot();
        s.extensions.push(ext(0x0000));
        assert_eq!(
            validate_group_context_extensions(&s, &good_view()),
            Err(GroupContextExtensionsError::ReservedExtensionIdForge)
        );
    }

    /// **GCX-04** — high-reserved extension ID (0xF000+) rejected.
    #[test]
    fn gcx_04_high_reserved_id_rejected() {
        let mut s = good_snapshot();
        s.extensions.push(ext(0xF123));
        assert_eq!(
            validate_group_context_extensions(&s, &good_view()),
            Err(GroupContextExtensionsError::ReservedExtensionIdForge)
        );
    }

    /// **GCX-05** — duplicate extension ID rejected.
    #[test]
    fn gcx_05_duplicate_extension_id_rejected() {
        let mut s = good_snapshot();
        // Re-add 0x0001 to trigger duplicate detection.
        s.extensions.push(ext(0x0001));
        assert_eq!(
            validate_group_context_extensions(&s, &good_view()),
            Err(GroupContextExtensionsError::DuplicateExtensionId)
        );
    }

    /// **GCX-06** — required extension dropped rejected.
    #[test]
    fn gcx_06_required_extension_dropped_rejected() {
        let mut s = good_snapshot();
        // Drop 0x0001 (required_capabilities).
        s.extensions.retain(|e| e.id != 0x0001);
        assert_eq!(
            validate_group_context_extensions(&s, &good_view()),
            Err(GroupContextExtensionsError::RequiredExtensionDropped(0x0001))
        );
    }

    /// **GCX-07** — forbidden extension injected rejected.
    #[test]
    fn gcx_07_forbidden_extension_injected_rejected() {
        let s = {
            let mut s = good_snapshot();
            // Inject 0x0042 then deny it in the view.
            s.extensions.push(ext(0x0042));
            s
        };
        let v = {
            let mut v = good_view();
            v.forbidden_extensions.insert(0x0042);
            v
        };
        assert_eq!(
            validate_group_context_extensions(&s, &v),
            Err(GroupContextExtensionsError::ForbiddenExtensionInjected(0x0042))
        );
    }

    /// **GCX-08** — valid current-epoch snapshot accepted.
    #[test]
    fn gcx_08_valid_current_epoch_accepted() {
        assert_eq!(
            validate_group_context_extensions(&good_snapshot(), &good_view()),
            Ok(())
        );
    }

    /// **GCX-09** — valid next-epoch snapshot accepted.
    #[test]
    fn gcx_09_valid_next_epoch_accepted() {
        let mut s = good_snapshot();
        s.epoch = 8;
        assert_eq!(
            validate_group_context_extensions(&s, &good_view()),
            Ok(())
        );
    }

    /// **GCX-10** — empty `required_extensions` accepts any well-formed
    /// snapshot (the guard does not invent requirements).
    #[test]
    fn gcx_10_empty_required_accepts_minimal_snapshot() {
        let mut v = good_view();
        v.required_extensions.clear();
        let s = GroupContextSnapshot {
            group_id: b"trinity-chat-room".to_vec(),
            epoch: 7,
            extensions: vec![],
        };
        assert_eq!(validate_group_context_extensions(&s, &v), Ok(()));
    }
}
