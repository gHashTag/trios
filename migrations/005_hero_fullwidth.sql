-- migrations/005_hero_fullwidth.sql
--
-- PhD v5 — guarantee that every chapter's body_md begins with a full-width
-- hero image referencing ssot.chapters.illustration_url (or .illustration_path
-- as fallback). Idempotent: re-running strips any pre-existing leading image
-- before re-prepending the canonical hero line.
--
-- After mutation, body_pdf_url / compile_ok / last_compiled are nulled so the
-- compile pipeline picks up every affected chapter on its next pass.
--
-- Schema reference (ssot.chapters):
--   ch_num text   — e.g. 'Ch.1', 'App.A', 'FA.07', 'FM.01', 'AP.A'
--   title  text
--   body_md text
--   body_pdf_url text
--   illustration_url text   — full https raw URL
--   illustration_path text  — repo-relative path fallback
--
-- Pairs with:
--   - templates/chapter.template.tex
--   - filters/force-fullwidth-hero.lua
--   - scripts/compile_chapter.sh

BEGIN;

WITH normalized AS (
  SELECT
    id,
    ch_num,
    title,
    illustration_url,
    illustration_path,
    -- Strip any existing leading Markdown image (with optional pandoc attrs).
    regexp_replace(
      COALESCE(body_md, ''),
      '^\s*!\[[^\]]*\]\([^\)]+\)(\{[^}]*\})?\s*\n+',
      '',
      ''
    ) AS body_no_hero
  FROM ssot.chapters
)
UPDATE ssot.chapters c
SET body_md =
      '![' || replace(COALESCE(n.title, ''), ']', '') || ']('
      || COALESCE(n.illustration_url, n.illustration_path)
      || '){width=100% .hero-fullwidth}'
      || E'\n\n'
      || n.body_no_hero,
    body_pdf_url   = NULL,   -- triggers recompile
    compile_ok     = NULL,
    last_compiled  = NULL,
    updated_at     = now()
FROM normalized n
WHERE c.id = n.id
  AND COALESCE(n.illustration_url, n.illustration_path) IS NOT NULL;

COMMIT;
