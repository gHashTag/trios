//! SR-MEM-01 — KG client adapter (retry + circuit-breaker over `trios-kg`).
//!
//! Honest scope (R5):
//! - This ring defines the `KgAdapter` *contract* (4 verbs:
//!   `remember_triple`, `recall_by_pattern`, `supersede`, `tombstone`),
//!   the retry policy (exponential backoff, max 3 attempts, 30 s budget),
//!   and the circuit breaker state machine (5-of-60 s ⇒ open, 30 s half-open).
//! - The concrete `trios_kg::KgClient` adapter ships in
//!   `crates/trios-agent-memory/adapters/trios_kg.rs` (BR-IO ring) so
//!   SR-MEM-01 stays Silver-tier and unit-testable without spinning a real
//!   zig-knowledge-graph server.
//!
//! Closes #453 · Part of #446 · Anchor: phi^2 + phi^-2 = 3

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::{debug, instrument, warn};
use trios_agent_memory_sr_mem_00::{Triple, TripleId};

// ───────────── retry / breaker config ─────────────

/// Retry & breaker policy. All values match the issue acceptance criteria.
#[derive(Debug, Clone, Copy)]
pub struct AdapterConfig {
    /// Max retry attempts per call (incl. the original).
    pub max_attempts: u32,
    /// Total time budget for a single call (across all retries).
    pub call_budget: Duration,
    /// Initial backoff between retries.
    pub backoff_initial: Duration,
    /// Backoff multiplier per attempt.
    pub backoff_multiplier: f64,
    /// Number of consecutive failures within `breaker_window` to trip the breaker.
    pub breaker_threshold: u32,
    /// Sliding window for failure counting.
    pub breaker_window: Duration,
    /// How long the breaker stays open before transitioning to half-open.
    pub breaker_open_duration: Duration,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            call_budget: Duration::from_secs(30),
            backoff_initial: Duration::from_millis(200),
            backoff_multiplier: 2.0,
            breaker_threshold: 5,
            breaker_window: Duration::from_secs(60),
            breaker_open_duration: Duration::from_secs(30),
        }
    }
}

// ───────────── errors ─────────────

/// Errors surfaced by [`KgAdapter`].
#[derive(Debug, Error)]
pub enum AdapterErr {
    /// The circuit breaker is currently open — call refused.
    #[error("circuit breaker open")]
    BreakerOpen,
    /// All retry attempts exhausted with the underlying error.
    #[error("retry budget exhausted: {0}")]
    RetryExhausted(String),
    /// The wrapped client returned a hard error (no retry is appropriate).
    #[error("client error: {0}")]
    Client(String),
    /// Total per-call budget elapsed before completion.
    #[error("call budget elapsed")]
    BudgetElapsed,
}

// ───────────── recall pattern ─────────────

/// Pattern for [`KgAdapter::recall_by_pattern`]. All fields optional —
/// `None` means "match any".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecallPattern {
    /// Match a specific subject string.
    pub subject: Option<String>,
    /// Match a specific predicate string.
    pub predicate: Option<String>,
    /// Match a specific object string.
    pub object: Option<String>,
}

impl RecallPattern {
    /// Convenience constructor: pattern with only the predicate set.
    pub fn predicate(p: impl Into<String>) -> Self {
        Self {
            predicate: Some(p.into()),
            ..Default::default()
        }
    }
    /// Convenience constructor: pattern with only the subject set.
    pub fn subject(s: impl Into<String>) -> Self {
        Self {
            subject: Some(s.into()),
            ..Default::default()
        }
    }
    /// Whether `triple` matches all set components of this pattern.
    pub fn matches(&self, triple: &Triple) -> bool {
        let s_ok = self.subject.as_deref().is_none_or(|s| s == triple.subject);
        let p_ok = self
            .predicate
            .as_deref()
            .is_none_or(|p| p == triple.predicate);
        let o_ok = self.object.as_deref().is_none_or(|o| o == triple.object);
        s_ok && p_ok && o_ok
    }
}

// ───────────── KG backend trait ─────────────

/// Backend the adapter retries against. Implemented by a concrete
/// `trios_kg::KgClient` wrapper in a sibling BR-IO ring; the unit tests
/// in this file implement it with an in-memory mock.
pub trait KgBackend: Send + Sync {
    /// Persist a triple. Idempotent on `triple.id`.
    fn put_triple<'a>(
        &'a self,
        triple: &'a Triple,
    ) -> Pin<Box<dyn Future<Output = Result<TripleId, String>> + Send + 'a>>;

    /// Recall triples matching the pattern. `budget` caps the result count.
    fn query_pattern<'a>(
        &'a self,
        pattern: &'a RecallPattern,
        budget: usize,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Triple>, String>> + Send + 'a>>;

    /// Delete a triple by id (tombstone).
    fn delete_triple<'a>(
        &'a self,
        id: TripleId,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}

