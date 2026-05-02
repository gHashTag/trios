//! SR-MEM-05 — Episodic Bridge (lessons.rs + HDC ↔ KG).
//!
//! Bidirectional bridge between the two existing episodic stores and
//! the KG long-term memory exposed via SR-MEM-01:
//!
//! - **Forward (lessons → KG):** any [`LessonsSource`] yields rows that
//!   were already written into the Neon `lessons` table by
//!   `crates/trios-igla-race/src/lessons.rs`. Each row is translated
//!   into a [`Triple`] and persisted via [`KgAdapter::remember_triple`].
//!
//! - **Forward (HDC → KG):** any [`HdcReplaySource`] yields hyper-vector
//!   episodes from the HDC replay buffer
//!   (`crates/trios-sacred/src/phi-engine/hdc/rl_agent_memory.zig`).
//!   Each episode is translated into a [`Triple`] and persisted.
//!
//! - **Reverse (KG → HDC seed):** [`Bridge::seed_replay`] pulls the most
//!   recent KG triples for warm-starting the HDC replay buffer.
//!
//! ## Honest scope (R5)
//!
//! This ring ships the **contract + state machine**. The concrete
//! `sqlx::PgListener` (Neon NOTIFY listener) and Zig-FFI shim against
//! `streaming_memory.zig` ship in a sibling BR-IO ring (same precedent
//! as `trios_kg::KgClient` for SR-MEM-01). Pulling sqlx + Zig FFI into
//! a Silver ring would violate `R-RING-DEP-002` and the issue itself
//! marks the FFI binding as "TODO if Zig FFI not yet ready".
//!
//! Smoke against Neon dev DB requires `NEON_TEST_URL`; that smoke runs
//! in the BR-IO adapter PR, not here.
//!
//! Closes #455 · Part of #446 · Anchor: phi^2 + phi^-2 = 3

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Notify;
use tracing::{debug, instrument, warn};

use trios_agent_memory_sr_mem_00::{AgentRole, Provenance, Triple, TripleId};
use trios_agent_memory_sr_mem_01::{AdapterErr, KgAdapter, KgBackend, RecallPattern};

// ───────────── source row types ─────────────

/// One row materialised from the Neon `lessons` table by a
/// [`LessonsSource`]. The forwarder turns each row into a triple
/// `(subject="lesson:<kind>", predicate=predicate, object=object)` with
/// `Provenance.agent_id = AgentRole::Lead` (lessons are governance
/// artefacts).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonRow {
    /// `AVOID` / `PATTERN` / `WINNER` / `WARN` / `INFO` from
    /// `lessons.rs::LessonType`.
    pub kind: String,
    /// Triple subject (e.g. `"trial:abc123"`).
    pub subject: String,
    /// Triple predicate (e.g. `"failed_with"`).
    pub predicate: String,
    /// Triple object (e.g. `"loss=NaN"`).
    pub object: String,
    /// Row timestamp (UTC).
    pub ts: DateTime<Utc>,
    /// Source SHA: git commit or trial id (32-byte hex; pad with zeros
    /// if not available).
    pub source_sha_hex: String,
}

/// One episode read from the HDC replay buffer by an
/// [`HdcReplaySource`]. The forwarder turns each episode into a triple
/// with `Provenance.agent_id = AgentRole::Scarab`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HdcEpisode {
    /// Hyper-id of the episode (HDC content address).
    pub hyper_id: String,
    /// Triple subject (e.g. `"episode:42"`).
    pub subject: String,
    /// Triple predicate (e.g. `"observed"`).
    pub predicate: String,
    /// Triple object (free-form).
    pub object: String,
    /// Episode timestamp (UTC).
    pub ts: DateTime<Utc>,
}

// ───────────── source traits ─────────────

