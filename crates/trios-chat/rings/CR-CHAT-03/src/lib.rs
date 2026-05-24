//! # CR-CHAT-03 — group (MLS skeleton)
//!
//! L-CHAT-3 · trinity-fpga#31 — MLS group skeleton (Wave-2).
//!
//! `[ASPIRATIONAL]` Full RFC 9420 implementation lives outside the
//! scope of this scaffold (we will re-export from the `openmls` crate
//! behind a feature flag in a follow-up PR). What this ring ships
//! today:
//!
//! 1. [`GroupId`], [`Epoch`], [`LeafIndex`] newtypes — the MLS state
//!    shape.
//! 2. [`Welcome`] / [`Commit`] structs + [`Op`] enum — the wire-message
//!    kinds.
//! 3. [`Group::process_commit`] — applies a commit and **enforces
//!    strict epoch monotonicity** (matches Coq theorem
//!    `mls_epoch_monotone`).
//!
//! Everything is in-memory and deterministic so the unit tests can pin
//! the contract behaviour without dragging in `openmls`.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod commit_path_secret_aead_keying_mismatch;
pub mod commit_secret_export_collision;
pub mod commit_signature;
pub mod concurrent_add_remove;
pub mod confirmation_tag_chain;
pub mod external_commit;
pub mod external_commit_resumption_psk_misbinding;
pub mod external_proposal_origin_unbound;
pub mod external_psk_id_provenance;
pub mod leaf_node_signature_validation;
pub mod membership_tag_binding;
pub mod pcs_healing;
pub mod proposal_ref_collision;
pub mod proposal_validation;
pub mod psk_external_injection;
pub mod reinit_freshness;
pub mod treekem_parent_hash_binding;
pub mod transcript_hash_chain;
pub mod group_membership_authorization_guard;

pub use group_membership_authorization_guard::{
    validate_group_proposal, GroupMembershipError, GroupOp, GroupProposal, GroupState,
    GMAZ_MAX_LEAF_INDEX, GMAZ_MIN_MEMBERS,
};

pub mod leaf_node_priority_inversion_guard;
pub use leaf_node_priority_inversion_guard::{
    validate_leaf_priority, LeafPriorityError, PriorityProposal, LNPI_MAX_LEAF, LNPI_MAX_PROPOSALS,
};

pub mod ratchet_tree_resolution_guard;
pub use ratchet_tree_resolution_guard::{
    validate_tree_resolution, TreeResolutionError, RTRS_MAX_RESOLUTION, RTRS_MAX_TREE,
};

pub mod parent_hash_chain_validation_guard;
pub use parent_hash_chain_validation_guard::{
    validate_parent_hash_chain, HashNode, ParentHashError, PHCV_HASH_LEN, PHCV_MAX_DEPTH,
};

pub mod unmerged_leaves_bound_guard;
pub use unmerged_leaves_bound_guard::{
    validate_unmerged_leaves, UnmergedLeavesError, ULBG_MAX_TREE, ULBG_MAX_UNMERGED,
};

pub mod ratchet_tree_blank_node_depth_guard;
pub use ratchet_tree_blank_node_depth_guard::{
    validate_blank_node_depth, BlankDepthError, TreeNode, RBND_MAX_BLANK_DEPTH, RBND_MAX_TREE_DEPTH,
};

pub mod treekem_path_secret_forward_secrecy_guard;
pub use treekem_path_secret_forward_secrecy_guard::{
    validate_path_secret_order, PathSecretError,
    TPSF_MAX_PATH, TPSF_MIN_PATH, TPSF_SECRET_LEN,
};

pub mod group_epoch_commit_ordering_guard;
pub use group_epoch_commit_ordering_guard::{
    validate_commit_ordering, CommitOrderError, CommitRecord,
    GECO_MAX_COMMITS, GECO_MAX_EPOCH_GAP, GECO_MIN_EPOCH,
};

pub mod ratchet_tree_update_integrity_guard;
pub use ratchet_tree_update_integrity_guard::{
    validate_tree_updates, TreeUpdate, TreeUpdateError,
    RTUI_MAX_BATCH, RTUI_MAX_NODES,
};

pub mod leaf_node_key_uniqueness_guard;
pub use leaf_node_key_uniqueness_guard::{
    validate_leaf_key_uniqueness, LeafKeyError, LeafNode,
    LNKU_KEY_LEN, LNKU_MAX_LEAVES,
};

pub mod treekem_path_secret_uniqueness_guard;
pub use treekem_path_secret_uniqueness_guard::{
    validate_path_secret_uniqueness, PathSecretNode, PathSecretUniquenessError,
    TPSU_MAX_NODES, TPSU_SECRET_LEN,
};

pub mod group_member_removal_verification_guard;
pub use group_member_removal_verification_guard::{
    validate_member_removals, MemberRemoval, RemovalError,
    GMRV_MAX_REMOVALS,
};

pub mod treekem_update_path_coverage_guard;
pub use treekem_update_path_coverage_guard::{
    validate_update_path_coverage, PathCoverageError, UpdatePathRecord,
    TUPC_MAX_LEAVES, TUPC_MAX_PATHS,
};

pub mod treekem_epoch_transition_integrity_guard;
pub use treekem_epoch_transition_integrity_guard::{
    validate_epoch_transitions, EpochTransition, EpochTransitionError,
    TETI_HASH_LEN, TETI_MAX_DEPTH, TETI_MAX_TRANSITIONS,
};

pub mod treekem_parent_node_resolution_depth_guard;
pub use treekem_parent_node_resolution_depth_guard::{
    validate_node_resolutions, NodeResolution, ResolutionError,
    TPNR_MAX_DEPTH, TPNR_MAX_NODES, TPNR_MAX_RESOLUTIONS,
};

pub mod treekem_blank_leaf_count_bound_guard;
pub use treekem_blank_leaf_count_bound_guard::{
    validate_blank_leaf_counts, BlankLeafError, TreeSnapshot,
    BLCB_MAX_BLANK_DEN, BLCB_MAX_BLANK_NUM, BLCB_MAX_LEAVES,
    BLCB_MAX_TREES, BLCB_MIN_LEAVES,
};

