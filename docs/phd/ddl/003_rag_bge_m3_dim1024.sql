-- =============================================================================
-- Flos Aureus PhD Monograph — RAG: switch to BGE-M3 (dim 1024) (003)
-- Anchor: phi^2 + phi^-2 = 3
-- Author: Дмитрий Васильев / Dmitrii Vasilev (ORCID 0009-0008-4294-6159)
-- DOI:    10.5281/zenodo.19227877
-- Refs:   trios#446, trios#518; supersedes 002_rag_vectors.sql dim default
-- Depends on: 002_rag_vectors.sql
-- Reason: BGE-M3 is the only fully-open multilingual MTEB top-3 model
--         covering EN+RU at 32k context; OpenAI text-embedding-3-small
--         is non-free and EN-biased.  We migrate the vector column from
--         dim 1536 to dim 1024 BEFORE any embeddings are written, so this
--         is a zero-loss schema change (rag_status reports 0 embedded).
-- =============================================================================

BEGIN;

-- 1. Drop dependents that reference the embedding column (view + index).
DROP VIEW IF EXISTS ssot.rag_status;
DROP INDEX IF EXISTS ssot.embeddings_ivfflat_idx;

-- 2. Wipe any stale rows (defensive — on the live DB this is 0).
DELETE FROM ssot.embeddings WHERE embedding IS NOT NULL;

-- 3. Resize the vector column.
ALTER TABLE ssot.embeddings
    ALTER COLUMN embedding TYPE vector(1024) USING NULL,
    ALTER COLUMN model_name SET DEFAULT 'BAAI/bge-m3';

-- 4. Backfill default model name on rows already present (374 chunks).
UPDATE ssot.embeddings
   SET model_name = 'BAAI/bge-m3'
 WHERE model_name = 'text-embedding-3-small'
   AND embedding IS NULL;

-- 5. Recreate the IVFFlat cosine index with lists=20 (sqrt(374) ≈ 19.3).
CREATE INDEX embeddings_ivfflat_idx
    ON ssot.embeddings USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 20);

-- 6a. Recreate the rag_status view (was dropped because it depends on the column).
CREATE OR REPLACE VIEW ssot.rag_status AS
SELECT count(*)                                            AS total_chunks,
       count(embedding)                                    AS embedded_chunks,
       count(*) FILTER (WHERE embedding IS NULL)           AS pending_chunks,
       CASE WHEN count(*) = 0 THEN 0::numeric
            ELSE round(100.0 * count(embedding)::numeric / count(*)::numeric, 2)
       END                                                 AS coverage_pct,
       count(DISTINCT chapter_slug)                        AS chapters_indexed
  FROM ssot.embeddings;

-- 6b. Drop the legacy rag_search overloads (any (vector, integer[, text])) so
--     the new dim-1024 function is unambiguously selectable.
DROP FUNCTION IF EXISTS ssot.rag_search(vector, integer);
DROP FUNCTION IF EXISTS ssot.rag_search(vector, integer, text);
DROP FUNCTION IF EXISTS ssot.rag_search(vector(1536), integer, text);
CREATE OR REPLACE FUNCTION ssot.rag_search(
    p_query     vector(1024),
    p_limit     integer DEFAULT 8,
    p_chapter   text    DEFAULT NULL
) RETURNS TABLE (
    chapter_slug text,
    chunk_index  integer,
    chunk_text   text,
    distance     real
)
LANGUAGE sql
STABLE
AS $$
    SELECT chapter_slug,
           chunk_index,
           chunk_text,
           (embedding <=> p_query)::real AS distance
      FROM ssot.embeddings
     WHERE embedding IS NOT NULL
       AND (p_chapter IS NULL OR chapter_slug = p_chapter)
     ORDER BY embedding <=> p_query
     LIMIT p_limit
$$;

-- 7. Anchor self-check survives schema migration.
DO $$
DECLARE
    a double precision;
BEGIN
    SELECT extract(epoch FROM '00:00:01'::interval) * 0 +
           ((1 + sqrt(5))/2)^2 + 1/((1 + sqrt(5))/2)^2 INTO a;
    IF abs(a - 3) > 1e-9 THEN
        RAISE EXCEPTION 'Trinity anchor failed in 003: phi^2 + 1/(phi*phi) = %', a;
    END IF;
END $$;

COMMIT;
