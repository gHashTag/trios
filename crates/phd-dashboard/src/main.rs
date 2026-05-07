//! PhD SSOT Dashboard — Axum + HTMX + Askama
//! R1: pure Rust, no .py/.sh, no JS frameworks
//! φ² + φ⁻² = 3 · TRINITY
//!
//! ENV: RAILWAY_SSOT_URL=postgresql://postgres:...@interchange.proxy.rlwy.net:30942/railway
//!      PORT=3030 (default)
//!
//! NOTE: do NOT use DATABASE_URL — Railway auto-injects it from linked Postgres
//!       as an unresolved ${{Postgres.DATABASE_URL}} template reference.

use anyhow::Result;
use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::State,
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

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    stats: Stats,
    chapters: Vec<ChapterRow>,
    duplicates: Vec<DuplicateRow>,
    rag: Vec<RagRow>,
}

async fn fetch_chapters(db: &tokio_postgres::Client) -> Result<Vec<ChapterRow>> {
    let rows = db
        .query(
            "SELECT chapter_slug, chapter_no, kind, title, \
                    COALESCE(line_count,0) AS line_count, \
                    (COALESCE(line_count,0) >= 1500) AS r3_full, \
                    (EXISTS(SELECT 1 FROM ssot.chapter_figures f WHERE f.chapter_slug = c.chapter_slug)) AS has_figure, \
                    (SELECT COUNT(*) FROM ssot.theorems t WHERE t.chapter_slug = c.chapter_slug) AS theorem_count, \
                    updated_at::text \
             FROM ssot.chapters c \
             ORDER BY chapter_no",
            &[],
        )
        .await?;
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
         FROM ssot.embeddings e \
         GROUP BY e.chapter_slug ORDER BY e.chapter_slug",
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
            (SELECT COUNT(*) FROM ssot.chapters) AS total_chapters, \
            (SELECT COUNT(*) FROM ssot.chapters WHERE COALESCE(line_count,0) >= 1500) AS r3_ok, \
            (SELECT COUNT(*) FROM ssot.chapter_figures) AS with_figure, \
            (SELECT COUNT(*) FROM ssot.theorems) AS total_theorems, \
            (SELECT COUNT(*) FROM ssot.embeddings) AS rag_total, \
            (SELECT COUNT(*) FROM ssot.embeddings WHERE embedding IS NOT NULL) AS rag_embedded, \
            (SELECT COUNT(*) FROM ( \
               SELECT chapter_slug FROM ssot.chapters \
               GROUP BY lower(trim(title)) HAVING COUNT(*) > 1 \
            ) d) AS duplicate_slugs",
        &[],
    ).await?;
    let rag_total: i64 = row.get(4);
    let rag_embedded: i64 = row.get(5);
    let rag_pct = if rag_total > 0 { (rag_embedded as f64 / rag_total as f64) * 100.0 } else { 0.0 };
    Ok(Stats {
        total_chapters: row.get(0),
        r3_ok: row.get(1),
        with_figure: row.get(2),
        total_theorems: row.get(3),
        rag_total,
        rag_embedded,
        rag_pct,
        rag_pct_int: rag_pct.round() as i64,
        duplicate_slugs: row.get(6),
    })
}

async fn index(State(db): State<Db>) -> impl IntoResponse {
    let (stats, chapters, duplicates, rag) = tokio::join!(
        fetch_stats(&db), fetch_chapters(&db), fetch_duplicates(&db), fetch_rag(&db),
    );
    IndexTemplate {
        stats: stats.unwrap_or(Stats {
            total_chapters: 0, r3_ok: 0, with_figure: 0,
            total_theorems: 0, rag_total: 0, rag_embedded: 0,
            rag_pct: 0.0, rag_pct_int: 0, duplicate_slugs: 0,
        }),
        chapters: chapters.unwrap_or_default(),
        duplicates: duplicates.unwrap_or_default(),
        rag: rag.unwrap_or_default(),
    }
}

async fn htmx_chapters(State(db): State<Db>) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "partials/chapters_table.html")]
    struct T { chapters: Vec<ChapterRow> }
    T { chapters: fetch_chapters(&db).await.unwrap_or_default() }
}

async fn htmx_rag(State(db): State<Db>) -> impl IntoResponse {
    #[derive(Template)]
    #[template(path = "partials/rag_table.html")]
    struct T { rag: Vec<RagRow> }
    T { rag: fetch_rag(&db).await.unwrap_or_default() }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let phi: f64 = (1.0 + 5.0_f64.sqrt()) / 2.0;
    assert!((phi * phi + 1.0 / (phi * phi) - 3.0).abs() < 1e-12, "Trinity anchor");

    // Use RAILWAY_SSOT_URL — NOT DATABASE_URL (Railway auto-fills that with
    // an unresolved ${{Postgres.*}} template from any linked Postgres service).
    let database_url = std::env::var("RAILWAY_SSOT_URL")
        .map_err(|_| anyhow::anyhow!(
            "RAILWAY_SSOT_URL must be set in Railway → Variables. \
             Value: postgresql://postgres:PASS@interchange.proxy.rlwy.net:30942/railway"
        ))?;
    let trimmed = database_url.trim();
    if !(trimmed.starts_with("postgres://") || trimmed.starts_with("postgresql://")) {
        anyhow::bail!("RAILWAY_SSOT_URL must start with postgres:// or postgresql://");
    }

    let (client, conn) = tokio_postgres::connect(trimmed, NoTls).await
        .map_err(|e| anyhow::anyhow!("connect failed: {e}"))?;
    tokio::spawn(async move { if let Err(e) = conn.await { eprintln!("db err: {e}"); } });

    let db: Db = Arc::new(client);
    let port: u16 = std::env::var("PORT").unwrap_or_else(|_| "3030".into()).parse().unwrap_or(3030);

    let app = Router::new()
        .route("/", get(index))
        .route("/htmx/chapters", get(htmx_chapters))
        .route("/htmx/rag", get(htmx_rag))
        .with_state(db);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("PhD SSOT Dashboard on http://{addr}");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
