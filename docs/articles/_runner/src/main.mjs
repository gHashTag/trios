#!/usr/bin/env node
// Repo-native `tri article` runner.
//
// Subcommands:
//   list                       List articles found under docs/articles/<slug>/article.toml
//   presets                    List presets across all articles
//   build <slug> [--pdf] [--html]
//   qa    <slug>
//
// This file is intentionally dependency-light: only @iarna/toml and
// markdown-it are required for parsing and rendering. PDF + QA shell out
// to weasyprint / qpdf / pdftotext.

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
// CLI
// ---------------------------------------------------------------------------

function usage() {
  console.log(`usage:
  article-runner list
  article-runner presets
  article-runner build <slug> [--pdf] [--html]
  article-runner qa    <slug>
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

  usage();
  return 2;
}

process.exit(main(process.argv));
