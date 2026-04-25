//! Layer 1 — Lamport Line.
//!
//! Per-agent monotonic u64 logical clock. Coq mirror:
//! `rainbow_bridge_consistency.v::lamport_monotone_step`.

use std::collections::HashMap;

use crate::subscriber::SubscriberId;

/// Agent-scoped Lamport clock.
#[derive(Debug, Default, Clone)]
pub struct LamportClock {
    /// Latest observed lamport value per agent. Monotone non-decreasing
    /// per key; enforced by [`Bridge::accept`].
    last_seen: HashMap<SubscriberId, u64>,
}

impl LamportClock {
    /// Construct an empty clock.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the last observed lamport for `agent`, or `None` if the agent
    /// has never emitted an event.
    #[must_use]
    pub fn last_seen(&self, agent: &SubscriberId) -> Option<u64> {
        self.last_seen.get(agent).copied()
    }

    /// Record `lamport` as observed from `agent`. Returns `true` when the
    /// new value strictly advances the clock (monotone), `false` when it
    /// regresses or stalls.
    ///
    /// Coq: `rainbow_bridge_consistency.v::lamport_monotone_step`.
    pub fn advance(&mut self, agent: SubscriberId, lamport: u64) -> bool {
        match self.last_seen.get(&agent) {
            Some(prev) if lamport <= *prev => false,
            _ => {
                self.last_seen.insert(agent, lamport);
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_event_advances() {
        let mut c = LamportClock::new();
        assert!(c.advance("alpha".into(), 1));
    }

    #[test]
    fn monotone_steps_accepted() {
        let mut c = LamportClock::new();
        assert!(c.advance("alpha".into(), 1));
        assert!(c.advance("alpha".into(), 2));
        assert!(c.advance("alpha".into(), 100));
    }

    #[test]
    fn regression_rejected() {
        let mut c = LamportClock::new();
        assert!(c.advance("alpha".into(), 10));
        assert!(!c.advance("alpha".into(), 5));
        assert!(!c.advance("alpha".into(), 10));
    }

    #[test]
    fn agents_are_independent() {
        let mut c = LamportClock::new();
        assert!(c.advance("alpha".into(), 10));
        // beta has never spoken — 1 is fine even though alpha is at 10.
        assert!(c.advance("beta".into(), 1));
    }
}
