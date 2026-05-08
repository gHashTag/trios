//! PhD SSOT Dashboard — Axum + HTMX + Askama
//! R1: pure Rust, no .py/.sh, no JS frameworks
//! φ² + φ⁻² = 3 · TRINITY
//!
//! ENV: DATABASE_URL or RAILWAY_SSOT_URL (set in Railway → Variables)
//!      PORT — set automatically by Railway (do NOT hardcode 3030)

use anyhow::Result;
use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::State,
    response::Json,
    routing::get,
    Router,
};
use serde::Serialize;
use std::sync::Arc;
use tokio_postgres::NoTls;
use tracing_subscriber::EnvFilter;

type Db = Arc<tokio_postgres::Client>;

#[derive(Debug, Serialize, Clone)]
struct ChapterRow {
    chapter_slug: String,
    chapter_no: i32,
    kind: String,
    title: String,
    line_count: i32,
    r3_full: bool,
    has_figure: bool,
    theorem_count: i64,
    updated_at: String,
}

#[derive(Debug, Serialize, Clone)]
struct DuplicateRow {
    chapter_slug: String,
    title: String,
    dup_count: i64,
}

#[derive(Debug, Serialize, Clone)]
struct RagRow {
    chapter_slug: String,
    total_chunks: i64,
    embedded_chunks: i64,
    pct: f64,
    last_embedded: String,
}

#[derive(Debug, Serialize, Clone)]
struct Stats {
    total_chapters: i64,
    r3_ok: i64,
    with_figure: i64,
    total_theorems: i64,
    rag_total: i64,
    rag_embedded: i64,
    rag_pct: f64,
    rag_pct_int: i64,
    duplicate_slugs: i64,
}

// === Coq Admitted ledger (audit 2026-05-08, R5 §honest-status) ===
// Source of truth: workspace/coq_audit_2026-05-08.md, also asset coq_audit_empire.
// 100 Coq files audited across trios + trinity-clara + t27 (excl. Verilog).
#[derive(Debug, Serialize, Clone)]
struct CoqRow {
    repo: &'static str,
    path: &'static str,
    theorem_lemma: i32,
    qed: i32,
    admitted: i32,
    covered_by: &'static str,     // human label, e.g. "trios#554" or ""
    covered_by_url: &'static str, // full URL or ""
    status: &'static str,         // "covered", "gap-critical", "gap-stale", "gap"
}

// IMMUTABLE inventory — updated only via PR after re-running audit.
// Sums (canonical: trios + trinity-clara only): 28 Admitted.
// Adding t27 stale fork: +32 stale (excluded from headline floor).
const COQ_INVENTORY: &[CoqRow] = &[
    // ── trios canonical ──────────────────────────────────────────
    CoqRow { repo: "trios",         path: "docs/phd/theorems/trinity/Bounds_LeptonMasses.v",            theorem_lemma:  8, qed:  0, admitted: 8, covered_by: "trios#554", covered_by_url: "https://github.com/gHashTag/trios/issues/554", status: "covered" },
    CoqRow { repo: "trios",         path: "docs/phd/theorems/trinity/ConsistencyChecks.v",              theorem_lemma: 14, qed:  7, admitted: 7, covered_by: "",          covered_by_url: "", status: "gap-critical" },
    CoqRow { repo: "trios",         path: "docs/phd/theorems/trinity/Bounds_QuarkMasses.v",             theorem_lemma:  8, qed:  4, admitted: 4, covered_by: "",          covered_by_url: "", status: "gap-critical" },
    CoqRow { repo: "trios",         path: "docs/phd/theorems/trinity/ExactIdentities.v",                theorem_lemma: 46, qed: 31, admitted: 3, covered_by: "trios#549", covered_by_url: "https://github.com/gHashTag/trios/issues/549", status: "covered" },
    CoqRow { repo: "trios",         path: "docs/phd/theorems/trinity/Unitarity.v",                      theorem_lemma:  7, qed:  5, admitted: 2, covered_by: "",          covered_by_url: "", status: "gap" },
    CoqRow { repo: "trios",         path: "trinity-clara/proofs/igla/rainbow_bridge_consistency.v",     theorem_lemma: 10, qed: 10, admitted: 2, covered_by: "",          covered_by_url: "", status: "gap" },
    // ── trinity-clara repo (frozen) ──────────────────────────────
    CoqRow { repo: "trinity-clara", path: "proofs/igla/lr_convergence.v",                              theorem_lemma:  1, qed:  0, admitted: 1, covered_by: "",          covered_by_url: "", status: "gap" },
    CoqRow { repo: "trinity-clara", path: "proofs/igla/lucas_closure_gf16.v",                          theorem_lemma:  1, qed:  0, admitted: 1, covered_by: "",          covered_by_url: "", status: "gap" },
    // ── t27 stale fork (informational) ───────────────────────────
    CoqRow { repo: "t27",           path: "proofs/trinity/ExactIdentities.v",                          theorem_lemma: 22, qed:  5, admitted:11, covered_by: "",          covered_by_url: "", status: "gap-stale" },
    CoqRow { repo: "t27",           path: "proofs/trinity/Bounds_LeptonMasses.v",                      theorem_lemma:  8, qed:  0, admitted: 8, covered_by: "",          covered_by_url: "", status: "gap-stale" },
    CoqRow { repo: "t27",           path: "proofs/trinity/ConsistencyChecks.v",                        theorem_lemma: 14, qed:  7, admitted: 7, covered_by: "",          covered_by_url: "", status: "gap-stale" },
    CoqRow { repo: "t27",           path: "proofs/trinity/Bounds_QuarkMasses.v",                       theorem_lemma:  8, qed:  4, admitted: 4, covered_by: "",          covered_by_url: "", status: "gap-stale" },
    CoqRow { repo: "t27",           path: "proofs/trinity/Unitarity.v",                                theorem_lemma:  7, qed:  5, admitted: 2, covered_by: "",          covered_by_url: "", status: "gap-stale" },
];

