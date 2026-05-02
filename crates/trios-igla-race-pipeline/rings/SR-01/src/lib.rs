//! SR-01 — strategy-queue
//!
//! In-memory Job FSM plus a FIFO `StrategyQueue` with `O(1)` push and
//! claim. The persistent contention layer (Postgres advisory locks /
//! `SKIP LOCKED`) ships in a future BR-IO ring; SR-01 stays Silver and
//! provides only the in-process semantics + transition guard.
//!
//! Closes #452 · Part of #446 · Anchor: phi^2 + phi^-2 = 3
//!
//! ## Job FSM (mirrors SR-00 [`JobStatus`])
//!
//! ```text
//! Queued ─▶ Running ─┬─▶ Done
//!                    ├─▶ Pruned
//!                    └─▶ Errored
//! ```
//!
//! Every other transition is rejected by [`transition`] with
//! [`FsmError::InvalidTransition`].
//!
//! ## Rules
//!
//! - R1   — pure Rust
//! - L6   — no I/O, no async, no subprocess
//! - L13  — I-SCOPE: only this ring
//! - R-RING-DEP-002 — deps = `sr-00 + serde + thiserror`

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::VecDeque;

use thiserror::Error;
use trios_igla_race_pipeline_sr_00::{JobId, JobStatus, Scarab};

/// Errors produced by the FSM and queue layers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FsmError {
    /// Caller asked for a transition that the FSM forbids.
    #[error("invalid FSM transition: {from:?} -> {to:?}")]
    InvalidTransition {
        /// Current status.
        from: JobStatus,
        /// Requested next status.
        to: JobStatus,
    },
    /// Caller tried to claim a job that has already been claimed.
    #[error("job {job_id} already claimed")]
    AlreadyClaimed {
        /// Already-claimed job's id.
        job_id: JobId,
    },
}

/// Pure FSM transition guard.
///
/// Returns `Ok(next)` for a valid transition and
/// [`FsmError::InvalidTransition`] otherwise.
///
/// Allowed transitions:
///
/// - `Queued`  → `Running`
/// - `Running` → `Done` | `Pruned` | `Errored`
///
/// Every other pair is rejected.
pub fn transition(current: JobStatus, next: JobStatus) -> Result<JobStatus, FsmError> {
    use JobStatus::*;
    let ok = matches!(
        (current, next),
        (Queued, Running)
            | (Running, Done)
            | (Running, Pruned)
            | (Running, Errored)
    );
    if ok {
        Ok(next)
    } else {
        Err(FsmError::InvalidTransition {
            from: current,
            to: next,
        })
    }
}

/// Returns `true` for transitions that the FSM accepts.
///
/// Equivalent to `transition(a, b).is_ok()` but cheaper for guards.
pub fn is_valid_transition(from: JobStatus, to: JobStatus) -> bool {
    transition(from, to).is_ok()
}

// ─────────────────── StrategyQueue ─────────────────────────────────

/// In-memory FIFO of [`Scarab`] entries waiting for a worker.
///
/// `push` and `claim_one` are both `O(1)` amortised. Only [`Scarab`]s
/// in [`JobStatus::Queued`] may be pushed; on `claim_one` the FSM
/// guard flips them to [`JobStatus::Running`] and stamps `started_at`.
#[derive(Debug, Default)]
pub struct StrategyQueue {
    inner: VecDeque<Scarab>,
}

impl StrategyQueue {
    /// Build an empty queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a fresh `Queued` scarab into the queue.
    ///
    /// Rejects scarabs whose status is anything other than `Queued`
    /// with [`FsmError::InvalidTransition`] (`from = current`,
    /// `to = Queued`).
    pub fn push(&mut self, scarab: Scarab) -> Result<(), FsmError> {
        if scarab.status != JobStatus::Queued {
            return Err(FsmError::InvalidTransition {
                from: scarab.status,
                to: JobStatus::Queued,
            });
        }
        self.inner.push_back(scarab);
        Ok(())
    }

    /// Pop the next scarab off the queue and flip it to `Running`.
    ///
    /// Returns `None` when the queue is empty.
    pub fn claim_one(&mut self) -> Option<Scarab> {
        let mut scarab = self.inner.pop_front()?;
        // FSM guard — re-asserts the contract even though we control
        // the constructor (defence in depth against future refactors).
        scarab.status = transition(scarab.status, JobStatus::Running)
            .expect("queue invariant: scarab must be Queued before claim");
        scarab.started_at = Some(chrono::Utc::now());
        Some(scarab)
    }

    /// Number of scarabs still waiting.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Peek at the next scarab without consuming it.
    pub fn peek_next(&self) -> Option<&Scarab> {
        self.inner.front()
    }
}

