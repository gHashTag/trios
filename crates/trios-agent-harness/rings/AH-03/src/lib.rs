//! AH-03 — active turn registry.
//!
//! Ported from `lib/agents/active-turn-registry.ts`. Provides the pure state
//! machine: `RingBuffer` (bounded frame log with a retained terminal frame)
//! and `TurnRegistry` (register / append / complete / cancel / query).
//!
//! The async streaming layer (subscribers, AbortController, SSE pump, sweep
//! timer) lives in the runtime/http ring — this ring keeps the buffering and
//! lifecycle logic deterministic and unit-testable. Depends only on AH-00.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use trios_agent_harness_ah00::AgentStreamEvent;

pub const DEFAULT_BUFFER_CAPACITY: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TurnStatus {
    Running,
    Done,
    Error,
    Cancelled,
}

impl TurnStatus {
    pub fn is_terminal(&self) -> bool {
        !matches!(self, TurnStatus::Running)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnFrame {
    pub seq: u64,
    pub event: AgentStreamEvent,
    pub created_at: i64,
}

/// Snapshot of a turn's lifecycle (TS `ActiveTurnInfo`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTurnInfo {
    pub turn_id: String,
    pub agent_id: String,
    pub status: TurnStatus,
    pub last_seq: i64,
    pub started_at: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ended_at: Option<i64>,
    pub prompt: Option<String>,
}

/// Bounded frame log. Keeps at most `capacity` frames and always retains the
/// terminal (`done`/`error`) frame so late subscribers still observe the end.
#[derive(Debug)]
pub struct RingBuffer {
    frames: Vec<TurnFrame>,
    capacity: usize,
    next_seq: u64,
    terminal: Option<TurnFrame>,
    pub truncated: bool,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self { frames: Vec::new(), capacity, next_seq: 0, terminal: None, truncated: false }
    }

    pub fn push(&mut self, event: AgentStreamEvent, now: i64) -> TurnFrame {
        let frame = TurnFrame { seq: self.next_seq, event, created_at: now };
        self.next_seq += 1;
        if matches!(frame.event, AgentStreamEvent::Done { .. } | AgentStreamEvent::Error { .. }) {
            self.terminal = Some(frame.clone());
        }
        self.frames.push(frame.clone());
        if self.frames.len() > self.capacity {
            self.frames.remove(0);
            self.truncated = true;
        }
        frame
    }

    /// Frames with `seq > from_seq`, plus the terminal frame if not included.
    pub fn slice(&self, from_seq: i64) -> Vec<TurnFrame> {
        let mut live: Vec<TurnFrame> =
            self.frames.iter().filter(|f| f.seq as i64 > from_seq).cloned().collect();
        if let Some(term) = &self.terminal {
            let present = live.iter().any(|f| f.seq == term.seq);
            if !present && term.seq as i64 > from_seq {
                live.push(term.clone());
            }
        }
        live
    }

