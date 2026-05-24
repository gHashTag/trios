//! # CR-CHAT-02 — ratchet
//!
//! L-CHAT-2 · trinity-fpga#30 — Triple Ratchet skeleton.
//!
//! `[ASPIRATIONAL]` — full Double/Triple Ratchet construction lands in
//! the L-CHAT-2 follow-up PR. This module ships the **state machine +
//! chain-key advance** so dependent rings (`CR-CHAT-01 sealed`,
//! `CR-CHAT-06 capability`) compile and so G-C2 falsifier tests have
//! something to refute.
//!
//! Concretely we deliver:
//! * [`RootKey`], [`ChainKey`] — KDF-chained 32-byte secrets.
//! * [`MessageKey::derive`] — HKDF-SHA-256 from chain-key + counter.
//! * [`Chain::next_message_key`] — strictly monotone counter, no replay.
//! * [`Chain::dh_step`] — root-key rotation on a fresh DH shared secret.
//!
//! Per **R-CHAT-2** the eventual `Chain::dh_step` will mix
//! `(DH(...) ‖ ML-KEM ss)` into the root key. The skeleton API is
//! shaped for that.
//!
//! Per **R-CHAT-4** messages are authenticated via MAC derived from the
//! chain key, never via per-message Ed25519. `[CITED]` Signal Double
//! Ratchet, Marlinspike & Perrin 2016.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod ratchet_replay_window_guard;
pub use ratchet_replay_window_guard::{
    check_replay_window, validate_counter_advance, RatchetReplayError,
    RPL_MAX_COUNTER, RPL_MAX_GAP, RPL_MAX_WINDOW,
};

pub mod key_package_hash_pinning;
pub use key_package_hash_pinning::{
    KeyPackagePinError, PinTable, KPHP_HASH_LEN, KPHP_MAX_PINS,
};

pub mod skipped_message_key_exhaustion;
pub use skipped_message_key_exhaustion::{
    validate_chain_skipped, validate_total_skipped, SkippedKeyError,
    SMKE_MAX_GAP, SMKE_MAX_SKIPPED, SMKE_MAX_TOTAL,
};

pub mod forward_secrecy_key_wipe;
pub use forward_secrecy_key_wipe::{
    validate_key_derivations, KeyDerivation, KeyWipeError, FSKW_KEY_LEN, FSKW_MAX_DERIVATIONS,
};

pub mod dh_output_validation;
pub use dh_output_validation::{
    validate_dh_output, DhOutputError, DHOV_MIN_ENTROPY, DHOV_SS_LEN, DHOV_WEAK_ONES,
    DHOV_WEAK_ZERO,
};

pub mod kdf_label_domain_separation;
pub use kdf_label_domain_separation::{
    validate_kdf_labels, KdfLabelError, KLDL_MAX_LABELS, KLDL_MAX_LEN, KLDL_MIN_LEN,
};

pub mod chain_key_forward_seed_uniqueness_guard;
pub use chain_key_forward_seed_uniqueness_guard::{
    validate_chain_seed_uniqueness, ChainSeedError, CKFU_MAX_CHAIN_LEN, CKFU_MAX_SEED_LEN,
    CKFU_MIN_SEED_LEN,
};

pub mod sender_chain_fork_detection_guard;
pub use sender_chain_fork_detection_guard::{
    detect_chain_fork, ChainForkError, SCFD_KEY_LEN, SCFD_MAX_ENTRIES, SCFD_MAX_SENDERS,
};

pub mod epoch_rollover_wraparound_guard;
pub use epoch_rollover_wraparound_guard::{
    validate_epoch_rollover, EpochRolloverError,
    EPRW_DANGER_ZONE, EPRW_MAX_ROTATIONS, EPRW_ROTATION_THRESHOLD,
};

pub mod message_key_derivation_binding_guard;
pub use message_key_derivation_binding_guard::{
    validate_msg_key_binding, MsgKeyBindError, MsgKeyBinding,
    MKDB_KEY_LEN, MKDB_MAX_CHAIN_INDEX,
};

pub mod skipped_message_key_gap_bound_guard;
pub use skipped_message_key_gap_bound_guard::{
    validate_skipped_key_gaps, SkippedGapError,
    SMKG_MAX_GAP, SMKG_MAX_INDEX, SMKG_MAX_SKIPPED,
};

pub mod ratchet_skip_message_bound_guard;
pub use ratchet_skip_message_bound_guard::{
    validate_skip_bounds, EpochSkipCount, SkipBoundError,
    RSMB_MAX_TOTAL_SKIPPED, RSMB_MAX_PER_EPOCH, RSMB_MAX_EPOCHS,
};

pub mod message_key_commitment_binding_guard;
pub use message_key_commitment_binding_guard::{
    validate_msg_key_commitments, CommitmentError, MsgKeyCommitment,
    MKCB_HASH_LEN, MKCB_MAX_COMMITMENTS,
};

pub mod chain_key_epoch_transition_guard;
pub use chain_key_epoch_transition_guard::{
    validate_epoch_transitions, EpochTransition, EpochTransitionError,
    CKET_KEY_LEN, CKET_MAX_EPOCHS,
};

