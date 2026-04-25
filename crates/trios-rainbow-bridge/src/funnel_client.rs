//! Layer-0 transport — the Tailscale Funnel client abstraction.
//!
//! Production transport lives in `trios-server::rainbow_routes` (re-export
//! only per R6 / ONE SHOT §3.3). This module provides a test-grade
//! in-memory fan-out used by falsification and integration tests.

use std::sync::{Arc, Mutex};

use crate::event::RainbowEvent;
use crate::subscriber::Subscriber;

/// Trait any funnel transport must implement.
pub trait FunnelClient: Send + Sync {
    /// Publish one event to all currently-attached subscribers.
    /// Returns the measured publish latency (ms).
    fn publish(&self, ev: &RainbowEvent) -> u64;
}

/// In-memory fan-out funnel. Fan-out is synchronous; the simulated latency
/// is configurable to allow tests to exercise the [`BridgeError::FunnelUnreachable`]
/// branch deterministically.
#[derive(Debug, Clone, Default)]
pub struct InMemoryFunnel {
    subscribers: Arc<Mutex<Vec<Subscriber>>>,
    /// Simulated publish latency in milliseconds.
    simulated_latency_ms: Arc<Mutex<u64>>,
}

impl InMemoryFunnel {
    /// Empty funnel.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach a subscriber.
    pub fn attach(&self, s: Subscriber) {
        self.subscribers
            .lock()
            .expect("funnel subscribers mutex poisoned")
            .push(s);
    }

    /// Snapshot of currently-attached subscribers (cloned).
    #[must_use]
    pub fn subscribers(&self) -> Vec<Subscriber> {
        self.subscribers
            .lock()
            .expect("funnel subscribers mutex poisoned")
            .clone()
    }

    /// Set the simulated publish latency. Exists only so falsification
    /// tests can deterministically trigger the p95 guard.
    pub fn set_simulated_latency_ms(&self, latency_ms: u64) {
        *self
            .simulated_latency_ms
            .lock()
            .expect("funnel latency mutex poisoned") = latency_ms;
    }

    fn current_latency_ms(&self) -> u64 {
        *self
            .simulated_latency_ms
            .lock()
            .expect("funnel latency mutex poisoned")
    }
}

impl FunnelClient for InMemoryFunnel {
    fn publish(&self, ev: &RainbowEvent) -> u64 {
        for s in self.subscribers() {
            s.deliver(ev.clone());
        }
        self.current_latency_ms()
    }
}