    pub fn last_seq(&self) -> i64 {
        self.next_seq as i64 - 1
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

struct Turn {
    info: ActiveTurnInfo,
    buffer: RingBuffer,
}

/// Registry of turns keyed by `turn_id`. Thread-safe state machine.
pub struct TurnRegistry {
    turns: Mutex<HashMap<String, Turn>>,
    capacity: usize,
}

impl TurnRegistry {
    pub fn new() -> Self {
        Self { turns: Mutex::new(HashMap::new()), capacity: DEFAULT_BUFFER_CAPACITY }
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self { turns: Mutex::new(HashMap::new()), capacity }
    }

    pub fn register(&self, turn_id: &str, agent_id: &str, prompt: Option<String>, started_at: i64) {
        let mut g = self.turns.lock().unwrap();
        g.insert(
            turn_id.to_string(),
            Turn {
                info: ActiveTurnInfo {
                    turn_id: turn_id.into(),
                    agent_id: agent_id.into(),
                    status: TurnStatus::Running,
                    last_seq: -1,
                    started_at,
                    ended_at: None,
                    prompt,
                },
                buffer: RingBuffer::new(self.capacity),
            },
        );
    }

    /// Append an event to a turn's buffer. Returns the assigned seq.
    pub fn append(&self, turn_id: &str, event: AgentStreamEvent, now: i64) -> Option<u64> {
        let mut g = self.turns.lock().unwrap();
        let turn = g.get_mut(turn_id)?;
        let frame = turn.buffer.push(event, now);
        turn.info.last_seq = frame.seq as i64;
        Some(frame.seq)
    }

    fn mark_terminal(&self, turn_id: &str, status: TurnStatus, ended_at: i64) -> bool {
        let mut g = self.turns.lock().unwrap();
        match g.get_mut(turn_id) {
            Some(turn) if turn.info.status == TurnStatus::Running => {
                turn.info.status = status;
                turn.info.ended_at = Some(ended_at);
                true
            }
            _ => false,
        }
    }

    pub fn complete(&self, turn_id: &str, ended_at: i64) -> bool {
        self.mark_terminal(turn_id, TurnStatus::Done, ended_at)
    }
    pub fn fail(&self, turn_id: &str, ended_at: i64) -> bool {
        self.mark_terminal(turn_id, TurnStatus::Error, ended_at)
    }
    pub fn cancel(&self, turn_id: &str, ended_at: i64) -> bool {
        self.mark_terminal(turn_id, TurnStatus::Cancelled, ended_at)
    }

    pub fn get(&self, turn_id: &str) -> Option<ActiveTurnInfo> {
        self.turns.lock().unwrap().get(turn_id).map(|t| t.info.clone())
    }

    pub fn slice(&self, turn_id: &str, from_seq: i64) -> Option<Vec<TurnFrame>> {
        self.turns.lock().unwrap().get(turn_id).map(|t| t.buffer.slice(from_seq))
    }

    pub fn list(&self) -> Vec<ActiveTurnInfo> {
        self.turns.lock().unwrap().values().map(|t| t.info.clone()).collect()
    }

    /// Drop turns whose terminal `ended_at` is older than `older_than`.
    pub fn sweep(&self, older_than: i64) -> usize {
        let mut g = self.turns.lock().unwrap();
        let before = g.len();
        g.retain(|_, t| match t.info.ended_at {
            Some(end) => end >= older_than,
            None => true,
        });
        before - g.len()
    }
}

impl Default for TurnRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trios_agent_harness_ah00::TextStream;

    fn text(t: &str) -> AgentStreamEvent {
        AgentStreamEvent::TextDelta { text: t.into(), stream: TextStream::Output }
    }

    #[test]
    fn ringbuffer_seq_and_slice() {
        let mut b = RingBuffer::new(10);
        b.push(text("a"), 1);
        b.push(text("b"), 2);
        assert_eq!(b.last_seq(), 1);
        let s = b.slice(0);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].seq, 1);
    }

    #[test]
    fn ringbuffer_retains_terminal_after_truncation() {
        let mut b = RingBuffer::new(2);
        b.push(AgentStreamEvent::Done { text: None, stop_reason: None }, 0); // seq 0 terminal
        b.push(text("x"), 1);
        b.push(text("y"), 2); // evicts seq 0 from live frames
        assert!(b.truncated);
        // slice from -1 must still include the terminal frame (seq 0)
        let s = b.slice(-1);
        assert!(s.iter().any(|f| matches!(f.event, AgentStreamEvent::Done { .. })));
    }

    #[test]
    fn registry_lifecycle() {
        let r = TurnRegistry::new();
        r.register("t1", "agentA", Some("hi".into()), 100);
        assert_eq!(r.get("t1").unwrap().status, TurnStatus::Running);
        r.append("t1", text("hello"), 101);
        assert_eq!(r.get("t1").unwrap().last_seq, 0);
        assert!(r.complete("t1", 200));
        assert_eq!(r.get("t1").unwrap().status, TurnStatus::Done);
        // second terminal transition is a no-op
        assert!(!r.cancel("t1", 300));
    }

    #[test]
    fn registry_sweep_drops_old_terminal_turns() {
        let r = TurnRegistry::new();
        r.register("old", "a", None, 1);
        r.complete("old", 10);
        r.register("live", "a", None, 5);
        let dropped = r.sweep(20);
        assert_eq!(dropped, 1);
        assert!(r.get("old").is_none());
        assert!(r.get("live").is_some()); // running, never swept
    }
}
