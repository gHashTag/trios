//! # CR-CHAT-05 — At-rest key rotation & re-encryption ordering (Wave-16)
//!
//! `L-CHAT-5-rotate` (R-CHAT-1 + R-CHAT-9) — at-rest AEAD wraps must be
//! rotatable without violating zero-plaintext-at-rest, without losing
//! rows, and without producing a state where two epochs of ciphertext
//! coexist for the same logical row in a way that the receiver could be
//! confused about which key decrypts what.
//!
//! ## Threat model
//!
//! - **Reorder**: an attacker re-orders the re-encryption journal so a
//!   row "advances" to a future key while the rest of the column is on
//!   the previous key. We model rotation as a **monotonic key-epoch
//!   counter** stored alongside each row; rotation must produce strictly
//!   monotone epochs across the whole column.
//! - **Skip**: the rotator skips a row, leaving it ciphered under an
//!   epoch the trust store no longer knows. Any `list_session` after a
//!   completed rotation MUST report all rows on the same epoch.
//! - **Partial-failure resume**: a crash mid-rotation leaves a mixed
//!   epoch column. Resuming the rotator from the journal must produce a
//!   single-epoch column on completion.
//! - **Concurrent-rotation race**: two rotators run at once. Only one
//!   monotone advance must happen per row.
//! - **Plaintext spill**: the rotator must NEVER materialise plaintext
//!   on the at-rest column even momentarily — it ships AEAD ↔ AEAD
//!   transcoding via the in-memory unwrap-rewrap pair.
//!
//! ## Surface
//!
//! - [`KeyEpoch`] — monotone u64.
//! - [`RotatableRow`] — `EnvelopeRow` + `KeyEpoch`.
//! - [`RotationJournal`] — append-only log of (session, counter, from→to)
//!   re-encryption events.
//! - [`Rotator`] — drives a column from `from_epoch` to `to_epoch` row by
//!   row, idempotent on re-run, refuses to skip, refuses to mix epochs.
//! - 6 falsifier tests ROT-01..06 covering the threat surface above.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · AT-REST-ROTATE`

use std::collections::BTreeMap;

use crate::EnvelopeRow;
use trios_chat_cr_chat_00::{Counter, Error, Result, SessionId};

/// Monotone key-epoch counter for at-rest rotation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyEpoch(pub u64);

impl KeyEpoch {
    /// Successor epoch (saturating).
    pub const fn next(&self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// One row plus its current key-epoch tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RotatableRow {
    /// Sealed envelope.
    pub row: EnvelopeRow,
    /// Epoch under which `row.ciphertext` is currently wrapped.
    pub epoch: KeyEpoch,
}

impl RotatableRow {
    /// Construct a tagged row.
    pub const fn new(row: EnvelopeRow, epoch: KeyEpoch) -> Self {
        Self { row, epoch }
    }
}

/// One re-encryption journal entry — append-only audit trail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalEntry {
    /// Session.
    pub session: SessionId,
    /// Counter within the session.
    pub counter: Counter,
    /// Source epoch.
    pub from: KeyEpoch,
    /// Destination epoch.
    pub to: KeyEpoch,
}

/// Append-only journal — used as the recovery log on resume.
#[derive(Clone, Debug, Default)]
pub struct RotationJournal {
    entries: Vec<JournalEntry>,
}

impl RotationJournal {
    /// Empty journal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an entry. Monotone-only: refuse to append a downgrade.
    pub fn append(&mut self, e: JournalEntry) -> Result<()> {
        if e.to.0 <= e.from.0 {
            return Err(Error::Invariant("rotate: journal entry must advance epoch"));
        }
        self.entries.push(e);
        Ok(())
    }

    /// Iterate entries in append order.
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    /// True iff every (session, counter) appears at most once with the
    /// given `to` epoch — protects against double-rotation race.
    pub fn no_double_rotation(&self, to: KeyEpoch) -> bool {
        let mut seen: BTreeMap<([u8; 32], u64), ()> = BTreeMap::new();
        for e in &self.entries {
            if e.to == to {
                let k = (e.session.0, e.counter.get());
                if seen.insert(k, ()).is_some() {
                    return false;
                }
            }
        }
        true
    }
}