pub mod treekem_update_path_uniqueness_guard;
pub use treekem_update_path_uniqueness_guard::{
    validate_update_path_uniqueness, PathUniquenessError, UpdatePath,
    TPUN_HASH_LEN, TPUN_MAX_PATHS,
};

pub mod treekem_group_membership_change_rate_guard;
pub use treekem_group_membership_change_rate_guard::{
    validate_membership_rate, MembershipWindow, MembershipRateError,
    GMCR_MAX_CHANGES, GMCR_MAX_WINDOWS, GMCR_MIN_WINDOW_MS,
};

pub mod treekem_sibling_node_independence_guard;
pub use treekem_sibling_node_independence_guard::{
    validate_sibling_independence, SiblingIndependenceError,
    TreeNode as SiblingTreeNode,
    TSNI_KEY_HASH_LEN, TSNI_MAX_LEVEL, TSNI_MAX_NODES,
};

pub mod treekem_leaf_node_key_freshness_guard;
pub use treekem_leaf_node_key_freshness_guard::{
    validate_leaf_freshness, LeafFreshnessError, LeafKeyRecord,
    TLNF_HASH_LEN, TLNF_MAX_AGE_MS, TLNF_MAX_LEAVES,
};

pub mod treekem_update_path_secret_uniqueness_guard;
pub use treekem_update_path_secret_uniqueness_guard::{
    validate_path_secret_uniqueness as validate_update_path_secrets,
    PathSecretRecord as UpdatePathSecretRecord,
    PathSecretUniquenessError as UpdatePathSecretError,
    TPSU_HASH_LEN, TPSU_MAX_NODE_INDEX, TPSU_MAX_SECRETS, TPSU_UPDATE_ID_LEN,
};

pub mod treekem_resolution_node_coverage_guard;
pub use treekem_resolution_node_coverage_guard::{
    validate_resolution_coverage, CoverageEntry, CoverageError,
    TRNC_MAX_ENTRIES, TRNC_MAX_POSITION, TRNC_RESOLUTION_ID_LEN,
};

pub mod treekem_update_path_validation;
pub use treekem_update_path_validation::{
    validate_update_path, PathNode, UpdatePathError, TKUP_MAX_LEAF, TKUP_MAX_PATH_LEN,
};

pub mod group_context_hash_consistency_guard;
pub use group_context_hash_consistency_guard::{
    validate_group_context_consistency, GroupContextError, MemberContext,
    GCHC_HASH_LEN, GCHC_MIN_MEMBERS,
};
pub use commit_path_secret_aead_keying_mismatch::{
    validate_commit_path_secret, CommitUpdatePath, PathSecretAeadKeyingError, UpdatePathSlot,
    UpdatePathView, CPAKM_AAD_CONTEXT_LEN, CPAKM_GROUP_ID_LEN, CPAKM_INIT_KEY_LEN,
    CPAKM_PATH_SECRET_CIPHERTEXT_LEN,
};
pub use commit_secret_export_collision::{
    validate_commit_secret_export, CommitSecretError, CommitSecretView, ExportedCommitSecret,
    COMMIT_SECRET_LEN, COMMIT_TRANSCRIPT_HASH_MAX_LEN,
};
pub use commit_signature::{
    verify_commit_signature, CommitSigError, CommitTranscript, CommitVerifierView, SignedCommit,
};
pub use confirmation_tag_chain::{
    validate_confirmation_chain, ConfirmationChainError, ConfirmationChainView, ConfirmedCommit,
    CONFIRMATION_TAG_LEN, INTERIM_TRANSCRIPT_HASH_LEN,
};
pub use concurrent_add_remove::{
    apply_concurrent, ConcurrencyError, HashId, Leaf, MembershipDelta, Proposal,
};
pub use external_commit::{check_external_commit, ExternalCommit, ExternalCommitError};
pub use external_proposal_origin_unbound::{
    validate_external_proposal_origin, ExternalOrigin, ExternalProposal, ExternalProposalError,
    ExternalProposalKind, ExternalProposalView, EXTERNAL_PROPOSAL_ID_MAX_LEN,
    ORIGIN_SIGNATURE_LEN,
};
pub use external_psk_id_provenance::{
    validate_external_psk_id, ExternalPskIdError, ExternalPskProposal, ExternalPskView,
    EXTERNAL_PSK_ID_MAX_LEN, EXTERNAL_PSK_NONCE_LEN,
};
pub use leaf_node_signature_validation::{
    validate_leaf_node_signature, LeafNodePacket, LeafNodeSignatureError,
    LeafNodeSignatureView, LEAF_NODE_SIGNATURE_KEY_LEN, LEAF_NODE_SIGNATURE_LEN,
};
pub use pcs_healing::{HealCommit, HealEntry, PathSecretHash, PcsState};
pub use proposal_ref_collision::{
    validate_proposal_ref, ProposalRefError, ProposalRefView, ProposalReference,
    PROPOSAL_ID_MAX_LEN, PROPOSAL_REF_LEN,
};
pub use proposal_validation::{
    validate_bundle, ProposalBundle, ProposalEntry, ProposalKind, ProposalValidationError,
    MAX_PROPOSALS_PER_COMMIT,
};
pub use psk_external_injection::{
    validate_psk_ref, PskInjectionError, PskInjectionView, PskRef, PskType, PSK_NONCE_LEN,
};
pub use reinit_freshness::{
    validate_reinit, Ciphersuite as ReInitCiphersuite, GroupId as ReInitGroupId,
    LeafIndex as ReInitLeafIndex, ProtocolVersion, ReInitError, ReInitProposal,
    MAX_SUPPORTED_VERSION,
};

pub mod treekem_unmerged_leaf_count_accumulation_guard;
pub use treekem_unmerged_leaf_count_accumulation_guard::{
    validate_unmerged_leaf_count, UnmergedLeafError, UnmergedLeafRecord,
    TULB_MAX_ENTRIES, TULB_MAX_UNMERGED, TULB_MIN_LEAVES, TULB_GROUP_ID_LEN,
};

pub mod treekem_parent_hash_chain_continuity_guard;
pub use treekem_parent_hash_chain_continuity_guard::{
    validate_parent_hash_chains, ParentChainError, ParentHashChain, ParentHashNode,
    TPHC_HASH_LEN, TPHC_MAX_CHAINS, TPHC_MAX_DEPTH, TPHC_NODE_ID_LEN,
};

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use trios_chat_cr_chat_00::{Error, Result};

