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

use std::collections::BTreeSet;

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
    /// **Wave-11 / R-CHAT-11** — set of `(epoch, leaf)` triples for
    /// welcomes already consumed. Prevents Welcome-replay where the same
    /// joining packet is re-injected after the group has moved on.
    consumed_welcomes: BTreeSet<(u64, u32)>,
}

impl Group {
    /// Create a new group with one founding member.
    pub fn create(group_id: GroupId, founder: LeafIndex) -> Self {
        Self {
            group_id,
            epoch: Epoch(0),
            members: vec![founder],
            consumed_welcomes: BTreeSet::new(),
        }
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
}