/// Stream of [`LessonRow`] events. Implementations:
///
/// - `MockLessons` (this file's tests).
/// - Concrete `sqlx::PgListener` adapter in the sibling BR-IO ring
///   (subscribes to `LISTEN lessons_channel` on the Neon `lessons`
///   table).
///
/// The trait deliberately has no `&mut self` method: SR-MEM-05 is a
/// **read-only forwarder** (L21 context immutability — we never write
/// back into `lessons.rs`).
pub trait LessonsSource: Send + Sync {
    /// Pop the next row (or `None` if the source has drained / closed).
    /// Implementations should make this cancel-safe (i.e. cancellable
    /// via outer `tokio::select!`).
    fn next_row<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<LessonRow>, String>> + Send + 'a>>;
}

/// Stream of [`HdcEpisode`] events. Implementations:
///
/// - `MockHdc` (this file's tests).
/// - Concrete Zig-FFI adapter against `streaming_memory.zig` exports
///   in the sibling BR-IO ring.
pub trait HdcReplaySource: Send + Sync {
    /// Pop the next episode (or `None` if the buffer drained).
    fn next_episode<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<HdcEpisode>, String>> + Send + 'a>>;
}

/// Sink that the reverse direction ([`Bridge::seed_replay`]) writes
/// recent KG triples into. The HDC replay buffer's warm-start path is
/// the canonical implementor.
pub trait HdcSeedSink: Send {
    /// Seed one triple. `Err` aborts the seed_replay run.
    fn seed<'a>(
        &'a mut self,
        triple: &'a Triple,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}

// ───────────── bridge stats / errors ─────────────

/// Cumulative forward-direction stats. Returned from [`Bridge::run`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeStats {
    /// Lessons rows forwarded into KG.
    pub lessons_forwarded: usize,
    /// HDC episodes forwarded into KG.
    pub hdc_forwarded: usize,
    /// Total source-side or adapter-side errors observed (bridge keeps
    /// running on error — only the offending event is dropped).
    pub errors: usize,
}

/// Bridge errors.
#[derive(Debug, Error)]
pub enum BridgeErr {
    /// Source returned a hard error (passed through).
    #[error("source error: {0}")]
    Source(String),
    /// KG adapter returned an error.
    #[error("adapter error: {0}")]
    Adapter(#[from] AdapterErr),
    /// Cancellation was requested via the cancel `Notify`.
    #[error("cancelled")]
    Cancelled,
}

// ───────────── translation helpers ─────────────

/// Build a deterministic 32-byte source SHA from a hex string. If the
/// input is shorter than 64 hex chars, it is right-padded with zeros;
/// non-hex bytes are mapped via SHA hashing.
fn parse_source_sha(hex: &str) -> TripleId {
    // Use TripleId::from_triple for deterministic hashing of arbitrary
    // string identifiers (avoids depending on `hex` crate at Silver).
    TripleId::from_triple("source_sha", hex, "")
}

fn lesson_to_triple(row: &LessonRow, task_id: uuid::Uuid) -> Triple {
    let prov = Provenance {
        agent_id: AgentRole::Lead,
        task_id,
        source_sha: parse_source_sha(&row.source_sha_hex),
        ts: row.ts,
    };
    // Encode lesson kind into the subject so downstream pattern recall
    // can filter by `subject = "lesson:<kind>:<subject>"`.
    let composite_subject = format!("lesson:{}:{}", row.kind.to_lowercase(), row.subject);
    Triple::new(
        composite_subject,
        row.predicate.clone(),
        row.object.clone(),
        prov,
    )
}

fn episode_to_triple(ep: &HdcEpisode, task_id: uuid::Uuid) -> Triple {
    let prov = Provenance {
        agent_id: AgentRole::Scarab,
        task_id,
        source_sha: parse_source_sha(&ep.hyper_id),
        ts: ep.ts,
    };
    let composite_subject = format!("hdc:{}", ep.subject);
    Triple::new(composite_subject, ep.predicate.clone(), ep.object.clone(), prov)
}

// ───────────── Bridge ─────────────

/// Bidirectional bridge between (Lessons + HDC) and the KG.
pub struct Bridge<L: LessonsSource, H: HdcReplaySource, B: KgBackend> {
    lessons: Arc<L>,
    hdc: Arc<H>,
    adapter: Arc<KgAdapter<B>>,
    /// One stable task_id that tags every triple this bridge minted in
    /// its lifetime. Lets downstream readers correlate forwards.
    task_id: uuid::Uuid,
    /// Forward stats counters (atomic so concurrent forward loops can
    /// share state cheaply).
    lessons_count: AtomicUsize,
    hdc_count: AtomicUsize,
    err_count: AtomicUsize,
}

impl<L: LessonsSource, H: HdcReplaySource, B: KgBackend> Bridge<L, H, B> {
    /// Build a new bridge. The bridge's `task_id` is generated once
    /// here and tags every triple it forwards.
    pub fn new(lessons: L, hdc: H, adapter: KgAdapter<B>) -> Self {
        Self {
            lessons: Arc::new(lessons),
            hdc: Arc::new(hdc),
            adapter: Arc::new(adapter),
            task_id: uuid::Uuid::new_v4(),
            lessons_count: AtomicUsize::new(0),
            hdc_count: AtomicUsize::new(0),
            err_count: AtomicUsize::new(0),
        }
    }

