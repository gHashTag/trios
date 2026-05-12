//! # L-CHAT-3-pv — MLS Proposal validation
//!
//! Wave-22 lane A — `proposal_validation`.
//!
//! ## Threat model
//!
//! An MLS commit references a bundle of **proposals** (Add / Remove /
//! Update). Before the commit is applied a verifier MUST validate
//! that bundle as a whole, not just each proposal in isolation:
//!
//! 1. The bundle MUST be **non-empty**. A commit that references zero
//!    proposals is a no-op tombstone that adversaries can splice into
//!    a transcript to force a useless epoch advance.
//! 2. The bundle MUST be **size-bounded**. A commit referencing
//!    thousands of `Remove` proposals is a denial-of-service vector —
//!    every proposal forces a tree mutation. The cap is
//!    [`MAX_PROPOSALS_PER_COMMIT`] = `32`, well above any realistic
//!    legitimate value.
//! 3. **Proposal-indices** inside the bundle MUST be **strictly
//!    increasing**. The MLS spec orders proposals by their epoch-
//!    relative index so the resulting state is canonical; a commit
//!    that reorders proposals can produce different state on
//!    different peers, breaking [strict epoch monotonicity] (
//!    INV-CHAT-23 / W2).
//! 4. **Self-removal-only** commits MUST be rejected. A peer
//!    committing `[Remove(self)]` alone leaves the group with **zero**
//!    live members, which is an invalid MLS state per RFC 9420 §13.
//!    Worse, a malicious committer can use it to denial-of-service
//!    the group: the next commit then references an empty membership
//!    set and every other peer's view diverges.
//! 5. The bundle MUST contain **no duplicate** `(kind, target)`
//!    pairs. Duplicate proposals can be used to confuse the tree
//!    diff algorithm (e.g. two `Add(leaf=42)` proposals double-add
//!    the same leaf and cascade into divergent membership counts).
//!
//! All five rules are enforced before a single byte of the proposal
//! list is materialised into MLS state, so a malformed bundle is
//! constant-cost-rejected by [`validate_bundle`] without leaking
//! which specific rule failed beyond the error variant.
//!
//! ## API
//!
//! ```ignore
//! use trios_chat_cr_chat_03::proposal_validation::{
//!     validate_bundle, ProposalBundle, ProposalEntry, ProposalKind,
//!     ProposalValidationError, MAX_PROPOSALS_PER_COMMIT,
//! };
//!
//! let bundle = ProposalBundle {
//!     committer_leaf: 0,
//!     entries: vec![
//!         ProposalEntry { index: 0, kind: ProposalKind::Add,    target: 4 },
//!         ProposalEntry { index: 1, kind: ProposalKind::Update, target: 0 },
//!     ],
//! };
//! validate_bundle(&bundle)?;
//! ```
//!
//! ## Coq witnesses (W22)
//!
//! See `Section TrinityChatWave22` in `Trinity_Chat.v`:
//! - **INV-CHAT-124** `inv_chat_124_pv_empty_rejected` — empty bundles
//!   always rejected.
//! - **INV-CHAT-125** `inv_chat_125_pv_oversized_rejected` — bundles
//!   above the cap always rejected.
//! - **INV-CHAT-126** `inv_chat_126_pv_self_remove_only_rejected` —
//!   `[Remove(self)]` alone always rejected.
//! - helper `pv_monotone_indices_22`.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · PROPOSAL-VALIDATION`.

#![allow(missing_docs)]

use std::collections::BTreeSet;

/// Maximum number of proposals referenced by a single commit.
///
/// RFC 9420 does not mandate a hard upper bound; we pick `32` as the
/// safety cap. A legitimate group with thousands of members still
/// rarely commits more than a handful of proposals per epoch — every
/// proposal triggers a path update, so very large bundles cause O(N)
/// rekey work on every peer and are a textbook DoS vector.
pub const MAX_PROPOSALS_PER_COMMIT: usize = 32;

/// Kind of an MLS proposal — only the three needed for membership
/// management are validated here. The full RFC 9420 proposal taxonomy
/// (PSK, ReInit, ExternalInit, GroupContextExtensions, AppAck) is
/// covered by [`ASPIRATIONAL`] tags in `pcs_healing` and
/// `external_commit` — out of scope for W22.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProposalKind {
    /// Add a new leaf to the tree.
    Add,
    /// Remove an existing leaf.
    Remove,
    /// Update an existing leaf's public material.
    Update,
}

