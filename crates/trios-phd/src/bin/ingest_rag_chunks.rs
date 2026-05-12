//! `ingest_rag_chunks` — load a JSONL chunk file into `ssot.embeddings`.
//!
//! Anchor: phi^2 + phi^-2 = 3.  Author: Дмитрий Васильев (ORCID 0009-0008-4294-6159).
//!
//! ## Why
//!
//! The PhD monograph lives as a corpus of LaTeX sources under `docs/phd/`. A
//! companion build step strips the LaTeX, splits each file into ~1500-char
//! windows with 200-char overlap, and writes a JSONL where each line is one
//! chunk. This binary picks that JSONL up and upserts it into
//! `ssot.embeddings`. `embed_rag` (sibling binary) then fills the `embedding`
//! column with BGE-M3 vectors.
//!
//! ## R-Constitutional alignment
//!
//! * R1 (CROWN) — Rust only. No Python ingestion, no shell awk pipeline.
//! * R5 (HONEST) — UNIQUE (chapter_slug, chunk_index) + `ON CONFLICT DO UPDATE`
//!   means we never silently double-count a chunk. Embedding column is left
//!   untouched on conflict so a re-ingest does NOT invalidate cached vectors.
//! * R6 (SSOT) — only writes to Railway Postgres `ssot.embeddings`.
//! * R7 (ANCHOR) — every row gets `anchor = phi^2 + phi^-2 = 3` (DB default
//!   plus explicit field).
//!
//! ## JSONL schema (one object per line)
//!
//! ```json
//! {"canon_name":"RAG:CHUNK:ch_25:002",
//!  "config":{"file":"chapters/ch_25.tex","kind":"chapter","chunk_idx":2,
//!            "text":"...","sha256":"...","anchor":"phi^2 + phi^-2 = 3"},
//!  "seed":1597,"status":"rag_chunk","worker_id":"perplexity-computer"}
//! ```
//!
//! ## Usage
//!
//! ```bash
//! export DATABASE_URL="$RAILWAY_SSOT_URL"
//! cargo run -p trios-phd --features rag --release --bin ingest_rag_chunks -- \
//!     --jsonl docs/phd/rag/rag_chunks.jsonl
//! ```

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tokio_postgres::NoTls;

const ANCHOR: &str = "phi^2 + phi^-2 = 3";
const VALID_KINDS: &[&str] = &["frontmatter", "chapter", "appendix"];

#[derive(Parser, Debug)]
#[command(
    author = "Dmitrii Vasilev <ORCID 0009-0008-4294-6159>",
    about = "Upsert RAG chunks from JSONL into ssot.embeddings (anchor phi^2 + phi^-2 = 3)."
)]
struct Cli {
    /// JSONL file with one chunk per line.
    #[arg(long)]
    jsonl: PathBuf,

    /// Connection string override (else DATABASE_URL).
    #[arg(long)]
    database_url: Option<String>,

