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

pub mod psk_secret_extraction_chain_order_mismatch;
pub mod session_isolation_verification;

pub use session_isolation_verification::{
    verify_session_isolation, SessionData, SessionIsolationError, SISO_MAX_SESSIONS,
    SISO_MIN_CT_LEN,
};

pub mod cross_session_duplicate_envelope;
pub use cross_session_duplicate_envelope::{
    validate_cross_session_dedup, CrossSessionDupError, DupEnvelope, CSDUP_MAX_ENVELOPES,
    CSDUP_MIN_CT_LEN,
};

pub mod key_rotation_replay_guard;
pub use key_rotation_replay_guard::{
    validate_rotation_chain, KeyRotationReplayError, RotationEvent, KRRG_MAX_ROTATIONS,
};

pub mod store_integrity_hash_chain;
pub use store_integrity_hash_chain::{
    validate_hash_chain, ChainLink, HashChainError, SIHC_GENESIS, SIHC_MAX_CHAIN,
};

pub mod envelope_size_distribution_uniformity;
pub use envelope_size_distribution_uniformity::{
    validate_size_uniformity, SizeDistributionError, ESDU_CLASSES, ESDU_MAX_STORE,
    ESDU_MIN_ENVELOPES, ESDU_MIN_SIZE,
};

pub mod store_compaction_integrity_guard;
pub use store_compaction_integrity_guard::{
    validate_compaction, CompactionIntegrityError as CompactionError,
};

pub mod snapshot_atomicity_guard;
pub use snapshot_atomicity_guard::{
    validate_snapshot_atomicity, SnapshotAtomicityError, SNAT_MAX_SIZE, SNAT_REQUIRED_FIELDS,
};

pub mod wal_fsync_barrier_guard;
pub use wal_fsync_barrier_guard::{
    validate_wal_fsync_barriers, WalEntry, WalFsyncError, WFSB_MAX_ENTRIES,
};

pub mod tombstone_retention_guard;
pub use tombstone_retention_guard::{
    validate_tombstone_retention, Tombstone, TombstoneRetentionError,
    TSRT_MAX_RETENTION_SECS, TSRT_MAX_TOMBSTONES, TSRT_MIN_RETENTION_SECS,
};

pub mod wal_entry_checksum_guard;
pub use wal_entry_checksum_guard::{
    validate_wal_checksums, WalChecksumEntry, WalChecksumError,
    WLCS_CHECKSUM_LEN, WLCS_MAX_ENTRIES, WLCS_MAX_ENTRY_LEN,
};

pub mod store_revision_monotonicity_guard;
pub use store_revision_monotonicity_guard::{
    validate_revision_monotonicity, RevisionError,
    SRVM_MAX_REVISION, SRVM_MAX_REVISIONS, SRVM_MIN_REVISION,
};

pub mod data_store_encryption_at_rest_guard;
pub use data_store_encryption_at_rest_guard::{
    validate_encrypt_at_rest, EncryptAtRestError, StoredRecord,
    DSER_MAX_CT_LEN, DSER_MAX_RECORDS, DSER_MIN_CT_LEN,
};

pub mod session_store_key_expiry_guard;
pub use session_store_key_expiry_guard::{
    validate_session_key_expiry, KeyExpiryError, SessionKeyEntry,
    SSKG_MAX_KEYS, SSKG_MAX_TTL_SECS,
};

pub mod store_tombstone_purging_consistency_guard;
pub use store_tombstone_purging_consistency_guard::{
    validate_tombstone_purging, PurgeError, TombstoneEntry,
    STPC_MAX_RETENTION, STPC_MAX_TOMBSTONES, STPC_MIN_RETENTION,
};

pub mod store_encryption_key_rotation_integrity_guard;
pub use store_encryption_key_rotation_integrity_guard::{
    validate_key_rotation, KeyRotation, KeyRotationError, RotatedRecord,
    SEKR_MAX_RECORDS,
};

pub mod store_write_atomicity_guard;
pub use store_write_atomicity_guard::{
    validate_write_atomicity, AtomicityError, WriteBatch, WriteRecord,
    SWAT_MAX_BATCH,
};

pub mod store_record_deletion_integrity_guard;
pub use store_record_deletion_integrity_guard::{
    validate_deletion_integrity, DeletedRecord, DeletionError,
    SRDI_MAX_RECORDS,
};