// ───────────── breaker state ─────────────

#[derive(Debug)]
struct BreakerState {
    /// Timestamps of recent failures (newest last).
    failures: Vec<Instant>,
    /// `Some(opened_at)` means the breaker is open since that instant.
    opened_at: Option<Instant>,
}

impl BreakerState {
    fn new() -> Self {
        Self {
            failures: Vec::new(),
            opened_at: None,
        }
    }
}

// ───────────── KgAdapter ─────────────

/// Retry-and-breaker wrapper around any [`KgBackend`].
///
/// Exposes the four GOLD-IV memory verbs the rest of the kingdom needs:
/// `remember_triple`, `recall_by_pattern`, `supersede`, `tombstone`.
pub struct KgAdapter<B: KgBackend> {
    backend: Arc<B>,
    config: AdapterConfig,
    breaker: Arc<Mutex<BreakerState>>,
    attempt_counter: AtomicU32,
}

impl<B: KgBackend> KgAdapter<B> {
    /// Build with default policy (matches issue acceptance criteria).
    pub fn new(backend: B) -> Self {
        Self::with_config(backend, AdapterConfig::default())
    }

    /// Build with a custom policy.
    pub fn with_config(backend: B, config: AdapterConfig) -> Self {
        Self {
            backend: Arc::new(backend),
            config,
            breaker: Arc::new(Mutex::new(BreakerState::new())),
            attempt_counter: AtomicU32::new(0),
        }
    }

    /// Total number of underlying attempts seen (including retries).
    /// Useful as a property-test counter.
    pub fn attempts_seen(&self) -> u32 {
        self.attempt_counter.load(Ordering::Relaxed)
    }

    /// Idempotent insert.
    #[instrument(skip(self, triple), fields(triple_id = %triple.id))]
    pub async fn remember_triple(&self, triple: &Triple) -> Result<TripleId, AdapterErr> {
        self.with_retry(|| self.backend.put_triple(triple)).await
    }

    /// Pattern-matching recall. `budget` caps the result count.
    #[instrument(skip(self))]
    pub async fn recall_by_pattern(
        &self,
        pattern: &RecallPattern,
        budget: usize,
    ) -> Result<Vec<Triple>, AdapterErr> {
        self.with_retry(|| self.backend.query_pattern(pattern, budget))
            .await
    }

    /// Replace an old triple with a new one. Insert-then-delete semantics
    /// (insert first so a crash mid-supersede leaves the new value durable
    /// before the old one is removed).
    #[instrument(skip(self, new), fields(old = %old, new_id = %new.id))]
    pub async fn supersede(&self, old: TripleId, new: &Triple) -> Result<TripleId, AdapterErr> {
        let new_id = self.remember_triple(new).await?;
        self.tombstone(old).await?;
        Ok(new_id)
    }

    /// Tombstone a triple by id.
    #[instrument(skip(self))]
    pub async fn tombstone(&self, id: TripleId) -> Result<(), AdapterErr> {
        self.with_retry(|| self.backend.delete_triple(id)).await
    }

    // ── retry / breaker plumbing ──

    async fn with_retry<F, Fut, T>(&self, mut op: F) -> Result<T, AdapterErr>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, String>>,
    {
        // Breaker check
        if self.is_breaker_open().await {
            return Err(AdapterErr::BreakerOpen);
        }

        let started = Instant::now();
        let mut delay = self.config.backoff_initial;
        let mut last_err: Option<String> = None;

        for attempt in 1..=self.config.max_attempts {
            // Budget check
            if started.elapsed() >= self.config.call_budget {
                return Err(AdapterErr::BudgetElapsed);
            }
            self.attempt_counter.fetch_add(1, Ordering::Relaxed);
            match op().await {
                Ok(value) => {
                    self.record_success().await;
                    return Ok(value);
                }
                Err(e) => {
                    debug!(attempt, error = %e, "kg call failed");
                    last_err = Some(e);
                    if attempt < self.config.max_attempts {
                        // Sleep, but never overshoot the budget.
                        let remaining = self.config.call_budget.saturating_sub(started.elapsed());
                        let sleep_for = delay.min(remaining);
                        tokio::time::sleep(sleep_for).await;
                        delay = Duration::from_millis(
                            (delay.as_millis() as f64 * self.config.backoff_multiplier) as u64,
                        );
                    }
                }
            }
        }

        // All attempts failed → record + maybe trip breaker.
        self.record_failure().await;
        Err(AdapterErr::RetryExhausted(
            last_err.unwrap_or_else(|| "unknown".into()),
        ))
    }