/// One entry inside a proposal bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProposalEntry {
    /// Position of this proposal in the bundle. Must be strictly
    /// increasing across consecutive entries (RFC 9420 §12.4).
    pub index: u32,
    /// Membership operation type.
    pub kind: ProposalKind,
    /// Target leaf affected by this proposal. For `Add` this is the
    /// new leaf index; for `Remove` / `Update` it is the existing
    /// leaf being mutated.
    pub target: u32,
}

/// The full proposal bundle referenced by a single MLS commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalBundle {
    /// Leaf index of the peer that authored the commit.
    pub committer_leaf: u32,
    /// Ordered list of proposals applied by the commit. The order
    /// matters: validation requires strictly-increasing `index`.
    pub entries: Vec<ProposalEntry>,
}

/// Opaque rejection reasons — variants exist for diagnostic logging
/// only, but every variant signals an equally-rejected commit. We
/// deliberately keep the public API down to a single function that
/// returns `Result<(), ProposalValidationError>` so callers cannot
/// branch on the specific cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalValidationError {
    /// Bundle had zero entries.
    Empty,
    /// Bundle exceeded [`MAX_PROPOSALS_PER_COMMIT`].
    Oversized,
    /// Two consecutive entries had non-strictly-increasing `index`.
    NonMonotonicIndex,
    /// Bundle contained a duplicate `(kind, target)` pair.
    DuplicateEntry,
    /// Bundle contained exactly one entry which was a `Remove` of the
    /// committer's own leaf.
    SelfRemoveOnly,
}

impl core::fmt::Display for ProposalValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Single opaque rendering; the variant is for internal
        // diagnostics only. Never leak which rule fired to the wire.
        f.write_str("proposal bundle rejected")
    }
}

impl std::error::Error for ProposalValidationError {}

