//! `rag_search` — pure-Rust RAG retrieval over `ssot.embeddings`.
//!
//! Anchor: phi^2 + phi^-2 = 3.  Author: Дмитрий Васильев (ORCID 0009-0008-4294-6159).
//!
//! ## Modes
//!
//! * **Lexical** (default) — pg_trgm + ILIKE over `chunk_text`. Deterministic,
//!   no model weights required, available as soon as `migrate_rag` has run.
//! * **Dense** (`--dense`) — cosine similarity over `vector(1024)` embeddings
//!   produced by the sibling `embed_rag` binary (BGE-M3). Requires
//!   `ssot.embeddings.embedding IS NOT NULL` for the rows in scope; otherwise
//!   the binary tells you and exits non-zero (R5 — no silent fallback).
//!
//! ## R-Constitutional alignment
//!
//! * R1 (CROWN) — Rust only.
//! * R5 (HONEST) — explicit errors when the SSOT is empty or unembedded.
//! * R6 (SSOT) — only reads from `ssot.embeddings`; no side caches.
//! * R7 (ANCHOR) — `phi^2 + phi^-2 = 3` echoed in help and JSON output.
//!
//! ## Usage
//!
//! ```bash
//! export DATABASE_URL="$RAILWAY_SSOT_URL"
//! # lexical (no model):
//! cargo run -p trios-phd --features rag --release --bin rag_search -- \
//!     --query "phi^2 + phi^-2 = 3" --limit 5
//! # dense (BGE-M3 embeddings; embed_rag must have populated the column):
//! cargo run -p trios-phd --features rag --release --bin rag_search -- \
//!     --query "trit exhaustive" --dense --limit 10 --kind chapter --json
//! ```

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use serde::Serialize;
use std::env;
use tokio_postgres::NoTls;

const ANCHOR: &str = "phi^2 + phi^-2 = 3";

#[derive(Parser, Debug)]
#[command(
    author = "Dmitrii Vasilev <ORCID 0009-0008-4294-6159>",
    about = "RAG retrieval over ssot.embeddings (lexical or dense BGE-M3).",
    long_about = "Anchor: phi^2 + phi^-2 = 3"
)]
struct Cli {
    /// Free-text query.
    #[arg(long, short = 'q')]
    query: String,

    /// Use dense (cosine over BGE-M3 vector(1024)) retrieval.
    #[arg(long, default_value_t = false)]
    dense: bool,

    /// Restrict to a chunk kind.
    #[arg(long)]
    kind: Option<String>,

    /// Max hits.
    #[arg(long, default_value_t = 5)]
    limit: i64,

    /// JSON output.
    #[arg(long, default_value_t = false)]
    json: bool,