// Aggregated Coq stats — split canonical vs stale.
#[derive(Debug, Serialize, Clone)]
struct CoqStats {
    files_total: i64,        // 100 (97 in inventory + 3 zero-Admitted skipped here)
    files_audited: i64,      // 13 with Admitted > 0
    admitted_canonical: i64, // trios + trinity-clara only
    admitted_stale: i64,     // t27 fork drift
    qed_total: i64,
    theorem_lemma_total: i64,
    covered: i64,            // Admitted under an open ONE SHOT
    uncovered_gap: i64,      // canonical Admitted with no ONE SHOT
}

fn coq_stats() -> CoqStats {
    let mut s = CoqStats {
        files_total: 100,
        files_audited: COQ_INVENTORY.len() as i64,
        admitted_canonical: 0,
        admitted_stale: 0,
        qed_total: 0,
        theorem_lemma_total: 0,
        covered: 0,
        uncovered_gap: 0,
    };
    for r in COQ_INVENTORY {
        s.qed_total += r.qed as i64;
        s.theorem_lemma_total += r.theorem_lemma as i64;
        match r.status {
            "gap-stale" => s.admitted_stale += r.admitted as i64,
            _ => {
                s.admitted_canonical += r.admitted as i64;
                if !r.covered_by.is_empty() {
                    s.covered += r.admitted as i64;
                } else if r.admitted > 0 {
                    s.uncovered_gap += r.admitted as i64;
                }
            }
        }
    }
    s
}

// Rehearsal #2 progress (deadline 2026-05-25, floor: 30 Admitted closed by then).
// Anchor: φ²+φ⁻²=3 · 28 canonical Admitted at session start.
#[derive(Debug, Serialize, Clone)]
struct ProgressBar {
    label: &'static str,
    target_label: &'static str,
    deadline: &'static str,
    days_left: i64,
    closed: i64,         // Admitted already closed (#548 + #550 + #551 swept = 8)
    in_flight: i64,      // covered by open ONE SHOTs awaiting merge
    remaining_gap: i64,  // uncovered Admitted
    target: i64,         // Rehearsal #2 floor
    pct: f64,
}

fn progress_rehearsal2() -> ProgressBar {
    let s = coq_stats();
    // closed = 8 (sweep-1 ExactIdentities Admitted → Qed already in main)
    //   ground truth = pre-sweep had 11; main now has 3 → 8 closed.
    let closed: i64 = 8;
    let target: i64 = 30; // floor for Rehearsal #2 = 30 Admitted closed
    let days_left = days_until("2026-05-25");
    let in_flight = s.covered;          // 8 Lepton + 3 ExID-DELETE = 11
    let remaining_gap = s.uncovered_gap;
    let pct = (closed as f64 / target as f64 * 100.0).clamp(0.0, 100.0);
    ProgressBar {
        label: "Coq Admitted Closure (sweep-2)",
        target_label: "Rehearsal #2 — close 30 Admitted",
        deadline: "2026-05-25",
        days_left,
        closed,
        in_flight,
        remaining_gap,
        target,
        pct,
    }
}

// Approximate days-until using chrono.
fn days_until(yyyy_mm_dd: &str) -> i64 {
    use chrono::NaiveDate;
    let target = NaiveDate::parse_from_str(yyyy_mm_dd, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Utc::now().date_naive());
    let today = chrono::Utc::now().date_naive();
    (target - today).num_days()
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    stats: Stats,
    chapters: Vec<ChapterRow>,
    duplicates: Vec<DuplicateRow>,
    rag: Vec<RagRow>,
    coq_stats: CoqStats,
    coq_inventory: &'static [CoqRow],
    progress: ProgressBar,
    progress_pct_int: i64,
}