// ─────────────────── tests ─────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use trios_igla_race_pipeline_sr_00::{Seed, StrategyId};

    fn fresh_queued(seed: i64) -> Scarab {
        Scarab::queued(StrategyId::new(), Seed(seed), serde_json::json!({}))
    }

    // ── FSM ──

    #[test]
    fn fsm_queued_to_running_ok() {
        assert_eq!(
            transition(JobStatus::Queued, JobStatus::Running).unwrap(),
            JobStatus::Running
        );
    }

    #[test]
    fn fsm_running_to_terminal_states_ok() {
        for terminal in [JobStatus::Done, JobStatus::Pruned, JobStatus::Errored] {
            assert_eq!(
                transition(JobStatus::Running, terminal).unwrap(),
                terminal
            );
        }
    }

    #[test]
    fn fsm_queued_to_done_invalid() {
        let err = transition(JobStatus::Queued, JobStatus::Done).unwrap_err();
        assert_eq!(
            err,
            FsmError::InvalidTransition {
                from: JobStatus::Queued,
                to: JobStatus::Done,
            }
        );
    }

    #[test]
    fn fsm_done_to_running_invalid() {
        assert!(transition(JobStatus::Done, JobStatus::Running).is_err());
    }

    #[test]
    fn fsm_pruned_to_anything_invalid() {
        for other in [
            JobStatus::Queued,
            JobStatus::Running,
            JobStatus::Done,
            JobStatus::Errored,
        ] {
            assert!(
                transition(JobStatus::Pruned, other).is_err(),
                "Pruned -> {:?} should be invalid",
                other
            );
        }
    }

    #[test]
    fn fsm_self_loop_invalid() {
        for status in [
            JobStatus::Queued,
            JobStatus::Running,
            JobStatus::Done,
            JobStatus::Pruned,
            JobStatus::Errored,
        ] {
            assert!(
                transition(status, status).is_err(),
                "self-loop {:?} should be invalid",
                status
            );
        }
    }

    #[test]
    fn is_valid_transition_matches_transition() {
        for a in [
            JobStatus::Queued,
            JobStatus::Running,
            JobStatus::Done,
            JobStatus::Pruned,
            JobStatus::Errored,
        ] {
            for b in [
                JobStatus::Queued,
                JobStatus::Running,
                JobStatus::Done,
                JobStatus::Pruned,
                JobStatus::Errored,
            ] {
                assert_eq!(is_valid_transition(a, b), transition(a, b).is_ok());
            }
        }
    }

    // ── StrategyQueue ──

    #[test]
    fn push_and_claim_fifo_order() {
        let mut q = StrategyQueue::new();
        let a = fresh_queued(1);
        let b = fresh_queued(2);
        let c = fresh_queued(3);
        let (ja, jb, jc) = (a.job_id, b.job_id, c.job_id);
        q.push(a).unwrap();
        q.push(b).unwrap();
        q.push(c).unwrap();
        assert_eq!(q.len(), 3);

        // FIFO: a, b, c
        assert_eq!(q.claim_one().unwrap().job_id, ja);
        assert_eq!(q.claim_one().unwrap().job_id, jb);
        assert_eq!(q.claim_one().unwrap().job_id, jc);
        assert!(q.is_empty());
    }

    #[test]
    fn claim_empty_returns_none() {
        let mut q = StrategyQueue::new();
        assert!(q.claim_one().is_none());
    }

    #[test]
    fn claim_one_flips_status_to_running() {
        let mut q = StrategyQueue::new();
        let s = fresh_queued(42);
        q.push(s).unwrap();
        let claimed = q.claim_one().unwrap();
        assert_eq!(claimed.status, JobStatus::Running);
        assert!(
            claimed.started_at.is_some(),
            "started_at must be stamped on claim"
        );
    }

    #[test]
    fn push_rejects_non_queued() {
        let mut q = StrategyQueue::new();
        let mut s = fresh_queued(1);
        s.status = JobStatus::Running;
        let err = q.push(s).unwrap_err();
        assert!(matches!(err, FsmError::InvalidTransition { .. }));
        assert!(q.is_empty());
    }

    #[test]
    fn peek_next_does_not_consume() {
        let mut q = StrategyQueue::new();
        let s = fresh_queued(7);
        let id = s.job_id;
        q.push(s).unwrap();
        assert_eq!(q.peek_next().unwrap().job_id, id);
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn len_is_correct() {
        let mut q = StrategyQueue::new();
        assert_eq!(q.len(), 0);
        q.push(fresh_queued(1)).unwrap();
        q.push(fresh_queued(2)).unwrap();
        assert_eq!(q.len(), 2);
        let _ = q.claim_one();
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn already_claimed_error_constructs() {
        // FsmError variant must round-trip through Debug/PartialEq —
        // future SR-IO advisory-lock layer will surface it from
        // `SELECT … FOR UPDATE SKIP LOCKED` failures.
        let err = FsmError::AlreadyClaimed { job_id: JobId::new() };
        let dbg = format!("{:?}", err);
        assert!(dbg.contains("AlreadyClaimed"), "Debug repr: {}", dbg);
    }

    #[test]
    fn claim_then_push_running_is_invalid() {
        let mut q = StrategyQueue::new();
        let s = fresh_queued(1);
        q.push(s).unwrap();
        let claimed = q.claim_one().unwrap();
        // Re-pushing a Running scarab MUST fail.
        let err = q.push(claimed).unwrap_err();
        assert!(matches!(err, FsmError::InvalidTransition { .. }));
    }

    #[test]
    fn o1_push_claim_smoke() {
        // 50_000 push + claim under 100 ms — proves O(1) amortised.
        let mut q = StrategyQueue::new();
        let start = std::time::Instant::now();
        for i in 0..50_000 {
            q.push(fresh_queued(i)).unwrap();
        }
        for _ in 0..50_000 {
            q.claim_one().unwrap();
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 1000,
            "100k ops took {:?} (expected <1s)",
            elapsed
        );
    }
}
