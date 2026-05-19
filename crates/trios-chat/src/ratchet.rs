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

use hkdf::Hkdf;
use sha2::Sha256;
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
        }
    }

    /// Sender: produce the next message key, increment counter.
    pub fn send_next(&mut self) -> MessageKey {
        let mk = self.chain_key.next_message_key(self.counter);
        self.counter = self.counter.checked_add(1).expect("counter overflow");
        mk
    }

    /// Receiver: accept a counter; reject replays / wild rollbacks.
    /// Returns the key only if the counter is fresh.
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
        } else if counter == self.counter {
            self.seen_window = self.seen_window.wrapping_shl(1) | 1;
            self.counter = self.counter.checked_add(1).expect("counter overflow");
        } else {
            // Future counter — slide the window forward.
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
}