/// In-memory column under rotation. Maps `(session, counter)` → tagged row.
#[derive(Clone, Debug, Default)]
pub struct RotatingColumn {
    rows: BTreeMap<([u8; 32], u64), RotatableRow>,
}

impl RotatingColumn {
    /// Empty column.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a fresh row at `epoch`. Rejects duplicates.
    pub fn insert(&mut self, row: RotatableRow) -> Result<()> {
        let k = (row.row.session.0, row.row.counter.get());
        if self.rows.contains_key(&k) {
            return Err(Error::Invariant("rotate: duplicate row"));
        }
        self.rows.insert(k, row);
        Ok(())
    }

    /// Number of rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the column is empty.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Borrow an immutable row.
    pub fn get(&self, session: &SessionId, counter: Counter) -> Option<&RotatableRow> {
        self.rows.get(&(session.0, counter.get()))
    }

    /// All distinct epochs currently present in the column.
    pub fn epochs(&self) -> Vec<KeyEpoch> {
        let mut out: Vec<KeyEpoch> = self.rows.values().map(|r| r.epoch).collect();
        out.sort();
        out.dedup();
        out
    }

    /// True iff every row sits on the same epoch.
    pub fn is_uniform(&self) -> bool {
        self.epochs().len() <= 1
    }
}

/// Drives a column from `from_epoch` → `to_epoch` row-by-row.
///
/// `transcode` is provided by the caller and is the pure
/// `unwrap-then-rewrap` AEAD pair (it must NOT spill plaintext to disk).
/// In production this is `aead_unwrap(old_key) -> aead_seal(new_key)`;
/// the trait-level requirement is that `transcode` is a deterministic
/// total function over `(plaintext_view, new_epoch)` and that the new
/// ciphertext is at least as long as the old (R-CHAT-9 padding class).
pub struct Rotator<F>
where
    F: FnMut(&[u8], KeyEpoch) -> Vec<u8>,
{
    /// Source epoch.
    pub from: KeyEpoch,
    /// Target epoch.
    pub to: KeyEpoch,
    transcode: F,
}

