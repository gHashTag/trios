//! SR-02 — A2A MCP Tools
//!
//! MCP-compatible tool definitions for A2A operations.
//! These tools can be registered with trios-server's MCP service.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use trios_a2a_sr00::{AgentCard, AgentId, AgentStatus};
use trios_a2a_sr01::{A2AMessage, Task, TaskState};

/// Max agents a single registry will hold (P3 — registration guard).
pub const MAX_AGENTS: usize = 1024;
/// Max buffered messages before oldest are evicted (P3 — message-log TTL/cap).
pub const MAX_MESSAGES: usize = 10_000;
/// Liveness TTL — an agent with no heartbeat for this long is stale
/// (parity with the retired TS `A2aRegistryService` 120s threshold).
pub const HEARTBEAT_TTL_MS: u64 = 120_000;
/// Max queued wire messages per offline recipient before oldest are evicted.
pub const MAX_PENDING_PER_AGENT: usize = 1_000;

/// A2A registry — holds agents and tasks.
#[derive(Debug, Clone)]
pub struct A2ARegistry {
    pub agents: HashMap<String, AgentCard>,
    pub tasks: HashMap<String, Task>,
    pub messages: Vec<A2AMessage>,
    max_agents: usize,
    max_messages: usize,
    /// Last heartbeat per agent, unix millis (REST liveness — TS parity).
    heartbeats: HashMap<String, u64>,
    /// Client-shaped agent cards, stored verbatim for wire-faithful
    /// `GET /a2a/agents` responses (Swift decodes its own card format).
    wire_cards: HashMap<String, Value>,
    /// Per-recipient queues of undelivered wire messages, drained when the
    /// agent (re)connects its SSE stream.
    pending: HashMap<String, Vec<Value>>,
}

impl A2ARegistry {
    pub fn new() -> Self {
        Self::with_limits(MAX_AGENTS, MAX_MESSAGES)
    }

    /// Construct with explicit bounds (useful for tests / tuning).
    pub fn with_limits(max_agents: usize, max_messages: usize) -> Self {
        Self {
            agents: HashMap::new(),
            tasks: HashMap::new(),
            messages: Vec::new(),
            max_agents,
            max_messages,
            heartbeats: HashMap::new(),
            wire_cards: HashMap::new(),
            pending: HashMap::new(),
        }
    }

    /// Register an agent.
    ///
    /// P3: re-registering an existing id is an idempotent update (not a new
    /// slot); registering a *new* id past `max_agents` is rejected so a
    /// misbehaving peer can't exhaust the registry.
    pub fn register_agent(&mut self, card: AgentCard) -> Value {
        let id = card.id.to_string();
        let is_new = !self.agents.contains_key(&id);
        if is_new && self.agents.len() >= self.max_agents {
            return json!({
                "ok": false,
                "error": format!("agent registry full (max {})", self.max_agents),
                "code": "registry_full"
            });
        }
        let updated = !is_new;
        self.agents.insert(id.clone(), card);
        json!({"ok": true, "agent_id": id, "updated": updated})
    }

    /// True when the agent id is currently registered.
    pub fn has_agent(&self, id: &str) -> bool {
        self.agents.contains_key(id)
    }

    /// Push a message onto the bounded log, evicting the oldest past the cap.
    fn push_message(&mut self, msg: A2AMessage) {
        self.messages.push(msg);
        if self.messages.len() > self.max_messages {
            let overflow = self.messages.len() - self.max_messages;
            self.messages.drain(0..overflow);
        }
    }

    /// List all registered agents.
    pub fn list_agents(&self) -> Value {
        let agents: Vec<&AgentCard> = self.agents.values().collect();
        serde_json::to_value(agents).unwrap_or(json!([]))
    }

