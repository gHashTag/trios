//! `embed_rag` — pure-Rust BGE-M3 embedder for the Flos Aureus RAG corpus.
//!
//! Anchor: phi^2 + phi^-2 = 3.  Author: Дмитрий Васильев (ORCID 0009-0008-4294-6159).
//!
//! R1 (CROWN): Rust only. No Python, no shell, no remote inference service.
//!   The model weights are pulled from Hugging Face on first run by `fastembed`
//!   and cached in `$HOME/.cache/fastembed`; no API key, no quota.
//! R5 (HONEST): writes embeddings only for chunks the schema migration 003 has
//!   already resized to `vector(1024)`; if the column is not 1024-d we exit with
//!   a non-zero status and a clear error rather than silently corrupting rows.
//! R6 (SSOT): Railway PostgreSQL `ssot.embeddings` is the single source of truth
//!   for the RAG index; this binary only writes there, it never builds a side
//!   cache or local sqlite.
//!
//! Usage:
//!     export DATABASE_URL="$RAILWAY_SSOT_URL"   # see ops/secrets handbook
//!     cargo run -p trios-phd --features rag --release --bin embed_rag -- \
//!         --batch-size 16 --chapter ch_00   # optional filter
//!
//! The binary is idempotent: rows with `embedding IS NOT NULL` are skipped by
//! default. Use `--force` to recompute everything.

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::env;
use std::time::Instant;
use tokio_postgres::NoTls;

#[derive(Parser, Debug)]
#[command(
    author = "Dmitrii Vasilev <ORCID 0009-0008-4294-6159>",
    about = "Embed ssot.embeddings rows with BAAI/bge-m3 (dim 1024).",
    long_about = "Pulls model weights via fastembed on first run; no remote API."
)]
struct Cli {
    /// PostgreSQL connection string. Falls back to DATABASE_URL.
    #[arg(long)]
    database_url: Option<String>,

    /// Limit to a single chapter (e.g. `ch_05`).
    #[arg(long)]
    chapter: Option<String>,

    /// Batch size for the embedder.
    #[arg(long, default_value_t = 16)]
    batch_size: usize,

    /// Recompute even rows that already have an embedding.
    #[arg(long, default_value_t = false)]
    force: bool,

    /// Hard cap on the number of chunks processed (0 = no cap).
    #[arg(long, default_value_t = 0)]
    limit: usize,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let database_url = cli
        .database_url
        .clone()
        .or_else(|| env::var("DATABASE_URL").ok())
        .ok_or_else(|| anyhow!("--database-url or DATABASE_URL is required"))?;