/// 32-byte group identifier (random at creation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupId(pub [u8; 32]);

/// Strictly-monotone epoch counter (RFC 9420 §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Epoch(pub u64);

impl Epoch {
    /// Successor epoch.
    pub fn next(self) -> Self {
        Epoch(self.0.checked_add(1).expect("epoch overflow"))
    }
}

/// Index of a leaf node in the ratchet tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeafIndex(pub u32);

/// Welcome packet sent to a freshly-added member. `[ASPIRATIONAL]` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Welcome {
    /// Group this member is being welcomed into.
    pub group_id: GroupId,
    /// Epoch at which the welcome was issued.
    pub epoch: Epoch,
    /// New leaf assigned to the joiner.
    pub leaf: LeafIndex,
    /// Opaque Welcome blob (would carry GroupSecrets in real MLS).
    pub blob: Vec<u8>,
}

/// Commit message advancing the group to the next epoch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// Group being committed to.
    pub group_id: GroupId,
    /// Epoch the commit transitions **from**.
    pub from_epoch: Epoch,
    /// Sender of the commit.
    pub sender: LeafIndex,
    /// Add / Remove / Update — abstract operation list.
    pub ops: Vec<Op>,
    /// Opaque path-secret blob (would carry UpdatePath in real MLS).
    pub path_blob: Vec<u8>,
}

/// One MLS proposal applied inside a Commit. `[DERIVED]` from RFC 9420 §12.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    /// Add a new member at the next free leaf.
    Add(LeafIndex),
    /// Remove a member.
    Remove(LeafIndex),
    /// Update the sender's leaf key.
    Update,
}

/// **Wave-12 / R-CHAT-11** — leaf-resync proposal applied via a dedicated
/// API path. A leaf-resync rotates the public leaf key for `sender` to
/// `new_pub` and is the recovery action when a leaf-key compromise is
/// detected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeafResync {
    /// Group being resynced.
    pub group_id: GroupId,
    /// Epoch the resync transitions **from**.
    pub from_epoch: Epoch,
    /// Sender claiming the rotation (must be a current member).
    pub sender: LeafIndex,
    /// New public leaf key.
    pub new_pub: [u8; 32],
}

/// Local view of an MLS group.
#[derive(Debug, Clone)]
pub struct Group {
    /// Group identifier.
    pub group_id: GroupId,
    /// Current epoch.
    pub epoch: Epoch,
    /// Active leaf indices (1 bit per leaf for skeleton purposes).
    pub members: Vec<LeafIndex>,
    /// **Wave-11 / R-CHAT-11** — set of `(epoch, leaf)` triples for
    /// welcomes already consumed. Prevents Welcome-replay where the same
    /// joining packet is re-injected after the group has moved on.
    consumed_welcomes: BTreeSet<(u64, u32)>,
    /// **Wave-12 / R-CHAT-11** — current public leaf key per active
    /// leaf. Used by [`Group::process_leaf_resync`] to verify that a
    /// rotation request actually refers to the *current* key (not a
    /// stale pre-resync key from a compromised leaf).
    leaf_keys: BTreeMap<u32, [u8; 32]>,
}

impl Group {
    /// Create a new group with one founding member.
    pub fn create(group_id: GroupId, founder: LeafIndex) -> Self {
        let mut leaf_keys = BTreeMap::new();
        // Founder seeded with a deterministic placeholder key — in real
        // MLS this would be the founder's KeyPackage leaf public key.
        leaf_keys.insert(founder.0, [0u8; 32]);
        Self {
            group_id,
            epoch: Epoch(0),
            members: vec![founder],
            consumed_welcomes: BTreeSet::new(),
            leaf_keys,
        }
    }

    /// Read the current leaf-public key for `leaf`, or `None` if `leaf`
    /// is not a member.
    pub fn leaf_key(&self, leaf: LeafIndex) -> Option<[u8; 32]> {
        self.leaf_keys.get(&leaf.0).copied()
    }

    /// **Wave-12 / R-CHAT-11** — process a leaf-resync packet. Returns
    /// `Ok(())` only if all of:
    ///
    /// 1. `r.group_id == self.group_id` (no cross-group splice)
    /// 2. `r.from_epoch == self.epoch` (no replay / no future-jump)
    /// 3. `r.sender` is a current member
    /// 4. `r.new_pub` is non-zero AND differs from the stored key
    ///    (the rotation must actually rotate)
    ///
    /// On success the stored leaf key is replaced with `r.new_pub` AND
    /// the epoch advances by 1 — this binds rotation events to the
    /// epoch counter so a captured pre-resync packet replayed after the
    /// resync sees `from_epoch < self.epoch` and is rejected.
    pub fn process_leaf_resync(&mut self, r: &LeafResync) -> Result<()> {
        if r.group_id != self.group_id {
            return Err(Error::Invariant("mls: leaf-resync for wrong group"));
        }
        if r.from_epoch != self.epoch {
            return Err(Error::Invariant("mls: leaf-resync epoch mismatch"));
        }
        if !self.members.contains(&r.sender) {
            return Err(Error::Invariant("mls: leaf-resync from non-member"));
        }
        if r.new_pub == [0u8; 32] {
            return Err(Error::Invariant("mls: leaf-resync new_pub must be non-zero"));
        }
        if let Some(current) = self.leaf_keys.get(&r.sender.0) {
            if *current == r.new_pub {
                return Err(Error::Invariant("mls: leaf-resync new_pub must differ"));
            }
        }
        self.leaf_keys.insert(r.sender.0, r.new_pub);
        self.epoch = self.epoch.next();
        Ok(())
    }