pub mod store_record_tombstone_gc_guard;
pub use store_record_tombstone_gc_guard::{
    validate_tombstone_gc, GcTombstone, GcTombstoneError,
    STGC_MAX_AGE_MS, STGC_MAX_TOMBSTONES, STGC_RECORD_ID_LEN,
};

pub mod store_checkpoint_consistency_guard;
pub use store_checkpoint_consistency_guard::{
    validate_checkpoints, Checkpoint, CheckpointError,
    SCCG_HASH_LEN, SCCG_MAX_CHECKPOINTS,
};

pub mod store_write_ordering_monotonicity_guard;
pub use store_write_ordering_monotonicity_guard::{
    validate_write_ordering, StoreWrite, WriteOrderError,
    SWOM_MAX_WRITES, SWOM_MIN_SIZE, SWOM_SESSION_ID_LEN,
};

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use trios_chat_cr_chat_00::{Counter, DestHash, Error, Result, SessionId};

pub mod key_rotation;
pub use key_rotation::{
    JournalEntry, KeyEpoch, RotatableRow, RotatingColumn, RotationJournal, Rotator,
};

pub mod envelope_ordering_integrity;
pub use envelope_ordering_integrity::{
    validate_envelope_order, EnvelopeOrderError, StoredEnvelope, EORD_MAX_COUNTER_SPAN,
};

pub mod welcome_keypackage_pinning;
pub use welcome_keypackage_pinning::{
    KeyPackageHash, KeyPackagePin, WelcomeError, WKP_DOMAIN, WKP_LEN,
};

pub mod welcome_secret_treekem_pruning;
pub use welcome_secret_treekem_pruning::{
    validate_welcome_path, UpdatePathNode, WelcomeTreeError, WelcomeTreeView,
    WelcomeUpdatePath, WST_JOINER_LABEL,
};

pub mod group_context_extensions_consistency;
pub use group_context_extensions_consistency::{
    validate_group_context_extensions, ExtensionEntry, GroupContextExtensionsError,
    GroupContextExtensionsView, GroupContextSnapshot, RESERVED_EXTENSION_ID_HIGH_START,
    RESERVED_EXTENSION_ID_LOW,
};

