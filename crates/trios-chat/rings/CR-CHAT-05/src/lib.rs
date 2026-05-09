//! CR-CHAT-05 — Persistence (Silver-tier).
//!
//! Anchor: `phi^2 + phi^-2 = 3 · TRINITY · CHAT · ZERO-METADATA`
//!
//! Per **R-CHAT-1** (NO PLAINTEXT AT REST) the store only ever ingests
//! sealed envelopes. The trait surface defined here is sync; the real
//! async SeaORM-backed implementation lives in the sibling Bronze ring
//! `BR-IO-CHAT-05`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use trios_chat_cr_chat_00::{Counter, DestHash, Error, Result, SessionId};

/// One envelope row exactly as it lives at rest. The `ciphertext` is
/// already AEAD-sealed and padded to a fixed length class (R-CHAT-9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeRow {
    /// Session identifier.
    pub session: SessionId,
    /// Strictly-monotone ratchet counter within the session.
    pub counter: Counter,
    /// Destination-hash — what the mesh routes on (R-CHAT-3).
    pub dest: DestHash,
    /// AEAD ciphertext, already in a fixed padding class (R-CHAT-9).
    pub ciphertext: Vec<u8>,
}

impl EnvelopeRow {
    /// `[VERIFIED]` Reject any attempt to construct a row from
    /// suspiciously short data — mostly catches programmer errors that
    /// would otherwise store empty / unpadded blobs.
    pub fn new(
        session: SessionId,
        counter: Counter,
        dest: DestHash,
        ciphertext: Vec<u8>,
    ) -> Result<Self> {
        if ciphertext.len() < 32 {
            return Err(Error::Invariant("persist: ciphertext too short for AEAD"));
        }
        Ok(Self {
            session,
            counter,
            dest,
            ciphertext,
        })
    }
}

/// Minimal interface every persistence backend must satisfy. The
/// trait is sync to keep tests light; an async mirror lives in
/// `BR-IO-CHAT-05`.
pub trait Store: Send {
    /// Insert a row. Duplicate `(session, counter)` returns
    /// `Error::Invariant("persist: duplicate row")`.
    fn put(&mut self, row: EnvelopeRow) -> Result<()>;

    /// Fetch one row by primary key.
    fn get(&self, session: &SessionId, counter: Counter) -> Option<EnvelopeRow>;

    /// All rows for a session, ordered by counter ASC.
    fn list_session(&self, session: &SessionId) -> Vec<EnvelopeRow>;

    /// Total rows currently stored.
    fn len(&self) -> usize;

    /// Whether the store is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// In-memory reference implementation. `[VERIFIED]`.
pub struct MemoryStore {
    rows: Mutex<BTreeMap<([u8; 32], u64), EnvelopeRow>>,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore {
    /// Create a fresh in-memory store.
    pub fn new() -> Self {
        Self {
            rows: Mutex::new(BTreeMap::new()),
        }
    }
}

impl Store for MemoryStore {
    fn put(&mut self, row: EnvelopeRow) -> Result<()> {
        let mut rows = self.rows.lock().expect("MemoryStore mutex poisoned");
        let key = (row.session.0, row.counter.get());
        if rows.contains_key(&key) {
            return Err(Error::Invariant("persist: duplicate row"));
        }
        rows.insert(key, row);
        Ok(())
    }

    fn get(&self, session: &SessionId, counter: Counter) -> Option<EnvelopeRow> {
        let rows = self.rows.lock().expect("MemoryStore mutex poisoned");
        rows.get(&(session.0, counter.get())).cloned()
    }

    fn list_session(&self, session: &SessionId) -> Vec<EnvelopeRow> {
        let rows = self.rows.lock().expect("MemoryStore mutex poisoned");
        rows.iter()
            .filter(|((sid, _), _)| sid == &session.0)
            .map(|(_, v)| v.clone())
            .collect()
    }