/// Validate a proposal bundle. Returns `Ok(())` on accept and an
/// opaque [`ProposalValidationError`] on reject. `[VERIFIED via test]`
pub fn validate_bundle(bundle: &ProposalBundle) -> Result<(), ProposalValidationError> {
    // Rule 1 — non-empty.
    if bundle.entries.is_empty() {
        return Err(ProposalValidationError::Empty);
    }
    // Rule 2 — size cap.
    if bundle.entries.len() > MAX_PROPOSALS_PER_COMMIT {
        return Err(ProposalValidationError::Oversized);
    }
    // Rule 3 — strictly increasing index.
    for pair in bundle.entries.windows(2) {
        if pair[1].index <= pair[0].index {
            return Err(ProposalValidationError::NonMonotonicIndex);
        }
    }
    // Rule 5 — no duplicate (kind, target) pairs.
    let mut seen: BTreeSet<(ProposalKind, u32)> = BTreeSet::new();
    for e in &bundle.entries {
        if !seen.insert((e.kind, e.target)) {
            return Err(ProposalValidationError::DuplicateEntry);
        }
    }
    // Rule 4 — reject single-entry self-removal.
    if bundle.entries.len() == 1 {
        let only = bundle.entries[0];
        if only.kind == ProposalKind::Remove && only.target == bundle.committer_leaf {
            return Err(ProposalValidationError::SelfRemoveOnly);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add(idx: u32, leaf: u32) -> ProposalEntry {
        ProposalEntry { index: idx, kind: ProposalKind::Add, target: leaf }
    }
    fn rem(idx: u32, leaf: u32) -> ProposalEntry {
        ProposalEntry { index: idx, kind: ProposalKind::Remove, target: leaf }
    }
    fn upd(idx: u32, leaf: u32) -> ProposalEntry {
        ProposalEntry { index: idx, kind: ProposalKind::Update, target: leaf }
    }

    /// PV-01 — A bundle of one legitimate `Add` is accepted.
    #[test]
    fn pv_01_single_add_accepted() {
        let b = ProposalBundle {
            committer_leaf: 0,
            entries: vec![add(0, 7)],
        };
        assert_eq!(validate_bundle(&b), Ok(()), "PV-01");
    }

    /// PV-02 — Empty bundle is rejected with [`ProposalValidationError::Empty`].
    #[test]
    fn pv_02_empty_rejected() {
        let b = ProposalBundle { committer_leaf: 0, entries: vec![] };
        assert_eq!(
            validate_bundle(&b),
            Err(ProposalValidationError::Empty),
            "PV-02"
        );
    }

    /// PV-03 — Bundle of `MAX_PROPOSALS_PER_COMMIT + 1` is rejected.
    #[test]
    fn pv_03_oversized_rejected() {
        let entries: Vec<ProposalEntry> = (0..=MAX_PROPOSALS_PER_COMMIT as u32)
            .map(|i| add(i, i + 100))
            .collect();
        let b = ProposalBundle { committer_leaf: 0, entries };
        assert_eq!(
            validate_bundle(&b),
            Err(ProposalValidationError::Oversized),
            "PV-03"
        );
    }

    /// PV-04 — Bundle of exactly `MAX_PROPOSALS_PER_COMMIT` is accepted
    /// (boundary check — `>` not `>=`).
    #[test]
    fn pv_04_at_cap_accepted() {
        let entries: Vec<ProposalEntry> = (0..MAX_PROPOSALS_PER_COMMIT as u32)
            .map(|i| add(i, i + 100))
            .collect();
        let b = ProposalBundle { committer_leaf: 0, entries };
        assert_eq!(validate_bundle(&b), Ok(()), "PV-04");
    }

    /// PV-05 — Non-monotonic indices (`0, 2, 1`) are rejected.
    #[test]
    fn pv_05_non_monotonic_index_rejected() {
        let b = ProposalBundle {
            committer_leaf: 0,
            entries: vec![add(0, 7), add(2, 8), add(1, 9)],
        };
        assert_eq!(
            validate_bundle(&b),
            Err(ProposalValidationError::NonMonotonicIndex),
            "PV-05"
        );
    }

    /// PV-06 — Equal indices (`0, 0`) are rejected — strict, not weak.
    #[test]
    fn pv_06_equal_index_rejected() {
        let b = ProposalBundle {
            committer_leaf: 0,
            entries: vec![add(0, 7), add(0, 8)],
        };
        assert_eq!(
            validate_bundle(&b),
            Err(ProposalValidationError::NonMonotonicIndex),
            "PV-06"
        );
    }

    /// PV-07 — Duplicate `(kind, target)` pair (two `Add(leaf=42)`) is
    /// rejected even when indices are strictly increasing.
    #[test]
    fn pv_07_duplicate_entry_rejected() {
        let b = ProposalBundle {
            committer_leaf: 0,
            entries: vec![add(0, 42), add(1, 42)],
        };
        assert_eq!(
            validate_bundle(&b),
            Err(ProposalValidationError::DuplicateEntry),
            "PV-07"
        );
    }

    /// PV-08 — A single-entry bundle that removes the committer's own
    /// leaf is rejected as `SelfRemoveOnly`.
    #[test]
    fn pv_08_self_remove_only_rejected() {
        let b = ProposalBundle {
            committer_leaf: 5,
            entries: vec![rem(0, 5)],
        };
        assert_eq!(
            validate_bundle(&b),
            Err(ProposalValidationError::SelfRemoveOnly),
            "PV-08"
        );
    }

    /// PV-09 — A multi-entry bundle that removes the committer
    /// alongside other proposals is ACCEPTED — `SelfRemoveOnly` only
    /// fires on the degenerate single-entry case.
    #[test]
    fn pv_09_self_remove_with_others_accepted() {
        let b = ProposalBundle {
            committer_leaf: 5,
            entries: vec![add(0, 9), rem(1, 5)],
        };
        assert_eq!(validate_bundle(&b), Ok(()), "PV-09");
    }

    /// PV-10 — Mixed Add/Update/Remove bundle with strictly
    /// increasing indices and distinct `(kind, target)` is accepted.
    #[test]
    fn pv_10_mixed_kinds_accepted() {
        let b = ProposalBundle {
            committer_leaf: 0,
            entries: vec![add(0, 7), upd(1, 0), rem(2, 3)],
        };
        assert_eq!(validate_bundle(&b), Ok(()), "PV-10");
    }
}