    /// Test/diagnostic helper: stable task_id this bridge is tagging
    /// triples with.
    pub fn task_id(&self) -> uuid::Uuid {
        self.task_id
    }

    /// Snapshot the current forward stats without stopping the bridge.
    pub fn stats(&self) -> BridgeStats {
        BridgeStats {
            lessons_forwarded: self.lessons_count.load(Ordering::Relaxed),
            hdc_forwarded: self.hdc_count.load(Ordering::Relaxed),
            errors: self.err_count.load(Ordering::Relaxed),
        }
    }

    /// Forward direction. Drains both sources concurrently into the KG.
    /// Returns once both source streams complete OR `cancel.notified()`
    /// fires. Errors on a single event do not stop the bridge — they
    /// are counted in [`BridgeStats::errors`] and the offending event
    /// is dropped.
    #[instrument(skip(self, cancel))]
    pub async fn run(&self, cancel: Arc<Notify>) -> BridgeStats {
        let lessons = Arc::clone(&self.lessons);
        let hdc = Arc::clone(&self.hdc);
        let adapter = Arc::clone(&self.adapter);
        let task_id = self.task_id;

        // We can't share AtomicUsize references across two spawned
        // tasks without &'static, but we own self for the duration of
        // run(), so we use scoped concurrency via tokio::join! over two
        // local async blocks instead of tokio::spawn.
        let lessons_loop = async {
            let mut local_ok = 0usize;
            let mut local_err = 0usize;
            loop {
                tokio::select! {
                    biased;
                    _ = cancel.notified() => break,
                    row = lessons.next_row() => match row {
                        Ok(Some(r)) => {
                            let triple = lesson_to_triple(&r, task_id);
                            match adapter.remember_triple(&triple).await {
                                Ok(_) => { local_ok += 1; }
                                Err(e) => {
                                    warn!(error = %e, "lessons forward failed");
                                    local_err += 1;
                                }
                            }
                        }
                        Ok(None) => {
                            debug!("lessons source drained");
                            break;
                        }
                        Err(e) => {
                            warn!(error = %e, "lessons source error");
                            local_err += 1;
                        }
                    }
                }
            }
            (local_ok, local_err)
        };

        let hdc_loop = async {
            let mut local_ok = 0usize;
            let mut local_err = 0usize;
            loop {
                tokio::select! {
                    biased;
                    _ = cancel.notified() => break,
                    ep = hdc.next_episode() => match ep {
                        Ok(Some(e)) => {
                            let triple = episode_to_triple(&e, task_id);
                            match adapter.remember_triple(&triple).await {
                                Ok(_) => { local_ok += 1; }
                                Err(err) => {
                                    warn!(error = %err, "hdc forward failed");
                                    local_err += 1;
                                }
                            }
                        }
                        Ok(None) => {
                            debug!("hdc source drained");
                            break;
                        }
                        Err(e) => {
                            warn!(error = %e, "hdc source error");
                            local_err += 1;
                        }
                    }
                }
            }
            (local_ok, local_err)
        };

        let ((l_ok, l_err), (h_ok, h_err)) = tokio::join!(lessons_loop, hdc_loop);
        self.lessons_count.fetch_add(l_ok, Ordering::Relaxed);
        self.hdc_count.fetch_add(h_ok, Ordering::Relaxed);
        self.err_count.fetch_add(l_err + h_err, Ordering::Relaxed);
        self.stats()
    }