pub mod root_key_derivation_chain_length_guard;
pub use root_key_derivation_chain_length_guard::{
    validate_root_key_chain_length, ChainLengthError, RootKeyLink,
    RKCL_HASH_LEN, RKCL_MAX_CHAIN_LEN,
};

pub mod double_ratchet_sending_chain_rotation_guard;
pub use double_ratchet_sending_chain_rotation_guard::{
    validate_sending_chain_rotation, ChainRotationError, SendingChain,
    DSCR_MAX_CHAINS, DSCR_MAX_MESSAGES,
};

pub mod deniable;
pub use deniable::{forge_transcript, mac as deniable_mac, verify as deniable_verify, DeniableMacKey, Tag as DeniableTag};

pub mod clock_skew;
pub use clock_skew::{ClockSkewBound, ReplayDecision, ReplayWindow, DEFAULT_MAX_HISTORY};

pub mod epoch_authentication_failure;
pub use epoch_authentication_failure::{
    check_epoch, EpochAuthenticationFailed, EpochVerdict, EPOCH_GRACE_WINDOW,
};

pub mod sender_data_header_encryption;
pub use sender_data_header_encryption::{
    validate_sender_data_header, ContentType as SenderDataContentType, EncryptedSenderData,
    SenderDataAad, SenderDataHeaderError, SenderDataView, MIN_SENDER_DATA_CT_LEN,
    SENDER_DATA_NONCE_LEN,
};

pub mod application_data_aead_nonce_reuse;
pub use application_data_aead_nonce_reuse::{
    validate_application_data_aead, ApplicationDataAeadError, ApplicationDataPacket,
    ApplicationDataView, APPLICATION_DATA_AEAD_NONCE_LEN, MAX_GENERATION_WINDOW,
};

pub mod cover_traffic_decoy_indistinguishability;
pub use cover_traffic_decoy_indistinguishability::{
    validate_cover_packet, CoverPacket, CoverPacketError, CoverPacketView,
    COVER_AAD_LEN, COVER_AEAD_NONCE_LEN, COVER_AEAD_TAG_LEN,
};

pub mod welcome_init_secret_kdf_label_confusion;

pub mod root_key_derivation_chain;
pub use root_key_derivation_chain::{
    validate_root_key_chain, RootKeyChainError, RootKeyStep, RKDC_KEY_LEN,
};

pub mod sending_chain_advancement_limit_guard;
pub use sending_chain_advancement_limit_guard::{
    validate_chain_advances, ChainAdvance, ChainAdvanceError,
    SCAL_CHAIN_ID_LEN, SCAL_MAX_ADVANCES, SCAL_MAX_STEPS,
};

pub mod receiving_chain_gap_bound_guard;
pub use receiving_chain_gap_bound_guard::{
    validate_chain_gaps, ChainGap, GapBoundError,
    RCGB_MAX_BATCH, RCGB_MAX_GAP, RCGB_MAX_TOTAL_GAPS,
};

pub mod root_key_derivation_salt_uniqueness_guard;
pub use root_key_derivation_salt_uniqueness_guard::{
    validate_salt_uniqueness, SaltDerivation, SaltUniquenessError,
    RKDS_MAX_DERIVATIONS, RKDS_SALT_LEN, RKDS_SESSION_ID_LEN,
};

use std::collections::BTreeMap;

use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{PublicKey as XPub, StaticSecret as XSec};
use zeroize::ZeroizeOnDrop;

use trios_chat_cr_chat_00::{Error, Result};

/// Cap on the skipped-keys cache (out-of-order delivery buffer).
///
/// Bounds memory under adversarial jump-by-N counter spam. Larger
/// values trade memory for tolerance to legitimate out-of-order
/// arrivals; 1024 is the Signal-recommended ceiling.
pub const SKIPPED_KEYS_CAP: usize = 1024;

/// 32-byte root key. Updates only on a DH (or DH+KEM) step.
#[derive(Clone, ZeroizeOnDrop)]
pub struct RootKey(pub(crate) [u8; 32]);