    /// Send a direct message from one agent to another.
    ///
    /// P2: a message to an unregistered recipient is rejected instead of
    /// silently queuing forever. The caller gets an actionable error.
    pub fn send_message(&mut self, from: &str, to: &str, payload: Value) -> Value {
        if !self.has_agent(to) {
            return json!({
                "ok": false,
                "error": format!("recipient '{}' is not a registered agent", to),
                "code": "unknown_recipient"
            });
        }
        let msg = A2AMessage::direct(AgentId::new(from), AgentId::new(to), payload);
        let result = serde_json::to_value(&msg).unwrap_or(json!({}));
        self.push_message(msg);
        json!({"ok": true, "message_id": result["id"]})
    }

    /// Broadcast a message to all agents.
    pub fn broadcast(&mut self, from: &str, payload: Value) -> Value {
        let msg = A2AMessage::broadcast(AgentId::new(from), payload);
        let result = serde_json::to_value(&msg).unwrap_or(json!({}));
        self.push_message(msg);
        json!({"ok": true, "message_id": result["id"], "recipients": self.agents.len()})
    }

    /// Assign a task to an agent.
    ///
    /// P2: assigning to an unregistered agent is rejected so tasks don't
    /// strand against a non-existent assignee.
    pub fn assign_task(&mut self, title: &str, created_by: &str, assign_to: &str) -> Value {
        if !self.has_agent(assign_to) {
            return json!({
                "ok": false,
                "error": format!("assignee '{}' is not a registered agent", assign_to),
                "code": "unknown_assignee"
            });
        }
        let task = Task::new(title, AgentId::new(created_by))
            .assign_to(AgentId::new(assign_to));
        let task_id = task.id.clone();
        self.tasks.insert(task_id.clone(), task);
        json!({"ok": true, "task_id": task_id})
    }

    /// Get task status.
    pub fn task_status(&self, task_id: &str) -> Value {
        match self.tasks.get(task_id) {
            Some(task) => serde_json::to_value(task).unwrap_or(json!({"error": "serialize failed"})),
            None => json!({"error": format!("task {} not found", task_id)}),
        }
    }

    /// Update task state.
    pub fn update_task(&mut self, task_id: &str, new_state: TaskState) -> Value {
        match self.tasks.get_mut(task_id) {
            Some(task) => {
                task.state = new_state;
                task.updated_at = chrono::Utc::now().to_rfc3339();
                json!({"ok": true, "task_id": task_id, "state": serde_json::to_value(&task.state).unwrap()})
            }
            None => json!({"error": format!("task {} not found", task_id)}),
        }
    }

    // ------------------------------------------------------------------
    // REST/wire parity layer — logic ported from the retired TS
    // `A2aRegistryService` (browseros apps/server), TS-retirement item 5.
    // Time is injected (`now_ms`) so every rule stays purely testable.
    // ------------------------------------------------------------------

    /// Register an agent from a client-shaped (wire) card.
    ///
    /// Stores the card verbatim for wire-faithful listing, mirrors it into
    /// the typed agent map, and stamps a heartbeat. Same guards as
    /// `register_agent` (P3 capacity, idempotent re-register).
    pub fn register_wire(&mut self, card: Value, now_ms: u64) -> Value {
        let id = card.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
        let name = card.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
        if id.is_empty() || name.is_empty() {
            return json!({"ok": false, "error": "missing agent id or name", "code": "invalid_card"});
        }
        let is_new = !self.agents.contains_key(&id);
        if is_new && self.agents.len() >= self.max_agents {
            return json!({
                "ok": false,
                "error": format!("agent registry full (max {})", self.max_agents),
                "code": "registry_full"
            });
        }
        let typed = AgentCard {
            id: AgentId::new(&id),
            name,
            capabilities: Vec::new(),
            status: AgentStatus::Idle,
            description: card
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        };
        self.agents.insert(id.clone(), typed);
        self.wire_cards.insert(id.clone(), card);
        self.heartbeats.insert(id.clone(), now_ms);
        json!({"ok": true, "agent_id": id, "updated": !is_new})
    }