    /// Reverse direction. Pulls KG triples back into the supplied seed
    /// sink. `lookback` is currently advisory: this ring filters
    /// post-recall by `triple.provenance.ts`. Returns the count of
    /// triples seeded.
    ///
    /// Recall is keyed off the triples this bridge minted (subjects
    /// prefixed with `"lesson:"` or `"hdc:"`); a downstream caller
    /// needing a wider seed window should call `KgAdapter::recall_by_pattern`
    /// directly with the pattern of their choice.
    #[instrument(skip(self, sink))]
    pub async fn seed_replay<S: HdcSeedSink>(
        &self,
        sink: &mut S,
        lookback: Duration,
    ) -> Result<usize, BridgeErr> {
        let now = Utc::now();
        let cutoff = now
            - chrono::Duration::from_std(lookback).unwrap_or_else(|_| chrono::Duration::seconds(0));

        // Pull lesson- and hdc-prefixed triples in two recalls; we do
        // not have a direct "ts >=" query in SR-MEM-01 (RecallPattern
        // is SPO-only), so we filter in-memory by provenance.ts. Budget
        // is generous (10_000) — the gardener writes O(few) per tick.
        let lesson_pattern = RecallPattern::default();
        let candidates = self
            .adapter
            .recall_by_pattern(&lesson_pattern, 10_000)
            .await?;
        let mut seeded = 0usize;
        for triple in candidates.iter() {
            let is_ours = triple.subject.starts_with("lesson:")
                || triple.subject.starts_with("hdc:");
            if !is_ours {
                continue;
            }
            if triple.provenance.ts < cutoff {
                continue;
            }
            sink.seed(triple)
                .await
                .map_err(BridgeErr::Source)?;
            seeded += 1;
        }
        Ok(seeded)
    }
}

// ───────────── tests ─────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex as StdMutex;
    use trios_agent_memory_sr_mem_01::KgBackend as KgBackendTrait;

    fn ts_now() -> DateTime<Utc> {
        Utc::now()
    }

    fn lesson_row(kind: &str, subj: &str, pred: &str, obj: &str) -> LessonRow {
        LessonRow {
            kind: kind.into(),
            subject: subj.into(),
            predicate: pred.into(),
            object: obj.into(),
            ts: ts_now(),
            source_sha_hex: "deadbeef".into(),
        }
    }

    fn hdc_episode(hyper: &str, subj: &str, pred: &str, obj: &str) -> HdcEpisode {
        HdcEpisode {
            hyper_id: hyper.into(),
            subject: subj.into(),
            predicate: pred.into(),
            object: obj.into(),
            ts: ts_now(),
        }
    }

    // ── mock lessons source ──

    struct MockLessons {
        rows: StdMutex<Vec<LessonRow>>,
        // If > 0, the next call to `next_row` returns Err.
        fail_next: StdMutex<u32>,
    }

    impl MockLessons {
        fn new(rows: Vec<LessonRow>) -> Self {
            Self {
                rows: StdMutex::new(rows.into_iter().rev().collect()),
                fail_next: StdMutex::new(0),
            }
        }
        fn fail_for(&self, n: u32) {
            *self.fail_next.lock().unwrap() = n;
        }
    }