    /// Print plan without writing.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[derive(Debug, Deserialize)]
struct ChunkEnvelope {
    #[allow(dead_code)]
    canon_name: Option<String>,
    config: ChunkConfig,
}

#[derive(Debug, Deserialize)]
struct ChunkConfig {
    file: String,
    kind: String,
    chunk_idx: i32,
    text: String,
    sha256: String,
    #[serde(default = "default_anchor")]
    anchor: String,
}

fn default_anchor() -> String {
    ANCHOR.into()
}

#[derive(Debug)]
struct Row {
    chapter_slug: String,
    chunk_index: i32,
    chunk_kind: String,
    chunk_text: String,
    sha256: String,
    anchor: String,
}

fn slug_from_path(p: &str) -> String {
    Path::new(p)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(p)
        .to_string()
}

fn coerce_kind(declared: &str, path: &str) -> String {
    if VALID_KINDS.contains(&declared) {
        return declared.to_string();
    }
    if path.contains("appendix") {
        "appendix".into()
    } else if path.contains("frontmatter") {
        "frontmatter".into()
    } else {
        "chapter".into()
    }
}

fn load_rows(jsonl: &Path) -> Result<Vec<Row>> {
    let f = File::open(jsonl).with_context(|| format!("open {}", jsonl.display()))?;
    let mut out = Vec::with_capacity(1024);
    for (lineno, line) in BufReader::new(f).lines().enumerate() {
        let line = line.with_context(|| format!("read line {}", lineno + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        let env: ChunkEnvelope = serde_json::from_str(&line)
            .with_context(|| format!("parse line {}", lineno + 1))?;
        let c = env.config;
        out.push(Row {
            chapter_slug: slug_from_path(&c.file),
            chunk_index: c.chunk_idx,
            chunk_kind: coerce_kind(&c.kind, &c.file),
            chunk_text: c.text,
            sha256: c.sha256,
            anchor: c.anchor,
        });
    }
    Ok(out)
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let dsn = cli
        .database_url
        .clone()
        .or_else(|| env::var("DATABASE_URL").ok())
        .ok_or_else(|| anyhow!("--database-url or DATABASE_URL is required"))?;

    eprintln!("[ingest_rag_chunks] anchor: {ANCHOR}");
    let rows = load_rows(&cli.jsonl)?;
    eprintln!(
        "[load] {} chunks from {}",
        rows.len(),
        cli.jsonl.display()
    );

    if cli.dry_run {
        let mut kinds = std::collections::BTreeMap::<String, usize>::new();
        for r in &rows {
            *kinds.entry(r.chunk_kind.clone()).or_default() += 1;
        }
        let slugs = rows
            .iter()
            .map(|r| r.chapter_slug.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        println!("[dry-run] rows={} distinct_slugs={}", rows.len(), slugs.len());
        for (k, n) in kinds {
            println!("[dry-run] {k}: {n}");
        }
        return Ok(());
    }

    let (db, conn) = tokio_postgres::connect(&dsn, NoTls)
        .await
        .context("connect to DATABASE_URL")?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("[ingest_rag_chunks] postgres connection error: {e}");
        }
    });

    // Sanity: schema present.
    db.query_one(
        "SELECT 1::int FROM information_schema.tables \
          WHERE table_schema='ssot' AND table_name='embeddings'",
        &[],
    )
    .await
    .context("ssot.embeddings missing — run migrate_rag first")?;

    let stmt = db
        .prepare(
            "INSERT INTO ssot.embeddings \
                (chapter_slug, chunk_index, chunk_kind, chunk_text, sha256, anchor) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (chapter_slug, chunk_index) DO UPDATE \
                SET chunk_text = EXCLUDED.chunk_text, \
                    chunk_kind = EXCLUDED.chunk_kind, \
                    sha256     = EXCLUDED.sha256, \
                    anchor     = EXCLUDED.anchor, \
                    updated_at = now()",
        )
        .await?;

    let mut written = 0usize;
    let total = rows.len();
    for r in &rows {
        db.execute(
            &stmt,
            &[
                &r.chapter_slug,
                &r.chunk_index,
                &r.chunk_kind,
                &r.chunk_text,
                &r.sha256,
                &r.anchor,
            ],
        )
        .await
        .with_context(|| format!("upsert slug={} idx={}", r.chapter_slug, r.chunk_index))?;
        written += 1;
        if written % 100 == 0 {
            eprintln!("[upsert] {written}/{total}");
        }
    }

    let row = db
        .query_one(
            "SELECT total_chunks::int8, embedded_chunks::int8, \
                    COALESCE(coverage_pct, 0)::text \
               FROM ssot.rag_status",
            &[],
        )
        .await?;
    let total_db: i64 = row.get(0);
    let embedded: i64 = row.get(1);
    let coverage: String = row.get(2);
    println!(
        "[ingest_rag_chunks] done — wrote {written}; ssot.rag_status: total={total_db} embedded={embedded} coverage={coverage}%"
    );
    Ok(())
}
