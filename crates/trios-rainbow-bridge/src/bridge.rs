//! The Rainbow Bridge state machine.
//!
//! [`Bridge::accept`] is the single entry point. It evaluates the full
//! seven-branch legality predicate (INV-8) for one incoming event; returns
//! `Ok(())` on success, `Err(BridgeError::…)` otherwise. Every variant is
//! exercised by a corresponding `falsify_*` test.

use std::collections::HashMap;

use thiserror::Error;

use crate::channel::{channel_of_payload, Channel, Payload};
use crate::event::RainbowEvent;
use crate::funnel_client::FunnelClient;
use crate::lamport::LamportClock;
use crate::merkle::MerkleChain;
use crate::signer::{HoneySigner, VerifyingKey};
use crate::subscriber::SubscriberId;
use crate::{HEARTBEAT_MAX_S, LATENCY_P95_MS};

/// Typed runtime failure modes of the Rainbow Bridge.
///
/// Each variant is one-to-one with:
///   - a `falsify_*` test in `tests/falsify.rs`,
///   - a `counter_*` lemma in `rainbow_bridge_consistency.v`,
///   - a row in `docs/infrastructure/rainbow-bridge.md §4`.
#[derive(Debug, Error, PartialEq)]
pub enum BridgeError {
    /// Two distinct agents claimed the same lane at the same lamport.
    #[error("duplicate claim on lane {lane} by agents {agent_a} and {agent_b} at lamport {lamport}")]
    DuplicateClaim {
        /// Lane identifier that was double-claimed.
        lane: String,
        /// First claimant.
        agent_a: SubscriberId,
        /// Second (racing) claimant.
        agent_b: SubscriberId,
        /// Shared lamport value.
        lamport: u64,
    },

    /// A lane stayed in WORK past [`HEARTBEAT_MAX_S`] without a heartbeat.
    #[error("heartbeat stale: {age_s}s > {max_s}s for lane {lane}")]
    HeartbeatStale {
        /// Lane id.
        lane: String,
        /// Observed age in seconds.
        age_s: u64,
        /// Threshold ([`HEARTBEAT_MAX_S`]).
        max_s: u64,
    },

    /// An event from the same agent arrived with a lamport value <= the
    /// previous one.
    #[error("lamport regression for agent {agent}: new={new} prev={prev}")]
    LamportRegression {
        /// Offending agent.
        agent: SubscriberId,
        /// Value attempted.
        new: u64,
        /// Previously-observed value.
        prev: u64,
    },

    /// A honey-channel event arrived without a valid signature.
    #[error("unsigned honey event from agent {agent}")]
    UnsignedHoney {
        /// Offending agent.
        agent: SubscriberId,
    },

    /// Two agents committed divergent state snapshots from the same base
    /// lamport without a merge event.
    #[error("split brain detected at lamport {lamport}")]
    SplitBrainDetected {
        /// Shared lamport.
        lamport: u64,
        /// First agent.
        agent_a: SubscriberId,
        /// Second agent.
        agent_b: SubscriberId,
    },

    /// Publish latency exceeded [`LATENCY_P95_MS`].
    #[error("funnel unreachable: latency {latency_ms}ms > {budget_ms}ms")]
    FunnelUnreachable {
        /// Observed latency.
        latency_ms: u64,
        /// Budget.
        budget_ms: u64,
    },

    /// The event's declared channel does not match its payload.
    #[error("channel mismatch: declared {declared:?} expected {expected:?}")]
    ChannelMismatch {
        /// Channel the event declared.
        declared: Channel,
        /// Channel implied by the payload type.
        expected: Channel,
    },
}

/// The Rainbow Bridge. Owns the three layers (Lamport · automerge-free CRDT
/// placeholder · Merkle) and dispatches to the funnel.
pub struct Bridge<F: FunnelClient> {
    funnel: F,
    lamport: LamportClock,
    merkle: MerkleChain,
    /// Map `lane -> (agent, lamport)` of first-observed claim.
    lane_claims: HashMap<String, (SubscriberId, u64)>,
    /// Map `lane -> (last_heartbeat_ts, now_for_tests)`.
    lane_heartbeats: HashMap<String, u64>,
    /// Map `lamport -> first agent that committed State at that lamport`.
    state_commits: HashMap<u64, SubscriberId>,
    /// Registered honey-channel verifying keys per agent.
    honey_keys: HashMap<SubscriberId, VerifyingKey>,
}

