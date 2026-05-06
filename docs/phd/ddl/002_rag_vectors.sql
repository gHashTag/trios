-- =============================================================================
-- Flos Aureus PhD Monograph — RAG vector embeddings (002)
-- Anchor: phi^2 + phi^-2 = 3
-- Author: Дмитрий Васильев / Dmitrii Vasilev (ORCID 0009-0008-4294-6159)
-- DOI:    10.5281/zenodo.19227877
-- Refs:   trios#512, trios#513, trios#516
-- Depends on: 001_ssot_init.sql (ssot.chapters)
-- =============================================================================

CREATE EXTENSION IF NOT EXISTS vector;

-- ---------------------------------------------------------------------------
-- ssot.embeddings — chunk-level vector store for RAG over the monograph prose
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ssot.embeddings (
    id            bigserial PRIMARY KEY,
    chapter_slug  text NOT NULL REFERENCES ssot.chapters(chapter_slug) ON DELETE CASCADE,
    chunk_index   integer NOT NULL,
    chunk_text    text NOT NULL,
    chunk_tokens  integer,
    embedding     vector(1536),
    model_name    text NOT NULL DEFAULT 'text-embedding-3-small',
    embedded_at   timestamptz,
    updated_at    timestamptz NOT NULL DEFAULT now(),
    UNIQUE (chapter_slug, chunk_index)
);

CREATE INDEX IF NOT EXISTS embeddings_chapter_idx
    ON ssot.embeddings (chapter_slug);

-- IVFFlat cosine index — lists≈sqrt(N), 60 covers 120-180 chunks comfortably.
CREATE INDEX IF NOT EXISTS embeddings_ivfflat_idx
    ON ssot.embeddings USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 60);

-- ---------------------------------------------------------------------------
-- ssot.chunk_chapters() — naive paragraph chunker (~1200 char target).
-- Idempotent: deletes prior chunks for each chapter before re-inserting.
-- Returns total chunks produced.
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION ssot.chunk_chapters(
    p_target_chars integer DEFAULT 1200,
    p_overlap_chars integer DEFAULT 120
) RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    r              record;
    txt            text;
    pos            integer;
    n              integer;
    chunk          text;
    chunk_no       integer;
    total          integer := 0;
BEGIN
    FOR r IN SELECT chapter_slug, body_md FROM ssot.chapters LOOP
        DELETE FROM ssot.embeddings WHERE chapter_slug = r.chapter_slug;
        txt := coalesce(r.body_md, '');
        n := length(txt);
        pos := 1;
        chunk_no := 0;
        WHILE pos <= n LOOP
            chunk := substring(txt FROM pos FOR p_target_chars);
            INSERT INTO ssot.embeddings (chapter_slug, chunk_index, chunk_text, chunk_tokens)
            VALUES (r.chapter_slug, chunk_no, chunk, ceil(length(chunk)::numeric / 4)::integer);
            chunk_no := chunk_no + 1;
            total := total + 1;
            pos := pos + p_target_chars - p_overlap_chars;
        END LOOP;
    END LOOP;
    RETURN total;
END;
$$;

-- ---------------------------------------------------------------------------
-- ssot.rag_search(query_embedding, k) — top-k cosine-similarity retrieval.
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION ssot.rag_search(
    p_query   vector(1536),
    p_k       integer DEFAULT 8
) RETURNS TABLE (
    chapter_slug  text,
    chunk_index   integer,
    chunk_text    text,
    similarity    real
)
LANGUAGE sql STABLE
AS $$
    SELECT chapter_slug,
           chunk_index,
           chunk_text,
           1 - (embedding <=> p_query) AS similarity
    FROM   ssot.embeddings
    WHERE  embedding IS NOT NULL
    ORDER  BY embedding <=> p_query
    LIMIT  p_k
$$;

-- ---------------------------------------------------------------------------
-- ssot.rag_status — coverage view (% of chunks with embedding populated).
-- ---------------------------------------------------------------------------
CREATE OR REPLACE VIEW ssot.rag_status AS
SELECT
    count(*)                                          AS total_chunks,
    count(embedding)                                  AS embedded_chunks,
    count(*) FILTER (WHERE embedding IS NULL)         AS pending_chunks,
    CASE WHEN count(*) = 0 THEN 0
         ELSE round(100.0 * count(embedding) / count(*), 2)
    END                                               AS coverage_pct,
    count(DISTINCT chapter_slug)                      AS chapters_indexed
FROM ssot.embeddings;

-- ---------------------------------------------------------------------------
-- Trinity anchor self-check (idempotent)
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    phi double precision := (1 + sqrt(5)) / 2;
BEGIN
    IF abs((phi * phi) + (1 / (phi * phi)) - 3) > 1e-9 THEN
        RAISE EXCEPTION 'Trinity anchor violation: phi^2 + phi^-2 != 3';
    END IF;
END $$;