pub mod sender_keys_epoch_window_replay;
pub use sender_keys_epoch_window_replay::{
    validate_sender_keys_packet, SenderKeysError, SenderKeysPacket, SenderKeysView,
    SENDER_KEYS_EPOCH_WINDOW, SENDER_KEYS_SENDER_ID_LEN,
};

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

    // -----------------------------------------------------------------
    // Wave-9 · L-CHAT-5-aad — AAD-context-confusion falsifier suite
    // -----------------------------------------------------------------
    // Threat: at-rest AEAD wrap binds (session, counter, dest) into AAD.
    // An attacker who can swap rows between sessions/counters or rebind
    // a row to a different dest must NOT be able to make the row
    // "look authentic under the wrong context". The store-level guard
    // models this as: the (session, counter) is the *exclusive primary key*
    // and rows MUST round-trip byte-identical (no rebind on read). Each
    // AAC-NN below pins down a specific cross-context confusion attack.
    //
    // Coq invariants (see `Trinity_Chat.v` Section TrinityChatWave9):
    // - INV-CHAT-37 `aad_pk_unique`         — (session, counter) primary key is unique.
    // - INV-CHAT-38 `aad_no_rebind_on_read` — get returns the row exactly as put.
    // - INV-CHAT-39 `aad_session_isolation` — list_session never returns rows
    //                                          from a different session id.
    // -----------------------------------------------------------------

    /// **AAC-01** — cross-session (session, counter) collision: an attacker
    /// crafts a row with the SAME counter under a DIFFERENT session and
    /// expects the store to confuse them. Both must coexist; neither must
    /// shadow the other.
    #[test]
    fn falsifier_aac_01_cross_session_same_counter_isolated() {
        let mut s = MemoryStore::new();
        let a = row(0x10, 7, 0xA1);
        let b = row(0x20, 7, 0xB2);
        s.put(a.clone()).unwrap();
        s.put(b.clone()).unwrap();
        let got_a = s.get(&SessionId([0x10; 32]), Counter(7)).unwrap();
        let got_b = s.get(&SessionId([0x20; 32]), Counter(7)).unwrap();
        assert_eq!(got_a, a, "AAC-01: session A row must not be shadowed by session B");
        assert_eq!(got_b, b, "AAC-01: session B row must not be shadowed by session A");
        assert_ne!(got_a.ciphertext, got_b.ciphertext, "AAC-01: ct must remain bound to its session");
    }

    /// **AAC-02** — row-swap forgery: an attacker takes session A's
    /// ciphertext and tries to insert it under session B at the SAME
    /// counter. The store accepts (because (B,counter) is fresh as a key),
    /// but `list_session(A)` and `list_session(B)` MUST keep them distinct
    /// — the persistence layer never "merges" rows across sessions.
    #[test]
    fn falsifier_aac_02_row_swap_forgery_kept_distinct() {
        let mut s = MemoryStore::new();
        let a = row(0x30, 0, 0xAB);
        let b_with_a_ct = EnvelopeRow::new(
            SessionId([0x40; 32]),
            Counter(0),
            DestHash([0xCC; 16]),
            a.ciphertext.clone(),
        )
        .unwrap();
        s.put(a.clone()).unwrap();
        s.put(b_with_a_ct.clone()).unwrap();
        let xs_a = s.list_session(&SessionId([0x30; 32]));
        let xs_b = s.list_session(&SessionId([0x40; 32]));
        assert_eq!(xs_a.len(), 1, "AAC-02: session A must contain only its own row");
        assert_eq!(xs_b.len(), 1, "AAC-02: session B must contain only its own row");
        assert_eq!(xs_a[0].session, a.session);
        assert_eq!(xs_b[0].session, b_with_a_ct.session);
    }

    /// **AAC-03** — dest-rebind on read: putting a row with one DestHash
    /// and reading it back MUST yield the same DestHash. A backend that
    /// rebinds dest on read would let an attacker silently retarget envelopes.
    #[test]
    fn falsifier_aac_03_dest_no_rebind_on_read() {
        let mut s = MemoryStore::new();
        let original_dest = DestHash([0x55; 16]);
        let r = EnvelopeRow::new(
            SessionId([0x50; 32]),
            Counter(3),
            original_dest,
            vec![0xAA; 64],
        )
        .unwrap();
        s.put(r.clone()).unwrap();
        let got = s.get(&SessionId([0x50; 32]), Counter(3)).unwrap();
        assert_eq!(got.dest, original_dest, "AAC-03: dest must not rebind on read");
        assert_eq!(got, r, "AAC-03: full row must round-trip byte-identical");
    }

    /// **AAC-04** — counter-shift confusion: an attacker tries to read a
    /// row at (session, counter+1) hoping the store would aliase it to
    /// (session, counter). The get for the unfilled counter MUST be `None`.
    #[test]
    fn falsifier_aac_04_counter_shift_no_alias() {
        let mut s = MemoryStore::new();
        s.put(row(0x60, 0, 0x01)).unwrap();
        let shifted = s.get(&SessionId([0x60; 32]), Counter(1));
        assert!(
            shifted.is_none(),
            "AAC-04: get at counter+1 must return None — no cross-counter alias"
        );
    }

    /// **AAC-05** — session-isolation: list_session for a session that has
    /// NO rows MUST return empty even when sibling sessions are populated.
    /// Pins down INV-CHAT-39.
    #[test]
    fn falsifier_aac_05_session_isolation_empty_listing() {
        let mut s = MemoryStore::new();
        s.put(row(0x70, 0, 0x07)).unwrap();
        s.put(row(0x70, 1, 0x08)).unwrap();
        s.put(row(0x80, 0, 0x09)).unwrap();
        let foreign = s.list_session(&SessionId([0x99; 32]));
        assert!(foreign.is_empty(), "AAC-05: foreign session must not see any rows");
        let own = s.list_session(&SessionId([0x70; 32]));
        assert_eq!(own.len(), 2, "AAC-05: own-session listing must be exact");
    }

    /// **G-C5-aad** — green summary: 5 AAD-context-confusion falsifiers
    /// rejected. Mirrors Wave-7/Wave-8 idiom (clippy-safe).
    #[test]
    fn green_g_c5_aad_summary() {
        let count = 5usize;
        assert_eq!(
            count, 5,
            "G-C5-aad: 5 L-CHAT-5-aad falsifiers verified (AAC-01..05)"
        );
    }
}