impl<F: FunnelClient> Bridge<F> {
    /// Create a new bridge bound to `funnel`.
    pub fn new(funnel: F) -> Self {
        Self {
            funnel,
            lamport: LamportClock::new(),
            merkle: MerkleChain::new(),
            lane_claims: HashMap::new(),
            lane_heartbeats: HashMap::new(),
            state_commits: HashMap::new(),
            honey_keys: HashMap::new(),
        }
    }

    /// Register an agent's honey-channel verifying key.
    pub fn register_honey_key(&mut self, agent: SubscriberId, key: VerifyingKey) {
        self.honey_keys.insert(agent, key);
    }

    /// The current Merkle root as a hex string (operator-facing).
    pub fn merkle_root_hex(&self) -> String {
        self.merkle.root_hex()
    }

    /// Underlying Lamport clock (for inspection by tests).
    pub fn lamport(&self) -> &LamportClock {
        &self.lamport
    }

    /// Enforce INV-8 for a single event. On success, the event is merkle-appended
    /// and fanned out to the funnel.
    pub fn accept(&mut self, ev: RainbowEvent, now_unix_s: u64) -> Result<(), BridgeError> {
        // (7) Channel ↔ payload mismatch — cheapest, structural check first.
        let expected = channel_of_payload(&ev.payload);
        if ev.channel != expected {
            return Err(BridgeError::ChannelMismatch {
                declared: ev.channel,
                expected,
            });
        }

        // (3) Lamport monotonicity per agent.
        if let Some(prev) = self.lamport.last_seen(&ev.agent) {
            if ev.lamport <= prev {
                return Err(BridgeError::LamportRegression {
                    agent: ev.agent.clone(),
                    new: ev.lamport,
                    prev,
                });
            }
        }

        match &ev.payload {
            // (1) Duplicate claim.
            Payload::Claim { lane } => {
                if let Some((first_agent, first_lamport)) = self.lane_claims.get(lane) {
                    if first_agent != &ev.agent && *first_lamport == ev.lamport {
                        return Err(BridgeError::DuplicateClaim {
                            lane: lane.clone(),
                            agent_a: first_agent.clone(),
                            agent_b: ev.agent.clone(),
                            lamport: ev.lamport,
                        });
                    }
                } else {
                    self.lane_claims
                        .insert(lane.clone(), (ev.agent.clone(), ev.lamport));
                }
            }
            // (2) Heartbeat staleness.
            Payload::Heartbeat { lane } => {
                if let Some(prev_ts) = self.lane_heartbeats.get(lane) {
                    let age = now_unix_s.saturating_sub(*prev_ts);
                    if age > HEARTBEAT_MAX_S {
                        return Err(BridgeError::HeartbeatStale {
                            lane: lane.clone(),
                            age_s: age,
                            max_s: HEARTBEAT_MAX_S,
                        });
                    }
                }
                self.lane_heartbeats.insert(lane.clone(), now_unix_s);
            }
            // (4) Unsigned honey.
            Payload::Honey { line } => {
                if !ev.is_signed() {
                    return Err(BridgeError::UnsignedHoney {
                        agent: ev.agent.clone(),
                    });
                }
                if let Some(key) = self.honey_keys.get(&ev.agent) {
                    if HoneySigner::verify(key, line.as_bytes(), &ev.signature).is_err() {
                        return Err(BridgeError::UnsignedHoney {
                            agent: ev.agent.clone(),
                        });
                    }
                }
            }
            // (5) Split-brain on State.
            Payload::State { .. } => {
                if let Some(first_agent) = self.state_commits.get(&ev.lamport) {
                    if first_agent != &ev.agent {
                        return Err(BridgeError::SplitBrainDetected {
                            lamport: ev.lamport,
                            agent_a: first_agent.clone(),
                            agent_b: ev.agent.clone(),
                        });
                    }
                } else {
                    self.state_commits.insert(ev.lamport, ev.agent.clone());
                }
            }
            // Other channels (Done, Violation, Victory) have no lane-specific
            // side-condition beyond the structural checks already done.
            _ => {}
        }

        // All checks passed — advance layers.
        self.lamport.advance(ev.agent.clone(), ev.lamport);
        self.merkle.append(&ev);

        // (6) Funnel reachability — latency budget.
        let latency_ms = self.funnel.publish(&ev);
        if latency_ms > LATENCY_P95_MS {
            return Err(BridgeError::FunnelUnreachable {
                latency_ms,
                budget_ms: LATENCY_P95_MS,
            });
        }

        Ok(())
    }
}
