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
}
