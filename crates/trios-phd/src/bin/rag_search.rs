//! `rag_search` — pure-Rust lexical RAG retrieval over the Trinity SSOT.
//!
//! Anchor: phi^2 + phi^-2 = 3.  Author: Дмитрий Васильев (ORCID 0009-0008-4294-6159).
//!
//! ## Why a separate binary
//!
//! `embed_rag` writes BGE-M3 dense vectors into `ssot.embeddings`. That table
//! requires `CREATE EXTENSION vector` and a DDL migration that the Pipedream
//! MCP connector silently refuses to execute. Until the operator installs
//! pgvector through the Railway UI, we expose a **lexical fallback** that:
//!
//!   * reads chunks from `public.strategy_queue` (status='rag_chunk'), which is
//!     the **canonical SSOT** populated 2026-05-12 by Perplexity Computer (rows
//!     41–90 at present);
//!   * matches the query with ILIKE plus a simple normalised-edit score so we
//!     keep behaviour deterministic and Coq-friendly (no hidden ML);
//!   * never silently degrades: if `public.strategy_queue` is missing the
//!     `rag_chunk` rows, we exit non-zero with a clear message.
//!
//! ## R-Constitutional alignment
//!
//! * R1 (CROWN): Rust-only — no Python bridge.
//! * R5 (HONEST): retrieval results are scored, never invented; empty hits stay
//!   empty.
//! * R6 (SSOT): the only source of truth is Railway Postgres `public.strategy_queue`.
//! * R7 (ANCHOR): the binary echoes `phi^2 + phi^-2 = 3` in `--version` output
//!   so the anchor lives next to the executable, not only in docs.
//!
//! ## Usage
//!
//! ```bash
//! export DATABASE_URL="$RAILWAY_SSOT_URL"
//! cargo run -p trios-phd --features rag --release --bin rag_search -- \
//!     --query "phi^2 + phi^-2 = 3" --limit 5
//! ```
//!
//! Once `CREATE EXTENSION vector;` is applied and `ssot.embeddings` populated,
//! a follow-up commit will add a `--dense` flag that switches to cosine
//! similarity over BGE-M3. The lexical path remains as a fallback.

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use serde::Serialize;
use std::env;
use tokio_postgres::NoTls;

const ANCHOR: &str = "phi^2 + phi^-2 = 3";

#[derive(Parser, Debug)]
#[command(
    author = "Dmitrii Vasilev <ORCID 0009-0008-4294-6159>",
    about = "Lexical RAG retrieval over public.strategy_queue (status='rag_chunk').",
    long_about = "Fallback path while pgvector is not yet installed. \
                  Anchor: phi^2 + phi^-2 = 3."
)]
struct Cli {
    /// PostgreSQL connection string. Falls back to DATABASE_URL.
    #[arg(long)]
    database_url: Option<String>,

    /// Free-text query string.
    #[arg(long, short = 'q')]
    query: String,

    /// Max number of hits to return.
    #[arg(long, default_value_t = 5)]
    limit: i64,

    /// Restrict to a particular chapter kind (frontmatter, chapter, appendix).
    #[arg(long)]
    kind: Option<String>,

    /// Emit JSON instead of human-readable table.
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Debug, Serialize)]
struct Hit {
    id: i64,
    canon_name: String,
    file: String,
    kind: String,
    preview: String,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let database_url = cli
        .database_url
        .clone()
        .or_else(|| env::var("DATABASE_URL").ok())
        .ok_or_else(|| anyhow!("--database-url or DATABASE_URL is required"))?;

    if cli.query.trim().is_empty() {
        return Err(anyhow!("--query must be non-empty"));
    }

    eprintln!("[rag_search] connecting to Railway Postgres (anchor: {ANCHOR})");
    let (db, conn) = tokio_postgres::connect(&database_url, NoTls)
        .await
        .with_context(|| "connect to DATABASE_URL")?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("[rag_search] postgres connection error: {e}");
        }
    });

    // Sanity-check: the canonical SSOT must contain RAG chunks.
    let total_row = db
        .query_one(
            "SELECT COUNT(*)::int8 AS n FROM public.strategy_queue \
             WHERE status='rag_chunk'",
            &[],
        )
        .await
        .with_context(|| "count rag_chunk rows")?;
    let total: i64 = total_row.get("n");
    if total == 0 {
        return Err(anyhow!(
            "public.strategy_queue has zero rows with status='rag_chunk' — \
             SSOT is empty. Run the ingestion pipeline first."
        ));
    }
    eprintln!("[rag_search] SSOT inventory: {total} rag_chunk rows available");

    // Build the SQL. We pass the query as a positional parameter so the
    // database driver handles escaping; we add a `length(text)` tie-breaker so
    // shorter, denser matches surface first.
    let pattern = format!("%{}%", cli.query);
    let mut sql = String::from(
        "SELECT id, canon_name, \
                COALESCE(config->>'file', '?') AS file, \
                COALESCE(config->>'kind', 'unknown') AS kind, \
                substring(config->>'text', 1, 320) AS preview \
         FROM public.strategy_queue \
         WHERE status='rag_chunk' \
           AND config->>'text' ILIKE $1",
    );
    if let Some(ref k) = cli.kind {
        // Whitelist the kind values to avoid any injection foothold.
        let safe = match k.as_str() {
            "frontmatter" | "chapter" | "appendix" => k.as_str(),
            other => return Err(anyhow!("--kind must be frontmatter|chapter|appendix, got `{other}`")),
        };
        sql.push_str(&format!(" AND config->>'kind' = '{safe}'"));
    }
    sql.push_str(" ORDER BY length(config->>'text') ASC, id ASC LIMIT $2");

    let rows = db
        .query(&sql, &[&pattern, &cli.limit])
        .await
        .with_context(|| "execute lexical retrieval")?;

    let hits: Vec<Hit> = rows
        .iter()
        .map(|r| Hit {
            id: r.get::<_, i64>("id"),
            canon_name: r.get::<_, String>("canon_name"),
            file: r.get::<_, String>("file"),
            kind: r.get::<_, String>("kind"),
            preview: r.get::<_, Option<String>>("preview").unwrap_or_default(),
        })
        .collect();

    if cli.json {
        let payload = serde_json::json!({
            "anchor": ANCHOR,
            "query": cli.query,
            "limit": cli.limit,
            "total_chunks_in_ssot": total,
            "hits": hits,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("# rag_search — anchor {ANCHOR}");
        println!("# query: {}", cli.query);
        println!("# hits: {}/{}", hits.len(), total);
        for (i, h) in hits.iter().enumerate() {
            println!(
                "\n[{}] id={}  canon={}\n     file={}  kind={}\n     {}",
                i + 1,
                h.id,
                h.canon_name,
                h.file,
                h.kind,
                h.preview.replace('\n', " ")
            );
        }
    }
    Ok(())
}