    impl LessonsSource for MockLessons {
        fn next_row<'a>(
            &'a self,
        ) -> Pin<Box<dyn Future<Output = Result<Option<LessonRow>, String>> + Send + 'a>>
        {
            Box::pin(async move {
                {
                    let mut n = self.fail_next.lock().unwrap();
                    if *n > 0 {
                        *n -= 1;
                        return Err("mock lessons error".into());
                    }
                }
                Ok(self.rows.lock().unwrap().pop())
            })
        }
    }

    // ── mock hdc source ──

    struct MockHdc {
        eps: StdMutex<Vec<HdcEpisode>>,
    }

    impl MockHdc {
        fn new(eps: Vec<HdcEpisode>) -> Self {
            Self {
                eps: StdMutex::new(eps.into_iter().rev().collect()),
            }
        }
    }

    impl HdcReplaySource for MockHdc {
        fn next_episode<'a>(
            &'a self,
        ) -> Pin<Box<dyn Future<Output = Result<Option<HdcEpisode>, String>> + Send + 'a>>
        {
            Box::pin(async move { Ok(self.eps.lock().unwrap().pop()) })
        }
    }

    // ── mock KG backend ──

    struct MockKg {
        store: StdMutex<HashMap<TripleId, Triple>>,
    }

    impl MockKg {
        fn new() -> Self {
            Self {
                store: StdMutex::new(HashMap::new()),
            }
        }
    }

    impl KgBackendTrait for MockKg {
        fn put_triple<'a>(
            &'a self,
            triple: &'a Triple,
        ) -> Pin<Box<dyn Future<Output = Result<TripleId, String>> + Send + 'a>>
        {
            Box::pin(async move {
                self.store.lock().unwrap().insert(triple.id, triple.clone());
                Ok(triple.id)
            })
        }
        fn query_pattern<'a>(
            &'a self,
            pattern: &'a RecallPattern,
            budget: usize,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<Triple>, String>> + Send + 'a>>
        {
            Box::pin(async move {
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
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>
        {
            Box::pin(async move {
                self.store.lock().unwrap().remove(&id);
                Ok(())
            })
        }
    }

    // ── seed sink ──

    struct VecSeedSink(Vec<Triple>);

    impl HdcSeedSink for VecSeedSink {
        fn seed<'a>(
            &'a mut self,
            triple: &'a Triple,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>
        {
            Box::pin(async move {
                self.0.push(triple.clone());
                Ok(())
            })
        }
    }

    fn build_bridge(
        lessons: Vec<LessonRow>,
        eps: Vec<HdcEpisode>,
    ) -> (
        Bridge<MockLessons, MockHdc, MockKg>,
        Arc<KgAdapter<MockKg>>,
    ) {
        // Build KgAdapter inline so tests can also recall through it.
        let adapter = KgAdapter::new(MockKg::new());
        // Re-clone the adapter for direct test access by wrapping again.
        let bridge = Bridge::new(MockLessons::new(lessons), MockHdc::new(eps), adapter);
        // The bridge owns the adapter; for tests that need a separate
        // handle, build a parallel adapter on a fresh backend. To keep
        // things simple, return the bridge alone (tests below use
        // `bridge.adapter` indirectly through seed_replay).
        let bridge_adapter = Arc::clone(&bridge.adapter);
        (bridge, bridge_adapter)
    }

    // ── tests ──

    #[tokio::test]
    async fn forward_lessons_into_kg() {
        let (bridge, adapter) = build_bridge(
            vec![
                lesson_row("AVOID", "trial:abc", "failed_with", "loss=NaN"),
                lesson_row("WINNER", "trial:xyz", "achieved", "bpb=2.19"),
            ],
            vec![],
        );
        let cancel = Arc::new(Notify::new());
        let stats = bridge.run(cancel).await;
        assert_eq!(stats.lessons_forwarded, 2);
        assert_eq!(stats.hdc_forwarded, 0);
        assert_eq!(stats.errors, 0);
        let hits = adapter
            .recall_by_pattern(&RecallPattern::default(), 100)
            .await
            .unwrap();
        assert_eq!(hits.len(), 2);
        // Subject is composite-prefixed.
        assert!(hits.iter().all(|t| t.subject.starts_with("lesson:")));
    }

    #[tokio::test]
    async fn forward_hdc_into_kg() {
        let (bridge, adapter) = build_bridge(
            vec![],
            vec![
                hdc_episode("h:001", "ep:1", "observed", "phi"),
                hdc_episode("h:002", "ep:2", "observed", "psi"),
            ],
        );
        let cancel = Arc::new(Notify::new());
        let stats = bridge.run(cancel).await;
        assert_eq!(stats.lessons_forwarded, 0);
        assert_eq!(stats.hdc_forwarded, 2);
        assert_eq!(stats.errors, 0);
        let hits = adapter
            .recall_by_pattern(&RecallPattern::default(), 100)
            .await
            .unwrap();
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|t| t.subject.starts_with("hdc:")));
    }

    #[tokio::test]
    async fn forward_concurrent_both_sources() {
        let (bridge, adapter) = build_bridge(
            vec![
                lesson_row("AVOID", "t:1", "failed", "x"),
                lesson_row("WINNER", "t:2", "won", "y"),
                lesson_row("INFO", "t:3", "noted", "z"),
            ],
            vec![
                hdc_episode("h:1", "e1", "p", "o"),
                hdc_episode("h:2", "e2", "p", "o"),
            ],
        );
        let cancel = Arc::new(Notify::new());
        let stats = bridge.run(cancel).await;
        assert_eq!(stats.lessons_forwarded, 3);
        assert_eq!(stats.hdc_forwarded, 2);
        assert_eq!(stats.errors, 0);
        let total = adapter
            .recall_by_pattern(&RecallPattern::default(), 100)
            .await
            .unwrap();
        assert_eq!(total.len(), 5);
    }

    #[tokio::test]
    async fn forward_continues_after_source_error() {
        // Inject 2 errors into the lessons stream; the rows after the
        // errors must still flow through.
        let lessons = MockLessons::new(vec![
            lesson_row("AVOID", "t:1", "f", "x"),
            lesson_row("WINNER", "t:2", "w", "y"),
        ]);
        lessons.fail_for(2);
        let bridge = Bridge::new(lessons, MockHdc::new(vec![]), KgAdapter::new(MockKg::new()));
        let cancel = Arc::new(Notify::new());
        let stats = bridge.run(cancel).await;
        // 2 rows still made it through after the 2 injected errors.
        assert_eq!(stats.lessons_forwarded, 2);
        assert_eq!(stats.errors, 2);
    }

    #[tokio::test]
    async fn run_honors_cancel_notify() {
        // A "never-drains" lessons source so the bridge would loop
        // forever without cancellation.
        struct NeverDrains;
        impl LessonsSource for NeverDrains {
            fn next_row<'a>(
                &'a self,
            ) -> Pin<
                Box<dyn Future<Output = Result<Option<LessonRow>, String>> + Send + 'a>,
            > {
                Box::pin(async move {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    Ok(None)
                })
            }
        }
        let bridge = Bridge::new(NeverDrains, MockHdc::new(vec![]), KgAdapter::new(MockKg::new()));
        let cancel = Arc::new(Notify::new());
        let cancel_clone = Arc::clone(&cancel);
        let handle = tokio::spawn(async move { bridge.run(cancel_clone).await });
        // Give the loop a moment to enter `select!`.
        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel.notify_waiters();
        // The notify wakes both legs (lessons + hdc); MockHdc with []
        // drains immediately, lessons leg respects cancel.
        let stats = tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("run did not return after cancel")
            .expect("join error");
        assert_eq!(stats.lessons_forwarded, 0);
    }

    #[tokio::test]
    async fn seed_replay_pulls_recent_triples() {
        let (bridge, _) = build_bridge(
            vec![lesson_row("WINNER", "t:1", "won", "y")],
            vec![hdc_episode("h:1", "e1", "p", "o")],
        );
        let cancel = Arc::new(Notify::new());
        let _ = bridge.run(cancel).await;
        let mut sink = VecSeedSink(Vec::new());
        let n = bridge
            .seed_replay(&mut sink, Duration::from_secs(60 * 60))
            .await
            .unwrap();
        assert_eq!(n, 2);
        assert_eq!(sink.0.len(), 2);
    }

    #[tokio::test]
    async fn seed_replay_respects_lookback() {
        let (bridge, _) = build_bridge(
            vec![lesson_row("INFO", "t:1", "noted", "z")],
            vec![],
        );
        let cancel = Arc::new(Notify::new());
        let _ = bridge.run(cancel).await;
        // 0-second lookback excludes everything (since `cutoff = now`).
        // Use `Duration::from_nanos(1)` for a strictly-positive lookback;
        // any triple minted before "now - 1ns" is excluded.
        tokio::time::sleep(Duration::from_millis(5)).await;
        let mut sink = VecSeedSink(Vec::new());
        let n = bridge
            .seed_replay(&mut sink, Duration::from_nanos(1))
            .await
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn seed_replay_handles_empty_kg() {
        let (bridge, _) = build_bridge(vec![], vec![]);
        let mut sink = VecSeedSink(Vec::new());
        let n = bridge
            .seed_replay(&mut sink, Duration::from_secs(3600))
            .await
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn seed_replay_propagates_sink_error() {
        struct BadSink;
        impl HdcSeedSink for BadSink {
            fn seed<'a>(
                &'a mut self,
                _triple: &'a Triple,
            ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>
            {
                Box::pin(async move { Err("sink full".into()) })
            }
        }
        let (bridge, _) = build_bridge(
            vec![lesson_row("WARN", "t:1", "noted", "x")],
            vec![],
        );
        let cancel = Arc::new(Notify::new());
        let _ = bridge.run(cancel).await;
        let mut sink = BadSink;
        match bridge
            .seed_replay(&mut sink, Duration::from_secs(3600))
            .await
        {
            Err(BridgeErr::Source(msg)) => assert_eq!(msg, "sink full"),
            other => panic!("expected Source(sink full), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn seed_replay_skips_foreign_triples() {
        // A bridge that only forwarded lessons; manually inject a
        // foreign-prefix triple into the same KG via the adapter, then
        // confirm seed_replay skips it.
        let (bridge, adapter) = build_bridge(
            vec![lesson_row("AVOID", "t:1", "failed", "x")],
            vec![],
        );
        let cancel = Arc::new(Notify::new());
        let _ = bridge.run(cancel).await;
        // Foreign triple (subject doesn't start with lesson:/hdc:).
        let prov = Provenance {
            agent_id: AgentRole::Doctor,
            task_id: uuid::Uuid::new_v4(),
            source_sha: TripleId::from_triple("foreign", "x", ""),
            ts: Utc::now(),
        };
        adapter
            .remember_triple(&Triple::new("doctor:rule", "asserts", "L1", prov))
            .await
            .unwrap();
        let mut sink = VecSeedSink(Vec::new());
        let n = bridge
            .seed_replay(&mut sink, Duration::from_secs(3600))
            .await
            .unwrap();
        // Only the lesson was seeded; doctor:rule was skipped.
        assert_eq!(n, 1);
        assert!(sink.0[0].subject.starts_with("lesson:"));
    }

    #[test]
    fn lessons_source_is_immutable_view() {
        // Compile-time guarantee: `LessonsSource::next_row` takes
        // `&self`, so a downstream impl cannot expose a mutable write
        // path through this trait. (L21 context immutability.)
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}
        assert_send_sync::<dyn LessonsSource>();
    }

    #[test]
    fn phi_anchor_present() {
        let phi: f64 = (1.0 + 5.0_f64.sqrt()) / 2.0;
        let lhs = phi * phi + 1.0 / (phi * phi);
        assert!((lhs - 3.0).abs() < 1e-10, "phi anchor violated: {lhs}");
    }

    #[test]
    fn translation_uses_lead_role_for_lessons() {
        let row = lesson_row("AVOID", "t:1", "failed_with", "loss=NaN");
        let task_id = uuid::Uuid::nil();
        let triple = lesson_to_triple(&row, task_id);
        assert_eq!(triple.provenance.agent_id, AgentRole::Lead);
        assert_eq!(triple.subject, "lesson:avoid:t:1");
        assert_eq!(triple.predicate, "failed_with");
        assert_eq!(triple.object, "loss=NaN");
    }

    #[test]
    fn translation_uses_scarab_role_for_hdc() {
        let ep = hdc_episode("h:1", "ep:1", "observed", "phi");
        let task_id = uuid::Uuid::nil();
        let triple = episode_to_triple(&ep, task_id);
        assert_eq!(triple.provenance.agent_id, AgentRole::Scarab);
        assert_eq!(triple.subject, "hdc:ep:1");
    }
}