    /// Remove an agent and all its liveness/queue state.
    pub fn unregister_agent(&mut self, id: &str) -> bool {
        let existed = self.agents.remove(id).is_some();
        self.wire_cards.remove(id);
        self.heartbeats.remove(id);
        self.pending.remove(id);
        existed
    }

    /// Record a heartbeat. Returns false for unknown agents (TS parity).
    pub fn heartbeat(&mut self, id: &str, now_ms: u64) -> bool {
        if self.agents.contains_key(id) {
            self.heartbeats.insert(id.to_string(), now_ms);
            true
        } else {
            false
        }
    }

    /// Wire cards of agents with a fresh heartbeat (TS `listAgents` parity).
    pub fn live_wire_cards(&self, now_ms: u64, ttl_ms: u64) -> Vec<Value> {
        self.wire_cards
            .iter()
            .filter(|(id, _)| {
                let last = self.heartbeats.get(*id).copied().unwrap_or(0);
                now_ms.saturating_sub(last) < ttl_ms
            })
            .map(|(_, card)| card.clone())
            .collect()
    }

    /// Drop agents whose heartbeat is older than `ttl_ms`; returns their ids
    /// (TS heartbeat-watchdog parity).
    pub fn prune_stale(&mut self, now_ms: u64, ttl_ms: u64) -> Vec<String> {
        let stale: Vec<String> = self
            .heartbeats
            .iter()
            .filter(|(_, last)| now_ms.saturating_sub(**last) >= ttl_ms)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &stale {
            self.unregister_agent(id);
        }
        stale
    }

    /// Queue a wire message for an offline-but-registered recipient.
    /// Bounded per agent (oldest evicted). False for unknown recipients (P2).
    pub fn queue_pending(&mut self, recipient: &str, wire_msg: Value) -> bool {
        if !self.agents.contains_key(recipient) {
            return false;
        }
        let queue = self.pending.entry(recipient.to_string()).or_default();
        queue.push(wire_msg);
        if queue.len() > MAX_PENDING_PER_AGENT {
            let overflow = queue.len() - MAX_PENDING_PER_AGENT;
            queue.drain(0..overflow);
        }
        true
    }

    /// Take all queued wire messages for an agent (SSE reconnect flush).
    pub fn drain_pending(&mut self, agent_id: &str) -> Vec<Value> {
        self.pending.remove(agent_id).unwrap_or_default()
    }

    /// Read a task by id.
    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    /// Insert a fully-formed task (client-supplied id preserved). Rejects an
    /// unregistered assignee (P2 parity with `assign_task`).
    pub fn upsert_task(&mut self, task: Task) -> Value {
        if let Some(assignee) = &task.assigned_to {
            if !self.has_agent(assignee.as_str()) {
                return json!({
                    "ok": false,
                    "error": format!("assignee '{}' is not a registered agent", assignee.as_str()),
                    "code": "unknown_assignee"
                });
            }
        }
        let id = task.id.clone();
        self.tasks.insert(id.clone(), task);
        json!({"ok": true, "task_id": id})
    }

    /// Snapshot (clone) of one recipient's pending queue — persistence hook.
    pub fn pending_snapshot(&self, recipient: &str) -> Vec<Value> {
        self.pending.get(recipient).cloned().unwrap_or_default()
    }

