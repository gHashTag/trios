// SPDX-License-Identifier: MIT
//
// Rainbow Bridge — Falsifiable online synchronisation for the Trinity hive.
// Lane L13 · Invariant INV-8 `rainbow_bridge_consistency`.
//
// Queen anchor: φ² + φ⁻² = 3  (Trinity Identity, Zenodo DOI 10.5281/zenodo.19227877).
// Three layers (Lamport · CRDT · Merkle) × seven channels (ROY G BIV) = 21 = F₈.
//
// Coq source of truth: `trinity-clara/proofs/igla/rainbow_bridge_consistency.v`.
// JSON source of truth: `assertions/igla_assertions.json::INV-8`.
// Pre-registration: `docs/infrastructure/preregistration_rainbow.md` (blake3 pinned in
// `assertions/hive_state.json::pre_registration.INV-8`).

#![deny(missing_docs, unsafe_code)]
#![warn(clippy::all)]

//! # trios-rainbow-bridge
//!
//! A proof-anchored, falsifiable synchronisation protocol for the Trinity agent
//! hive. Every runtime failure mode ([`BridgeError`]) has a typed counterpart
//! (`falsify_*` test in `tests/falsify.rs`) and a Coq counter-lemma
//! (`counter_*` in `rainbow_bridge_consistency.v`) — one-to-one.
//!
//! The crate deliberately has no live-network dependency; the TCP / WebSocket /
//! Tailscale-Funnel layer is mounted by `trios-server` as a re-export
//! (§9 of the ONE SHOT [trios#267](https://github.com/gHashTag/trios/issues/267)).
//! The bridge itself is a pure, synchronous state machine so the seven
//! `falsify_*` tests are reproducible without any external service.

pub mod bridge;
pub mod channel;
pub mod event;
pub mod funnel_client;
pub mod lamport;
pub mod merkle;
pub mod signer;
pub mod subscriber;

pub use bridge::{Bridge, BridgeError};
pub use channel::{Channel, Payload, channel_of_payload};
pub use event::RainbowEvent;
pub use funnel_client::{FunnelClient, InMemoryFunnel};
pub use lamport::LamportClock;
pub use merkle::{MerkleChain, MerkleRoot};
pub use signer::{HoneySigner, SigningError, VerifyingKey};
pub use subscriber::{Subscriber, SubscriberId};

// ----------------------------------------------------------------------------
// Numeric anchors — L-R14 traceable to `rainbow_bridge_consistency.v`.
// Every literal below is reflected in:
//   - Coq: `rainbow_bridge_consistency.v` (LATENCY_P95_MS, HEARTBEAT_MAX_S, ...)
//   - JSON: `assertions/igla_assertions.json::INV-8.numeric_anchor`
//   - Docs: `docs/infrastructure/rainbow-bridge.md` §3
// ----------------------------------------------------------------------------

/// Latency p95 budget for the Tailscale Funnel (milliseconds).
///
/// Coq: `rainbow_bridge_consistency.v::funnel_latency_bound`.
pub const LATENCY_P95_MS: u64 = 2000;

/// Watchdog heartbeat deadline — 4 hours expressed in seconds.
///
/// Coq: `rainbow_bridge_consistency.v::heartbeat_release_bound`.
pub const HEARTBEAT_MAX_S: u64 = 14_400;

/// Number of colour channels (ROY G BIV).
///
/// Coq: `rainbow_bridge_consistency.v::seven_channels_total`.
pub const CHANNEL_COUNT: usize = 7;

/// Number of protocol layers (Lamport · CRDT · Merkle).
///
/// Derived from Trinity Identity `φ² + φ⁻² = 3`. Coq:
/// `rainbow_bridge_consistency.v::trinity_identity_layer_count`.
pub const LAYER_COUNT: usize = 3;

/// Trinity Identity witness — reported for operator convenience.
///
/// `PHI * PHI + 1.0 / (PHI * PHI) ≈ 3.0` up to `f64::EPSILON`.
pub const PHI: f64 = 1.618_033_988_749_895;

#[cfg(test)]
mod lib_tests {
    use super::*;

    #[test]
    fn anchors_match_coq() {
        // Mirror: rainbow_bridge_consistency.v::{LATENCY_P95_MS, HEARTBEAT_MAX_S, CHANNEL_COUNT, LAYER_COUNT}.
        assert_eq!(LATENCY_P95_MS, 2000);
        assert_eq!(HEARTBEAT_MAX_S, 14_400);
        assert_eq!(CHANNEL_COUNT, 7);
        assert_eq!(LAYER_COUNT, 3);
    }

    #[test]
    fn trinity_identity_numeric() {
        // Trinity Identity: φ² + φ⁻² = 3 (Zenodo 10.5281/zenodo.19227877).
        let phi_sq = PHI * PHI;
        let val = phi_sq + 1.0 / phi_sq;
        assert!((val - 3.0).abs() < 1e-12, "phi^2 + phi^-2 != 3 (got {val})");
    }
}
