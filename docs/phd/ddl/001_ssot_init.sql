-- =============================================================================
-- Flos Aureus PhD Monograph — SSOT schema bootstrap
-- Anchor: phi^2 + phi^-2 = 3
-- Author: Дмитрий Васильев / Dmitrii Vasilev
-- ORCID:  0009-0008-4294-6159
-- DOI:    10.5281/zenodo.19227877
-- Refs:   trios#516, trios#380
-- =============================================================================

CREATE SCHEMA IF NOT EXISTS ssot;

-- ---------------------------------------------------------------------------
-- ssot.chapters — primary monograph prose, one row per chapter (FA + appendix)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ssot.chapters (
    chapter_slug   text PRIMARY KEY,
    chapter_no     integer NOT NULL,
    kind           text NOT NULL CHECK (kind IN ('fa','trinity','appendix')),
    title          text NOT NULL,
    title_ru       text,
    body_md        text NOT NULL,
    body_md_ru     text,
    line_count     integer GENERATED ALWAYS AS (
                       length(body_md) - length(replace(body_md, E'\n', '')) + 1
                   ) STORED,
    citations      jsonb NOT NULL DEFAULT '[]'::jsonb,
    coq_refs       jsonb NOT NULL DEFAULT '[]'::jsonb,
    qed_seal       text,
    updated_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS chapters_chapter_no_idx ON ssot.chapters (chapter_no);
CREATE INDEX IF NOT EXISTS chapters_kind_idx       ON ssot.chapters (kind);

-- ---------------------------------------------------------------------------
-- ssot.theorems — formal results carried by the monograph (R14 Coq citation map)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ssot.theorems (
    theorem_id     text PRIMARY KEY,
    chapter_slug   text NOT NULL REFERENCES ssot.chapters (chapter_slug) ON DELETE CASCADE,
    statement      text NOT NULL,
    statement_ru   text,
    proof_sketch   text,
    coq_file       text,
    coq_lemma      text,
    citation_no    integer,
    updated_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS theorems_chapter_idx    ON ssot.theorems (chapter_slug);
CREATE INDEX IF NOT EXISTS theorems_coq_lemma_idx  ON ssot.theorems (coq_lemma);

-- ---------------------------------------------------------------------------
-- ssot.chapter_figures — image registry (figure-registry.json mirror)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ssot.chapter_figures (
    chapter_slug      text PRIMARY KEY REFERENCES ssot.chapters (chapter_slug) ON DELETE CASCADE,
    figure_filename   text NOT NULL,
    figure_source_url text,
    alt_text          text NOT NULL,
    license           text NOT NULL DEFAULT 'CC-BY-4.0',
    sha256            text,
    width_px          integer,
    height_px         integer,
    updated_at        timestamptz NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------------------
-- ssot.one_shots — pre-registered ONE SHOT missions / falsification witnesses
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ssot.one_shots (
    one_shot_id    text PRIMARY KEY,
    chapter_slug   text REFERENCES ssot.chapters (chapter_slug) ON DELETE SET NULL,
    title          text NOT NULL,
    falsifier      text NOT NULL,
    accept_band    jsonb NOT NULL,
    reject_band    jsonb NOT NULL,
    status         text NOT NULL CHECK (status IN ('pre-registered','running','accepted','rejected','withdrawn')),
    seed_set       jsonb NOT NULL DEFAULT '[]'::jsonb,
    bpb_observed   numeric,
    artifact_uri   text,
    updated_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS one_shots_status_idx ON ssot.one_shots (status);

-- ---------------------------------------------------------------------------
-- View: ssot.coverage — at-a-glance progress for chapter army dispatch
-- ---------------------------------------------------------------------------
CREATE OR REPLACE VIEW ssot.coverage AS
SELECT
    c.chapter_slug,
    c.chapter_no,
    c.kind,
    c.title,
    c.line_count,
    (c.line_count >= 1500)        AS r3_full,
    (cf.figure_filename IS NOT NULL) AS has_figure,
    (SELECT count(*) FROM ssot.theorems t WHERE t.chapter_slug = c.chapter_slug) AS theorem_count,
    c.updated_at
FROM ssot.chapters c
LEFT JOIN ssot.chapter_figures cf USING (chapter_slug)
ORDER BY c.chapter_no;

-- ---------------------------------------------------------------------------
-- Anchor seal — every fresh DDL run must reaffirm the Trinity invariant.
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF abs((((1 + sqrt(5))/2)^2 + ((1 + sqrt(5))/2)^(-2)) - 3) > 1e-12 THEN
        RAISE EXCEPTION 'Trinity anchor violated: phi^2 + phi^-2 != 3';
    END IF;
END$$;