    // ---- 1. Connect & verify column dim --------------------------------------
    eprintln!("[embed_rag] connecting to railway postgres ...");
    let (db, conn) = tokio_postgres::connect(&database_url, NoTls)
        .await
        .with_context(|| "connect to DATABASE_URL")?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("[embed_rag] postgres connection error: {e}");
        }
    });

    let dim_row = db
        .query_one(
            "SELECT format_type(atttypid, atttypmod) \
             FROM pg_attribute \
             WHERE attrelid = 'ssot.embeddings'::regclass AND attname = 'embedding'",
            &[],
        )
        .await?;
    let dim_str: String = dim_row.get(0);
    if !dim_str.contains("(1024)") {
        return Err(anyhow!(
            "ssot.embeddings.embedding is `{dim_str}`, expected `vector(1024)` — \
             apply ddl/003_rag_bge_m3_dim1024.sql first"
        ));
    }
    eprintln!("[embed_rag] column dim ok: {dim_str}");

    // ---- 2. Snapshot work --------------------------------------------------
    let mut sql = String::from(
        "SELECT id, chapter_slug, chunk_index, chunk_text \
         FROM ssot.embeddings WHERE 1=1",
    );
    if !cli.force {
        sql.push_str(" AND embedding IS NULL");
    }
    if let Some(ref ch) = cli.chapter {
        sql.push_str(&format!(" AND chapter_slug = '{}'", ch.replace('\'', "''")));
    }
    sql.push_str(" ORDER BY chapter_slug, chunk_index");
    if cli.limit > 0 {
        sql.push_str(&format!(" LIMIT {}", cli.limit));
    }

    let rows = db.query(&sql, &[]).await?;
    let total = rows.len();
    if total == 0 {
        println!("[embed_rag] nothing to do — 0 chunks pending. (Use --force to recompute.)");
        return Ok(());
    }
    eprintln!("[embed_rag] pending chunks: {total}");

    // ---- 3. Boot the embedder ---------------------------------------------
    eprintln!(
        "[embed_rag] loading BAAI/bge-m3 (first run downloads ~600MB to fastembed cache) ..."
    );
    let cache_dir = env::var("FASTEMBED_CACHE").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        format!("{home}/.cache/fastembed")
    });
    std::fs::create_dir_all(&cache_dir).ok();
    let opts = InitOptions::new(EmbeddingModel::BGEM3)
        .with_cache_dir(cache_dir.into())
        .with_show_download_progress(true);
    let mut model = TextEmbedding::try_new(opts).context("load BGEM3 model")?;

    // ---- 4. Embed in batches & write back ----------------------------------
    let started = Instant::now();
    let mut written = 0usize;

    for batch in rows.chunks(cli.batch_size) {
        let texts: Vec<&str> = batch
            .iter()
            .map(|r| {
                let s: &str = r.get("chunk_text");
                s
            })
            .collect();
        let embeddings = model
            .embed(texts.clone(), None)
            .with_context(|| "embed batch")?;
        if embeddings.len() != batch.len() {
            return Err(anyhow!(
                "embedder returned {} rows for {} inputs",
                embeddings.len(),
                batch.len()
            ));
        }
        for (row, vec) in batch.iter().zip(embeddings.iter()) {
            let id: i64 = row.get("id");
            if vec.len() != 1024 {
                return Err(anyhow!("BGE-M3 returned dim {} (expected 1024)", vec.len()));
            }
            let pg_vec = format_pg_vector(vec);
            db.execute(
                "UPDATE ssot.embeddings \
                   SET embedding   = ($1::text)::vector, \
                       model_name  = 'BAAI/bge-m3', \
                       embedded_at = now(), \
                       updated_at  = now() \
                 WHERE id = $2",
                &[&pg_vec, &id],
            )
            .await
            .with_context(|| format!("update id={id}"))?;
            written += 1;
        }
        let elapsed = started.elapsed().as_secs_f64();
        let rate = if elapsed > 0.0 {
            written as f64 / elapsed
        } else {
            0.0
        };
        eprintln!(
            "[embed_rag] {:>4}/{:>4} chunks  ({:.1}/s)",
            written, total, rate
        );
    }

    // ---- 5. Final coverage report -----------------------------------------
    let row = db
        .query_one(
            "SELECT total_chunks::int8, embedded_chunks::int8, \
                    coverage_pct::text AS coverage_pct \
             FROM ssot.rag_status",
            &[],
        )
        .await?;
    let total_chunks: i64 = row.get("total_chunks");
    let embedded_chunks: i64 = row.get("embedded_chunks");
    let coverage_pct: String = row.get("coverage_pct");
    println!(
        "[embed_rag] done — wrote {written} new vectors in {:.1}s; \
         coverage: {embedded_chunks}/{total_chunks} ({coverage_pct}%)",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

/// pgvector text format: `[v1,v2,...,vN]`
fn format_pg_vector(v: &[f32]) -> String {
    let mut s = String::with_capacity(v.len() * 8 + 2);
    s.push('[');
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        // 6 fp digits is plenty for cosine similarity and avoids monster strings.
        s.push_str(&format!("{:.6}", x));
    }
    s.push(']');
    s
}
