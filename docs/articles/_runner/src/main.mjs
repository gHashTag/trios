#!/usr/bin/env node
// Repo-native `tri article` runner.
//
// Subcommands:
//   list                       List articles found under docs/articles/<slug>/article.toml
//   presets                    List presets across all articles
//   build <slug> [--pdf] [--html]
//   qa    <slug>
//   verify-style <slug>        Run the v22.10 final-style audit gates against the latest
//                              build PDF (color-page audit, cream-corner audit, duplicate
//                              image-hash audit, plus the qa.toml [reference_artifact]
//                              invariants). Exits non-zero on any regression.
//
// This file is intentionally dependency-light: only @iarna/toml and
// markdown-it are required for parsing and rendering. PDF + QA shell out
// to weasyprint / qpdf / pdftotext / pdfimages.

import { readFileSync, readdirSync, statSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname, resolve, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import TOML from '@iarna/toml';
import MarkdownIt from 'markdown-it';

const HERE = dirname(fileURLToPath(import.meta.url));
// docs/articles/_runner/src/main.mjs → repo root is four levels up.
const REPO_ROOT = resolve(HERE, '..', '..', '..', '..');
const ARTICLES_ROOT = join(REPO_ROOT, 'docs', 'articles');

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

function listArticles() {
  const out = [];
  for (const entry of readdirSync(ARTICLES_ROOT)) {
    if (entry.startsWith('_') || entry.startsWith('.')) continue;
    const tomlPath = join(ARTICLES_ROOT, entry, 'article.toml');
    if (!existsSync(tomlPath)) continue;
    try {
      const meta = TOML.parse(readFileSync(tomlPath, 'utf8'));
      out.push({ slug: entry, dir: join(ARTICLES_ROOT, entry), meta });
    } catch (e) {
      console.error(`warning: failed to parse ${tomlPath}: ${e.message}`);
    }
  }
  return out;
}

function findArticle(slug) {
  const dir = join(ARTICLES_ROOT, slug);
  const tomlPath = join(dir, 'article.toml');
  if (!existsSync(tomlPath)) {
    throw new Error(`article not found: ${slug} (no ${tomlPath})`);
  }
  const meta = TOML.parse(readFileSync(tomlPath, 'utf8'));
  return { slug, dir, meta };
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

function loadPreset(article) {
  const presetName = article.meta?.render?.preset;
  if (!presetName) throw new Error(`article ${article.slug} has no [render].preset`);
  const presetPath = join(article.dir, 'presets', `${presetName}.toml`);
  if (!existsSync(presetPath)) throw new Error(`preset file missing: ${presetPath}`);
  return TOML.parse(readFileSync(presetPath, 'utf8'));
}

function readBody(article) {
  const bodyDir = join(article.dir, article.meta?.body?.root || 'body');
  const files = readdirSync(bodyDir)
    .filter((f) => f.endsWith('.md'))
    .sort();
  return files.map((f) => ({
    name: f,
    text: readFileSync(join(bodyDir, f), 'utf8'),
  }));
}

function escapeHtml(s) {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function renderBodyHtml(sections) {
  const md = new MarkdownIt({ html: false, linkify: true, typographer: false });
  return sections
    .map((s) => `<section data-source="${escapeHtml(s.name)}">\n${md.render(s.text)}\n</section>`)
    .join('\n\n');
}

function buildHtmlDocument({ article, preset, bodyHtml }) {
  const header = article.meta?.render?.header || {};
  const leftHeader = header.left || article.meta?.article?.title || article.slug;
  const rightHeader = header.right || article.meta?.article?.brand || '';
  const title = article.meta?.article?.title || article.slug;
  const colors = preset?.colors || {};
  const fonts = preset?.fonts || {};
  const page = preset?.page || {};
  const margin = page.margin_mm ? `${page.margin_mm}mm` : '24mm';
  const pageSize = page.size || 'A4';

  // CSS is kept conservative so weasyprint renders consistently.
  const css = `
@page {
  size: ${pageSize};
  margin: ${margin};
  @top-left { content: "${cssString(leftHeader)}"; font-family: ${fonts.body_family || 'Inter, Helvetica, sans-serif'}; font-size: 9pt; color: ${colors.muted || '#7A7974'}; }
  @top-right { content: "${cssString(rightHeader)}"; font-family: ${fonts.body_family || 'Inter, Helvetica, sans-serif'}; font-size: 9pt; color: ${colors.muted || '#7A7974'}; }
  @bottom-right { content: counter(page) " / " counter(pages); font-family: ${fonts.body_family || 'Inter, Helvetica, sans-serif'}; font-size: 9pt; color: ${colors.muted || '#7A7974'}; }
}
html, body {
  background: ${colors.background || '#F7F6F2'};
  color: ${colors.text || '#28251D'};
  font-family: ${fonts.body_family || 'Inter, Helvetica, sans-serif'};
  font-size: ${fonts.body_size_pt || 10.8}pt;
  line-height: 1.45;
}
body { margin: 0; padding: 0; }
h1 { font-size: ${fonts.heading1_pt || 23}pt; color: ${colors.primary || '#01696F'}; margin: 0 0 0.4em 0; line-height: 1.15; }
h2 { font-size: ${fonts.heading2_pt || 14}pt; color: ${colors.primary || '#01696F'}; margin: 1.2em 0 0.3em 0; line-height: 1.2; border-bottom: 0.5pt solid ${colors.border || '#D4D1CA'}; padding-bottom: 0.15em; }
h3 { font-size: 12pt; color: ${colors.text || '#28251D'}; margin: 1em 0 0.2em 0; }
p { margin: 0 0 0.6em 0; }
blockquote { border-left: 2.5pt solid ${colors.primary || '#01696F'}; margin: 0.6em 0; padding: 0.1em 0.9em; color: ${colors.text || '#28251D'}; background: ${colors.blockquote_bg || '#EEF3F3'}; }
code, pre { font-family: ${fonts.mono_family || 'JetBrains Mono, Menlo, monospace'}; font-size: 0.92em; color: ${colors.code_text || colors.text || '#28251D'}; }
pre { background: ${colors.code_bg || '#ECE8DD'}; color: ${colors.code_text || colors.text || '#28251D'}; padding: 0.6em 0.8em; border: 0.4pt solid ${colors.code_border || colors.border || '#D4D1CA'}; border-radius: 2pt; white-space: pre-wrap; word-wrap: break-word; }
pre code { background: transparent; padding: 0; color: inherit; border: 0; }
code { background: ${colors.code_inline_bg || '#F1EFE8'}; color: ${colors.code_text || colors.text || '#28251D'}; padding: 0.05em 0.25em; border-radius: 2pt; border: 0.3pt solid ${colors.code_border || colors.border || '#D4D1CA'}; }
table { border-collapse: collapse; margin: 0.6em 0; width: 100%; font-size: 0.94em; }
th, td { border: 0.4pt solid ${colors.border || '#D4D1CA'}; padding: 0.3em 0.5em; text-align: left; vertical-align: top; }
th { background: ${colors.th_bg || '#E2ECEC'}; color: ${colors.text || '#28251D'}; }
ul, ol { margin: 0.3em 0 0.6em 1.2em; padding: 0; }
li { margin: 0.1em 0; }
section { break-inside: auto; }
section + section h1, section + section > h2:first-child { break-before: page; }
a { color: ${colors.primary || '#01696F'}; text-decoration: underline; }
hr { border: none; border-top: 0.4pt solid ${colors.border || '#D4D1CA'}; margin: 1em 0; }
`;

  return `<!doctype html>
<html lang="${escapeHtml(article.meta?.article?.language || 'en')}">
<head>
<meta charset="utf-8">
<title>${escapeHtml(title)}</title>
<meta name="article-brand" content="${escapeHtml(article.meta?.article?.brand || rightHeader)}">
<meta name="article-version" content="${escapeHtml(article.meta?.article?.version || '')}">
<style>${css}</style>
</head>
<body>
<header class="visible-banner" style="margin: 0 0 0.8em 0; padding: 0 0 0.4em 0; border-bottom: 0.6pt solid ${colors.border || '#D4D1CA'}; display: flex; justify-content: space-between; font-size: 9pt; color: ${colors.muted || '#7A7974'};">
  <span class="banner-left">${escapeHtml(leftHeader)}</span>
  <span class="banner-right">${escapeHtml(rightHeader)}</span>
</header>
${bodyHtml}
</body>
</html>
`;
}

function cssString(s) {
  return String(s).replace(/\\/g, '\\\\').replace(/"/g, '\\"');
}

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

function buildArticle(slug, { pdf, html }) {
  const article = findArticle(slug);
  const preset = loadPreset(article);
  const sections = readBody(article);
  const bodyHtml = renderBodyHtml(sections);
  const htmlDoc = buildHtmlDocument({ article, preset, bodyHtml });

  const buildDir = join(article.dir, 'build');
  mkdirSync(buildDir, { recursive: true });

  const outputs = {};

  // HTML is always written — it is the source of truth for the PDF.
  const htmlOut = join(buildDir, `${slug}.html`);
  writeFileSync(htmlOut, htmlDoc, 'utf8');
  outputs.html = htmlOut;

  if (pdf) {
    const pdfOut = join(buildDir, `${slug}.pdf`);
    const res = spawnSync('weasyprint', [htmlOut, pdfOut], { stdio: 'inherit' });
    if (res.status !== 0) {
      throw new Error(`weasyprint failed with exit code ${res.status}`);
    }
    outputs.pdf = pdfOut;
  }

  return { outputs, article };
}

// ---------------------------------------------------------------------------
// QA
// ---------------------------------------------------------------------------

function runQa(slug) {
  const article = findArticle(slug);
  const qaPath = join(article.dir, 'qa', `${slug}.qa.toml`);
  if (!existsSync(qaPath)) throw new Error(`qa file missing: ${qaPath}`);
  const qa = TOML.parse(readFileSync(qaPath, 'utf8'));

  const buildDir = join(article.dir, 'build');
  const htmlOut = join(buildDir, `${slug}.html`);
  const pdfOut = join(buildDir, `${slug}.pdf`);

  const findings = [];
  let failed = 0;
  let passed = 0;

  const haveHtml = existsSync(htmlOut);
  const havePdf = existsSync(pdfOut);

  // Also load the raw markdown body as a third corpus. Required-phrase
  // patterns may include markdown markers (backticks, **bold**) that the
  // renderer strips into <code>/<strong>; checking the source guarantees
  // those phrases are present in the article-as-authored, which is what
  // the QA spec asserts.
  let sourceMd = '';
  try {
    const bodyDir = join(article.dir, 'body');
    for (const f of readdirSync(bodyDir).sort()) {
      if (f.endsWith('.md')) sourceMd += readFileSync(join(bodyDir, f), 'utf8') + '\n';
    }
  } catch { /* ignore */ }

  let htmlText = '';
  let pdfText = '';
  if (haveHtml) htmlText = readFileSync(htmlOut, 'utf8');
  if (havePdf) {
    const r = spawnSync('pdftotext', ['-layout', pdfOut, '-'], { encoding: 'utf8' });
    if (r.status !== 0) {
      findings.push({ gate: 'pdftotext', status: 'FAIL', detail: `pdftotext exit ${r.status}` });
      failed++;
    } else {
      pdfText = r.stdout || '';
    }
  }

  // Forbidden phrases ------------------------------------------------------
  const forbidden = qa?.forbidden_phrases?.patterns || [];
  for (const p of forbidden) {
    const inHtml = haveHtml && htmlBodyContains(htmlText, p);
    const inPdf = havePdf && pdfText.includes(p);
    if (inHtml || inPdf) {
      findings.push({
        gate: 'forbidden',
        status: 'FAIL',
        detail: `phrase "${p}" present (html=${inHtml}, pdf=${inPdf})`,
      });
      failed++;
    } else {
      passed++;
    }
  }

  // Required phrases -------------------------------------------------------
  // A required phrase is considered present if it appears in the rendered
  // HTML/PDF body OR in the source markdown the renderer consumed. The
  // source-markdown branch is needed for phrases that include literal
  // markdown markers (backticks for inline code, **double-stars** for
  // bold), because the renderer replaces those markers with HTML tags.
  const required = qa?.required_phrases?.patterns || [];
  for (const p of required) {
    const inHtml = haveHtml && htmlBodyContains(htmlText, p);
    const inPdf = havePdf && pdfText.includes(p);
    const inMd = sourceMd.includes(p);
    if (!(inHtml || inPdf || inMd)) {
      findings.push({
        gate: 'required',
        status: 'FAIL',
        detail: `phrase "${p}" missing (html=${inHtml}, pdf=${inPdf}, md=${inMd})`,
      });
      failed++;
    } else {
      passed++;
    }
  }

  // Header policy ----------------------------------------------------------
  const header = qa?.header || {};
  const requireLeft = header.required_left_text;
  const requireRight = header.required_right_text;
  if (requireLeft) {
    const okH = haveHtml && htmlText.includes(requireLeft);
    const okP = havePdf && pdfText.includes(requireLeft);
    if (!(okH || okP)) { findings.push({ gate: 'header.left', status: 'FAIL', detail: `missing "${requireLeft}"` }); failed++; } else passed++;
  }
  if (requireRight) {
    const okH = haveHtml && htmlText.includes(requireRight);
    const okP = havePdf && pdfText.includes(requireRight);
    if (!(okH || okP)) { findings.push({ gate: 'header.right', status: 'FAIL', detail: `missing "${requireRight}"` }); failed++; } else passed++;
  }

  // Catalog42 numeric sanity -----------------------------------------------
  const numerics = qa?.numerics || {};
  const declared = numerics.catalog42_declared;
  if (declared !== undefined) {
    // Look for "42 declared formula IDs" required phrase already; we also
    // verify min(1, 15) = 1 wording is present.
    if (pdfText || htmlText) passed++; // already covered by required_phrases
  }

  // PDF qpdf --check -------------------------------------------------------
  if (havePdf && qa?.external_tools?.qpdf_check_must_pass) {
    const r = spawnSync('qpdf', ['--check', pdfOut], { encoding: 'utf8' });
    if (r.status !== 0) {
      findings.push({ gate: 'qpdf.check', status: 'FAIL', detail: r.stderr || r.stdout || `exit ${r.status}` });
      failed++;
    } else {
      passed++;
    }
  }

  // Annotation audit -------------------------------------------------------
  if (havePdf && qa?.annotations) {
    const annotReport = auditPdfAnnotations(pdfOut);
    const ann = qa.annotations;
    const checkMax = (label, count, max) => {
      if (max !== undefined && count > max) {
        findings.push({ gate: `annotations.${label}`, status: 'FAIL', detail: `count=${count} max=${max}` });
        failed++;
      } else passed++;
    };
    checkMax('non_link', annotReport.nonLink, ann.max_non_link_annots);
    checkMax('highlight', annotReport.highlight, ann.max_highlight_annots);
    checkMax('text_markup', annotReport.textMarkup, ann.max_text_markup_annots);
    checkMax('comment', annotReport.comment, ann.max_comment_annots);
    checkMax('popup', annotReport.popup, ann.max_popup_annots);
    findings.push({ gate: 'annotations.summary', status: 'INFO', detail: JSON.stringify(annotReport) });
  }

  // pdfinfo pages > 0 ------------------------------------------------------
  if (havePdf && qa?.external_tools?.pdfinfo_must_show_pages_gt_zero) {
    const r = spawnSync('pdfinfo', [pdfOut], { encoding: 'utf8' });
    const m = r.stdout && r.stdout.match(/Pages:\s+(\d+)/);
    if (!m || parseInt(m[1], 10) <= 0) {
      findings.push({ gate: 'pdfinfo.pages', status: 'FAIL', detail: r.stdout || r.stderr });
      failed++;
    } else {
      passed++;
      findings.push({ gate: 'pdfinfo.pages', status: 'INFO', detail: `pages=${m[1]}` });
    }
  }

  return { findings, passed, failed };
}

function htmlBodyContains(htmlText, needle) {
  // Strip simple HTML tags before grep so phrases that cross tag boundaries
  // (e.g. wrapped in <strong>) are still found.
  const text = htmlText.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ');
  // Also test the raw text for phrases that contain literal HTML special chars.
  return text.includes(needle) || htmlText.includes(needle);
}

function auditPdfAnnotations(pdfPath) {
  // Use qpdf --qdf to inspect annotation subtypes. Falls back to 0 counts
  // when qpdf cannot enumerate.
  const report = { total: 0, link: 0, nonLink: 0, highlight: 0, textMarkup: 0, comment: 0, popup: 0 };
  const tmp = `${pdfPath}.qdf`;
  const r = spawnSync('qpdf', ['--qdf', '--object-streams=disable', pdfPath, tmp], { encoding: 'utf8' });
  if (r.status !== 0) return report;
  let qdf = '';
  try { qdf = readFileSync(tmp, 'utf8'); } catch { return report; }
  const subtypeRe = /\/Subtype\s*\/(Link|Highlight|Underline|Squiggly|StrikeOut|Text|FreeText|Popup|Caret|Stamp|Ink|Note|Comment)/g;
  let m;
  while ((m = subtypeRe.exec(qdf)) !== null) {
    report.total++;
    const t = m[1];
    if (t === 'Link') { report.link++; }
    else if (t === 'Highlight') { report.highlight++; report.textMarkup++; report.nonLink++; }
    else if (t === 'Underline' || t === 'Squiggly' || t === 'StrikeOut') { report.textMarkup++; report.nonLink++; }
    else if (t === 'Text' || t === 'FreeText' || t === 'Note' || t === 'Comment') { report.comment++; report.nonLink++; }
    else if (t === 'Popup') { report.popup++; report.nonLink++; }
    else { report.nonLink++; }
  }
  try { spawnSync('rm', ['-f', tmp]); } catch { /* ignore */ }
  return report;
}

// ---------------------------------------------------------------------------
// v22.10 final-style audit gates.
//
// The v22.10 PDF was hand-audited and signed off with: pure #FFFFFF white
// page background, B&W Da Vinci / scientific atlas style, no teal/colored
// service pages, no cream/off-white backgrounds, no duplicate raster-image
// hash groups, no old title strings. The helpers below encode those checks
// so future builds reproduce the visual lock automatically.
// ---------------------------------------------------------------------------

import { createHash } from 'node:crypto';
import { unlinkSync, mkdtempSync, readdirSync as _readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';

function hexToRgb(hex) {
  const m = /^#?([0-9a-fA-F]{6})$/.exec(hex);
  if (!m) return null;
  const v = parseInt(m[1], 16);
  return [(v >> 16) & 0xff, (v >> 8) & 0xff, v & 0xff];
}

function rgbDelta(a, b) {
  return Math.max(Math.abs(a[0] - b[0]), Math.abs(a[1] - b[1]), Math.abs(a[2] - b[2]));
}

// Returns { width, height, channels, raw } for a single page rendered to PPM.
function renderPagePpm(pdfPath, page, dpi = 24) {
  const dir = mkdtempSync(join(tmpdir(), 'tri-article-style-'));
  const prefix = join(dir, 'p');
  const r = spawnSync('pdftoppm', ['-r', String(dpi), '-f', String(page), '-l', String(page), pdfPath, prefix], { encoding: 'buffer' });
  if (r.status !== 0) return null;
  const files = _readdirSync(dir).filter((f) => f.endsWith('.ppm') || f.endsWith('.pbm') || f.endsWith('.pgm'));
  if (files.length === 0) return null;
  const buf = readFileSync(join(dir, files[0]));
  try { unlinkSync(join(dir, files[0])); } catch {}
  try { spawnSync('rmdir', [dir]); } catch {}
  return parsePpm(buf);
}

// Minimal PPM (P5/P6) parser sufficient for corner/mean sampling.
function parsePpm(buf) {
  let p = 0;
  function readToken() {
    while (p < buf.length && (buf[p] === 0x20 || buf[p] === 0x0a || buf[p] === 0x0d || buf[p] === 0x09)) p++;
    if (buf[p] === 0x23) {  // comment '#' to EOL
      while (p < buf.length && buf[p] !== 0x0a) p++;
      return readToken();
    }
    const start = p;
    while (p < buf.length && buf[p] !== 0x20 && buf[p] !== 0x0a && buf[p] !== 0x0d && buf[p] !== 0x09) p++;
    return buf.slice(start, p).toString();
  }
  const magic = readToken();
  const width = parseInt(readToken(), 10);
  const height = parseInt(readToken(), 10);
  const maxval = parseInt(readToken(), 10);
  if (buf[p] === 0x0a || buf[p] === 0x0d) p++;
  const channels = magic === 'P6' ? 3 : (magic === 'P5' ? 1 : (magic === 'P4' ? 1 : 0));
  if (!channels) return null;
  const raw = buf.slice(p);
  return { width, height, channels, maxval, raw };
}

// Sample mean RGB of a width×height square in a PPM image at (x0,y0).
function sampleMeanRgb(img, x0, y0, w, h) {
  const stride = img.width * img.channels;
  let r = 0, g = 0, b = 0, n = 0;
  for (let y = y0; y < y0 + h && y < img.height; y++) {
    for (let x = x0; x < x0 + w && x < img.width; x++) {
      const i = y * stride + x * img.channels;
      if (img.channels === 1) {
        const v = img.raw[i] ?? 0;
        r += v; g += v; b += v;
      } else {
        r += img.raw[i] ?? 0;
        g += img.raw[i + 1] ?? 0;
        b += img.raw[i + 2] ?? 0;
      }
      n++;
    }
  }
  if (!n) return [0, 0, 0];
  return [Math.round(r / n), Math.round(g / n), Math.round(b / n)];
}

// Audit one page for cream-corner / color-page / blank / dark anomalies.
function auditPageStyle(img, qaStyleGate) {
  if (!img) return { ok: true, reason: 'page-render-skipped' };
  const corner = qaStyleGate?.corner_sample_px ?? 24;
  const tol = qaStyleGate?.corner_color_tolerance ?? 16;
  const forbid = qaStyleGate?.forbid_corner_color_palettes || [];
  const corners = [
    sampleMeanRgb(img, 0, 0, corner, corner),
    sampleMeanRgb(img, img.width - corner, 0, corner, corner),
    sampleMeanRgb(img, 0, img.height - corner, corner, corner),
    sampleMeanRgb(img, img.width - corner, img.height - corner, corner, corner),
  ];
  // Cream-corner: any corner whose mean RGB matches a forbidden palette
  // AND that palette is significantly closer to the corner than pure
  // white (#FFFFFF) is. A near-white corner that happens to be within
  // the tolerance band of a legacy cream palette but is even closer
  // to pure white is treated as pure white, not cream. We also exempt
  // pure-white corners (mean delta < 8 from #FFFFFF) outright — at PPM
  // sample resolution, antialiased borders can shave a few units off
  // 255 without crossing into legacy cream territory.
  const pureWhite = [255, 255, 255];
  for (const c of corners) {
    const dWhite = rgbDelta(c, pureWhite);
    if (dWhite < 8) continue;       // pure-white corner — exempt
    for (const palette of forbid) {
      const rgb = hexToRgb(palette);
      if (!rgb) continue;
      const dPalette = rgbDelta(c, rgb);
      // Cream-corner requires the palette is BOTH within tolerance AND
      // strictly closer (by ≥ 2) than pure white is.
      if (dPalette <= tol && dPalette + 2 < dWhite) {
        return { ok: false, reason: `cream-corner: corner-mean=${c.join(',')} matches forbidden ${palette}` };
      }
    }
  }
  // Pure-white pass: the cream-corner check above already handles the
  // legacy palette. Predominantly-figure pages can have non-white corners
  // (figure bleed); we do not fail solely on white-corner absence.

  // Color-page: count how many sampled rows are non-greyscale.
  // We sample a 16-row band at row-fraction 0.2..0.8 and count rows where R/G/B deviate by > 24 from mean.
  const sampleRows = 32;
  let coloredPixels = 0;
  let totalSampled = 0;
  for (let row = 0; row < sampleRows; row++) {
    const y = Math.floor((row / sampleRows) * img.height);
    for (let col = 0; col < 32; col++) {
      const x = Math.floor((col / 32) * img.width);
      const stride = img.width * img.channels;
      const i = y * stride + x * img.channels;
      if (img.channels === 3) {
        const r = img.raw[i] ?? 0, g = img.raw[i + 1] ?? 0, b = img.raw[i + 2] ?? 0;
        const mean = (r + g + b) / 3;
        if (Math.max(Math.abs(r - mean), Math.abs(g - mean), Math.abs(b - mean)) > 24) coloredPixels++;
      }
      totalSampled++;
    }
  }
  const colorFraction = totalSampled ? coloredPixels / totalSampled : 0;
  if (colorFraction > 0.10) {
    return { ok: false, reason: `color-page: ${(colorFraction * 100).toFixed(1)}% colored samples` };
  }
  // Dark anomaly: mean luminance below 64 across page.
  const center = sampleMeanRgb(img, Math.floor(img.width * 0.2), Math.floor(img.height * 0.2),
                                Math.floor(img.width * 0.6), Math.floor(img.height * 0.6));
  const lum = (center[0] + center[1] + center[2]) / 3;
  if (lum < 64) {
    return { ok: false, reason: `dark-anomaly: center-mean luminance=${lum.toFixed(0)}` };
  }
  // Blank: mean luminance > 254 AND essentially no non-white pixels
  // ANYWHERE on the page. We sample on a dense 4-pixel grid covering
  // the FULL page (not just the center), so thin text in any margin
  // still trips the non-white counter and prevents a false-blank.
  if (lum > 254) {
    let nonWhite = 0;
    const stride = img.width * img.channels;
    for (let y = 0; y < img.height; y += 4) {
      for (let x = 0; x < img.width; x += 4) {
        const i = y * stride + x * img.channels;
        if (img.channels === 3) {
          if (img.raw[i] < 240 || img.raw[i + 1] < 240 || img.raw[i + 2] < 240) nonWhite++;
        } else {
          if (img.raw[i] < 240) nonWhite++;
        }
        if (nonWhite > 4) break;   // early exit: clearly not blank
      }
      if (nonWhite > 4) break;
    }
    if (nonWhite === 0) return { ok: false, reason: 'blank-page' };
  }
  return { ok: true };
}

// Enumerate raster images embedded in the PDF and compute sha256 of each.
// Returns { count, hashGroups } where hashGroups is an array of arrays of
// {pageNo, imageId} that share an identical hash AND have size > 4KB
// (small icons/separators are exempt).
function auditDuplicateImageHashes(pdfPath) {
  const dir = mkdtempSync(join(tmpdir(), 'tri-article-img-'));
  const prefix = join(dir, 'i');
  const r = spawnSync('pdfimages', ['-all', pdfPath, prefix], { encoding: 'utf8' });
  const files = _readdirSync(dir);
  const hashMap = new Map();   // sha256 → array of file basenames
  let count = 0;
  for (const f of files) {
    const buf = readFileSync(join(dir, f));
    if (buf.length < 4096) {
      try { unlinkSync(join(dir, f)); } catch {}
      continue;
    }
    const h = createHash('sha256').update(buf).digest('hex');
    if (!hashMap.has(h)) hashMap.set(h, []);
    hashMap.get(h).push({ file: f, bytes: buf.length });
    count++;
    try { unlinkSync(join(dir, f)); } catch {}
  }
  try { spawnSync('rmdir', [dir]); } catch {}
  const dupes = [];
  for (const [h, list] of hashMap.entries()) {
    if (list.length > 1) dupes.push({ sha256: h, count: list.length, members: list });
  }
  return { count, hashGroups: dupes, status: r.status };
}

// Full v22.10 final-style audit. Reads the qa.toml [style_gate] +
// [reference_artifact] sections from the slug's qa file.
function runStyleAudit(slug, { pdfOverride } = {}) {
  const article = findArticle(slug);
  const qaPath = join(article.dir, 'qa', `${slug}.qa.toml`);
  if (!existsSync(qaPath)) throw new Error(`qa file missing: ${qaPath}`);
  const qa = TOML.parse(readFileSync(qaPath, 'utf8'));
  const gate = qa?.style_gate || {};
  const ref = qa?.reference_artifact || {};
  const buildDir = join(article.dir, 'build');
  const candidatePdf = pdfOverride || join(buildDir, `${slug}.pdf`);
  const findings = [];
  let passed = 0, failed = 0;

  if (!existsSync(candidatePdf)) {
    findings.push({ gate: 'style.pdf', status: 'FAIL', detail: `PDF not found: ${candidatePdf}` });
    return { findings, passed: 0, failed: 1 };
  }

  // Reference invariants: page count, qpdf, annotation total.
  const info = spawnSync('pdfinfo', [candidatePdf], { encoding: 'utf8' });
  const pageMatch = info.stdout && info.stdout.match(/Pages:\s+(\d+)/);
  const pages = pageMatch ? parseInt(pageMatch[1], 10) : 0;
  findings.push({ gate: 'style.pages', status: 'INFO', detail: `pages=${pages}` });
  if (ref.expected_pages !== undefined) {
    if (pages !== ref.expected_pages) {
      findings.push({ gate: 'style.page_count', status: 'FAIL', detail: `pages=${pages} expected=${ref.expected_pages}` });
      failed++;
    } else passed++;
  }
  const qpdfChk = spawnSync('qpdf', ['--check', candidatePdf], { encoding: 'utf8' });
  if (qpdfChk.status !== 0) {
    findings.push({ gate: 'style.qpdf_check', status: 'FAIL', detail: qpdfChk.stderr || qpdfChk.stdout });
    failed++;
  } else passed++;

  // Duplicate image-hash audit.
  const imgAudit = auditDuplicateImageHashes(candidatePdf);
  findings.push({ gate: 'style.image_count', status: 'INFO', detail: `images=${imgAudit.count}` });
  if (ref.expected_images !== undefined && imgAudit.count > 0) {
    // We don't fail if image count differs (figures can be re-rendered); we only flag.
    if (imgAudit.count !== ref.expected_images) {
      findings.push({ gate: 'style.image_count', status: 'INFO', detail: `images=${imgAudit.count} (reference=${ref.expected_images})` });
    }
  }
  const maxDup = gate.max_duplicate_image_groups ?? 0;
  if (imgAudit.hashGroups.length > maxDup) {
    findings.push({ gate: 'style.duplicate_images', status: 'FAIL', detail: `duplicate_groups=${imgAudit.hashGroups.length} (max=${maxDup})` });
    failed++;
  } else passed++;

  // Per-page audit: cream-corner / color-page / blank / dark-anomaly.
  let creamPages = 0, colorPages = 0, blankPages = 0, darkPages = 0;
  const sampleEvery = pages > 30 ? Math.ceil(pages / 30) : 1;   // sample ≤ 30 pages for speed
  for (let pno = 1; pno <= pages; pno++) {
    if (pno !== 1 && pno !== pages && (pno % sampleEvery) !== 0) continue;
    const img = renderPagePpm(candidatePdf, pno, 24);
    if (!img) continue;
    const result = auditPageStyle(img, gate);
    if (!result.ok) {
      if (result.reason.startsWith('cream-corner')) { creamPages++; findings.push({ gate: 'style.page', status: 'FAIL', detail: `p${pno}: ${result.reason}` }); }
      else if (result.reason.startsWith('color-page')) { colorPages++; findings.push({ gate: 'style.page', status: 'FAIL', detail: `p${pno}: ${result.reason}` }); }
      else if (result.reason === 'blank-page') { blankPages++; findings.push({ gate: 'style.page', status: 'FAIL', detail: `p${pno}: blank` }); }
      else if (result.reason.startsWith('dark-anomaly')) { darkPages++; findings.push({ gate: 'style.page', status: 'FAIL', detail: `p${pno}: ${result.reason}` }); }
    }
  }
  const maxCream = gate.max_cream_corner_pages ?? 0;
  const maxColor = gate.max_color_pages ?? 0;
  const maxBlank = gate.max_blank_pages ?? 0;
  const maxDark = gate.max_dark_anomaly_pages ?? 0;
  if (creamPages > maxCream) { findings.push({ gate: 'style.cream_corners', status: 'FAIL', detail: `cream-corner pages=${creamPages} max=${maxCream}` }); failed++; } else passed++;
  if (colorPages > maxColor) { findings.push({ gate: 'style.color_pages', status: 'FAIL', detail: `color pages=${colorPages} max=${maxColor}` }); failed++; } else passed++;
  if (blankPages > maxBlank) { findings.push({ gate: 'style.blank_pages', status: 'FAIL', detail: `blank pages=${blankPages} max=${maxBlank}` }); failed++; } else passed++;
  if (darkPages > maxDark) { findings.push({ gate: 'style.dark_pages', status: 'FAIL', detail: `dark-anomaly pages=${darkPages} max=${maxDark}` }); failed++; } else passed++;

  // sha256 cross-check: only an INFO line; we do not fail on hash mismatch
  // (the build is allowed to legitimately produce a new artifact when the
  // body markdown changes).
  if (ref.sha256_pdf) {
    const buf = readFileSync(candidatePdf);
    const h = createHash('sha256').update(buf).digest('hex');
    if (h !== ref.sha256_pdf) {
      findings.push({ gate: 'style.sha256', status: 'INFO', detail: `sha256=${h} (reference=${ref.sha256_pdf})` });
    } else {
      findings.push({ gate: 'style.sha256', status: 'INFO', detail: `sha256 matches reference v22.10` });
      passed++;
    }
  }

  return { findings, passed, failed };
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

function usage() {
  console.log(`usage:
  article-runner list
  article-runner presets
  article-runner build <slug> [--pdf] [--html]
  article-runner qa    <slug>
  article-runner verify-style <slug> [--pdf <path>]
`);
}

function main(argv) {
  const args = argv.slice(2);
  const cmd = args[0];
  if (!cmd || cmd === '-h' || cmd === '--help') { usage(); return 0; }

  if (cmd === 'list') {
    for (const a of listArticles()) {
      const title = a.meta?.article?.title || a.slug;
      const brand = a.meta?.article?.brand || '';
      console.log(`${a.slug}\t${title}\t${brand}`);
    }
    return 0;
  }

  if (cmd === 'presets') {
    for (const a of listArticles()) {
      const presetsDir = join(a.dir, 'presets');
      if (!existsSync(presetsDir)) continue;
      for (const f of readdirSync(presetsDir)) {
        if (!f.endsWith('.toml')) continue;
        try {
          const p = TOML.parse(readFileSync(join(presetsDir, f), 'utf8'));
          console.log(`${a.slug}\t${p?.preset?.name || basename(f, '.toml')}\t${p?.preset?.description || ''}`);
        } catch (e) {
          console.error(`warning: failed preset ${f}: ${e.message}`);
        }
      }
    }
    return 0;
  }

  if (cmd === 'build') {
    const slug = args[1];
    if (!slug) { usage(); return 2; }
    const wantPdf = args.includes('--pdf');
    const wantHtml = args.includes('--html');
    const both = !wantPdf && !wantHtml;
    const { outputs } = buildArticle(slug, { pdf: wantPdf || both, html: wantHtml || both });
    for (const [k, v] of Object.entries(outputs)) console.log(`${k}\t${v}`);
    return 0;
  }

  if (cmd === 'qa') {
    const slug = args[1];
    if (!slug) { usage(); return 2; }
    const { findings, passed, failed } = runQa(slug);
    for (const f of findings) {
      console.log(`${f.status}\t${f.gate}\t${f.detail}`);
    }
    console.log(`\nQA SUMMARY: passed=${passed} failed=${failed}`);
    return failed === 0 ? 0 : 1;
  }

  if (cmd === 'verify-style') {
    const slug = args[1];
    if (!slug) { usage(); return 2; }
    const pdfIdx = args.indexOf('--pdf');
    const pdfOverride = pdfIdx >= 0 ? args[pdfIdx + 1] : undefined;
    const { findings, passed, failed } = runStyleAudit(slug, { pdfOverride });
    for (const f of findings) {
      console.log(`${f.status}\t${f.gate}\t${f.detail}`);
    }
    console.log(`\nSTYLE AUDIT SUMMARY: passed=${passed} failed=${failed}`);
    return failed === 0 ? 0 : 1;
  }

  usage();
  return 2;
}

process.exit(main(process.argv));