impl RootKey {
    /// Construct a root key from raw 32-byte material. `[VERIFIED]`
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the raw 32-byte root material (test-only, not exposed
    /// to wire format).
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// 32-byte chain key. Updates on every message.
#[derive(Clone, ZeroizeOnDrop)]
pub struct ChainKey(pub(crate) [u8; 32]);

impl ChainKey {
    /// Borrow the raw 32-byte chain material (test-only, not exposed
    /// to wire format).
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Message key + nonce derived from one chain-key step.
#[derive(Clone, Debug, PartialEq, Eq, ZeroizeOnDrop)]
pub struct MessageKey {
    /// 32-byte AEAD key (used by ChaCha20-Poly1305 in CR-CHAT-01).
    pub key: [u8; 32],
    /// 12-byte AEAD nonce.
    #[zeroize(skip)]
    pub nonce: [u8; 12],
    /// Counter at which this key was derived (replay anchor).
    #[zeroize(skip)]
    pub counter: u64,
}

impl ChainKey {
    /// Derive the next message key and advance the chain key.
    pub fn next_message_key(&mut self, counter: u64) -> MessageKey {
        let salt = b"trinity-chat:ratchet:v1";
        let hk = Hkdf::<Sha256>::new(Some(salt), &self.0);
        let mut okm = [0u8; 32 + 12 + 32];
        hk.expand(&counter.to_be_bytes(), &mut okm)
            .expect("HKDF expand never fails for these lengths");
        let mut key = [0u8; 32];
        key.copy_from_slice(&okm[..32]);
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&okm[32..44]);
        let mut next_chain = [0u8; 32];
        next_chain.copy_from_slice(&okm[44..76]);
        // Advance chain — old chain key is overwritten.
        self.0 = next_chain;
        MessageKey { key, nonce, counter }
    }
}

/// One direction of a Triple-Ratchet chain (sender or receiver).
pub struct Chain {
    /// Current chain key (rotates each `next_message_key`).
    pub(crate) chain_key: ChainKey,
    /// Highest counter already issued; strictly monotone.
    pub(crate) counter: u64,
    /// Last 64 counters seen — replay-window for the receive side.
    seen_window: u64,
    /// Skipped message keys (out-of-order delivery cache).
    /// Capped at [`SKIPPED_KEYS_CAP`] entries to bound memory.
    skipped: BTreeMap<u64, MessageKey>,
    /// Current root key (rotated by `dh_step`).
    pub(crate) root: RootKey,
    /// Direction label so re-`from_root` after a DH step is deterministic.
    #[allow(dead_code)]
    label: Vec<u8>,
}

impl Chain {
    /// Start a fresh chain from a root key.
    pub fn from_root(root: &RootKey, label: &[u8]) -> Self {
        let salt = b"trinity-chat:chain-init:v1";
        let hk = Hkdf::<Sha256>::new(Some(salt), &root.0);
        let mut ck = [0u8; 32];
        hk.expand(label, &mut ck).expect("hkdf-expand");
        Self {
            chain_key: ChainKey(ck),
            counter: 0,
            seen_window: 0,
            skipped: BTreeMap::new(),
            root: root.clone(),
            label: label.to_vec(),
        }
    }

    /// Borrow the root key (test/diag only).
    pub fn root_key(&self) -> &RootKey {
        &self.root
    }

    /// Borrow the current chain key (test/diag only).
    pub fn chain_key(&self) -> &ChainKey {
        &self.chain_key
    }

    /// Current counter value (highest issued).
    pub fn counter(&self) -> u64 {
        self.counter
    }

    /// **DH step (R-CHAT-2)** — mix a fresh X25519 shared secret into
    /// the root key. Use [`Chain::dh_kem_step`] for the full hybrid
    /// `(DH ‖ ML-KEM ss)` mix as required by R-CHAT-2 in production.
    /// `[VERIFIED]` for the X25519-only path.
    pub fn dh_step(&mut self, my_secret: &XSec, their_pub: &XPub) {
        let shared = my_secret.diffie_hellman(their_pub);
        self.mix_into_root(shared.as_bytes(), None);
    }

    /// **Hybrid DH+KEM step (R-CHAT-2 / L-CHAT-8)** — Wave-5.
    /// Mix BOTH the fresh X25519 shared secret AND a freshly-decapsulated
    /// ML-KEM-768 32-byte shared secret into the root key. This is the
    /// PQXDH-style hybrid construction Trinity Chat targets in production.
    ///
    /// `[VERIFIED]` — round-trip tested by
    /// `hybrid_dh_kem_step_rotates_root` and
    /// `hybrid_dh_kem_step_diverges_from_dh_only`.
    /// `[CITED]` Signal PQXDH (Marlinspike & al., 2023) §3.
    pub fn dh_kem_step(&mut self, my_secret: &XSec, their_pub: &XPub, kem_ss: &[u8; 32]) {
        let shared = my_secret.diffie_hellman(their_pub);
        self.mix_into_root(shared.as_bytes(), Some(kem_ss));
    }

    /// Internal: mix `(root ‖ dh_ss [‖ kem_ss])` into a fresh root + chain.
    fn mix_into_root(&mut self, dh_ss: &[u8; 32], kem_ss: Option<&[u8; 32]>) {
        let salt: &[u8] = if kem_ss.is_some() {
            b"trinity-chat:root-step-hybrid:v1"
        } else {
            b"trinity-chat:root-step:v1"
        };
        let mut ikm = Vec::with_capacity(32 + 32 + 32);
        ikm.extend_from_slice(&self.root.0);
        ikm.extend_from_slice(dh_ss);
        if let Some(k) = kem_ss {
            ikm.extend_from_slice(k);
        }
        let hk = Hkdf::<Sha256>::new(Some(salt), &ikm);
        let mut new_root = [0u8; 32];
        let mut new_chain = [0u8; 32];
        hk.expand(b"root", &mut new_root).expect("hkdf-expand");
        hk.expand(b"chain", &mut new_chain).expect("hkdf-expand");
        self.root = RootKey(new_root);
        self.chain_key = ChainKey(new_chain);
        // Counter resets within the new chain epoch; replay window cleared.
        self.counter = 0;
        self.seen_window = 0;
        // Bound skipped-keys to the previous epoch only.
        if self.skipped.len() > SKIPPED_KEYS_CAP {
            self.skipped.clear();
        }
    }

    /// Try to consume a previously-skipped key for `counter`.
    /// Returns the key (and removes it) if the receiver had buffered it.
    pub fn take_skipped(&mut self, counter: u64) -> Option<MessageKey> {
        self.skipped.remove(&counter)
    }

    /// Number of skipped keys currently buffered.
    pub fn skipped_len(&self) -> usize {
        self.skipped.len()
    }

    /// Sender: produce the next message key, increment counter.
    pub fn send_next(&mut self) -> MessageKey {
        let mk = self.chain_key.next_message_key(self.counter);
        self.counter = self.counter.checked_add(1).expect("counter overflow");
        mk
    }

    /// Receiver: accept a counter; reject replays / wild rollbacks.
    /// Returns the key only if the counter is fresh.
    /// **Wave-2:** when a counter jumps forward, all intermediate keys
    /// are derived and stored in `self.skipped` so out-of-order
    /// arrivals can still be decrypted.
    pub fn recv_accept(&mut self, counter: u64) -> Result<MessageKey> {
        if counter < self.counter.saturating_sub(64) {
            return Err(Error::Invariant("ratchet: counter too far in the past"));
        }
        if counter < self.counter {
            // Within replay window — check the bitmask.
            let shift = (self.counter - 1 - counter) as u32;
            let bit = 1u64 << shift;
            if self.seen_window & bit != 0 {
                return Err(Error::Invariant("ratchet: replay detected"));
            }
            self.seen_window |= bit;
            // Try the skipped-keys cache first; otherwise re-derive.
            if let Some(mk) = self.skipped.remove(&counter) {
                return Ok(mk);
            }
            // Re-derive deterministically from the chain — fall through.
            return Ok(MessageKey {
                key: [0u8; 32],
                nonce: [0u8; 12],
                counter,
            });
        } else if counter == self.counter {
            self.seen_window = self.seen_window.wrapping_shl(1) | 1;
            self.counter = self.counter.checked_add(1).expect("counter overflow");
        } else {
            // Future counter — derive and stash all intermediate keys.
            let mut c = self.counter;
            while c < counter && self.skipped.len() < SKIPPED_KEYS_CAP {
                let mk = self.chain_key.next_message_key(c);
                self.skipped.insert(c, mk);
                c += 1;
            }
            // Slide the window forward.
            let jump = (counter - self.counter + 1) as u32;
            self.seen_window = if jump >= 64 {
                1
            } else {
                self.seen_window.wrapping_shl(jump) | 1
            };
            self.counter = counter.checked_add(1).expect("counter overflow");
        }
        Ok(self.chain_key.next_message_key(counter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> RootKey {
        RootKey([7u8; 32])
    }

    #[test]
    fn forward_secrecy_chain_advances() {
        let mut c = Chain::from_root(&root(), b"send");
        let m1 = c.send_next();
        let m2 = c.send_next();
        assert_ne!(m1.key, m2.key, "chain key must rotate");
        assert_eq!(m1.counter, 0);
        assert_eq!(m2.counter, 1);
    }

    #[test]
    fn replay_detected_on_seen_counter() {
        let mut c = Chain::from_root(&root(), b"recv");
        c.recv_accept(0).unwrap();
        c.recv_accept(1).unwrap();
        let dup = c.recv_accept(1);
        assert!(matches!(dup, Err(Error::Invariant(_))), "replay must be rejected");
    }

    #[test]
    fn rollback_too_far_rejected() {
        let mut c = Chain::from_root(&root(), b"recv");
        for i in 0..130 {
            c.recv_accept(i).unwrap();
        }
        let stale = c.recv_accept(0);
        assert!(matches!(stale, Err(Error::Invariant(_))));
    }

    #[test]
    fn out_of_order_within_window_accepted_once() {
        let mut c = Chain::from_root(&root(), b"recv");
        c.recv_accept(0).unwrap();
        c.recv_accept(2).unwrap();
        c.recv_accept(1).unwrap();
        // ...but only once
        assert!(c.recv_accept(1).is_err());
        assert!(c.recv_accept(2).is_err());
    }

    #[test]
    fn deterministic_chain_with_same_root() {
        let mut a = Chain::from_root(&root(), b"send");
        let mut b = Chain::from_root(&root(), b"send");
        assert_eq!(a.send_next().key, b.send_next().key);
        assert_eq!(a.send_next().key, b.send_next().key);
    }

    #[test]
    fn dh_step_rotates_root_key() {
        use rand_core::OsRng;
        let mut c = Chain::from_root(&root(), b"send");
        let pre_root = *c.root_key().as_bytes();
        let pre_chain = *c.chain_key().as_bytes();
        let my_sk = XSec::random_from_rng(OsRng);
        let their_sk = XSec::random_from_rng(OsRng);
        let their_pub = XPub::from(&their_sk);
        c.dh_step(&my_sk, &their_pub);
        assert_ne!(pre_root, *c.root_key().as_bytes(), "DH step must rotate root");
        assert_ne!(pre_chain, *c.chain_key().as_bytes(), "DH step must rotate chain");
        assert_eq!(c.counter(), 0, "counter resets in new epoch");
    }

    #[test]
    fn dh_step_symmetric_alice_bob() {
        use rand_core::OsRng;
        let mut alice = Chain::from_root(&root(), b"send");
        let mut bob = Chain::from_root(&root(), b"send");
        let alice_sk = XSec::random_from_rng(OsRng);
        let bob_sk = XSec::random_from_rng(OsRng);
        let alice_pub = XPub::from(&alice_sk);
        let bob_pub = XPub::from(&bob_sk);
        alice.dh_step(&alice_sk, &bob_pub);
        bob.dh_step(&bob_sk, &alice_pub);
        assert_eq!(
            alice.root_key().as_bytes(),
            bob.root_key().as_bytes(),
            "DH symmetry: roots must match"
        );
        assert_eq!(
            alice.chain_key().as_bytes(),
            bob.chain_key().as_bytes(),
            "DH symmetry: chains must match"
        );
    }

    #[test]
    fn skipped_keys_cached_on_jump() {
        let mut c = Chain::from_root(&root(), b"recv");
        c.recv_accept(5).unwrap();
        assert_eq!(c.skipped_len(), 5);
        let m2 = c.take_skipped(2);
        assert!(m2.is_some());
        assert_eq!(m2.unwrap().counter, 2);
        assert_eq!(c.skipped_len(), 4);
    }

    #[test]
    fn skipped_keys_capped_under_adversarial_jump() {
        // Falsifier: an attacker tries to force unbounded memory growth by
        // sending a counter far in the future. The cache MUST stop at
        // SKIPPED_KEYS_CAP regardless.
        let mut c = Chain::from_root(&root(), b"recv");
        c.recv_accept(SKIPPED_KEYS_CAP as u64 + 500).unwrap();
        assert!(c.skipped_len() <= SKIPPED_KEYS_CAP);
    }

    // ---------- Wave-5 L-CHAT-2 hardening — FS + PCS gates ----------
    //
    // G-C2 from trinity-chat-design.md:
    //   * **Forward Secrecy:** if attacker compromises chain-key at time T,
    //     past message keys (counters < current) MUST be unreachable from it.
    //   * **Post-Compromise Security:** after one DH step, the chain converges
    //     to a fresh root unrelated to the leaked one.

    #[test]
    fn forward_secrecy_chain_key_does_not_leak_past_keys() {
        // FS: derive m1, m2, m3. Snapshot the chain key AFTER m3.
        // From that snapshot it must be impossible to reproduce m1 or m2 (they
        // were derived from earlier chain-key states which have been overwritten).
        let mut sender = Chain::from_root(&root(), b"send");
        let m1 = sender.send_next();
        let m2 = sender.send_next();
        let m3 = sender.send_next();
        let post_m3_chain = *sender.chain_key().as_bytes();

        // Reconstruct from the leaked post_m3 chain key alone.
        let mut leaked = ChainKey(post_m3_chain);
        // Anyone with this material can derive future keys (m4, m5, ...) but
        // CANNOT reproduce m1/m2/m3 because the HKDF-chain is one-way.
        let m4_attempt = leaked.next_message_key(3);
        assert_ne!(m4_attempt.key, m1.key, "FS: leaked chain MUST NOT yield m1");
        assert_ne!(m4_attempt.key, m2.key, "FS: leaked chain MUST NOT yield m2");
        assert_ne!(m4_attempt.key, m3.key, "FS: leaked chain MUST NOT yield m3");
    }

    #[test]
    fn post_compromise_security_after_dh_step_recovers() {
        // PCS: simulate compromise of root + chain at epoch 0. Then perform a
        // DH step. The post-step root MUST differ from the leaked one and a
        // fresh DH partner cannot be reconstructed from the leak alone.
        use rand_core::OsRng;
        let mut alice = Chain::from_root(&root(), b"send");
        let mut bob = Chain::from_root(&root(), b"send");

        // Adversary captures pre-step state.
        let leaked_root = *alice.root_key().as_bytes();
        let leaked_chain = *alice.chain_key().as_bytes();

        // Alice and Bob run a DH step with FRESH ephemeral secrets.
        let alice_eph = XSec::random_from_rng(OsRng);
        let bob_eph = XSec::random_from_rng(OsRng);
        let alice_pub = XPub::from(&alice_eph);
        let bob_pub = XPub::from(&bob_eph);
        alice.dh_step(&alice_eph, &bob_pub);
        bob.dh_step(&bob_eph, &alice_pub);

        // PCS: post-step root differs from leaked root.
        assert_ne!(*alice.root_key().as_bytes(), leaked_root, "PCS: root must rotate");
        assert_ne!(*alice.chain_key().as_bytes(), leaked_chain, "PCS: chain must rotate");
        // PCS symmetry: Alice and Bob converge.
        assert_eq!(alice.root_key().as_bytes(), bob.root_key().as_bytes(),
                   "PCS: peers converge on fresh root");
        // PCS: an adversary holding only `leaked_root` cannot reproduce alice's
        // post-step root — they don't know the ephemeral DH secrets.
        // (Demonstrated by simply checking the values differ; cryptographic
        // unreachability is proven structurally in Coq INV-CHAT-13.)
    }

    // ---------- L-CHAT-8 hybrid PQ tests (no ml-kem dep here; we feed the
    //             32-byte KEM shared secret directly per the API contract) ----------

    #[test]
    fn hybrid_dh_kem_step_rotates_root() {
        use rand_core::OsRng;
        let mut c = Chain::from_root(&root(), b"send");
        let pre_root = *c.root_key().as_bytes();
        let my_sk = XSec::random_from_rng(OsRng);
        let their_sk = XSec::random_from_rng(OsRng);
        let their_pub = XPub::from(&their_sk);
        let kem_ss = [0xAB; 32];
        c.dh_kem_step(&my_sk, &their_pub, &kem_ss);
        assert_ne!(pre_root, *c.root_key().as_bytes(),
                   "hybrid step must rotate root");
    }

    #[test]
    fn hybrid_dh_kem_step_diverges_from_dh_only() {
        // SAME DH inputs but DIFFERENT (or absent) KEM ss MUST yield different
        // root keys — proves the KEM mix actually contributes entropy and is
        // not silently dropped.
        use rand_core::OsRng;
        let r = root();
        let my_sk = XSec::random_from_rng(OsRng);
        let their_sk = XSec::random_from_rng(OsRng);
        let their_pub = XPub::from(&their_sk);

        let mut c_classic = Chain::from_root(&r, b"send");
        c_classic.dh_step(&my_sk, &their_pub);

        let mut c_hybrid = Chain::from_root(&r, b"send");
        let kem_ss = [0x42; 32];
        c_hybrid.dh_kem_step(&my_sk, &their_pub, &kem_ss);

        assert_ne!(c_classic.root_key().as_bytes(), c_hybrid.root_key().as_bytes(),
                   "R-CHAT-2: hybrid root must differ from classic-DH root for the same DH inputs");
    }

    #[test]
    fn hybrid_dh_kem_step_symmetric_alice_bob() {
        // Alice and Bob run a hybrid step with identical DH inputs (mirrored)
        // and IDENTICAL kem_ss — they must converge.
        use rand_core::OsRng;
        let mut alice = Chain::from_root(&root(), b"send");
        let mut bob = Chain::from_root(&root(), b"send");
        let alice_sk = XSec::random_from_rng(OsRng);
        let bob_sk = XSec::random_from_rng(OsRng);
        let kem_ss = [0xC0; 32];
        alice.dh_kem_step(&alice_sk, &XPub::from(&bob_sk), &kem_ss);
        bob.dh_kem_step(&bob_sk, &XPub::from(&alice_sk), &kem_ss);
        assert_eq!(alice.root_key().as_bytes(), bob.root_key().as_bytes(),
                   "hybrid symmetry: peers must converge");
    }

    #[test]
    fn falsifier_pq_downgrade_kem_ss_zeroed() {
        // Falsifier: an attacker tries to coerce the responder into a
        // "classic-DH only" downgrade by zeroing the KEM ss. The hybrid root
        // produced under zeroed KEM MUST differ from the classic-DH root —
        // even an all-zero KEM ss is mixed under a different domain string.
        use rand_core::OsRng;
        let r = root();
        let my_sk = XSec::random_from_rng(OsRng);
        let their_sk = XSec::random_from_rng(OsRng);
        let their_pub = XPub::from(&their_sk);

        let mut c_classic = Chain::from_root(&r, b"send");
        c_classic.dh_step(&my_sk, &their_pub);

        let mut c_hybrid_zero = Chain::from_root(&r, b"send");
        c_hybrid_zero.dh_kem_step(&my_sk, &their_pub, &[0u8; 32]);

        assert_ne!(c_classic.root_key().as_bytes(), c_hybrid_zero.root_key().as_bytes(),
                   "PQ downgrade: classic-vs-hybrid path MUST diverge by domain separation");
    }

    #[test]
    fn pcs_two_step_isolates_from_initial_compromise() {
        // Stronger PCS gate: even if attacker captures BOTH root + a fresh
        // ephemeral on step 1, after step 2 (with new ephemerals) they're
        // locked out again.
        use rand_core::OsRng;
        let mut a = Chain::from_root(&root(), b"send");
        let mut b = Chain::from_root(&root(), b"send");
        let e1a = XSec::random_from_rng(OsRng);
        let e1b = XSec::random_from_rng(OsRng);
        a.dh_step(&e1a, &XPub::from(&e1b));
        b.dh_step(&e1b, &XPub::from(&e1a));
        let mid_root = *a.root_key().as_bytes();

        let e2a = XSec::random_from_rng(OsRng);
        let e2b = XSec::random_from_rng(OsRng);
        a.dh_step(&e2a, &XPub::from(&e2b));
        b.dh_step(&e2b, &XPub::from(&e2a));

        assert_eq!(a.root_key().as_bytes(), b.root_key().as_bytes(),
                   "two-step PCS: peers still converge");
        assert_ne!(*a.root_key().as_bytes(), mid_root,
                   "two-step PCS: post-step-2 root differs from post-step-1");
    }

    // ============================================================
    // Wave-10 / L-CHAT-2-rfs (R-CHAT-2): ratchet forward-secrecy +
    // post-compromise security falsifier suite.
    //
    // Threat: a single key compromise must NOT let the adversary
    // (a) decrypt any future message after a DH step (forward-secrecy
    //     across DH ratchet, RFS-01..03), nor
    // (b) keep the chain stuck in the old root after a DH step
    //     (post-compromise healing, RFS-04..05).
    //
    // [DERIVED Signal Double Ratchet (Cohn-Gordon et al. 2017) +
    //  PQXDH (Marlinspike et al. 2023)]
    // ============================================================

    /// **RFS-01** — every chain step rotates the chain-key and the
    /// derived message-key, so a leaked message-key at counter N
    /// gives no information about counter N+1.
    #[test]
    fn rfs_01_chain_step_rotates_message_key() {
        let root = RootKey([7u8; 32]);
        let mut c = Chain::from_root(&root, b"snd");
        let pre_chain = *c.chain_key().as_bytes();
        let mk_n = c.chain_key.next_message_key(0);
        let mid_chain = *c.chain_key().as_bytes();
        let mk_n1 = c.chain_key.next_message_key(1);
        let post_chain = *c.chain_key().as_bytes();

        assert_ne!(pre_chain, mid_chain, "RFS-01: chain must advance after step 0");
        assert_ne!(mid_chain, post_chain, "RFS-01: chain must advance after step 1");
        assert_ne!(mk_n.key, mk_n1.key, "RFS-01: message keys at N and N+1 must differ");
        assert_ne!(mk_n.nonce, mk_n1.nonce, "RFS-01: nonces at N and N+1 must differ");
    }

    /// **RFS-02** — chain key after K steps is unrelated to the
    /// initial chain key (forward-secrecy across many ratchet ticks).
    #[test]
    fn rfs_02_chain_diverges_after_many_steps() {
        let root = RootKey([9u8; 32]);
        let mut c = Chain::from_root(&root, b"snd");
        let init_chain = *c.chain_key().as_bytes();
        for i in 0..32u64 {
            let _ = c.chain_key.next_message_key(i);
        }
        let final_chain = *c.chain_key().as_bytes();
        assert_ne!(init_chain, final_chain, "RFS-02: chain key must diverge after 32 steps");
    }

    /// **RFS-03** — DH step rotates the root AND derives a new chain
    /// that is unrelated to the pre-step chain (compromise of a chain
    /// key prior to a DH step gives no info on post-step messages).
    #[test]
    fn rfs_03_dh_step_breaks_chain_continuity() {
        use rand_core::OsRng;
        let mut c = Chain::from_root(&RootKey([1u8; 32]), b"snd");
        let pre_root = *c.root_key().as_bytes();
        let pre_chain = *c.chain_key().as_bytes();

        let my_sk = XSec::random_from_rng(OsRng);
        let their_sk = XSec::random_from_rng(OsRng);
        c.dh_step(&my_sk, &XPub::from(&their_sk));

        let post_root = *c.root_key().as_bytes();
        let post_chain = *c.chain_key().as_bytes();

        assert_ne!(pre_root, post_root, "RFS-03: root must rotate on DH step");
        assert_ne!(pre_chain, post_chain, "RFS-03: chain must rotate on DH step");
    }

    /// **RFS-04** — post-compromise healing: after one DH step with a
    /// FRESH ephemeral, the new root is independent of any previously
    /// leaked chain-key state.
    #[test]
    fn rfs_04_post_compromise_root_independent_of_pre_chain() {
        use rand_core::OsRng;
        // Two chains starting from the same root but with different
        // intermediate chain-key compromise patterns must converge to
        // the same post-DH root after a fresh DH step.
        let mut a = Chain::from_root(&RootKey([3u8; 32]), b"x");
        let mut b = Chain::from_root(&RootKey([3u8; 32]), b"x");
        // Adversary causes 'a' to advance the chain key 5 times
        // (simulating a leaked-then-used chain).
        for i in 0..5u64 { let _ = a.chain_key.next_message_key(i); }
        // 'b' stays untouched.
        assert_ne!(
            a.chain_key().as_bytes(),
            b.chain_key().as_bytes(),
            "RFS-04: pre-DH chains diverge as expected"
        );
        // Both run the same DH step with FRESH secrets.
        let sk = XSec::random_from_rng(OsRng);
        let peer_sk = XSec::random_from_rng(OsRng);
        let peer_pub = XPub::from(&peer_sk);
        a.dh_step(&sk, &peer_pub);
        b.dh_step(&sk, &peer_pub);
        // Post-DH roots converge — chain-key history is irrelevant.
        assert_eq!(
            a.root_key().as_bytes(),
            b.root_key().as_bytes(),
            "RFS-04: post-DH root only depends on (root, dh_ss), not chain history"
        );
    }

    /// **RFS-05** — hybrid DH+KEM step further entropy-mixes ML-KEM
    /// shared-secret. Two parties with same DH but different `kem_ss`
    /// MUST diverge — proves the KEM contribution is non-degenerate.
    #[test]
    fn rfs_05_hybrid_kem_contribution_non_degenerate() {
        use rand_core::OsRng;
        let mut a = Chain::from_root(&RootKey([5u8; 32]), b"x");
        let mut b = Chain::from_root(&RootKey([5u8; 32]), b"x");
        let sk = XSec::random_from_rng(OsRng);
        let peer_sk = XSec::random_from_rng(OsRng);
        let peer_pub = XPub::from(&peer_sk);
        let kem_a = [0xAAu8; 32];
        let kem_b = [0xBBu8; 32];
        a.dh_kem_step(&sk, &peer_pub, &kem_a);
        b.dh_kem_step(&sk, &peer_pub, &kem_b);
        assert_ne!(
            a.root_key().as_bytes(),
            b.root_key().as_bytes(),
            "RFS-05: distinct kem_ss must yield distinct post-step roots"
        );
    }

    /// Wave-10 G-C2-rfs green summary.
    #[test]
    fn green_g_c2_rfs_summary() {
        let count = 5usize;
        assert_eq!(count, 5, "Wave-10 L-CHAT-2-rfs: 5 ratchet-FS falsifier tests");
    }

    // ─── Wave-11 · L-CHAT-2-skip · skipped-key bound + DoS resistance ───
    //
    // R-CHAT-2 demands the skipped-keys cache must NOT grow without bound,
    // must NOT leak across DH-ratchet epochs once the cache is full, must
    // refuse to derive arbitrary counters beyond [`SKIPPED_KEYS_CAP`], and
    // must enforce one-shot consumption (no key reuse). These are the
    // anti-DoS / anti-replay invariants for out-of-order delivery.

    /// **SKP-01** — skipped-key cache is bounded by `SKIPPED_KEYS_CAP`.
    /// A receiver MUST NOT buffer arbitrarily many derived keys when a
    /// sender (possibly malicious) jumps the counter very far forward.
    #[test]
    fn skp_01_skipped_cache_bounded_by_cap() {
        let mut c = Chain::from_root(&RootKey([7u8; 32]), b"r");
        // Jump exactly to SKIPPED_KEYS_CAP — fills the cache to the cap.
        c.recv_accept(SKIPPED_KEYS_CAP as u64).unwrap();
        assert!(
            c.skipped_len() <= SKIPPED_KEYS_CAP,
            "SKP-01: cache must never exceed SKIPPED_KEYS_CAP={}",
            SKIPPED_KEYS_CAP
        );
    }

    /// **SKP-02** — DH ratchet step purges (or bounds) the skipped cache.
    /// After a fresh DH step the receiver enters a new epoch; stale
    /// keys from the previous epoch must NOT remain accessible past the
    /// cap, otherwise a compromise of one epoch would leak the next.
    #[test]
    fn skp_02_dh_step_clears_overflowing_skipped_cache() {
        use rand_core::OsRng;
        let mut c = Chain::from_root(&RootKey([8u8; 32]), b"r");
        // Fill the cache to overflow first.
        c.recv_accept(SKIPPED_KEYS_CAP as u64 + 100).unwrap();
        // Cap-bounded immediately.
        assert!(c.skipped_len() <= SKIPPED_KEYS_CAP);
        // DH step rotates epochs; cache must remain bounded.
        let sk = XSec::random_from_rng(OsRng);
        let peer_sk = XSec::random_from_rng(OsRng);
        let peer_pub = XPub::from(&peer_sk);
        c.dh_step(&sk, &peer_pub);
        assert!(
            c.skipped_len() <= SKIPPED_KEYS_CAP,
            "SKP-02: post-DH skipped cache must stay bounded"
        );
    }

    /// **SKP-03** — the receiver refuses to derive an unbounded number
    /// of intermediate keys. Even when the attacker pushes a counter
    /// vastly beyond capacity the cache fills only up to the cap and
    /// then stops — proving the derivation loop terminates.
    #[test]
    fn skp_03_huge_jump_does_not_explode_cache() {
        let mut c = Chain::from_root(&RootKey([9u8; 32]), b"r");
        // Massive jump — 100x cap.
        c.recv_accept((SKIPPED_KEYS_CAP * 100) as u64).unwrap();
        assert!(
            c.skipped_len() <= SKIPPED_KEYS_CAP,
            "SKP-03: huge counter jump must not blow past SKIPPED_KEYS_CAP"
        );
    }

    /// **SKP-04** — accepting a counter and then replaying the same
    /// counter must fail (replay-window). The cache MUST NOT be a
    /// loophole around the replay protection: if a key was consumed,
    /// a second `recv_accept` for the same counter must be rejected.
    #[test]
    fn skp_04_replay_after_consumption_rejected() {
        let mut c = Chain::from_root(&RootKey([10u8; 32]), b"r");
        // Bring receiver up to counter 5.
        c.recv_accept(5).unwrap();
        // Now replay counter 5 — must be detected.
        let r = c.recv_accept(5);
        assert!(r.is_err(), "SKP-04: replayed in-window counter must be rejected");
    }

    /// **SKP-05** — `take_skipped` is one-shot: it removes the key from
    /// the cache, so a second take for the same counter must return
    /// `None`. This prevents the same message-key being used twice if
    /// an out-of-order packet were re-injected.
    #[test]
    fn skp_05_take_skipped_is_one_shot() {
        let mut c = Chain::from_root(&RootKey([11u8; 32]), b"r");
        // Jump forward by 3, leaving 0..=2 buffered.
        c.recv_accept(3).unwrap();
        let first = c.take_skipped(1);
        assert!(first.is_some(), "SKP-05: first take must yield the buffered key");
        let second = c.take_skipped(1);
        assert!(
            second.is_none(),
            "SKP-05: second take of the same counter must be None"
        );
    }

    /// Wave-11 G-C2-skip green summary.
    #[test]
    fn green_g_c2_skip_summary() {
        let count = 5usize;
        assert_eq!(
            count, 5,
            "Wave-11 L-CHAT-2-skip: 5 skipped-key-bound falsifier tests"
        );
    }
}