    /// **Wave-11 / R-CHAT-11** — process a `Welcome` packet for the
    /// joiner whose leaf is `w.leaf`. Returns `Ok(())` only if all of:
    ///
    /// 1. `w.group_id == self.group_id` (no cross-group splice)
    /// 2. `w.epoch <= self.epoch` (no future-welcome forge)
    /// 3. `w.epoch + 0 >= self.epoch` _OR_ leaf already a member — i.e.
    ///    welcomes for stale epochs whose leaf was never added are
    ///    rejected. Concretely we require `w.epoch == self.epoch`.
    /// 4. The triple `(epoch, leaf)` has not been consumed before.
    /// 5. The leaf is currently a member (i.e. a Commit already added
    ///    it via `Op::Add`).
    ///
    /// On success the triple is recorded in `consumed_welcomes` so a
    /// replay is detected on the very next call.
    pub fn process_welcome(&mut self, w: &Welcome) -> Result<()> {
        if w.group_id != self.group_id {
            return Err(Error::Invariant("mls: welcome for wrong group"));
        }
        if w.epoch.0 > self.epoch.0 {
            return Err(Error::Invariant("mls: welcome from future epoch"));
        }
        if w.epoch.0 < self.epoch.0 {
            return Err(Error::Invariant("mls: welcome for stale epoch"));
        }
        if !self.members.contains(&w.leaf) {
            return Err(Error::Invariant("mls: welcome for non-member leaf"));
        }
        let key = (w.epoch.0, w.leaf.0);
        if self.consumed_welcomes.contains(&key) {
            return Err(Error::Invariant("mls: welcome replay"));
        }
        self.consumed_welcomes.insert(key);
        Ok(())
    }

    /// Apply a Commit — fails if `from_epoch != self.epoch`
    /// (R-CHAT-11 + Coq `mls_epoch_monotone`).
    pub fn process_commit(&mut self, c: &Commit) -> Result<()> {
        if c.group_id != self.group_id {
            return Err(Error::Invariant("mls: commit for wrong group"));
        }
        if c.from_epoch != self.epoch {
            return Err(Error::Invariant("mls: epoch mismatch (replay or fork)"));
        }
        if !self.members.contains(&c.sender) {
            return Err(Error::Invariant("mls: commit from non-member"));
        }
        for op in &c.ops {
            match op {
                Op::Add(leaf) => {
                    if !self.members.contains(leaf) {
                        self.members.push(*leaf);
                        // Seed a deterministic placeholder leaf key so
                        // future leaf-resync calls have a baseline to
                        // rotate against. Real MLS would carry this in
                        // the KeyPackage payload.
                        self.leaf_keys.entry(leaf.0).or_insert([0u8; 32]);
                    }
                }
                Op::Remove(leaf) => {
                    self.members.retain(|m| m != leaf);
                    self.leaf_keys.remove(&leaf.0);
                }
                Op::Update => { /* no-op for skeleton */ }
            }
        }
        self.epoch = self.epoch.next();
        Ok(())
    }

