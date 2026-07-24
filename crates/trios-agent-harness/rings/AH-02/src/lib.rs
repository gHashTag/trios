//! AH-02 — per-agent message queue.
//!
//! Ported from `lib/agents/message-queue.ts` (`FileMessageQueue`). The TS
//! version persists to a JSON file behind a write-lock; here we provide the
//! bounded FIFO semantics as a pure in-memory structure guarded by a mutex.
//! Durable persistence is layered separately (trios-store / SR-04-style),
//! keeping this ring's logic I/O-free and unit-testable.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

pub const DEFAULT_MAX_LENGTH: usize = 100;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedMessageAttachment {
    pub media_type: String,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedMessage {
    pub id: String,
    pub text: String,
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<QueuedMessageAttachment>,
}

impl QueuedMessage {
    pub fn new(id: &str, text: &str, created_at: i64) -> Self {
        Self { id: id.into(), text: text.into(), created_at, attachments: Vec::new() }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
#[error("message queue for agent '{agent_id}' is full (max {max})")]
pub struct QueueFullError {
    pub agent_id: String,
    pub max: usize,
}

/// Bounded per-agent FIFO queue. Mirrors the TS `FileMessageQueue` API
/// (`append`, `pop_oldest`, `push_front`, `remove`, `list`, `snapshot_all`,
/// `agents_with_pending`).
pub struct MessageQueue {
    inner: Mutex<HashMap<String, Vec<QueuedMessage>>>,
    max_length: usize,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self::with_max(DEFAULT_MAX_LENGTH)
    }
    pub fn with_max(max_length: usize) -> Self {
        Self { inner: Mutex::new(HashMap::new()), max_length }
    }

    pub fn append(&self, agent_id: &str, message: QueuedMessage) -> Result<(), QueueFullError> {
        let mut g = self.inner.lock().unwrap();
        let q = g.entry(agent_id.to_string()).or_default();
        if q.len() >= self.max_length {
            return Err(QueueFullError { agent_id: agent_id.into(), max: self.max_length });
        }
        q.push(message);
        Ok(())
    }

    pub fn pop_oldest(&self, agent_id: &str) -> Option<QueuedMessage> {
        let mut g = self.inner.lock().unwrap();
        let q = g.get_mut(agent_id)?;
        if q.is_empty() {
            return None;
        }
        Some(q.remove(0))
    }

    /// Re-queue a message at the front (used when a turn is interrupted).
    pub fn push_front(&self, agent_id: &str, message: QueuedMessage) {
        let mut g = self.inner.lock().unwrap();
        g.entry(agent_id.to_string()).or_default().insert(0, message);
    }

    pub fn remove(&self, agent_id: &str, message_id: &str) -> bool {
        let mut g = self.inner.lock().unwrap();
        if let Some(q) = g.get_mut(agent_id) {
            if let Some(pos) = q.iter().position(|m| m.id == message_id) {
                q.remove(pos);
                return true;
            }
        }
        false
    }

    pub fn list(&self, agent_id: &str) -> Vec<QueuedMessage> {
        let g = self.inner.lock().unwrap();
        g.get(agent_id).cloned().unwrap_or_default()
    }

    pub fn snapshot_all(&self) -> HashMap<String, Vec<QueuedMessage>> {
        self.inner.lock().unwrap().clone()
    }

    pub fn agents_with_pending(&self) -> Vec<String> {
        let g = self.inner.lock().unwrap();
        g.iter().filter(|(_, q)| !q.is_empty()).map(|(k, _)| k.clone()).collect()
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_order() {
        let q = MessageQueue::new();
        q.append("a", QueuedMessage::new("1", "first", 1)).unwrap();
        q.append("a", QueuedMessage::new("2", "second", 2)).unwrap();
        assert_eq!(q.pop_oldest("a").unwrap().id, "1");
        assert_eq!(q.pop_oldest("a").unwrap().id, "2");
        assert!(q.pop_oldest("a").is_none());
    }

    #[test]
    fn bounded_returns_full_error() {
        let q = MessageQueue::with_max(2);
        q.append("a", QueuedMessage::new("1", "x", 1)).unwrap();
        q.append("a", QueuedMessage::new("2", "y", 2)).unwrap();
        let err = q.append("a", QueuedMessage::new("3", "z", 3)).unwrap_err();
        assert_eq!(err.max, 2);
        assert_eq!(err.agent_id, "a");
    }

    #[test]
    fn push_front_and_remove() {
        let q = MessageQueue::new();
        q.append("a", QueuedMessage::new("1", "a", 1)).unwrap();
        q.push_front("a", QueuedMessage::new("0", "front", 0));
        assert_eq!(q.list("a")[0].id, "0");
        assert!(q.remove("a", "1"));
        assert!(!q.remove("a", "nope"));
        assert_eq!(q.list("a").len(), 1);
    }

    #[test]
    fn agents_with_pending_filters_empty() {
        let q = MessageQueue::new();
        q.append("a", QueuedMessage::new("1", "x", 1)).unwrap();
        q.append("b", QueuedMessage::new("1", "y", 1)).unwrap();
        q.pop_oldest("b");
        assert_eq!(q.agents_with_pending(), vec!["a".to_string()]);
    }
}
