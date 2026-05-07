# RAG over Flos Aureus — operator guide

Anchor: `phi^2 + phi^-2 = 3`.  Author: Дмитрий Васильев (ORCID 0009-0008-4294-6159).

## Stack

| Layer | Choice | Why |
|---|---|---|
| Vector store | Railway PostgreSQL `phd-postgres-ssot` + pgvector | SSOT (R6); no separate Neon |
| Chunker | `ssot.chunk_chapters()` (plpgsql, ~1200 chars + 120 overlap) | Idempotent, runs on the SSOT itself |
| Embedder | **BAAI/bge-m3** (dim 1024, MTEB multilingual top-3, Apache 2.0) | EN+RU, 32k context, fully open |
| Runtime | `fastembed-rs 5.13` → ONNX, no Python, no API key | R1 (Rust-only) |
| Search | `ssot.rag_search(query::vector(1024), k, chapter?)` | cosine via `vector_cosine_ops` ivfflat |

## One-time apply (already done on Railway)

```bash
psql "$RAILWAY_URL" -f docs/phd/ddl/001_ssot_init.sql
psql "$RAILWAY_URL" -f docs/phd/ddl/002_rag_vectors.sql      # creates table at dim 1536
psql "$RAILWAY_URL" -f docs/phd/ddl/003_rag_bge_m3_dim1024.sql   # migrates to 1024 + bge-m3
psql "$RAILWAY_URL" -c "SELECT ssot.chunk_chapters();"
```

## Embed

```bash
cargo build -p trios-phd --features rag --release --bin embed_rag
DATABASE_URL="$RAILWAY_SSOT_URL" \
  ./target/release/embed_rag --batch-size 8

# RAILWAY_SSOT_URL is provisioned via Railway service variables on
# `phd-postgres-ssot`; never hard-code the host:port:credentials triple here.
```

The first run downloads `BAAI/bge-m3` (~600 MB) into `~/.cache/fastembed`; subsequent
runs are instant.  Throughput on a CPU-only sandbox: ~0.5 chunks/s, so the full
374-chunk corpus completes in ~3–4 minutes after the model is cached.

## Sanity

```sql
SELECT * FROM ssot.rag_status;
-- expect: total_chunks=374, embedded_chunks=374, coverage_pct=100.00

WITH probe AS (SELECT embedding FROM ssot.embeddings
               WHERE chunk_text ILIKE '%golden ratio%' LIMIT 1)
SELECT s.chapter_slug, s.chunk_index, round(s.distance::numeric, 4) AS dist
  FROM probe p, ssot.rag_search(p.embedding, 5) s;
```