async fn fetch_chapters(db: &tokio_postgres::Client) -> Result<Vec<ChapterRow>> {
    let rows = db.query(
        "SELECT chapter_slug, chapter_no, kind, title, \
                COALESCE(line_count,0) AS line_count, \
                (COALESCE(line_count,0) >= 1500) AS r3_full, \
                (EXISTS(SELECT 1 FROM ssot.chapter_figures f WHERE f.chapter_slug = c.chapter_slug)) AS has_figure, \
                (SELECT COUNT(*) FROM ssot.theorems t WHERE t.chapter_slug = c.chapter_slug) AS theorem_count, \
                updated_at::text \
         FROM ssot.chapters c ORDER BY chapter_no",
        &[],
    ).await?;
    Ok(rows.iter().map(|r| ChapterRow {
        chapter_slug: r.get(0),
        chapter_no: r.get(1),
        kind: r.get(2),
        title: r.get(3),
        line_count: r.get::<_, i32>(4),
        r3_full: r.get(5),
        has_figure: r.get(6),
        theorem_count: r.get(7),
        updated_at: { let s: String = r.get(8); s.chars().take(10).collect() },
    }).collect())
}

async fn fetch_duplicates(db: &tokio_postgres::Client) -> Result<Vec<DuplicateRow>> {
    let rows = db.query(
        "SELECT chapter_slug, title, cnt FROM (\
           SELECT chapter_slug, title, \
                  COUNT(*) OVER (PARTITION BY lower(trim(title))) AS cnt \
           FROM ssot.chapters\
         ) sub WHERE cnt > 1 ORDER BY cnt DESC, title",
        &[],
    ).await?;
    Ok(rows.iter().map(|r| DuplicateRow {
        chapter_slug: r.get(0),
        title: r.get(1),
        dup_count: r.get(2),
    }).collect())
}

async fn fetch_rag(db: &tokio_postgres::Client) -> Result<Vec<RagRow>> {
    let rows = db.query(
        "SELECT e.chapter_slug, \
                COUNT(e.id) AS total_chunks, \
                COUNT(e.id) FILTER (WHERE e.embedding IS NOT NULL) AS embedded_chunks, \
                (ROUND(100.0 * COUNT(e.id) FILTER (WHERE e.embedding IS NOT NULL) \
                      / NULLIF(COUNT(e.id),0), 1))::float8 AS pct, \
                COALESCE(MAX(e.embedded_at)::text, 'pending') AS last_embedded \
         FROM ssot.embeddings e GROUP BY e.chapter_slug ORDER BY e.chapter_slug",
        &[],
    ).await?;
    Ok(rows.iter().map(|r| RagRow {
        chapter_slug: r.get(0),
        total_chunks: r.get(1),
        embedded_chunks: r.get(2),
        pct: r.get::<_, f64>(3),
        last_embedded: { let s: String = r.get(4); s.chars().take(19).collect() },
    }).collect())
}

async fn fetch_stats(db: &tokio_postgres::Client) -> Result<Stats> {
    let row = db.query_one(
        "SELECT \
            (SELECT COUNT(*) FROM ssot.chapters), \
            (SELECT COUNT(*) FROM ssot.chapters WHERE COALESCE(line_count,0) >= 1500), \
            (SELECT COUNT(*) FROM ssot.chapter_figures), \
            (SELECT COUNT(*) FROM ssot.theorems), \
            (SELECT COUNT(*) FROM ssot.embeddings), \
            (SELECT COUNT(*) FROM ssot.embeddings WHERE embedding IS NOT NULL), \
            (SELECT COUNT(*) FROM (SELECT lower(trim(title)) AS t FROM ssot.chapters GROUP BY lower(trim(title)) HAVING COUNT(*) > 1) d)",
        &[],
    ).await?;
    let rag_total: i64 = row.get(4);
    let rag_embedded: i64 = row.get(5);
    let rag_pct = if rag_total > 0 { (rag_embedded as f64 / rag_total as f64) * 100.0 } else { 0.0 };
    Ok(Stats {
        total_chapters: row.get(0), r3_ok: row.get(1), with_figure: row.get(2),
        total_theorems: row.get(3), rag_total, rag_embedded, rag_pct,
        rag_pct_int: rag_pct.round() as i64, duplicate_slugs: row.get(6),
    })
}