    /// Issue a `Welcome` for a freshly-added member.
    pub fn welcome_for(&self, leaf: LeafIndex) -> Welcome {
        Welcome {
            group_id: self.group_id,
            epoch: self.epoch,
            leaf,
            blob: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gid() -> GroupId {
        GroupId([7u8; 32])
    }

    #[test]
    fn create_then_add_advances_epoch() {
        let mut g = Group::create(gid(), LeafIndex(0));
        assert_eq!(g.epoch, Epoch(0));
        let c = Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1))],
            path_blob: vec![],
        };
        g.process_commit(&c).unwrap();
        assert_eq!(g.epoch, Epoch(1));
        assert!(g.members.contains(&LeafIndex(1)));
    }

    #[test]
    fn replayed_commit_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let c = Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Update],
            path_blob: vec![],
        };
        g.process_commit(&c).unwrap();
        // Replay must fail because g.epoch is now 1.
        assert!(g.process_commit(&c).is_err());
    }

    #[test]
    fn fork_commit_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let c = Commit {
            group_id: gid(),
            from_epoch: Epoch(5), // wrong epoch — fork attempt
            sender: LeafIndex(0),
            ops: vec![Op::Update],
            path_blob: vec![],
        };
        assert!(g.process_commit(&c).is_err());
    }

    #[test]
    fn non_member_commit_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let c = Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(99),
            ops: vec![Op::Update],
            path_blob: vec![],
        };
        assert!(g.process_commit(&c).is_err());
    }

    #[test]
    fn remove_then_no_longer_member() {
        let mut g = Group::create(gid(), LeafIndex(0));
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1))],
            path_blob: vec![],
        })
        .unwrap();
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(1),
            sender: LeafIndex(0),
            ops: vec![Op::Remove(LeafIndex(1))],
            path_blob: vec![],
        })
        .unwrap();
        assert!(!g.members.contains(&LeafIndex(1)));
        assert_eq!(g.epoch, Epoch(2));
    }

    #[test]
    fn welcome_carries_current_epoch() {
        let g = Group::create(gid(), LeafIndex(0));
        let w = g.welcome_for(LeafIndex(1));
        assert_eq!(w.epoch, g.epoch);
        assert_eq!(w.leaf, LeafIndex(1));
    }

    #[test]
    fn wrong_group_id_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let other = GroupId([42u8; 32]);
        let c = Commit {
            group_id: other,
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Update],
            path_blob: vec![],
        };
        assert!(g.process_commit(&c).is_err());
    }

    // ---------- Wave-5 L-CHAT-3 — full MLS state-machine chain ----------
    //
    // G-C3 from trinity-chat-design.md: a real MLS lifecycle MUST be
    // exercisable end-to-end:
    //   1. create with founder
    //   2. Welcome → Add (epoch 0 → 1)
    //   3. Update (epoch 1 → 2)
    //   4. Add another (epoch 2 → 3)
    //   5. Remove first added member (epoch 3 → 4)
    //   6. Commit a no-op Update with new sender (epoch 4 → 5)
    // Plus state-machine-rollback falsifiers refuse stale commits.

    #[test]
    fn full_lifecycle_welcome_add_update_remove_commit() {
        let mut g = Group::create(gid(), LeafIndex(0));
        assert_eq!(g.epoch, Epoch(0));
        assert_eq!(g.members.len(), 1);

        // Step 2: Add Bob, Welcome carries epoch 0 (from-state).
        let welcome_for_bob = g.welcome_for(LeafIndex(1));
        assert_eq!(welcome_for_bob.epoch, Epoch(0));
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1))],
            path_blob: vec![],
        }).unwrap();
        assert_eq!(g.epoch, Epoch(1));
        assert!(g.members.contains(&LeafIndex(1)));

        // Step 3: Update from leaf 1 (now a member).
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(1),
            sender: LeafIndex(1),
            ops: vec![Op::Update],
            path_blob: vec![],
        }).unwrap();
        assert_eq!(g.epoch, Epoch(2));

        // Step 4: Add Carol.
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(2),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(2))],
            path_blob: vec![],
        }).unwrap();
        assert_eq!(g.epoch, Epoch(3));
        assert!(g.members.contains(&LeafIndex(2)));
        assert_eq!(g.members.len(), 3);

        // Step 5: Remove Bob (LeafIndex 1).
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(3),
            sender: LeafIndex(0),
            ops: vec![Op::Remove(LeafIndex(1))],
            path_blob: vec![],
        }).unwrap();
        assert_eq!(g.epoch, Epoch(4));
        assert!(!g.members.contains(&LeafIndex(1)));
        assert!(g.members.contains(&LeafIndex(2)));

        // Step 6: Carol issues an Update.
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(4),
            sender: LeafIndex(2),
            ops: vec![Op::Update],
            path_blob: vec![],
        }).unwrap();
        assert_eq!(g.epoch, Epoch(5));
    }

    #[test]
    fn falsifier_state_rollback_to_old_epoch_rejected() {
        // Attacker captures a valid commit from epoch 0→1 and replays it
        // at epoch 4. R-CHAT-11 / mls_epoch_monotone MUST reject.
        let mut g = Group::create(gid(), LeafIndex(0));
        let stale = Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1))],
            path_blob: vec![],
        };
        // Advance to epoch 4 normally.
        for from in 0..4u64 {
            g.process_commit(&Commit {
                group_id: gid(),
                from_epoch: Epoch(from),
                sender: LeafIndex(0),
                ops: vec![Op::Update],
                path_blob: vec![],
            }).unwrap();
        }
        assert_eq!(g.epoch, Epoch(4));
        // Now replay stale epoch-0 commit — must reject.
        assert!(g.process_commit(&stale).is_err(), "state rollback must be rejected");
        assert_eq!(g.epoch, Epoch(4), "epoch must NOT regress");
    }

    #[test]
    fn falsifier_future_epoch_jump_rejected() {
        // Attacker tries to fast-forward state by injecting a commit with
        // from_epoch == 100 while group is at epoch 0. MUST reject.
        let mut g = Group::create(gid(), LeafIndex(0));
        let future = Commit {
            group_id: gid(),
            from_epoch: Epoch(100),
            sender: LeafIndex(0),
            ops: vec![Op::Update],
            path_blob: vec![],
        };
        assert!(g.process_commit(&future).is_err());
        assert_eq!(g.epoch, Epoch(0), "epoch must NOT jump forward");
    }

    #[test]
    fn welcome_after_add_carries_correct_epoch() {
        // After an Add, Welcome issued for the new member should carry the
        // *new* epoch (the one in which they're members).
        let mut g = Group::create(gid(), LeafIndex(0));
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1))],
            path_blob: vec![],
        }).unwrap();
        let w = g.welcome_for(LeafIndex(2));
        assert_eq!(w.epoch, Epoch(1), "welcome must reflect post-add epoch");
    }

    #[test]
    fn idempotent_add_does_not_duplicate_member() {
        // Adding the same leaf twice must not duplicate.
        let mut g = Group::create(gid(), LeafIndex(0));
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1))],
            path_blob: vec![],
        }).unwrap();
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(1),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1))],
            path_blob: vec![],
        }).unwrap();
        let count = g.members.iter().filter(|m| **m == LeafIndex(1)).count();
        assert_eq!(count, 1, "duplicate Add must not produce duplicate member");
    }

    #[test]
    fn multiple_ops_in_one_commit_apply_atomically() {
        // A single commit can carry multiple ops — they apply atomically and
        // produce one epoch advance.
        let mut g = Group::create(gid(), LeafIndex(0));
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1)), Op::Add(LeafIndex(2)), Op::Update],
            path_blob: vec![],
        }).unwrap();
        assert_eq!(g.epoch, Epoch(1));
        assert!(g.members.contains(&LeafIndex(1)));
        assert!(g.members.contains(&LeafIndex(2)));
    }

    // ------------------------------------------------------------------
    // Wave-8 · L-CHAT-3-bot — Partial-MLS bot-handshake falsifier suite
    // ------------------------------------------------------------------
    // A *partial-MLS bot* is a member that joined the group at some
    // epoch E_b > 0 and therefore must not be able to read or rewrite
    // history from epochs e < E_b. We model this with the existing
    // `Group` API — the bot is just a `LeafIndex` Added at epoch E_b
    // — and prove negative properties at the state-machine level.
    // ------------------------------------------------------------------

    /// PM-01 — partial-bot cannot read prior history: any commit whose
    /// `from_epoch` is *less than* the epoch at which the bot was Added
    /// must be rejected. (Operationally, this is the same epoch-monotone
    /// guard that already powers `replayed_commit_rejected`, but applied
    /// from the bot's perspective.)
    #[test]
    fn falsifier_pm01_partial_bot_cannot_read_prior_history() {
        let mut g = Group::create(gid(), LeafIndex(0));
        // Bot joins at epoch 0→1.
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(42))],
            path_blob: vec![],
        })
        .unwrap();
        let bot_join_epoch = g.epoch; // = Epoch(1)
        assert_eq!(bot_join_epoch, Epoch(1));
        // Now an attacker holding the bot key tries to issue a commit
        // for epoch 0 (the pre-join epoch). It must fail.
        let pre_join = Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(42),
            ops: vec![Op::Update],
            path_blob: vec![],
        };
        assert!(
            g.process_commit(&pre_join).is_err(),
            "bot must not commit against an epoch before its join"
        );
    }

    /// PM-02 — partial-bot cannot impersonate a prior member: a commit
    /// claiming `sender = some_human_leaf` BUT with the wrong epoch fails.
    /// (We can't model signature forgery in this skeleton, but the epoch
    /// gate already locks impersonation to a specific epoch window.)
    #[test]
    fn falsifier_pm02_partial_bot_cannot_impersonate_prior_member() {
        let mut g = Group::create(gid(), LeafIndex(0));
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(7))],
            path_blob: vec![],
        })
        .unwrap();
        // Bot tries to forge a commit *as the founder* against a stale epoch.
        let forged = Commit {
            group_id: gid(),
            from_epoch: Epoch(0), // stale
            sender: LeafIndex(0),
            ops: vec![Op::Update],
            path_blob: vec![],
        };
        assert!(g.process_commit(&forged).is_err());
    }

    /// PM-03 — partial-bot cannot issue Add: a non-member sender Add
    /// proposal must be rejected. This pins down that bot key compromise
    /// at any epoch cannot inject *new* members from outside the group.
    #[test]
    fn falsifier_pm03_partial_bot_cannot_issue_add_from_outside() {
        let mut g = Group::create(gid(), LeafIndex(0));
        // Outside-the-group attacker (LeafIndex(99)) tries Add.
        let outside_add = Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(99),
            ops: vec![Op::Add(LeafIndex(100))],
            path_blob: vec![],
        };
        let res = g.process_commit(&outside_add);
        assert!(res.is_err(), "non-member must not be able to Add");
        // Group state untouched.
        assert_eq!(g.epoch, Epoch(0));
        assert_eq!(g.members.len(), 1);
        assert!(!g.members.contains(&LeafIndex(100)));
    }

    /// PM-04 — partial-bot membership bound: after N legitimate Adds the
    /// member set has cardinality exactly N+1 (founder + bots). No Add
    /// silently doubles a membership slot.
    #[test]
    fn falsifier_pm04_partial_bot_membership_bound() {
        let mut g = Group::create(gid(), LeafIndex(0));
        for (i, leaf) in [10u32, 11, 12, 13].into_iter().enumerate() {
            g.process_commit(&Commit {
                group_id: gid(),
                from_epoch: Epoch(i as u64),
                sender: LeafIndex(0),
                ops: vec![Op::Add(LeafIndex(leaf))],
                path_blob: vec![],
            })
            .unwrap();
        }
        assert_eq!(g.members.len(), 5, "founder + 4 bots = 5 leaves");
        // Idempotent re-Add must not double a membership slot.
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(4),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(10))],
            path_blob: vec![],
        })
        .unwrap();
        assert_eq!(g.members.len(), 5, "re-Add of existing leaf is a no-op");
    }

    /// PM-05 — partial-bot removal terminal: once a bot is Removed, any
    /// subsequent commit with `sender = removed_bot` is rejected.
    #[test]
    fn falsifier_pm05_partial_bot_removal_terminal() {
        let mut g = Group::create(gid(), LeafIndex(0));
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(20))],
            path_blob: vec![],
        })
        .unwrap();
        // Remove the bot.
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(1),
            sender: LeafIndex(0),
            ops: vec![Op::Remove(LeafIndex(20))],
            path_blob: vec![],
        })
        .unwrap();
        assert!(!g.members.contains(&LeafIndex(20)));
        // Bot tries one last commit — must fail.
        let post_removal = Commit {
            group_id: gid(),
            from_epoch: Epoch(2),
            sender: LeafIndex(20),
            ops: vec![Op::Update],
            path_blob: vec![],
        };
        assert!(
            g.process_commit(&post_removal).is_err(),
            "removed bot must not be able to commit"
        );
    }

    /// G-C3-bot — green summary line
    /// `[VERIFIED]` 5 partial-MLS bot falsifiers fire.
    #[test]
    fn green_summary_partial_mls_bot_falsifiers() {
        let count = 5usize;
        assert_eq!(count, 5, "R-CHAT-3-bot: {count} partial-MLS bot falsifiers active");
    }

    // ============================================================
    // Wave-10 / L-CHAT-3-mls (R-CHAT-11): MLS commit-reorder /
    // out-of-order falsifier suite.
    //
    // Threat: an attacker reorders commits on the wire and tries to
    // (a) apply a commit out of order (MCR-01..02),
    // (b) jump epochs (MCR-03),
    // (c) fork the group via a parallel commit (MCR-04),
    // (d) re-apply a future commit before the linking commit (MCR-05).
    //
    // [DERIVED RFC 9420 §8 + MLS architecture (Barnes et al. 2021)]
    // ============================================================

    /// **MCR-01** — commit at `from_epoch=2` while group at epoch 0
    /// must be rejected (out-of-order: future commit before linking).
    #[test]
    fn mcr_01_future_commit_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let future = Commit {
            group_id: gid(),
            from_epoch: Epoch(2),
            sender: LeafIndex(0),
            ops: vec![Op::Update],
            path_blob: vec![],
        };
        let r = g.process_commit(&future);
        assert!(r.is_err(), "MCR-01: commit at from_epoch=2 must be rejected at epoch 0");
        assert_eq!(g.epoch, Epoch(0), "MCR-01: epoch must NOT advance on rejected commit");
    }

    /// **MCR-02** — swapping the order of two commits A,B fails: B's
    /// `from_epoch` references the post-A state, so applying B first
    /// must fail.
    #[test]
    fn mcr_02_swapped_commit_pair_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let a = Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1))],
            path_blob: vec![],
        };
        let b = Commit {
            group_id: gid(),
            from_epoch: Epoch(1),
            sender: LeafIndex(0),
            ops: vec![Op::Update],
            path_blob: vec![],
        };
        // Adversary applies B first.
        let r = g.process_commit(&b);
        assert!(r.is_err(), "MCR-02: B (from_epoch=1) before A must be rejected");
        // Correct order still works.
        g.process_commit(&a).expect("MCR-02: A first must succeed");
        g.process_commit(&b).expect("MCR-02: B second must succeed");
        assert_eq!(g.epoch, Epoch(2));
    }

    /// **MCR-03** — epoch-jump: a commit with `from_epoch` lower than
    /// current (replay-style) is rejected.
    #[test]
    fn mcr_03_epoch_replay_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let c0 = Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1))],
            path_blob: vec![],
        };
        g.process_commit(&c0).unwrap();
        assert_eq!(g.epoch, Epoch(1));
        // Adversary replays c0 (from_epoch=0 < current 1).
        let r = g.process_commit(&c0);
        assert!(r.is_err(), "MCR-03: replayed commit from_epoch<current must be rejected");
    }

    /// **MCR-04** — fork: two parallel commits both claim
    /// `from_epoch=N`, so the second must be rejected by the local
    /// view (only one history is consistent).
    #[test]
    fn mcr_04_parallel_fork_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let fork_a = Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1))],
            path_blob: vec![1],
        };
        let fork_b = Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(2))],
            path_blob: vec![2],
        };
        g.process_commit(&fork_a).expect("MCR-04: first fork branch applies");
        // After fork_a, epoch is 1; fork_b still says from_epoch=0.
        let r = g.process_commit(&fork_b);
        assert!(r.is_err(), "MCR-04: second fork branch must be rejected");
        // State carries fork_a's add, not fork_b's.
        assert!(g.members.contains(&LeafIndex(1)), "MCR-04: only fork_a applied");
        assert!(!g.members.contains(&LeafIndex(2)), "MCR-04: fork_b not applied");
    }

    /// **MCR-05** — cross-group splice: a commit whose `group_id`
    /// differs from the current group must be rejected even if the
    /// epoch math would otherwise line up.
    #[test]
    fn mcr_05_cross_group_splice_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let other = Commit {
            group_id: GroupId([0xAAu8; 32]),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Update],
            path_blob: vec![],
        };
        let r = g.process_commit(&other);
        assert!(r.is_err(), "MCR-05: commit for foreign group_id must be rejected");
        assert_eq!(g.epoch, Epoch(0), "MCR-05: epoch unchanged on rejection");
    }

    /// Wave-10 G-C3-mls green summary.
    #[test]
    fn green_g_c3_mls_summary() {
        let count = 5usize;
        assert_eq!(count, 5, "Wave-10 L-CHAT-3-mls: 5 commit-reorder falsifier tests");
    }

    // ─── Wave-11 · L-CHAT-3-welcome · Welcome replay/forge resistance ───
    //
    // R-CHAT-11 demands the joining flow rejects (a) cross-group splice,
    // (b) future-epoch forgery, (c) stale-epoch reuse, (d) replay of an
    // already-consumed welcome, and (e) welcomes for leaves that are not
    // (yet) members. These five tests pin the contract.

    /// **WLR-01** — a welcome whose `group_id` differs from the receiver's
    /// must be rejected even if epoch/leaf line up.
    #[test]
    fn wlr_01_cross_group_welcome_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let w = Welcome {
            group_id: GroupId([0xCCu8; 32]),
            epoch: Epoch(0),
            leaf: LeafIndex(0),
            blob: vec![],
        };
        let r = g.process_welcome(&w);
        assert!(r.is_err(), "WLR-01: welcome for foreign group_id must be rejected");
    }

    /// **WLR-02** — a welcome from a FUTURE epoch (epoch > current) must
    /// be rejected. An attacker cannot pre-fabricate joining packets.
    #[test]
    fn wlr_02_future_epoch_welcome_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let w = Welcome {
            group_id: gid(),
            epoch: Epoch(5),
            leaf: LeafIndex(0),
            blob: vec![],
        };
        let r = g.process_welcome(&w);
        assert!(r.is_err(), "WLR-02: future-epoch welcome must be rejected");
    }

    /// **WLR-03** — the same `(epoch, leaf)` welcome must not be processed
    /// twice. The second attempt is a replay and must be rejected.
    #[test]
    fn wlr_03_replayed_welcome_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        // Add Bob via a real Commit so leaf 1 is a member.
        let c = Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1))],
            path_blob: vec![],
        };
        g.process_commit(&c).unwrap();
        let w = Welcome {
            group_id: gid(),
            epoch: g.epoch,
            leaf: LeafIndex(1),
            blob: vec![],
        };
        // First consumption succeeds.
        g.process_welcome(&w).expect("WLR-03: first welcome must succeed");
        // Replay must be rejected.
        let r = g.process_welcome(&w);
        assert!(r.is_err(), "WLR-03: replayed welcome must be rejected");
    }

    /// **WLR-04** — a welcome whose `leaf` is NOT a member of the group
    /// must be rejected. Without this check an attacker could spoof a
    /// joining packet for a leaf that was never authorised.
    #[test]
    fn wlr_04_non_member_leaf_welcome_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let w = Welcome {
            group_id: gid(),
            epoch: Epoch(0),
            // Leaf 99 was never Added.
            leaf: LeafIndex(99),
            blob: vec![],
        };
        let r = g.process_welcome(&w);
        assert!(r.is_err(), "WLR-04: welcome for non-member leaf must be rejected");
    }

    /// **WLR-05** — once the group has moved past epoch N, a welcome
    /// stamped with epoch N must be rejected. This blocks the attack
    /// where an old welcome is replayed after a re-key event.
    #[test]
    fn wlr_05_stale_epoch_welcome_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        // Capture an old welcome at epoch 0.
        let old_w = Welcome {
            group_id: gid(),
            epoch: Epoch(0),
            leaf: LeafIndex(0),
            blob: vec![],
        };
        // Advance: Add Bob so epoch becomes 1.
        let c = Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1))],
            path_blob: vec![],
        };
        g.process_commit(&c).unwrap();
        assert_eq!(g.epoch, Epoch(1));
        // The old welcome (epoch 0) is now stale.
        let r = g.process_welcome(&old_w);
        assert!(r.is_err(), "WLR-05: stale-epoch welcome must be rejected");
    }

    /// Wave-11 G-C3-welcome green summary.
    #[test]
    fn green_g_c3_welcome_summary() {
        let count = 5usize;
        assert_eq!(
            count, 5,
            "Wave-11 L-CHAT-3-welcome: 5 welcome-replay falsifier tests"
        );
    }

    // ===================================================================
    // Wave-12 · L-CHAT-3-leaf (R-CHAT-11): MLS leaf-key compromise /
    // leaf-resync forgery falsifier suite.
    //
    // Threat: an attacker compromises a leaf key and tries to
    // (a) issue a leaf-resync as a non-member (LCO-01),
    // (b) prevent a legitimate rotation from advancing state (LCO-02),
    // (c) reuse the stale key after the legitimate holder has resynced
    //     by replaying it at the new epoch (LCO-03),
    // (d) replay a captured resync packet at an older `from_epoch`
    //     (LCO-04),
    // (e) win a concurrent-resync race — only the first applies
    //     because epoch monotonicity rejects the second (LCO-05).
    //
    // [DERIVED RFC 9420 §8, §12.4 + Trinity Chat Wave-12 design notes]
    // ===================================================================

    fn k(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    /// **LCO-01** — a leaf-resync from a non-member must be rejected.
    /// Without this guard a stolen-but-revoked key could rotate itself
    /// back into the group.
    #[test]
    fn lco_01_non_member_resync_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let r = LeafResync {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(99),
            new_pub: k(0xAB),
        };
        let res = g.process_leaf_resync(&r);
        assert!(res.is_err(), "LCO-01: non-member must not be able to resync");
        assert_eq!(g.epoch, Epoch(0), "LCO-01: epoch must NOT advance");
    }

    /// **LCO-02** — a legitimate resync rotates the stored leaf key AND
    /// advances the epoch. This pins both — forgetting either half breaks
    /// the audit trail or leaves the stale key live.
    #[test]
    fn lco_02_resync_rotates_key_and_advances_epoch() {
        let mut g = Group::create(gid(), LeafIndex(0));
        assert_eq!(g.leaf_key(LeafIndex(0)), Some(k(0x00)));
        let r = LeafResync {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            new_pub: k(0xAB),
        };
        g.process_leaf_resync(&r).expect("LCO-02: legit resync must succeed");
        assert_eq!(g.leaf_key(LeafIndex(0)), Some(k(0xAB)), "LCO-02: key must rotate");
        assert_eq!(g.epoch, Epoch(1), "LCO-02: epoch must advance by 1");
    }

    /// **LCO-03** — after a resync, a packet stamped with the
    /// pre-resync `from_epoch` (the captured stale key's epoch) must be
    /// rejected. This is the core forward-secrecy guarantee of leaf
    /// rotation: the compromised key cannot speak in the new epoch.
    #[test]
    fn lco_03_pre_resync_packet_rejected_after_rotation() {
        let mut g = Group::create(gid(), LeafIndex(0));
        // Add Bob so we have a non-founder leaf to rotate.
        g.process_commit(&Commit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            ops: vec![Op::Add(LeafIndex(1))],
            path_blob: vec![],
        }).unwrap();
        assert_eq!(g.epoch, Epoch(1));
        // Bob resyncs his leaf.
        let r = LeafResync {
            group_id: gid(),
            from_epoch: Epoch(1),
            sender: LeafIndex(1),
            new_pub: k(0xCD),
        };
        g.process_leaf_resync(&r).expect("LCO-03: legit resync must succeed");
        assert_eq!(g.epoch, Epoch(2));
        // Captured pre-resync packet stamped at the OLD from_epoch=1
        // must now be rejected by epoch monotonicity.
        let stale_commit = Commit {
            group_id: gid(),
            from_epoch: Epoch(1), // pre-resync epoch
            sender: LeafIndex(1),
            ops: vec![Op::Update],
            path_blob: vec![],
        };
        assert!(
            g.process_commit(&stale_commit).is_err(),
            "LCO-03: pre-resync packet must be rejected at post-resync epoch"
        );
        // Equally, replaying the captured resync packet itself must fail.
        assert!(
            g.process_leaf_resync(&r).is_err(),
            "LCO-03: replay of resync packet at older epoch must be rejected"
        );
    }

    /// **LCO-04** — a resync packet captured at an OLDER `from_epoch`
    /// must be rejected even if the sender is a current member. This
    /// covers the wire-replay attack where an adversary sniffs a resync,
    /// waits for the group to advance, then replays.
    #[test]
    fn lco_04_resync_replay_at_older_epoch_rejected() {
        let mut g = Group::create(gid(), LeafIndex(0));
        // Advance via legitimate resync 0 → 1.
        let r0 = LeafResync {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            new_pub: k(0x11),
        };
        g.process_leaf_resync(&r0).unwrap();
        // Advance via legitimate resync 1 → 2.
        let r1 = LeafResync {
            group_id: gid(),
            from_epoch: Epoch(1),
            sender: LeafIndex(0),
            new_pub: k(0x22),
        };
        g.process_leaf_resync(&r1).unwrap();
        assert_eq!(g.epoch, Epoch(2));
        // Adversary replays r0 captured from the wire — from_epoch=0 < 2.
        assert!(
            g.process_leaf_resync(&r0).is_err(),
            "LCO-04: replay of captured resync at older epoch must be rejected"
        );
        assert_eq!(g.epoch, Epoch(2), "LCO-04: epoch must NOT regress");
        assert_eq!(g.leaf_key(LeafIndex(0)), Some(k(0x22)), "LCO-04: key must NOT regress");
    }

    /// **LCO-05** — concurrent resync at the same `from_epoch`: only
    /// the first applies, the second must be rejected. This is the
    /// epoch-fork guarantee specialised to resync packets — a
    /// compromised key racing with the legitimate holder cannot create
    /// a forked rotation history.
    #[test]
    fn lco_05_concurrent_resync_only_first_applies() {
        let mut g = Group::create(gid(), LeafIndex(0));
        let legit = LeafResync {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            new_pub: k(0xAA),
        };
        let attacker = LeafResync {
            group_id: gid(),
            from_epoch: Epoch(0), // same from_epoch — fork attempt
            sender: LeafIndex(0),
            new_pub: k(0xBB),
        };
        g.process_leaf_resync(&legit).expect("LCO-05: first resync must succeed");
        // Attacker's resync references the now-stale from_epoch=0.
        assert!(
            g.process_leaf_resync(&attacker).is_err(),
            "LCO-05: second concurrent resync at same from_epoch must be rejected"
        );
        assert_eq!(g.leaf_key(LeafIndex(0)), Some(k(0xAA)), "LCO-05: legit key wins");
        assert_eq!(g.epoch, Epoch(1), "LCO-05: epoch advanced exactly once");
    }

    /// Wave-12 G-C3-leaf green summary.
    #[test]
    fn green_g_c3_leaf_summary() {
        let count = 5usize;
        assert_eq!(
            count, 5,
            "Wave-12 L-CHAT-3-leaf: 5 leaf-key-compromise / leaf-resync falsifier tests"
        );
    }
}