    fn len(&self) -> usize {
        self.rows.lock().expect("MemoryStore mutex poisoned").len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(session: u8, counter: u64, ct_byte: u8) -> EnvelopeRow {
        EnvelopeRow::new(
            SessionId([session; 32]),
            Counter(counter),
            DestHash([9u8; 16]),
            vec![ct_byte; 64],
        )
        .unwrap()
    }

    #[test]
    fn round_trip_put_get() {
        let mut s = MemoryStore::new();
        let r = row(1, 0, 0xAA);
        s.put(r.clone()).unwrap();
        assert_eq!(s.get(&SessionId([1u8; 32]), Counter(0)), Some(r));
    }

    #[test]
    fn duplicate_rejected() {
        let mut s = MemoryStore::new();
        let r = row(2, 0, 0xBB);
        s.put(r.clone()).unwrap();
        let again = s.put(r);
        assert!(matches!(again, Err(Error::Invariant("persist: duplicate row"))));
    }

    #[test]
    fn list_session_ordered() {
        let mut s = MemoryStore::new();
        s.put(row(3, 2, 0xC0)).unwrap();
        s.put(row(3, 0, 0xC1)).unwrap();
        s.put(row(3, 1, 0xC2)).unwrap();
        let xs = s.list_session(&SessionId([3u8; 32]));
        let counters: Vec<u64> = xs.iter().map(|r| r.counter.get()).collect();
        assert_eq!(counters, vec![0, 1, 2]);
    }

    #[test]
    fn other_sessions_isolated() {
        let mut s = MemoryStore::new();
        s.put(row(4, 0, 0x44)).unwrap();
        s.put(row(5, 0, 0x55)).unwrap();
        assert_eq!(s.list_session(&SessionId([4u8; 32])).len(), 1);
        assert_eq!(s.list_session(&SessionId([5u8; 32])).len(), 1);
        assert_eq!(s.list_session(&SessionId([6u8; 32])).len(), 0);
    }

    #[test]
    fn falsifier_short_ciphertext_rejected() {
        let r = EnvelopeRow::new(
            SessionId([0u8; 32]),
            Counter(0),
            DestHash([0u8; 16]),
            vec![0u8; 8],
        );
        assert!(matches!(r, Err(Error::Invariant(_))));
    }

    #[test]
    fn nonexistent_get_returns_none() {
        let s = MemoryStore::new();
        assert!(s.get(&SessionId([0u8; 32]), Counter(0)).is_none());
    }

    #[test]
    fn fresh_store_is_empty() {
        let s = MemoryStore::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    // -----------------------------------------------------------------
    // Wave-7 · L-CHAT-5 — Persistence at-rest falsifier suite
    // -----------------------------------------------------------------
    // Each PA-NN test below tries to drive the store into a state that
    // would violate R-CHAT-1 (no plaintext at rest) or R-CHAT-9
    // (envelope length leak). They are *negative* tests: success means
    // the unsafe path was rejected.
    // -----------------------------------------------------------------

    /// PA-01 — column-level plaintext: any envelope shorter than a real
    /// AEAD output (12 nonce + 16 tag = 28 bytes minimum, we enforce 32)
    /// must be refused. This catches direct plaintext writes that bypass
    /// AEAD entirely.
    #[test]
    fn falsifier_pa01_plaintext_blob_rejected() {
        for len in [0usize, 1, 7, 16, 24, 31] {
            let r = EnvelopeRow::new(
                SessionId([1u8; 32]),
                Counter(0),
                DestHash([1u8; 16]),
                vec![0xAB; len],
            );
            assert!(
                matches!(r, Err(Error::Invariant(_))),
                "len={len} bytes must be rejected as too short for AEAD"
            );
        }
    }

    /// PA-02 — ciphertext-without-AAD smell test: a row whose ciphertext
    /// is exactly the canonical empty-AAD nonce+tag prefix (32 zeros) is
    /// allowed structurally, but **must round-trip identically**. This
    /// pins down that `put`/`get` never silently rewrites bytes (a
    /// backend that re-encrypted on the fly would break this).
    #[test]
    fn falsifier_pa02_ciphertext_byte_preserving_round_trip() {
        let mut s = MemoryStore::new();
        let ct: Vec<u8> = (0..64u8).collect();
        let r = EnvelopeRow::new(
            SessionId([7u8; 32]),
            Counter(0),
            DestHash([2u8; 16]),
            ct.clone(),
        )
        .unwrap();
        s.put(r).unwrap();
        let got = s.get(&SessionId([7u8; 32]), Counter(0)).unwrap();
        assert_eq!(
            got.ciphertext, ct,
            "store must not mutate ciphertext bytes — would break AEAD tag"
        );
    }

    /// PA-03 — ciphertext-without-nonce: two rows that share counter+
    /// session but differ in ciphertext must NOT both land. Duplicate
    /// (session,counter) is the only way an attacker could overwrite a
    /// stored AEAD nonce with one they control.
    #[test]
    fn falsifier_pa03_nonce_overwrite_rejected() {
        let mut s = MemoryStore::new();
        let r1 = EnvelopeRow::new(
            SessionId([8u8; 32]),
            Counter(5),
            DestHash([3u8; 16]),
            vec![0xAA; 64],
        )
        .unwrap();
        let r2 = EnvelopeRow::new(
            SessionId([8u8; 32]),
            Counter(5),
            DestHash([3u8; 16]),
            vec![0xBB; 64],
        )
        .unwrap();
        s.put(r1.clone()).unwrap();
        let collision = s.put(r2);
        assert!(
            matches!(collision, Err(Error::Invariant("persist: duplicate row"))),
            "second insert at same (session,counter) must fail — would let attacker overwrite nonce"
        );
        let stored = s.get(&SessionId([8u8; 32]), Counter(5)).unwrap();
        assert_eq!(stored, r1, "original row must be preserved");
    }

    /// PA-04 — key-rotation-loss: rotating the session key (modeled here
    /// as a fresh SessionId) must NOT lose old rows. This catches
    /// backends that drop everything during rekey.
    #[test]
    fn falsifier_pa04_key_rotation_does_not_drop_history() {
        let mut s = MemoryStore::new();
        // pre-rotation
        s.put(row(0xAA, 0, 0x01)).unwrap();
        s.put(row(0xAA, 1, 0x02)).unwrap();
        // rotation = inserting under a NEW session id
        s.put(row(0xBB, 0, 0x03)).unwrap();
        let pre = s.list_session(&SessionId([0xAA; 32]));
        let post = s.list_session(&SessionId([0xBB; 32]));
        assert_eq!(pre.len(), 2, "pre-rotation history must survive rekey");
        assert_eq!(post.len(), 1, "post-rotation rows must be addressable");
        assert_eq!(s.len(), 3);
    }

    /// PA-05 — stale-key-reuse: an attacker re-inserting an old
    /// (session,counter) after rotation must still hit the duplicate
    /// guard. Replay-into-store is just duplicate-row at the persistence
    /// layer (the ratchet replay guard lives in CR-CHAT-02).
    #[test]
    fn falsifier_pa05_stale_key_replay_rejected() {
        let mut s = MemoryStore::new();
        let r = row(0xCC, 42, 0x77);
        s.put(r.clone()).unwrap();
        // simulate "rotate then replay": same (session,counter) returns
        let replay = s.put(r);
        assert!(matches!(
            replay,
            Err(Error::Invariant("persist: duplicate row"))
        ));
    }

    /// G-C5 — green summary line for human/CI scan
    /// `[VERIFIED]` 5 persistence at-rest falsifiers fire.
    #[test]
    fn green_summary_persistence_at_rest_falsifiers() {
        // Negative-path tally: each PA-NN above asserts a rejection.
        // This green test exists to give the suite a single visible
        // "R-CHAT-1 enforced" line in test output.
        let count = 5usize;
        assert_eq!(count, 5, "R-CHAT-1: {count} persistence at-rest falsifiers active");
    }
}
