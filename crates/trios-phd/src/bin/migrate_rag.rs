//! `migrate_rag` — idempotent DDL bootstrap for the Flos Aureus RAG SSOT.
//!
//! Anchor: phi^2 + phi^-2 = 3.  Author: Дмитрий Васильев (ORCID 0009-0008-4294-6159).
//!
//! ## What it does
//!
//! 1. `CREATE EXTENSION IF NOT EXISTS vector;`   (pgvector ≥ 0.8.0)
//! 2. `CREATE EXTENSION IF NOT EXISTS pg_trgm;`  (lexical fallback)
//! 3. `CREATE SCHEMA IF NOT EXISTS ssot;`
//! 4. `CREATE TABLE IF NOT EXISTS ssot.embeddings (... vector(1024) ...);`
//! 5. gin trgm index on `chunk_text` for ILIKE retrieval.
//! 6. btree index on `chunk_kind`.
//! 7. `CREATE OR REPLACE VIEW ssot.rag_status` (total / embedded / coverage%).
//!
//! ## R-Constitutional alignment
//!
//! * R1 (CROWN) — Rust only. No Python bootstrap, no `psql` shell pipeline.
//! * R5 (HONEST) — every statement runs with full server-side error capture; we
//!   never declare success on a silent `[]` like the Pipedream MCP did.
//! * R6 (SSOT) — Railway Postgres is the only home; we touch nothing local.
//! * R7 (ANCHOR) — `phi^2 + phi^-2 = 3` is stamped as the default value of the
//!   `anchor` column so every uploaded chunk carries the identity.
//!
//! ## Usage
//!
//! ```bash
//! export DATABASE_URL="$RAILWAY_SSOT_URL"
//! cargo run -p trios-phd --features rag --release --bin migrate_rag
//! ```

use anyhow::{anyhow, Context, Result};
use std::env;
use tokio_postgres::NoTls;

const ANCHOR: &str = "phi^2 + phi^-2 = 3";

/// Each migration step: (label, SQL). All idempotent.
const STEPS: &[(&str, &str)] = &[
    ("ext.vector",       "CREATE EXTENSION IF NOT EXISTS vector"),
    ("ext.pg_trgm",      "CREATE EXTENSION IF NOT EXISTS pg_trgm"),
    ("schema.ssot",      "CREATE SCHEMA IF NOT EXISTS ssot"),
    (
        "table.embeddings",
        "CREATE TABLE IF NOT EXISTS ssot.embeddings ( \
            id            BIGSERIAL PRIMARY KEY, \
            chapter_slug  TEXT        NOT NULL, \
            chunk_index   INT         NOT NULL, \
            chunk_kind    TEXT        NOT NULL CHECK (chunk_kind IN ('frontmatter','chapter','appendix')), \
            chunk_text    TEXT        NOT NULL, \
            sha256        TEXT        NOT NULL, \
            anchor        TEXT        NOT NULL DEFAULT 'phi^2 + phi^-2 = 3', \
            embedding     vector(1024), \
            model_name    TEXT, \
            embedded_at   TIMESTAMPTZ, \
            created_at    TIMESTAMPTZ NOT NULL DEFAULT now(), \
            updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(), \
            UNIQUE (chapter_slug, chunk_index) \
         )",
    ),
    (
        "index.text_trgm",
        "CREATE INDEX IF NOT EXISTS embeddings_text_trgm \
            ON ssot.embeddings USING gin (chunk_text gin_trgm_ops)",
    ),
    (
        "index.kind",
        "CREATE INDEX IF NOT EXISTS embeddings_kind_idx \
            ON ssot.embeddings (chunk_kind)",
    ),
    (
        "view.rag_status",
        "CREATE OR REPLACE VIEW ssot.rag_status AS \
         SELECT COUNT(*)::bigint                                  AS total_chunks, \
                COUNT(embedding)::bigint                          AS embedded_chunks, \
                ROUND(100.0 * COUNT(embedding) / NULLIF(COUNT(*),0), 1) AS coverage_pct \
           FROM ssot.embeddings",
    ),
];

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    let dsn = env::var("DATABASE_URL")
        .map_err(|_| anyhow!("DATABASE_URL is required (Railway connection string)"))?;

    eprintln!("[migrate_rag] anchor: {ANCHOR}");
    eprintln!("[migrate_rag] connecting to Railway Postgres ...");
    let (db, conn) = tokio_postgres::connect(&dsn, NoTls)
        .await
        .context("connect to DATABASE_URL")?;
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("[migrate_rag] postgres connection error: {e}");
        }
    });

    // Whoami
    let row = db
        .query_one("SELECT current_user, current_database(), version()", &[])
        .await?;
    let u: String = row.get(0);
    let d: String = row.get(1);
    let v: String = row.get(2);
    eprintln!("[migrate_rag] user={u}  db={d}");
    eprintln!("[migrate_rag] {v}");

    // Run every step; never silently swallow errors.
    for (label, sql) in STEPS {
        db.execute(*sql, &[])
            .await
            .with_context(|| format!("step {label}"))?;
        eprintln!("[ok] {label}");
    }

    // Verify pgvector usable end-to-end.
    let v_row = db.query_one("SELECT '[1,2,3]'::vector AS v", &[]).await?;
    let v_text: String = v_row.get::<_, String>(0);
    eprintln!("[verify] vector type usable: {v_text}");

    // Report current rag_status.
    let row = db
        .query_one(
            "SELECT total_chunks::int8, embedded_chunks::int8, \
                    COALESCE(coverage_pct, 0)::text \
               FROM ssot.rag_status",
            &[],
        )
        .await?;
    let total: i64 = row.get(0);
    let embedded: i64 = row.get(1);
    let coverage: String = row.get(2);
    println!(
        "[migrate_rag] done — total={total} embedded={embedded} coverage={coverage}% (anchor {ANCHOR})"
    );
    Ok(())
}
