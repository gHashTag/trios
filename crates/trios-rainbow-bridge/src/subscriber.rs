//! Subscribers and their identifiers.
//!
//! A `SubscriberId` is an opaque agent string, e.g. `"perplexity-computer-l13"`.
//! Subscribers are sinks that consume events delivered by a [`FunnelClient`].

use std::sync::{Arc, Mutex};

use crate::event::RainbowEvent;

/// Opaque agent identifier (`String` newtype for clarity).
pub type SubscriberId = String;

/// A synchronous, in-memory subscriber used in tests and the reference
/// implementation. Production subscribers are expected to implement the
/// same interface in `trios-server::rainbow_routes`.
#[derive(Debug, Clone, Default)]
pub struct Subscriber {
    /// Unique identifier for this subscriber.
    pub id: SubscriberId,
    inbox: Arc<Mutex<Vec<RainbowEvent>>>,
}

impl Subscriber {
    /// Build a new subscriber tagged with `id`.
    #[must_use]
    pub fn new(id: impl Into<SubscriberId>) -> Self {
        Self {
            id: id.into(),
            inbox: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Push one event into the subscriber's inbox (called by [`FunnelClient`]).
    pub fn deliver(&self, ev: RainbowEvent) {
        // Poisoned-mutex recovery: tests never intentionally panic while
        // holding the lock, and any panic inside `Mutex` is a programmer
        // error worthy of surfacing.
        self.inbox.lock().expect("inbox mutex poisoned").push(ev);
    }

    /// Snapshot of delivered events (cloned for ergonomics).
    #[must_use]
    pub fn received(&self) -> Vec<RainbowEvent> {
        self.inbox.lock().expect("inbox mutex poisoned").clone()
    }

    /// Number of events delivered so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inbox.lock().expect("inbox mutex poisoned").len()
    }

    /// Whether no events have been delivered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