    /// Override DATABASE_URL.
    #[arg(long)]
    database_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct Hit {
    id: i64,
    chapter_slug: String,
    chunk_index: i32,
    chunk_kind: String,
    preview: String,
    score: f64,
}

fn validate_kind(k: &str) -> Result<&str> {
    match k {
        "frontmatter" | "chapter" | "appendix" => Ok(k),
        other => Err(anyhow!("--kind must be frontmatter|chapter|appendix, got `{other}`")),
    }
}

fn format_pg_vector(v: &[f32]) -> String {
    let mut s = String::with_capacity(v.len() * 8 + 2);
    s.push('[');
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!("{:.6}", x));
    }
    s.push(']');
    s
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.query.trim().is_empty() {
        return Err(anyhow!("--query must be non-empty"));
    }

    let dsn = cli
        .database_url
        .clone()
        .or_else(|| env::var("DATABASE_URL").ok())
        .ok_or_else(|| anyhow!("--database-url or DATABASE_URL is required"))?;

    eprintln!("[rag_search] anchor: {ANCHOR}");
    let (db, conn) = tokio_postgres::connect(&dsn, NoTls)
        .await
        .context("connect to DATABASE_URL")?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("[rag_search] postgres connection error: {e}");
        }
    });

    let row = db
        .query_one("SELECT total_chunks::int8, embedded_chunks::int8 FROM ssot.rag_status", &[])
        .await
        .context("read ssot.rag_status (did you run migrate_rag?)")?;
    let total: i64 = row.get(0);
    let embedded: i64 = row.get(1);
    if total == 0 {
        return Err(anyhow!(
            "ssot.embeddings is empty — run `ingest_rag_chunks` first"
        ));
    }
    eprintln!("[ssot] total={total} embedded={embedded}");

    let hits: Vec<Hit> = if cli.dense {
        if embedded == 0 {
            return Err(anyhow!(
                "ssot.embeddings.embedding is NULL for all rows — run `embed_rag` first"
            ));
        }
        let cache = env::var("FASTEMBED_CACHE").unwrap_or_else(|_| {
            let home = env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            format!("{home}/.cache/fastembed")
        });
        std::fs::create_dir_all(&cache).ok();
        let opts = InitOptions::new(EmbeddingModel::BGEM3)
            .with_cache_dir(cache.into())
            .with_show_download_progress(false);
        let mut model = TextEmbedding::try_new(opts).context("load BGE-M3 for query embedding")?;
        let qv = model
            .embed(vec![cli.query.clone()], None)
            .context("embed query")?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("BGE-M3 returned no embedding"))?;
        if qv.len() != 1024 {
            return Err(anyhow!("BGE-M3 returned dim {} (expected 1024)", qv.len()));
        }
        let qv_pg = format_pg_vector(&qv);

        let mut sql = String::from(
            "SELECT id, chapter_slug, chunk_index, chunk_kind, \
                    substring(chunk_text, 1, 320) AS preview, \
                    1.0 - (embedding <=> ($1::text)::vector) AS score \
             FROM ssot.embeddings \
             WHERE embedding IS NOT NULL",
        );
        if let Some(ref k) = cli.kind {
            let safe = validate_kind(k)?;
            sql.push_str(&format!(" AND chunk_kind = '{safe}'"));
        }
        sql.push_str(" ORDER BY embedding <=> ($1::text)::vector ASC LIMIT $2");
        let rows = db.query(&sql, &[&qv_pg, &cli.limit]).await?;
        rows.iter()
            .map(|r| Hit {
                id: r.get::<_, i64>("id"),
                chapter_slug: r.get::<_, String>("chapter_slug"),
                chunk_index: r.get::<_, i32>("chunk_index"),
                chunk_kind: r.get::<_, String>("chunk_kind"),
                preview: r.get::<_, Option<String>>("preview").unwrap_or_default(),
                score: r.get::<_, f64>("score"),
            })
            .collect()
    } else {
        // Lexical via pg_trgm similarity, fallback to ILIKE substring score.
        let pattern = format!("%{}%", cli.query);
        let mut sql = String::from(
            "SELECT id, chapter_slug, chunk_index, chunk_kind, \
                    substring(chunk_text, 1, 320) AS preview, \
                    similarity(chunk_text, $2)::float8 AS score \
             FROM ssot.embeddings \
             WHERE chunk_text ILIKE $1",
        );
        if let Some(ref k) = cli.kind {
            let safe = validate_kind(k)?;
            sql.push_str(&format!(" AND chunk_kind = '{safe}'"));
        }
        sql.push_str(" ORDER BY score DESC, length(chunk_text) ASC LIMIT $3");
        let rows = db.query(&sql, &[&pattern, &cli.query, &cli.limit]).await?;
        rows.iter()
            .map(|r| Hit {
                id: r.get::<_, i64>("id"),
                chapter_slug: r.get::<_, String>("chapter_slug"),
                chunk_index: r.get::<_, i32>("chunk_index"),
                chunk_kind: r.get::<_, String>("chunk_kind"),
                preview: r.get::<_, Option<String>>("preview").unwrap_or_default(),
                score: r.get::<_, f64>("score"),
            })
            .collect()
    };

    if cli.json {
        let payload = serde_json::json!({
            "anchor": ANCHOR,
            "query": cli.query,
            "mode": if cli.dense { "dense_bge_m3" } else { "lexical_pg_trgm" },
            "limit": cli.limit,
            "total_chunks": total,
            "embedded_chunks": embedded,
            "hits": hits,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("# rag_search — anchor {ANCHOR}");
        println!(
            "# query: {}  mode: {}",
            cli.query,
            if cli.dense { "dense (BGE-M3)" } else { "lexical (pg_trgm)" }
        );
        println!("# hits: {}/{}  (embedded {} of {})", hits.len(), cli.limit, embedded, total);
        for (i, h) in hits.iter().enumerate() {
            println!(
                "\n[{}] id={}  {}#{:03}  kind={}  score={:.4}\n     {}",
                i + 1,
                h.id,
                h.chapter_slug,
                h.chunk_index,
                h.chunk_kind,
                h.score,
                h.preview.replace('\n', " ")
            );
        }
    }
    Ok(())
}