    /// All wire cards as `(agent_id, card)` pairs — persistence/matrix hook.
    pub fn all_wire_cards(&self) -> Vec<(String, Value)> {
        self.wire_cards.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// One agent's verbatim wire card, if registered — persistence hook.
    pub fn wire_card_of(&self, id: &str) -> Option<Value> {
        self.wire_cards.get(id).cloned()
    }

    /// All tasks (clones) — persistence/matrix hook.
    pub fn all_tasks(&self) -> Vec<Task> {
        self.tasks.values().cloned().collect()
    }

    /// Last heartbeat (unix ms) of an agent, if known.
    pub fn heartbeat_of(&self, id: &str) -> Option<u64> {
        self.heartbeats.get(id).copied()
    }

    /// Pending-queue length per agent (matrix/dashboard hook).
    pub fn pending_len(&self, id: &str) -> usize {
        self.pending.get(id).map_or(0, Vec::len)
    }

    /// Rebuild in-memory state from a durable store snapshot (SR-04).
    ///
    /// Wire cards re-register with `now_ms` as the heartbeat: restored agents
    /// get one full TTL of grace to reconnect before the watchdog prunes
    /// them. Tasks and pending queues are restored verbatim. Tasks are
    /// inserted directly (not via `upsert_task`) so tasks whose assignee has
    /// not re-registered yet survive the restart.
    pub fn hydrate(
        &mut self,
        wire_cards: Vec<(String, Value)>,
        tasks: Vec<Task>,
        pending: Vec<(String, Vec<Value>)>,
        now_ms: u64,
    ) {
        for (_, card) in wire_cards {
            let _ = self.register_wire(card, now_ms);
        }
        for task in tasks {
            self.tasks.insert(task.id.clone(), task);
        }
        for (recipient, queue) in pending {
            if self.agents.contains_key(&recipient) && !queue.is_empty() {
                let mut q = queue;
                if q.len() > MAX_PENDING_PER_AGENT {
                    let overflow = q.len() - MAX_PENDING_PER_AGENT;
                    q.drain(0..overflow);
                }
                self.pending.insert(recipient, q);
            }
        }
    }
}

/// Thread-safe wrapper for A2A registry.
pub type SharedRegistry = Arc<Mutex<A2ARegistry>>;

/// Create a new shared registry.
pub fn shared_registry() -> SharedRegistry {
    Arc::new(Mutex::new(A2ARegistry::new()))
}

/// MCP tool definitions for A2A.
pub fn mcp_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "a2a_list_agents",
            "description": "List all registered A2A agents",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "a2a_send",
            "description": "Send a direct A2A message to another agent",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": {"type": "string"},
                    "to": {"type": "string"},
                    "payload": {"type": "object"}
                },
                "required": ["from", "to", "payload"]
            }
        }),
        json!({
            "name": "a2a_broadcast",
            "description": "Broadcast a message to all A2A agents",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": {"type": "string"},
                    "payload": {"type": "object"}
                },
                "required": ["from", "payload"]
            }
        }),
        json!({
            "name": "a2a_assign_task",
            "description": "Assign a task to an A2A agent",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": {"type": "string"},
                    "created_by": {"type": "string"},
                    "assign_to": {"type": "string"}
                },
                "required": ["title", "created_by", "assign_to"]
            }
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_agent() {
        let mut reg = A2ARegistry::new();
        let card = AgentCard::new("alpha-1", "Alpha");
        let result = reg.register_agent(card);
        assert_eq!(result["ok"], true);
        assert_eq!(reg.agents.len(), 1);
    }

    #[test]
    fn test_send_message() {
        let mut reg = A2ARegistry::new();
        reg.register_agent(AgentCard::new("alpha", "Alpha"));
        reg.register_agent(AgentCard::new("beta", "Beta"));
        let result = reg.send_message("alpha", "beta", json!({"text": "hello"}));
        assert_eq!(result["ok"], true);
        assert_eq!(reg.messages.len(), 1);
    }

    #[test]
    fn test_assign_task() {
        let mut reg = A2ARegistry::new();
        reg.register_agent(AgentCard::new("alpha", "Alpha"));
        let result = reg.assign_task("Fix bug", "lead", "alpha");
        assert_eq!(result["ok"], true);
        assert_eq!(reg.tasks.len(), 1);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut reg = A2ARegistry::new();
        reg.register_agent(AgentCard::new("alpha", "Alpha"));
        let result = reg.assign_task("Test task", "lead", "alpha");
        let task_id = result["task_id"].as_str().unwrap();
        
        let status = reg.task_status(task_id);
        assert_eq!(status["state"], "assigned");
        
        let update = reg.update_task(task_id, TaskState::Completed);
        assert_eq!(update["ok"], true);
        
        let status = reg.task_status(task_id);
        assert_eq!(status["state"], "completed");
    }

    #[test]
    fn test_mcp_tool_definitions() {
        let tools = mcp_tool_definitions();
        assert_eq!(tools.len(), 4);
        assert_eq!(tools[0]["name"], "a2a_list_agents");
    }

    // ---- P2: reject sends/assigns to unregistered agents ----

    #[test]
    fn send_to_unknown_recipient_is_rejected() {
        let mut reg = A2ARegistry::new();
        reg.register_agent(AgentCard::new("alpha", "Alpha"));
        let result = reg.send_message("alpha", "ghost", json!({"text": "hi"}));
        assert_eq!(result["ok"], false);
        assert_eq!(result["code"], "unknown_recipient");
        // nothing queued
        assert_eq!(reg.messages.len(), 0);
    }

    #[test]
    fn assign_to_unknown_agent_is_rejected() {
        let mut reg = A2ARegistry::new();
        let result = reg.assign_task("Fix", "lead", "ghost");
        assert_eq!(result["ok"], false);
        assert_eq!(result["code"], "unknown_assignee");
        assert_eq!(reg.tasks.len(), 0);
    }

    // ---- P3: registration guard + bounded message log ----

    #[test]
    fn reregister_is_idempotent_update_not_new_slot() {
        let mut reg = A2ARegistry::with_limits(1, 100);
        let r1 = reg.register_agent(AgentCard::new("a", "A"));
        assert_eq!(r1["updated"], false);
        // same id again: allowed even at capacity, marked as update
        let r2 = reg.register_agent(AgentCard::new("a", "A2"));
        assert_eq!(r2["ok"], true);
        assert_eq!(r2["updated"], true);
        assert_eq!(reg.agents.len(), 1);
    }

    #[test]
    fn registry_full_rejects_new_agents() {
        let mut reg = A2ARegistry::with_limits(1, 100);
        reg.register_agent(AgentCard::new("a", "A"));
        let result = reg.register_agent(AgentCard::new("b", "B"));
        assert_eq!(result["ok"], false);
        assert_eq!(result["code"], "registry_full");
        assert_eq!(reg.agents.len(), 1);
    }

    #[test]
    fn message_log_is_bounded() {
        let mut reg = A2ARegistry::with_limits(10, 3);
        reg.register_agent(AgentCard::new("a", "A"));
        reg.register_agent(AgentCard::new("b", "B"));
        for i in 0..5 {
            reg.send_message("a", "b", json!({"n": i}));
        }
        // capped at 3, oldest evicted
        assert_eq!(reg.messages.len(), 3);
    }

    // --- REST/wire parity layer (TS A2aRegistryService port) ---

    fn wire_card(id: &str) -> Value {
        json!({
            "id": id,
            "name": format!("Agent {id}"),
            "description": "test agent",
            "capabilities": ["chat", "browserControl"],
            "version": "1.0.0"
        })
    }

    #[test]
    fn register_wire_stores_card_and_heartbeat() {
        let mut reg = A2ARegistry::new();
        let res = reg.register_wire(wire_card("swift-1"), 1_000);
        assert_eq!(res["ok"], true);
        assert!(reg.has_agent("swift-1"));
        let live = reg.live_wire_cards(2_000, HEARTBEAT_TTL_MS);
        assert_eq!(live.len(), 1);
        assert_eq!(live[0]["capabilities"][1], "browserControl");
    }

    #[test]
    fn register_wire_rejects_missing_id_or_name() {
        let mut reg = A2ARegistry::new();
        assert_eq!(reg.register_wire(json!({"name": "x"}), 0)["code"], "invalid_card");
        assert_eq!(reg.register_wire(json!({"id": "x"}), 0)["code"], "invalid_card");
    }

    #[test]
    fn heartbeat_only_for_registered() {
        let mut reg = A2ARegistry::new();
        assert!(!reg.heartbeat("ghost", 0));
        reg.register_wire(wire_card("a"), 0);
        assert!(reg.heartbeat("a", 50));
    }

    #[test]
    fn stale_agents_hidden_and_pruned() {
        let mut reg = A2ARegistry::new();
        reg.register_wire(wire_card("old"), 0);
        reg.register_wire(wire_card("fresh"), 200_000);
        // old: 300_000 - 0 >= 120_000 → stale
        assert_eq!(reg.live_wire_cards(300_000, HEARTBEAT_TTL_MS).len(), 1);
        let pruned = reg.prune_stale(300_000, HEARTBEAT_TTL_MS);
        assert_eq!(pruned, vec!["old".to_string()]);
        assert!(!reg.has_agent("old"));
        assert!(reg.has_agent("fresh"));
    }

    #[test]
    fn unregister_clears_all_state() {
        let mut reg = A2ARegistry::new();
        reg.register_wire(wire_card("a"), 0);
        assert!(reg.queue_pending("a", json!({"m": 1})));
        assert!(reg.unregister_agent("a"));
        assert!(!reg.unregister_agent("a"));
        assert!(reg.drain_pending("a").is_empty());
        assert!(reg.live_wire_cards(0, HEARTBEAT_TTL_MS).is_empty());
    }

    #[test]
    fn pending_queue_is_bounded_and_drains_fifo() {
        let mut reg = A2ARegistry::new();
        reg.register_wire(wire_card("a"), 0);
        for i in 0..(MAX_PENDING_PER_AGENT + 5) {
            reg.queue_pending("a", json!({"n": i}));
        }
        let drained = reg.drain_pending("a");
        assert_eq!(drained.len(), MAX_PENDING_PER_AGENT);
        // oldest evicted → first kept element is n=5
        assert_eq!(drained[0]["n"], 5);
        // second drain is empty
        assert!(reg.drain_pending("a").is_empty());
    }

    #[test]
    fn queue_pending_rejects_unknown_recipient() {
        let mut reg = A2ARegistry::new();
        assert!(!reg.queue_pending("ghost", json!({})));
    }

    #[test]
    fn upsert_task_preserves_id_and_guards_assignee() {
        let mut reg = A2ARegistry::new();
        reg.register_wire(wire_card("worker"), 0);
        let task = Task::new("wired", AgentId::new("system")).assign_to(AgentId::new("worker"));
        let id = task.id.clone();
        let res = reg.upsert_task(task);
        assert_eq!(res["ok"], true);
        assert_eq!(res["task_id"], id.as_str());
        assert!(reg.get_task(&id).is_some());

        let bad = Task::new("strand", AgentId::new("system")).assign_to(AgentId::new("ghost"));
        assert_eq!(reg.upsert_task(bad)["code"], "unknown_assignee");
    }

    #[test]
    fn hydrate_restores_state_with_heartbeat_grace() {
        let mut reg = A2ARegistry::new();
        let cards = vec![("w".to_string(), wire_card("w"))];
        let task = Task::new("restored", AgentId::new("system")).assign_to(AgentId::new("gone"));
        let task_id = task.id.clone();
        let pending = vec![
            ("w".to_string(), vec![json!({"n": 1})]),
            ("ghost".to_string(), vec![json!({"n": 2})]), // unknown agent — dropped
        ];
        reg.hydrate(cards, vec![task], pending, 1_000);

        assert!(reg.has_agent("w"));
        assert_eq!(reg.heartbeat_of("w"), Some(1_000));
        // task restored even though assignee "gone" is not registered
        assert!(reg.get_task(&task_id).is_some());
        assert_eq!(reg.pending_len("w"), 1);
        assert_eq!(reg.pending_len("ghost"), 0);
        // live within TTL grace after restore
        assert_eq!(reg.live_wire_cards(1_000 + HEARTBEAT_TTL_MS - 1, HEARTBEAT_TTL_MS).len(), 1);
        // snapshot helper mirrors the queue
        assert_eq!(reg.pending_snapshot("w").len(), 1);
        assert_eq!(reg.all_wire_cards().len(), 1);
    }
}
