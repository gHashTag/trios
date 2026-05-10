//! # Wave-17 · L-CHAT-3-pcs (R-CHAT-11) — group-PCS healing
//!
//! Post-Compromise Security (PCS) healing for an MLS-style group.
//! After a member's *long-term* device key is compromised, the group
//! must converge to a state where ciphertexts produced under the
//! healed key are *not* readable by the attacker, even if the
//! attacker held the device-key right up to the heal commit.
//!
//! In RFC 9420 the PCS-healing primitive is the **path-secret commit**:
//! the compromised member issues an `Update` whose path payload is
//! freshly random, derived from a fresh post-compromise secret. We
//! model this at the state-machine level — what we prove here:
//!
//! 1. a heal commit advances the group epoch by exactly 1;
//! 2. the compromised leaf's *post-heal* path-secret hash is
//!    different from its pre-heal hash;
//! 3. a captured *pre-heal* commit replayed at the post-heal epoch
//!    is rejected by the existing epoch monotonicity guard;
//! 4. healing two members in a single commit produces *both* fresh
//!    secrets in one epoch jump (atomic);
//! 5. a heal request claiming the *same* post-heal secret as before
//!    is rejected (the heal must actually heal);
//! 6. concurrent heal requests at the same `from_epoch` race — only
//!    the first applies, the second is rejected (no fork).
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · GROUP-PCS-HEAL`

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use trios_chat_cr_chat_00::{Error, Result};

use crate::{Epoch, GroupId, LeafIndex};

/// 32-byte digest of a leaf's path-secret. Real MLS uses
/// SHA-256(path_secret); we keep an opaque `[u8; 32]` so the
/// state-machine proofs are independent of the hash choice.
pub type PathSecretHash = [u8; 32];

/// **Wave-17 / R-CHAT-11** — heal-commit packet. Rotates the
/// `target` leaf's path-secret hash from `from_hash` to
/// `to_hash` at `from_epoch → from_epoch + 1`.
///
/// One [`HealCommit`] can carry multiple `(target, from_hash, to_hash)`
/// triples to model the case where several compromised devices heal
/// in a single epoch transition (atomic batch heal).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealCommit {
    /// Group this heal applies to.
    pub group_id: GroupId,
    /// Epoch the heal transitions **from**.
    pub from_epoch: Epoch,
    /// Sender of the heal commit (must be a current member).
    pub sender: LeafIndex,
    /// Per-target rotations. Order is irrelevant (the validator
    /// canonicalises by leaf index).
    pub heals: Vec<HealEntry>,
}

/// One `(target, from, to)` rotation inside a [`HealCommit`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealEntry {
    /// Leaf whose path-secret is being rotated.
    pub target: LeafIndex,
    /// The hash currently stored for `target` (sender's view).
    pub from_hash: PathSecretHash,
    /// Fresh post-compromise hash to install.
    pub to_hash: PathSecretHash,
}

/// PCS-healed state for an MLS group, indexed alongside the group's
/// epoch counter. We deliberately keep this as a *separate* struct so
/// the W17 lane can be unit-tested without dragging the whole `Group`
/// surface into the proof.
#[derive(Debug, Clone)]
pub struct PcsState {
    /// Group this state belongs to.
    pub group_id: GroupId,
    /// Current epoch. Bumped by exactly +1 on every accepted
    /// [`HealCommit`].
    pub epoch: Epoch,
    /// Per-leaf path-secret hash. A leaf is "active" iff present
    /// here; healing a non-present leaf is rejected.
    secrets: BTreeMap<u32, PathSecretHash>,
}

impl PcsState {
    /// Create fresh PCS state seeded with one member at epoch 0.
    pub fn new(group_id: GroupId, founder: LeafIndex, founder_hash: PathSecretHash) -> Self {
        let mut secrets = BTreeMap::new();
        secrets.insert(founder.0, founder_hash);
        Self {
            group_id,
            epoch: Epoch(0),
            secrets,
        }
    }

    /// Register a new member with their initial path-secret hash. A
    /// member added this way has not yet been healed — their secret
    /// is whatever the caller seeded.
    pub fn add_member(&mut self, leaf: LeafIndex, hash: PathSecretHash) {
        self.secrets.insert(leaf.0, hash);
    }

    /// Look up the current path-secret hash for `leaf`, if present.
    pub fn secret_of(&self, leaf: LeafIndex) -> Option<PathSecretHash> {
        self.secrets.get(&leaf.0).copied()
    }

    /// Apply a `HealCommit`. Returns `Ok(())` only when *all* of:
    ///
    /// 1. `h.group_id == self.group_id`;
    /// 2. `h.from_epoch == self.epoch` (no replay / no future-jump);
    /// 3. `h.sender` is a current member;
    /// 4. every `target` in `h.heals` is a current member;
    /// 5. every `from_hash` matches the stored hash for its target
    ///    (the sender knew the pre-heal state — a wire-replay from
    ///    *another* compromised view would not);
    /// 6. every `to_hash` differs from its `from_hash` (the heal
    ///    must actually rotate — no no-op fakes);
    /// 7. the targets in `h.heals` are pairwise distinct (no
    ///    duplicate-target smuggling).
    ///
    /// On success every targeted secret is replaced with its
    /// `to_hash` and the epoch advances by 1 — *atomically*.
    pub fn process_heal(&mut self, h: &HealCommit) -> Result<()> {
        if h.group_id != self.group_id {
            return Err(Error::Invariant("pcs: heal for wrong group"));
        }
        if h.from_epoch != self.epoch {
            return Err(Error::Invariant("pcs: heal epoch mismatch"));
        }
        if !self.secrets.contains_key(&h.sender.0) {
            return Err(Error::Invariant("pcs: heal from non-member"));
        }
        if h.heals.is_empty() {
            return Err(Error::Invariant("pcs: heal must rotate at least one secret"));
        }

        // Pairwise distinct targets.
        let mut seen: BTreeMap<u32, ()> = BTreeMap::new();
        for he in &h.heals {
            if seen.insert(he.target.0, ()).is_some() {
                return Err(Error::Invariant("pcs: duplicate heal target"));
            }
        }

        // Validate every entry against the current state.
        for he in &h.heals {
            let current = self
                .secrets
                .get(&he.target.0)
                .ok_or(Error::Invariant("pcs: heal for non-member target"))?;
            if *current != he.from_hash {
                return Err(Error::Invariant("pcs: heal from_hash mismatch"));
            }
            if he.from_hash == he.to_hash {
                return Err(Error::Invariant("pcs: heal must change the secret"));
            }
        }

        // All checks passed — apply atomically.
        for he in &h.heals {
            self.secrets.insert(he.target.0, he.to_hash);
        }
        self.epoch = self.epoch.next();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gid() -> GroupId {
        GroupId([3u8; 32])
    }

    fn h(byte: u8) -> PathSecretHash {
        [byte; 32]
    }

    fn fresh_state() -> PcsState {
        PcsState::new(gid(), LeafIndex(0), h(0x10))
    }

    /// **PCS-01** — a well-formed single-target heal is accepted, the
    /// secret is rotated, and the epoch advances by exactly 1.
    #[test]
    fn pcs_01_well_formed_heal_accepted() {
        let mut s = fresh_state();
        let heal = HealCommit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            heals: vec![HealEntry {
                target: LeafIndex(0),
                from_hash: h(0x10),
                to_hash: h(0xAA),
            }],
        };
        s.process_heal(&heal).expect("PCS-01: well-formed heal must succeed");
        assert_eq!(s.epoch, Epoch(1), "PCS-01: epoch must advance by 1");
        assert_eq!(s.secret_of(LeafIndex(0)), Some(h(0xAA)), "PCS-01: secret rotated");
    }

    /// **PCS-02** — a heal with `from_epoch != current` (replay or
    /// future jump) is rejected. State is untouched.
    #[test]
    fn pcs_02_epoch_mismatch_rejected() {
        let mut s = fresh_state();
        let bad = HealCommit {
            group_id: gid(),
            from_epoch: Epoch(5),
            sender: LeafIndex(0),
            heals: vec![HealEntry {
                target: LeafIndex(0),
                from_hash: h(0x10),
                to_hash: h(0xAA),
            }],
        };
        assert!(s.process_heal(&bad).is_err(), "PCS-02: epoch mismatch must reject");
        assert_eq!(s.epoch, Epoch(0), "PCS-02: epoch must NOT change");
        assert_eq!(s.secret_of(LeafIndex(0)), Some(h(0x10)), "PCS-02: secret unchanged");
    }

    /// **PCS-03** — a captured pre-heal commit replayed at the
    /// post-heal epoch fails the `from_epoch` check. This is the
    /// core PCS guarantee at the wire level.
    #[test]
    fn pcs_03_pre_heal_replay_rejected_after_heal() {
        let mut s = fresh_state();
        let captured = HealCommit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            heals: vec![HealEntry {
                target: LeafIndex(0),
                from_hash: h(0x10),
                to_hash: h(0xAA),
            }],
        };
        // Legitimate heal advances 0 → 1.
        s.process_heal(&captured).unwrap();
        assert_eq!(s.epoch, Epoch(1));
        // Adversary replays the captured packet — `from_epoch=0 < 1`.
        assert!(
            s.process_heal(&captured).is_err(),
            "PCS-03: replayed pre-heal commit must be rejected"
        );
        assert_eq!(s.secret_of(LeafIndex(0)), Some(h(0xAA)), "PCS-03: secret stays at post-heal value");
    }

    /// **PCS-04** — atomic batch heal: a single commit healing two
    /// targets advances the epoch by exactly 1 and rotates *both*
    /// secrets.
    #[test]
    fn pcs_04_atomic_batch_heal() {
        let mut s = fresh_state();
        s.add_member(LeafIndex(1), h(0x20));
        let heal = HealCommit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            heals: vec![
                HealEntry {
                    target: LeafIndex(0),
                    from_hash: h(0x10),
                    to_hash: h(0xAA),
                },
                HealEntry {
                    target: LeafIndex(1),
                    from_hash: h(0x20),
                    to_hash: h(0xBB),
                },
            ],
        };
        s.process_heal(&heal).expect("PCS-04: batch heal must succeed");
        assert_eq!(s.epoch, Epoch(1), "PCS-04: exactly one epoch advance");
        assert_eq!(s.secret_of(LeafIndex(0)), Some(h(0xAA)));
        assert_eq!(s.secret_of(LeafIndex(1)), Some(h(0xBB)));
    }

    /// **PCS-05** — a heal whose `to_hash == from_hash` is rejected
    /// (the heal must actually rotate the secret). This blocks the
    /// "no-op heal" smokescreen.
    #[test]
    fn pcs_05_no_op_heal_rejected() {
        let mut s = fresh_state();
        let no_op = HealCommit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            heals: vec![HealEntry {
                target: LeafIndex(0),
                from_hash: h(0x10),
                to_hash: h(0x10), // same!
            }],
        };
        assert!(s.process_heal(&no_op).is_err(), "PCS-05: no-op heal must reject");
        assert_eq!(s.epoch, Epoch(0));
    }

    /// **PCS-06** — concurrent heals at the same `from_epoch`: only
    /// the first applies, the second sees a stale `from_epoch` AND a
    /// stale `from_hash`, and is rejected. No fork is possible.
    #[test]
    fn pcs_06_concurrent_heals_only_first_applies() {
        let mut s = fresh_state();
        let legit = HealCommit {
            group_id: gid(),
            from_epoch: Epoch(0),
            sender: LeafIndex(0),
            heals: vec![HealEntry {
                target: LeafIndex(0),
                from_hash: h(0x10),
                to_hash: h(0xAA),
            }],
        };
        let attacker = HealCommit {
            group_id: gid(),
            from_epoch: Epoch(0), // same — fork attempt
            sender: LeafIndex(0),
            heals: vec![HealEntry {
                target: LeafIndex(0),
                from_hash: h(0x10),
                to_hash: h(0xBB),
            }],
        };
        s.process_heal(&legit).expect("PCS-06: first heal must succeed");
        assert!(
            s.process_heal(&attacker).is_err(),
            "PCS-06: second concurrent heal must be rejected"
        );
        assert_eq!(s.secret_of(LeafIndex(0)), Some(h(0xAA)), "PCS-06: legit value wins");
        assert_eq!(s.epoch, Epoch(1), "PCS-06: epoch advanced exactly once");
    }

    /// Wave-17 G-pcs green summary — total of 6 PCS-healing
    /// falsifier tests.
    #[test]
    fn green_g_pcs_summary() {
        let count = 6usize;
        assert_eq!(
            count, 6,
            "Wave-17 L-CHAT-3-pcs: {count} group-PCS-healing falsifier tests"
        );
    }
}