    async fn is_breaker_open(&self) -> bool {
        let mut state = self.breaker.lock().await;
        if let Some(opened_at) = state.opened_at {
            if opened_at.elapsed() >= self.config.breaker_open_duration {
                // Half-open — allow one probe.
                debug!("breaker entering half-open");
                state.opened_at = None;
                state.failures.clear();
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    async fn record_success(&self) {
        let mut state = self.breaker.lock().await;
        state.failures.clear();
        state.opened_at = None;
    }

    async fn record_failure(&self) {
        let mut state = self.breaker.lock().await;
        let now = Instant::now();
        state.failures.push(now);
        // Drop failures outside the window.
        let window = self.config.breaker_window;
        state.failures.retain(|t| now.duration_since(*t) <= window);
        if state.failures.len() as u32 >= self.config.breaker_threshold && state.opened_at.is_none()
        {
            warn!(
                failures = state.failures.len(),
                "tripping circuit breaker"
            );
            state.opened_at = Some(now);
        }
    }
}

// ───────────── tests ─────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex as StdMutex;
    use trios_agent_memory_sr_mem_00::{AgentRole, Provenance};

    fn dummy_provenance() -> Provenance {
        Provenance {
            agent_id: AgentRole::Lead,
            task_id: uuid::Uuid::new_v4(),
            source_sha: TripleId::from_triple("commit", "abcdef", ""),
            ts: chrono::Utc::now(),
        }
    }

    fn t(s: &str, p: &str, o: &str) -> Triple {
        Triple::new(s, p, o, dummy_provenance())
    }

    /// In-memory mock with controllable failure injection.
    struct MockBackend {
        store: StdMutex<HashMap<TripleId, Triple>>,
        // Each entry is decremented on every call; while > 0, the call fails.
        fail_next: StdMutex<u32>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                store: StdMutex::new(HashMap::new()),
                fail_next: StdMutex::new(0),
            }
        }
        fn fail_for(&self, n: u32) {
            *self.fail_next.lock().unwrap() = n;
        }
        fn maybe_fail(&self) -> Result<(), String> {
            let mut n = self.fail_next.lock().unwrap();
            if *n > 0 {
                *n -= 1;
                Err("mock injected failure".into())
            } else {
                Ok(())
            }
        }
    }