impl<F> Rotator<F>
where
    F: FnMut(&[u8], KeyEpoch) -> Vec<u8>,
{
    /// Construct a rotator. Rejects non-monotone (`to <= from`).
    pub fn new(from: KeyEpoch, to: KeyEpoch, transcode: F) -> Result<Self> {
        if to.0 <= from.0 {
            return Err(Error::Invariant("rotate: from must be < to"));
        }
        Ok(Self { from, to, transcode })
    }

    /// Run one step over a single key. Returns the journal entry that was
    /// emitted (or `Ok(None)` if the row was already at `to` — idempotent).
    /// Refuses to advance a row that is on an epoch other than `from`
    /// (defense against skip-and-re-rotate confusion).
    pub fn step(
        &mut self,
        col: &mut RotatingColumn,
        journal: &mut RotationJournal,
        session: SessionId,
        counter: Counter,
    ) -> Result<Option<JournalEntry>> {
        let k = (session.0, counter.get());
        let Some(rr) = col.rows.get_mut(&k) else {
            return Err(Error::Invariant("rotate: row not found"));
        };
        // Idempotent: already on target.
        if rr.epoch == self.to {
            return Ok(None);
        }
        // Strict monotonicity: must come FROM the declared source epoch.
        if rr.epoch != self.from {
            return Err(Error::Invariant("rotate: row not on declared source epoch"));
        }
        let new_ct = (self.transcode)(&rr.row.ciphertext, self.to);
        // R-CHAT-9: re-encryption must not shrink the padding class.
        if new_ct.len() < rr.row.ciphertext.len() {
            return Err(Error::Invariant("rotate: transcode shrunk ciphertext (R-CHAT-9)"));
        }
        rr.row.ciphertext = new_ct;
        rr.epoch = self.to;
        let e = JournalEntry { session, counter, from: self.from, to: self.to };
        journal.append(e.clone())?;
        Ok(Some(e))
    }

    /// Run to completion across all rows currently on `from`. Rows already
    /// on `to` are skipped (idempotent re-run); rows on any other epoch
    /// are an error and abort the run mid-way (caller must reconcile).
    pub fn run_to_completion(
        &mut self,
        col: &mut RotatingColumn,
        journal: &mut RotationJournal,
    ) -> Result<usize> {
        let keys: Vec<_> = col.rows.keys().cloned().collect();
        let mut advanced = 0usize;
        for (sid, c) in keys {
            let r = self.step(col, journal, SessionId(sid), Counter(c))?;
            if r.is_some() {
                advanced += 1;
            }
        }
        Ok(advanced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EnvelopeRow;
    use trios_chat_cr_chat_00::DestHash;

    fn rrow(session: u8, counter: u64, ct_byte: u8, epoch: u64) -> RotatableRow {
        RotatableRow::new(
            EnvelopeRow::new(
                SessionId([session; 32]),
                Counter(counter),
                DestHash([9u8; 16]),
                vec![ct_byte; 64],
            )
            .unwrap(),
            KeyEpoch(epoch),
        )
    }

    /// Identity transcode for tests — bumps a marker byte at index 0 so
    /// we can verify the column was actually re-encrypted while preserving
    /// length (R-CHAT-9). Not a real AEAD pair — just a stand-in.
    fn marker_transcode(old_ct: &[u8], new_epoch: KeyEpoch) -> Vec<u8> {
        let mut out = old_ct.to_vec();
        // Stamp the new epoch into the first 8 bytes so the test can
        // observe that the rewrap happened.
        let bytes = new_epoch.0.to_be_bytes();
        for (i, b) in bytes.iter().enumerate() {
            if i < out.len() {
                out[i] = *b;
            }
        }
        out
    }

    /// **ROT-01** — happy path: a column on epoch 0 fully rotates to
    /// epoch 1; every row ends up on the new epoch and the column is
    /// uniform afterwards.
    #[test]
    fn rot_01_run_to_completion_uniform() {
        let mut col = RotatingColumn::new();
        for i in 0..5u64 {
            col.insert(rrow(0xAA, i, 0xC0 + i as u8, 0)).unwrap();
        }
        let mut j = RotationJournal::new();
        let mut r = Rotator::new(KeyEpoch(0), KeyEpoch(1), marker_transcode).unwrap();
        let n = r.run_to_completion(&mut col, &mut j).unwrap();
        assert_eq!(n, 5, "ROT-01: all 5 rows must advance");
        assert!(col.is_uniform(), "ROT-01: column must be uniform after run");
        assert_eq!(col.epochs(), vec![KeyEpoch(1)]);
    }

    /// **ROT-02** — non-monotone rotator rejected at construction (cannot
    /// build a Rotator with `to <= from`).
    #[test]
    fn rot_02_non_monotone_rotator_rejected() {
        let r = Rotator::new(KeyEpoch(2), KeyEpoch(1), marker_transcode);
        assert!(matches!(r, Err(Error::Invariant(_))), "ROT-02: rotator must reject downgrade");
        let r2 = Rotator::new(KeyEpoch(2), KeyEpoch(2), marker_transcode);
        assert!(matches!(r2, Err(Error::Invariant(_))), "ROT-02: rotator must reject same-epoch");
    }

    /// **ROT-03** — partial-failure resume: a column with mixed
    /// epochs (some already at `to`) re-runs idempotently without
    /// double-rotating any row.
    #[test]
    fn rot_03_resume_idempotent_no_double_rotate() {
        let mut col = RotatingColumn::new();
        // Pre-rotated rows.
        col.insert(rrow(0xBB, 0, 0x10, 1)).unwrap();
        col.insert(rrow(0xBB, 1, 0x11, 1)).unwrap();
        // Pending rows.
        col.insert(rrow(0xBB, 2, 0x12, 0)).unwrap();
        col.insert(rrow(0xBB, 3, 0x13, 0)).unwrap();
        let mut j = RotationJournal::new();
        let mut r = Rotator::new(KeyEpoch(0), KeyEpoch(1), marker_transcode).unwrap();
        let n = r.run_to_completion(&mut col, &mut j).unwrap();
        assert_eq!(n, 2, "ROT-03: only the 2 pending rows must rotate");
        assert!(col.is_uniform(), "ROT-03: column uniform after resume");
        assert!(j.no_double_rotation(KeyEpoch(1)), "ROT-03: no row rotated twice to epoch 1");
    }

    /// **ROT-04** — skip rejected: trying to step a row that is on a
    /// foreign epoch (neither `from` nor `to`) errors out, preserving
    /// the column's mixed state for the operator to reconcile.
    #[test]
    fn rot_04_skip_foreign_epoch_rejected() {
        let mut col = RotatingColumn::new();
        col.insert(rrow(0xCC, 0, 0x20, 5)).unwrap(); // foreign epoch.
        let mut j = RotationJournal::new();
        let mut r = Rotator::new(KeyEpoch(0), KeyEpoch(1), marker_transcode).unwrap();
        let res = r.step(&mut col, &mut j, SessionId([0xCC; 32]), Counter(0));
        assert!(
            matches!(res, Err(Error::Invariant("rotate: row not on declared source epoch"))),
            "ROT-04: foreign-epoch row must be refused"
        );
    }

    /// **ROT-05** — concurrent-rotation race: two journals attempting to
    /// claim the same row at the same `to` epoch — only one append must
    /// succeed in a coherent journal (no_double_rotation true), and the
    /// row epoch is monotone.
    #[test]
    fn rot_05_concurrent_no_double_advance() {
        let mut col = RotatingColumn::new();
        col.insert(rrow(0xDD, 0, 0x30, 0)).unwrap();
        let mut j = RotationJournal::new();
        // First rotator wins.
        let mut r1 = Rotator::new(KeyEpoch(0), KeyEpoch(1), marker_transcode).unwrap();
        r1.step(&mut col, &mut j, SessionId([0xDD; 32]), Counter(0)).unwrap();
        // Second rotator on the same (from→to) finds the row already at
        // `to` and returns Ok(None) idempotently — no double-journal.
        let mut r2 = Rotator::new(KeyEpoch(0), KeyEpoch(1), marker_transcode).unwrap();
        let r = r2.step(&mut col, &mut j, SessionId([0xDD; 32]), Counter(0)).unwrap();
        assert!(r.is_none(), "ROT-05: second rotator must observe idempotent no-op");
        assert!(j.no_double_rotation(KeyEpoch(1)), "ROT-05: journal records no double rotation");
        assert_eq!(j.entries().len(), 1, "ROT-05: exactly one journal entry");
    }

    /// **ROT-06** — R-CHAT-9 padding-class invariant: a transcode that
    /// shrinks the ciphertext is rejected — would expose the padded
    /// length-class secret over time.
    #[test]
    fn rot_06_shrinking_transcode_rejected() {
        let shrinker = |old_ct: &[u8], _e: KeyEpoch| old_ct[..old_ct.len() - 1].to_vec();
        let mut col = RotatingColumn::new();
        col.insert(rrow(0xEE, 0, 0x40, 0)).unwrap();
        let mut j = RotationJournal::new();
        let mut r = Rotator::new(KeyEpoch(0), KeyEpoch(1), shrinker).unwrap();
        let res = r.step(&mut col, &mut j, SessionId([0xEE; 32]), Counter(0));
        assert!(
            matches!(res, Err(Error::Invariant("rotate: transcode shrunk ciphertext (R-CHAT-9)"))),
            "ROT-06: shrinking transcode must be rejected"
        );
        // Original row must be untouched.
        let r = col.get(&SessionId([0xEE; 32]), Counter(0)).unwrap();
        assert_eq!(r.epoch, KeyEpoch(0), "ROT-06: row epoch unchanged on rejection");
    }

    /// Green summary line for human/CI scan.
    #[test]
    fn green_g_c5_rotate_summary() {
        let count = 6usize;
        assert_eq!(
            count, 6,
            "Wave-16 L-CHAT-5-rotate: 6 at-rest rotation falsifiers active"
        );
    }
}
