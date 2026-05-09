-- =====================================================================
-- 004_chapter_figures_v5_3.sql
-- PhD Monograph v5.3 — engraved-triptych figures on white #FFFFFF
-- 34 unique illustrations, one per chapter, generated 2026-05-09.
-- Apply after PR #627 merges to main (URLs settle on main branch).
-- Anchor: phi^2 + phi^-2 = 3  ·  DOI 10.5281/zenodo.19227877
-- Refs: trios#380, trios#265, trios#619
-- =====================================================================

BEGIN;

-- Idempotent UPSERT: keeps existing rows but rewrites figure pointers
-- for the 34 modern chapter slugs (00-monad..33-epilogue).

INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('00-monad', '00-monad.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/00-monad.png', 'Engraved scientific triptych on white #FFFFFF for 00-monad; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('01-golden-egg', '01-golden-egg.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/01-golden-egg.png', 'Engraved scientific triptych on white #FFFFFF for 01-golden-egg; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('02-golden-cut', '02-golden-cut.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/02-golden-cut.png', 'Engraved scientific triptych on white #FFFFFF for 02-golden-cut; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('03-golden-harvest', '03-golden-harvest.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/03-golden-harvest.png', 'Engraved scientific triptych on white #FFFFFF for 03-golden-harvest; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('04-golden-scales', '04-golden-scales.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/04-golden-scales.png', 'Engraved scientific triptych on white #FFFFFF for 04-golden-scales; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('05-golden-bridge', '05-golden-bridge.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/05-golden-bridge.png', 'Engraved scientific triptych on white #FFFFFF for 05-golden-bridge; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('06-golden-mantissa', '06-golden-mantissa.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/06-golden-mantissa.png', 'Engraved scientific triptych on white #FFFFFF for 06-golden-mantissa; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('07-golden-sprout', '07-golden-sprout.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/07-golden-sprout.png', 'Engraved scientific triptych on white #FFFFFF for 07-golden-sprout; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('08-golden-crystal', '08-golden-crystal.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/08-golden-crystal.png', 'Engraved scientific triptych on white #FFFFFF for 08-golden-crystal; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('09-golden-seal', '09-golden-seal.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/09-golden-seal.png', 'Engraved scientific triptych on white #FFFFFF for 09-golden-seal; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('10-golden-bloom', '10-golden-bloom.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/10-golden-bloom.png', 'Engraved scientific triptych on white #FFFFFF for 10-golden-bloom; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('11-vesica-piscis', '11-vesica-piscis.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/11-vesica-piscis.png', 'Engraved scientific triptych on white #FFFFFF for 11-vesica-piscis; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('12-flower-of-life', '12-flower-of-life.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/12-flower-of-life.png', 'Engraved scientific triptych on white #FFFFFF for 12-flower-of-life; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('13-metatron-cube', '13-metatron-cube.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/13-metatron-cube.png', 'Engraved scientific triptych on white #FFFFFF for 13-metatron-cube; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('14-platonic-solids', '14-platonic-solids.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/14-platonic-solids.png', 'Engraved scientific triptych on white #FFFFFF for 14-platonic-solids; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('15-kepler-solids', '15-kepler-solids.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/15-kepler-solids.png', 'Engraved scientific triptych on white #FFFFFF for 15-kepler-solids; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('16-sacred-ratios', '16-sacred-ratios.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/16-sacred-ratios.png', 'Engraved scientific triptych on white #FFFFFF for 16-sacred-ratios; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('17-golden-spiral', '17-golden-spiral.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/17-golden-spiral.png', 'Engraved scientific triptych on white #FFFFFF for 17-golden-spiral; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('18-torus-geometry', '18-torus-geometry.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/18-torus-geometry.png', 'Engraved scientific triptych on white #FFFFFF for 18-torus-geometry; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('19-fibonacci-tesselation', '19-fibonacci-tesselation.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/19-fibonacci-tesselation.png', 'Engraved scientific triptych on white #FFFFFF for 19-fibonacci-tesselation; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('20-standard-model', '20-standard-model.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/20-standard-model.png', 'Engraved scientific triptych on white #FFFFFF for 20-standard-model; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('21-quantum-field', '21-quantum-field.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/21-quantum-field.png', 'Engraved scientific triptych on white #FFFFFF for 21-quantum-field; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('22-e8-symmetry', '22-e8-symmetry.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/22-e8-symmetry.png', 'Engraved scientific triptych on white #FFFFFF for 22-e8-symmetry; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('23-gf16-algebra', '23-gf16-algebra.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/23-gf16-algebra.png', 'Engraved scientific triptych on white #FFFFFF for 23-gf16-algebra; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('24-igla-architecture', '24-igla-architecture.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/24-igla-architecture.png', 'Engraved scientific triptych on white #FFFFFF for 24-igla-architecture; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('25-benchmarks', '25-benchmarks.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/25-benchmarks.png', 'Engraved scientific triptych on white #FFFFFF for 25-benchmarks; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('26-data-analysis', '26-data-analysis.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/26-data-analysis.png', 'Engraved scientific triptych on white #FFFFFF for 26-data-analysis; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('27-trinity-identity', '27-trinity-identity.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/27-trinity-identity.png', 'Engraved scientific triptych on white #FFFFFF for 27-trinity-identity; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('28-momentum-algebra', '28-momentum-algebra.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/28-momentum-algebra.png', 'Engraved scientific triptych on white #FFFFFF for 28-momentum-algebra; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('29-lucas-closure', '29-lucas-closure.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/29-lucas-closure.png', 'Engraved scientific triptych on white #FFFFFF for 29-lucas-closure; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('30-golden-imagery', '30-golden-imagery.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/30-golden-imagery.png', 'Engraved scientific triptych on white #FFFFFF for 30-golden-imagery; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('31-philosophy', '31-philosophy.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/31-philosophy.png', 'Engraved scientific triptych on white #FFFFFF for 31-philosophy; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('32-conclusion', '32-conclusion.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/32-conclusion.png', 'Engraved scientific triptych on white #FFFFFF for 32-conclusion; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();
INSERT INTO ssot.chapter_figures (chapter_slug, figure_filename, figure_source_url, alt_text, license, updated_at)
VALUES ('33-epilogue', '33-epilogue.png', 'https://raw.githubusercontent.com/gHashTag/trios/main/assets/illustrations/33-epilogue.png', 'Engraved scientific triptych on white #FFFFFF for 33-epilogue; Trinity S3AI v5.3', 'CC-BY-4.0', now())
ON CONFLICT (chapter_slug) DO UPDATE SET
  figure_filename   = EXCLUDED.figure_filename,
  figure_source_url = EXCLUDED.figure_source_url,
  alt_text          = EXCLUDED.alt_text,
  license           = EXCLUDED.license,
  updated_at        = now();

COMMIT;

-- Verify: SELECT chapter_slug, figure_filename FROM ssot.chapter_figures WHERE chapter_slug LIKE '__-%' ORDER BY chapter_slug;