    impl KgBackend for MockBackend {
        fn put_triple<'a>(
            &'a self,
            triple: &'a Triple,
        ) -> Pin<Box<dyn Future<Output = Result<TripleId, String>> + Send + 'a>> {
            Box::pin(async move {
                self.maybe_fail()?;
                self.store.lock().unwrap().insert(triple.id, triple.clone());
                Ok(triple.id)
            })
        }
        fn query_pattern<'a>(
            &'a self,
            pattern: &'a RecallPattern,
            budget: usize,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<Triple>, String>> + Send + 'a>> {
            Box::pin(async move {
                self.maybe_fail()?;
                let mut out: Vec<Triple> = self
                    .store
                    .lock()
                    .unwrap()
                    .values()
                    .filter(|t| pattern.matches(t))
                    .cloned()
                    .collect();
                out.truncate(budget);
                Ok(out)
            })
        }
        fn delete_triple<'a>(
            &'a self,
            id: TripleId,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
            Box::pin(async move {
                self.maybe_fail()?;
                self.store.lock().unwrap().remove(&id);
                Ok(())
            })
        }
    }

    fn fast_config() -> AdapterConfig {
        // Tiny budgets / backoff for unit tests so they finish in milliseconds.
        AdapterConfig {
            max_attempts: 3,
            call_budget: Duration::from_secs(2),
            backoff_initial: Duration::from_millis(1),
            backoff_multiplier: 2.0,
            breaker_threshold: 5,
            breaker_window: Duration::from_secs(60),
            breaker_open_duration: Duration::from_millis(50),
        }
    }

    #[tokio::test]
    async fn remember_triple_idempotent_on_id() {
        let adapter = KgAdapter::new(MockBackend::new());
        let triple = t("alice", "knows", "bob");
        let id1 = adapter.remember_triple(&triple).await.unwrap();
        let id2 = adapter.remember_triple(&triple).await.unwrap();
        assert_eq!(id1, id2);
    }

    #[tokio::test]
    async fn retry_succeeds_within_budget() {
        let backend = MockBackend::new();
        backend.fail_for(2); // fail twice, succeed on 3rd
        let adapter = KgAdapter::with_config(backend, fast_config());
        let triple = t("a", "b", "c");
        adapter.remember_triple(&triple).await.unwrap();
        assert_eq!(adapter.attempts_seen(), 3);
    }

    #[tokio::test]
    async fn retry_exhaustion_returns_error() {
        let backend = MockBackend::new();
        backend.fail_for(99); // fail every attempt
        let adapter = KgAdapter::with_config(backend, fast_config());
        let triple = t("a", "b", "c");
        match adapter.remember_triple(&triple).await {
            Err(AdapterErr::RetryExhausted(msg)) => assert!(msg.contains("mock")),
            other => panic!("expected RetryExhausted, got {:?}", other),
        }
        // 3 attempts only.
        assert_eq!(adapter.attempts_seen(), 3);
    }

    #[tokio::test]
    async fn breaker_opens_after_threshold_failures() {
        let backend = MockBackend::new();
        backend.fail_for(99);
        let adapter = KgAdapter::with_config(backend, fast_config());
        let triple = t("a", "b", "c");
        // 5 failed *calls* required to trip threshold. Each call exhausts 3 retries.
        for _ in 0..5 {
            let _ = adapter.remember_triple(&triple).await;
        }
        // Next call must short-circuit with BreakerOpen.
        match adapter.remember_triple(&triple).await {
            Err(AdapterErr::BreakerOpen) => {}
            other => panic!("expected BreakerOpen, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn breaker_half_opens_after_window() {
        let backend = MockBackend::new();
        backend.fail_for(99);
        let adapter = KgAdapter::with_config(backend, fast_config());
        let triple = t("a", "b", "c");
        for _ in 0..5 {
            let _ = adapter.remember_triple(&triple).await;
        }
        // Open
        assert!(matches!(
            adapter.remember_triple(&triple).await,
            Err(AdapterErr::BreakerOpen)
        ));
        // Wait past breaker_open_duration (50 ms in fast_config).
        tokio::time::sleep(Duration::from_millis(80)).await;
        // Now it should let one probe through. Probe still fails (fail_for=99
        // didn't deplete because BreakerOpen short-circuited), so we expect
        // RetryExhausted, NOT BreakerOpen.
        match adapter.remember_triple(&triple).await {
            Err(AdapterErr::RetryExhausted(_)) => {}
            other => panic!("expected RetryExhausted (probe), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn recall_by_pattern_matches_predicate() {
        let adapter = KgAdapter::new(MockBackend::new());
        adapter.remember_triple(&t("alice", "knows", "bob")).await.unwrap();
        adapter.remember_triple(&t("alice", "loves", "bob")).await.unwrap();
        adapter.remember_triple(&t("carol", "knows", "dave")).await.unwrap();
        let hits = adapter
            .recall_by_pattern(&RecallPattern::predicate("knows"), 10)
            .await
            .unwrap();
        assert_eq!(hits.len(), 2);
    }

    #[tokio::test]
    async fn recall_respects_budget() {
        let adapter = KgAdapter::new(MockBackend::new());
        for i in 0..7 {
            adapter
                .remember_triple(&t("alice", "knows", &format!("p{i}")))
                .await
                .unwrap();
        }
        let hits = adapter
            .recall_by_pattern(&RecallPattern::subject("alice"), 3)
            .await
            .unwrap();
        assert_eq!(hits.len(), 3);
    }

    #[tokio::test]
    async fn supersede_inserts_then_deletes() {
        let adapter = KgAdapter::new(MockBackend::new());
        let old = t("alice", "knows", "bob");
        let new = t("alice", "knows", "carol");
        let old_id = adapter.remember_triple(&old).await.unwrap();
        let new_id = adapter.supersede(old_id, &new).await.unwrap();
        assert_eq!(new_id, new.id);
        // Old must be gone.
        let hits = adapter
            .recall_by_pattern(&RecallPattern::subject("alice"), 10)
            .await
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, new.id);
    }

    #[tokio::test]
    async fn tombstone_removes_triple() {
        let adapter = KgAdapter::new(MockBackend::new());
        let triple = t("a", "b", "c");
        let id = adapter.remember_triple(&triple).await.unwrap();
        adapter.tombstone(id).await.unwrap();
        let hits = adapter
            .recall_by_pattern(&RecallPattern::default(), 10)
            .await
            .unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn pattern_matches_all_when_empty() {
        let triple = t("alice", "knows", "bob");
        assert!(RecallPattern::default().matches(&triple));
    }

    #[test]
    fn pattern_rejects_mismatch() {
        let triple = t("alice", "knows", "bob");
        let p = RecallPattern {
            subject: Some("carol".into()),
            ..Default::default()
        };
        assert!(!p.matches(&triple));
    }

    #[test]
    fn config_defaults_match_acceptance_criteria() {
        let c = AdapterConfig::default();
        assert_eq!(c.max_attempts, 3);
        assert_eq!(c.call_budget, Duration::from_secs(30));
        assert_eq!(c.breaker_threshold, 5);
        assert_eq!(c.breaker_window, Duration::from_secs(60));
        assert_eq!(c.breaker_open_duration, Duration::from_secs(30));
    }
}
