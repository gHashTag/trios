//! BR-OUTPUT — AgentMemory assembler ring
//! Closes #461 (GOLD IV · trios-agent-memory)
//!
//! Exports the unified `AgentMemory` trait consumed by every agent.
//! phi² + phi⁻² = 3

#![forbid(unsafe_code)]

use async_trait::async_trait;

// ---------------------------------------------------------------------------
// Core types (re-exported from SR-MEM-00 once wired)
// ---------------------------------------------------------------------------

/// Opaque retrieval context carrying budget and session metadata.
#[derive(Debug, Clone)]
pub struct Context {
    pub budget_tokens: usize,
    pub session_id: String,
}

/// A single memory fact (triple: subject / predicate / object).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// Provenance metadata for remembered facts.
#[derive(Debug, Clone)]
pub struct Provenance {
    pub source: String,
    pub timestamp_unix: u64,
}

/// The result of a `reflect` call.
#[derive(Debug, Clone)]
pub struct Reflection {
    pub answer: String,
    pub confidence: f64,
}

/// Policy controlling what to forget.
#[derive(Debug, Clone)]
pub enum ForgetPolicy {
    /// GDPR: forget all facts about a given subject.
    GdprErase { subject: String },
    /// TTL: forget facts older than `older_than_secs` seconds.
    Ttl { older_than_secs: u64 },
}

// ---------------------------------------------------------------------------
// AgentMemory trait — the four-verb interface
// ---------------------------------------------------------------------------

/// Unified memory interface consumed by all Trinity agents.
///
/// Implementations:
/// - `KgAgentMemory` (default, wires SR-MEM-01 + SR-MEM-05)
///
/// Planned extensions (tracked in sub-issues):
/// - HyDE expansion for `recall` → SR-MEM-02
/// - Supersede dedup for `reflect` → SR-MEM-03
/// - GDPR audit log for `forget` → SR-MEM-04
/// - Vector search for `recall` → SR-MEM-06
#[async_trait]
pub trait AgentMemory: Send + Sync {
    /// Retrieve relevant facts within the token budget.
    async fn recall(&self, ctx: &Context, budget_tokens: usize) -> Vec<Fact>;

    /// Persist new facts with provenance; returns count stored.
    async fn remember(
        &mut self,
        facts: Vec<Fact>,
        prov: Provenance,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>>;

    /// Answer a free-form question by reasoning over stored facts.
    async fn reflect(&mut self, question: &str) -> Reflection;

    /// Erase facts matching the policy; returns count removed.
    async fn forget(&mut self, policy: ForgetPolicy) -> usize;
}

// ---------------------------------------------------------------------------
// KgAgentMemory — default stub implementation
// TODO: wire SR-MEM-01 (KG writer) + SR-MEM-05 (episodic bridge) after #455
// ---------------------------------------------------------------------------

/// Default implementation over the KG long-term memory store.
///
/// Currently a stub — full wiring pending SR-MEM-05 (#455) merge.
pub struct KgAgentMemory {
    /// In-memory store for scaffold / tests (replace with SR-MEM-01 client)
    store: Vec<(Fact, Provenance)>,
}

impl KgAgentMemory {
    pub fn new() -> Self {
        Self { store: Vec::new() }
    }
}

impl Default for KgAgentMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentMemory for KgAgentMemory {
    async fn recall(&self, _ctx: &Context, budget_tokens: usize) -> Vec<Fact> {
        // TODO SR-MEM-02: HyDE expansion + semantic search
        self.store
            .iter()
            .take(budget_tokens / 10 + 1)
            .map(|(f, _)| f.clone())
            .collect()
    }

    async fn remember(
        &mut self,
        facts: Vec<Fact>,
        prov: Provenance,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        // TODO SR-MEM-01: write to KG via sr-mem-01 client
        let count = facts.len();
        for fact in facts {
            self.store.push((fact, prov.clone()));
        }
        Ok(count)
    }

    async fn reflect(&mut self, question: &str) -> Reflection {
        // TODO SR-MEM-03: supersede dedup + LLM reasoning chain
        Reflection {
            answer: format!("[stub] No reflection implemented yet. Question: {question}"),
            confidence: 0.0,
        }
    }

    async fn forget(&mut self, policy: ForgetPolicy) -> usize {
        // TODO SR-MEM-04: GDPR audit log
        let before = self.store.len();
        match &policy {
            ForgetPolicy::GdprErase { subject } => {
                self.store.retain(|(f, _)| &f.subject != subject);
            }
            ForgetPolicy::Ttl { older_than_secs } => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                self.store
                    .retain(|(_, p)| now.saturating_sub(p.timestamp_unix) <= *older_than_secs);
            }
        }
        before - self.store.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn prov() -> Provenance {
        Provenance {
            source: "test".into(),
            timestamp_unix: 0,
        }
    }

    #[tokio::test]
    async fn test_remember_recall_roundtrip() {
        let mut mem = KgAgentMemory::new();
        let fact = Fact {
            subject: "agent:scarab".into(),
            predicate: "knows".into(),
            object: "phi^2+phi^-2=3".into(),
        };
        let stored = mem.remember(vec![fact.clone()], prov()).await.unwrap();
        assert_eq!(stored, 1);

        let ctx = Context {
            budget_tokens: 100,
            session_id: "test-session".into(),
        };
        let recalled = mem.recall(&ctx, 100).await;
        assert_eq!(recalled.len(), 1);
        assert_eq!(recalled[0], fact);
    }

    #[tokio::test]
    async fn test_forget_gdpr_erase() {
        let mut mem = KgAgentMemory::new();
        let fact = Fact {
            subject: "user:alice".into(),
            predicate: "email".into(),
            object: "alice@example.com".into(),
        };
        mem.remember(vec![fact], prov()).await.unwrap();
        let removed = mem
            .forget(ForgetPolicy::GdprErase {
                subject: "user:alice".into(),
            })
            .await;
        assert_eq!(removed, 1);
        let ctx = Context {
            budget_tokens: 100,
            session_id: "test".into(),
        };
        assert!(mem.recall(&ctx, 100).await.is_empty());
    }

    #[tokio::test]
    async fn test_phi_anchor() {
        // phi² + phi⁻² = 3
        let phi: f64 = (1.0 + 5.0_f64.sqrt()) / 2.0;
        let lhs = phi * phi + 1.0 / (phi * phi);
        assert!((lhs - 3.0).abs() < 1e-10, "phi anchor violated: {lhs}");
    }
}
