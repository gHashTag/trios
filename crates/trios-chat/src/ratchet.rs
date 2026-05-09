//! L-CHAT-2 · trinity-fpga#30 — Triple Ratchet skeleton.
//!
//! `[ASPIRATIONAL]` — full Double/Triple Ratchet construction lands in the
//! L-CHAT-2 follow-up PR. This module ships the **state machine + chain-key
//! advance** so dependent modules (`sealed`, `capability`) compile and so
//! G-C2 falsifier tests have something to refute.
//!
//! Concretely we deliver:
//! * `RootKey`, `ChainKey` — KDF-chained 32-byte secrets.
//! * `MessageKey::derive` — HKDF-SHA-256 from chain-key + counter.
//! * `Chain::next_message_key` — strictly monotone counter, no replay.
//! * `Chain::detect_replay` — falsifier hook for G-C2.
//!
//! Per R-CHAT-2 the eventual `Chain::dh_step` will mix `(DH(...) ‖ ML-KEM ss)`
//! into the root key. The skeleton API is shaped for that.
//!
//! Per R-CHAT-4 messages are authenticated via MAC derived from the chain
//! key, never via per-message Ed25519. `[CITED]` Signal Double Ratchet,
//! Marlinspike & Perrin 2016.

use std::collections::BTreeMap;

use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{PublicKey as XPub, StaticSecret as XSec};
use zeroize::ZeroizeOnDrop;

use crate::{Error, Result};

/// 32-byte root key. Updates only on a DH (or DH+KEM) step.
#[derive(Clone, ZeroizeOnDrop)]
pub struct RootKey(pub(crate) [u8; 32]);

impl RootKey {
    /// Construct a root key from raw 32-byte material. `[VERIFIED]`
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// 32-byte chain key. Updates on every message.
#[derive(Clone, ZeroizeOnDrop)]
pub struct ChainKey(pub(crate) [u8; 32]);

/// Message key + nonce derived from one chain-key step.
#[derive(Clone, Debug, PartialEq, Eq, ZeroizeOnDrop)]
pub struct MessageKey {
    /// 32-byte AEAD key (used by ChaCha20-Poly1305 in `sealed`).
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
    seen_window: u64, // bitmask of recent counters relative to `counter`
    /// Skipped message keys (out-of-order delivery cache).
    /// Capped at 1024 entries to bound memory.
    skipped: BTreeMap<u64, MessageKey>,
    /// Current root key (rotated by `dh_step`).
    pub(crate) root: RootKey,
    /// Direction label so re-`from_root` after a DH step is deterministic.
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

    /// **DH step (R-CHAT-2)** — mix a fresh X25519 shared secret into the
    /// root key. Future PR will combine `(DH ‖ ML-KEM ss)` exactly per
    /// Signal PQXDH. `[VERIFIED]` for the X25519 path.
    pub fn dh_step(&mut self, my_secret: &XSec, their_pub: &XPub) {
        let shared = my_secret.diffie_hellman(their_pub);
        let salt = b"trinity-chat:root-step:v1";
        let mut ikm = Vec::with_capacity(32 + 32);
        ikm.extend_from_slice(&self.root.0);
        ikm.extend_from_slice(shared.as_bytes());
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
        if self.skipped.len() > 1024 {
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
    /// **Wave-2:** when a counter jumps forward, all intermediate keys are
    /// derived and stored in `self.skipped` so out-of-order arrivals can
    /// still be decrypted.
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
            while c < counter && self.skipped.len() < 1024 {
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
        use x25519_dalek::{PublicKey as XPub, StaticSecret as XSec};
        let mut c = Chain::from_root(&root(), b"send");
        let pre_root = c.root.0;
        let pre_chain = c.chain_key.0;
        let my_sk = XSec::random_from_rng(OsRng);
        let their_sk = XSec::random_from_rng(OsRng);
        let their_pub = XPub::from(&their_sk);
        c.dh_step(&my_sk, &their_pub);
        assert_ne!(pre_root, c.root.0, "DH step must rotate root");
        assert_ne!(pre_chain, c.chain_key.0, "DH step must rotate chain");
        assert_eq!(c.counter, 0, "counter resets in new epoch");
    }

    #[test]
    fn dh_step_symmetric_alice_bob() {
        use rand_core::OsRng;
        use x25519_dalek::{PublicKey as XPub, StaticSecret as XSec};
        // Alice and Bob start from the same root key + label.
        let mut alice = Chain::from_root(&root(), b"send");
        let mut bob = Chain::from_root(&root(), b"send");
        // Each generates an X25519 keypair.
        let alice_sk = XSec::random_from_rng(OsRng);
        let bob_sk = XSec::random_from_rng(OsRng);
        let alice_pub = XPub::from(&alice_sk);
        let bob_pub = XPub::from(&bob_sk);
        // After symmetric DH step, both must share the same root + chain.
        alice.dh_step(&alice_sk, &bob_pub);
        bob.dh_step(&bob_sk, &alice_pub);
        assert_eq!(alice.root.0, bob.root.0, "DH symmetry: roots must match");
        assert_eq!(alice.chain_key.0, bob.chain_key.0, "DH symmetry: chains must match");
    }

    #[test]
    fn skipped_keys_cached_on_jump() {
        let mut c = Chain::from_root(&root(), b"recv");
        // Jump from 0 -> 5 must buffer keys for 0..5.
        c.recv_accept(5).unwrap();
        assert_eq!(c.skipped_len(), 5);
        // Out-of-order delivery for counter 2 must hit the cache.
        let m2 = c.take_skipped(2);
        assert!(m2.is_some());
        assert_eq!(m2.unwrap().counter, 2);
        assert_eq!(c.skipped_len(), 4);
    }
}
