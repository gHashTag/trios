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

/// Local view of an MLS group.
#[derive(Debug, Clone)]
pub struct Group {
    /// Group identifier.
    pub group_id: GroupId,
    /// Current epoch.
    pub epoch: Epoch,
    /// Active leaf indices (1 bit per leaf for skeleton purposes).
    pub members: Vec<LeafIndex>,
}

impl Group {
    /// Create a new group with one founding member.
    pub fn create(group_id: GroupId, founder: LeafIndex) -> Self {
        Self {
            group_id,
            epoch: Epoch(0),
            members: vec![founder],
        }
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
                    }
                }
                Op::Remove(leaf) => {
                    self.members.retain(|m| m != leaf);
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
}