async fn index(State(db): State<Db>) -> impl IntoResponse {
    let (stats, chapters, duplicates, rag) = tokio::join!(
        fetch_stats(&db), fetch_chapters(&db), fetch_duplicates(&db), fetch_rag(&db),
    );
    if let Err(e) = &stats { tracing::error!("fetch_stats failed: {e:?}"); }
    if let Err(e) = &chapters { tracing::error!("fetch_chapters failed: {e:?}"); }
    if let Err(e) = &duplicates { tracing::error!("fetch_duplicates failed: {e:?}"); }
    if let Err(e) = &rag { tracing::error!("fetch_rag failed: {e:?}"); }
    let progress = progress_rehearsal2();
    let progress_pct_int = progress.pct.round() as i64;
    IndexTemplate {
        stats: stats.unwrap_or(Stats {
            total_chapters: 0, r3_ok: 0, with_figure: 0,
            total_theorems: 0, rag_total: 0, rag_embedded: 0,
            rag_pct: 0.0, rag_pct_int: 0, duplicate_slugs: 0,
        }),
        chapters: chapters.unwrap_or_default(),
        duplicates: duplicates.unwrap_or_default(),
        rag: rag.unwrap_or_default(),
        coq_stats: coq_stats(),
        coq_inventory: COQ_INVENTORY,
        progress,
        progress_pct_int,
    }
}

async fn htmx_chapters(State(db): State<Db>) -> impl IntoResponse {
    #[derive(Template)] #[template(path = "partials/chapters_table.html")]
    struct T { chapters: Vec<ChapterRow> }
    T { chapters: fetch_chapters(&db).await.unwrap_or_default() }
}

async fn htmx_rag(State(db): State<Db>) -> impl IntoResponse {
    #[derive(Template)] #[template(path = "partials/rag_table.html")]
    struct T { rag: Vec<RagRow> }
    T { rag: fetch_rag(&db).await.unwrap_or_default() }
}

async fn htmx_coq() -> impl IntoResponse {
    #[derive(Template)] #[template(path = "partials/coq_table.html")]
    struct T { coq_inventory: &'static [CoqRow], coq_stats: CoqStats }
    T { coq_inventory: COQ_INVENTORY, coq_stats: coq_stats() }
}

async fn htmx_progress() -> impl IntoResponse {
    #[derive(Template)] #[template(path = "partials/progress_bar.html")]
    struct T { progress: ProgressBar, progress_pct_int: i64 }
    let p = progress_rehearsal2();
    let pct_int = p.pct.round() as i64;
    T { progress: p, progress_pct_int: pct_int }
}

// JSON APIs (R7 falsifiability — public read-only ground truth).
async fn api_coq() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "anchor": "phi^2 + phi^-2 = 3",
        "doi": "10.5281/zenodo.19227877",
        "audit_date": "2026-05-08",
        "stats": coq_stats(),
        "inventory": COQ_INVENTORY,
    }))
}

async fn api_progress() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "anchor": "phi^2 + phi^-2 = 3",
        "progress": progress_rehearsal2(),
    }))
}

async fn api_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "anchor": "phi^2 + phi^-2 = 3",
    }))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let phi: f64 = (1.0 + 5.0_f64.sqrt()) / 2.0;
    assert!((phi * phi + 1.0 / (phi * phi) - 3.0).abs() < 1e-12, "Trinity anchor");

    // Try RAILWAY_SSOT_URL first (explicit), then DATABASE_URL.
    // Railway auto-injects DATABASE_URL as ${{Postgres.*}} which may be unresolved
    // if the linked service name doesn't match — so we prefer RAILWAY_SSOT_URL.
    let raw = std::env::var("RAILWAY_SSOT_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| anyhow::anyhow!(
            "Set RAILWAY_SSOT_URL in Railway → Variables: \
             postgresql://postgres:PASS@interchange.proxy.rlwy.net:30942/railway"
        ))?;
    let url = raw.trim().to_string();
    if !(url.starts_with("postgres://") || url.starts_with("postgresql://")) {
        anyhow::bail!("DB URL must start with postgres:// or postgresql:// — got: {:?}", &url[..url.len().min(20)]);
    }
    tracing::info!("Connecting to DB (len={} chars)", url.len());

    let (client, conn) = tokio_postgres::connect(&url, NoTls).await
        .map_err(|e| anyhow::anyhow!("DB connect failed: {e}"))?;
    tokio::spawn(async move { if let Err(e) = conn.await { eprintln!("db err: {e}"); } });

    let db: Db = Arc::new(client);
    // PORT is injected by Railway — do NOT default to 3030 or healthcheck will mismatch
    let port: u16 = std::env::var("PORT").unwrap_or_else(|_| "8080".into()).parse().unwrap_or(8080);

    let app = Router::new()
        .route("/", get(index))
        .route("/htmx/chapters", get(htmx_chapters))
        .route("/htmx/rag", get(htmx_rag))
        .route("/htmx/coq", get(htmx_coq))
        .route("/htmx/progress", get(htmx_progress))
        .route("/api/coq", get(api_coq))
        .route("/api/progress", get(api_progress))
        .route("/healthz", get(api_health))
        .with_state(db);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("PhD SSOT Dashboard on http://{addr}");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
