#!/usr/bin/env -S npx tsx
/**
 * scripts/matrix_coverage_report.ts — coverage report for the
 * 351-cell Format×Algorithm matrix (gHashTag/trios#446).
 *
 * Sources, in priority order:
 *   1. assertions/matrix_samples.jsonl (local JSONL produced by the runner)
 *   2. fallback: stdin if --stdin is given (allows piping ssot.bpb_samples
 *      from psql on the matrix-bot CI host).
 *
 * Output: JSON report on stdout, optional Markdown table to a file via
 * --markdown-out=<path>, optional comparison against
 * `assertions/matrix_coverage_baseline.json` via --baseline.
 *
 * R7-honest definition (matches `.github/scripts/closure_gate.py`):
 *   A cell (format, algo) is HONEST iff
 *     COUNT(*) >= 3                             (at least 3 sample rows)
 *     COUNT(DISTINCT seed_phi) >= 2             (at least 2 distinct seeds)
 *     MAX(step) >= 3000                         (at least one full run)
 *   else SYNTHETIC (placeholder).
 *
 * Usage:
 *   npx tsx scripts/matrix_coverage_report.ts
 *   npx tsx scripts/matrix_coverage_report.ts --baseline --markdown-out=cov.md
 *
 * Anchor: phi^2 + phi^-2 = 3 · DOI 10.5281/zenodo.19227877.
 */

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const REPO_ROOT = resolve(__dirname, "..");
const SAMPLES_JSONL = resolve(
  REPO_ROOT,
  process.env.SAMPLES_JSONL || "assertions/matrix_samples.jsonl",
);
const BASELINE_PATH = resolve(
  REPO_ROOT,
  "assertions/matrix_coverage_baseline.json",
);
const ANCHOR = "phi^2 + phi^-2 = 3";
const DOI = "10.5281/zenodo.19227877";

const FORMATS_ORDERED: string[] = [
  "f32", "f64", "fp16", "bf16", "tf32",
  "fp8_e4m3", "fp8_e5m2", "fp6_e2m3", "fp6_e3m2", "fp4_e2m1",
  "gf4", "gf8", "gf12", "gf16", "gf20", "gf24", "gf32", "gf64",
  "int4", "int8", "int16", "int32", "uint8",
  "nf4", "nf8",
  "posit8", "posit16", "posit32", "posit64",
  "lns8",
  "mxfp4", "mxfp6", "mxfp8",
  "decimal32", "decimal64", "decimal128",
  "binary128", "binary256", "fp80",
];
const ALGOS_ORDERED: string[] = [
  "adamw", "muon", "sgdm", "lion", "adafactor",
  "lamb", "schedulefree", "rmsprop", "soap",
];
const TOTAL_CELLS = FORMATS_ORDERED.length * ALGOS_ORDERED.length; // 39 * 9 = 351

const HONEST_MIN_ROWS = 3;
const HONEST_MIN_SEEDS = 2;
const HONEST_MIN_STEP = 3000;

interface SampleRow {
  format: string;
  algo: string;
  seed_phi: number;
  step: number;
  bpb: number;
}

interface CellAgg {
  rows: number;
  seeds: Set<number>;
  max_step: number;
}

interface CellReport {
  format: string;
  algo: string;
  rows: number;
  distinct_seeds: number;
  max_step: number;
  honest: boolean;
}

interface CoverageReport {
  anchor: string;
  doi: string;
  generated_at: string;
  total_cells: number;
  measured_R7_honest: number;
  synthetic_flag_1: number;
  coverage_pct: number;
  formats: number;
  algos: number;
  by_algo: Record<string, { honest: number; total: number }>;
  by_format: Record<string, { honest: number; total: number }>;
  honest_cells: CellReport[];
  delta?: {
    baseline_pct: number;
    delta_pct: number;
    new_honest_cells: number;
  };
}

function parseRows(text: string): SampleRow[] {
  const rows: SampleRow[] = [];
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (!line) continue;
    let obj: unknown;
    try {
      obj = JSON.parse(line);
    } catch {
      continue;
    }
    if (typeof obj !== "object" || obj === null) continue;
    const o = obj as Record<string, unknown>;
    if (Object.keys(o).every((k) => k.startsWith("_"))) continue;
    if (
      typeof o.format === "string" &&
      typeof o.algo === "string" &&
      typeof o.seed_phi === "number" &&
      typeof o.step === "number" &&
      typeof o.bpb === "number"
    ) {
      rows.push({
        format: o.format,
        algo: o.algo,
        seed_phi: o.seed_phi,
        step: o.step,
        bpb: o.bpb,
      });
    }
  }
  return rows;
}

function aggregate(rows: SampleRow[]): Map<string, CellAgg> {
  const m = new Map<string, CellAgg>();
  for (const r of rows) {
    const k = `${r.format}::${r.algo}`;
    const a = m.get(k) || { rows: 0, seeds: new Set<number>(), max_step: 0 };
    a.rows += 1;
    a.seeds.add(r.seed_phi);
    if (r.step > a.max_step) a.max_step = r.step;
    m.set(k, a);
  }
  return m;
}

function buildReport(rows: SampleRow[]): CoverageReport {
  const agg = aggregate(rows);
  const honestCells: CellReport[] = [];
  const byAlgo: Record<string, { honest: number; total: number }> = {};
  const byFormat: Record<string, { honest: number; total: number }> = {};
  for (const a of ALGOS_ORDERED) byAlgo[a] = { honest: 0, total: 0 };
  for (const f of FORMATS_ORDERED) byFormat[f] = { honest: 0, total: 0 };

  let honestCount = 0;
  for (const f of FORMATS_ORDERED) {
    for (const a of ALGOS_ORDERED) {
      byAlgo[a].total += 1;
      byFormat[f].total += 1;
      const k = `${f}::${a}`;
      const cell = agg.get(k);
      const rowsCount = cell?.rows ?? 0;
      const seedsCount = cell?.seeds.size ?? 0;
      const maxStep = cell?.max_step ?? 0;
      const honest =
        rowsCount >= HONEST_MIN_ROWS &&
        seedsCount >= HONEST_MIN_SEEDS &&
        maxStep >= HONEST_MIN_STEP;
      if (honest) {
        honestCount += 1;
        byAlgo[a].honest += 1;
        byFormat[f].honest += 1;
        honestCells.push({
          format: f,
          algo: a,
          rows: rowsCount,
          distinct_seeds: seedsCount,
          max_step: maxStep,
          honest: true,
        });
      }
    }
  }
  const synthetic = TOTAL_CELLS - honestCount;
  const pct = (honestCount / TOTAL_CELLS) * 100;
  return {
    anchor: ANCHOR,
    doi: DOI,
    generated_at: new Date().toISOString(),
    total_cells: TOTAL_CELLS,
    measured_R7_honest: honestCount,
    synthetic_flag_1: synthetic,
    coverage_pct: Number(pct.toFixed(2)),
    formats: FORMATS_ORDERED.length,
    algos: ALGOS_ORDERED.length,
    by_algo: byAlgo,
    by_format: byFormat,
    honest_cells: honestCells,
  };
}

function readBaseline(): {
  measured_R7_honest: number;
  coverage_pct: number;
} | null {
  if (!existsSync(BASELINE_PATH)) return null;
  try {
    const obj = JSON.parse(readFileSync(BASELINE_PATH, "utf8"));
    if (
      typeof obj === "object" &&
      obj !== null &&
      typeof obj.measured_R7_honest === "number" &&
      typeof obj.coverage_pct === "number"
    ) {
      return {
        measured_R7_honest: obj.measured_R7_honest,
        coverage_pct: obj.coverage_pct,
      };
    }
  } catch {
    /* fall through to null */
  }
  return null;
}

function markdownTable(r: CoverageReport): string {
  const lines: string[] = [];
  lines.push(`# Matrix Coverage Report`);
  lines.push("");
  lines.push(`Anchor: \`${r.anchor}\` · DOI ${r.doi}`);
  lines.push("");
  lines.push(`Generated: ${r.generated_at}`);
  lines.push("");
  lines.push(`## Summary`);
  lines.push("");
  lines.push(`- Total cells: **${r.total_cells}** (${r.formats}×${r.algos})`);
  lines.push(`- Measured R7-honest: **${r.measured_R7_honest}**`);
  lines.push(`- Synthetic placeholders: **${r.synthetic_flag_1}**`);
  lines.push(`- Coverage: **${r.coverage_pct}%**`);
  if (r.delta) {
    const sign = r.delta.delta_pct >= 0 ? "+" : "";
    lines.push(
      `- vs baseline: ${sign}${r.delta.delta_pct.toFixed(2)} pp ` +
        `(${r.delta.new_honest_cells >= 0 ? "+" : ""}${r.delta.new_honest_cells} cells)`,
    );
  }
  lines.push("");
  lines.push(`## Coverage by algorithm`);
  lines.push("");
  lines.push(`| algo | honest | total | pct |`);
  lines.push(`|------|-------:|------:|----:|`);
  for (const a of ALGOS_ORDERED) {
    const v = r.by_algo[a];
    const pct = v.total === 0 ? 0 : (v.honest / v.total) * 100;
    lines.push(`| ${a} | ${v.honest} | ${v.total} | ${pct.toFixed(0)}% |`);
  }
  lines.push("");
  lines.push(`## Coverage by format`);
  lines.push("");
  lines.push(`| format | honest | total | pct |`);
  lines.push(`|--------|-------:|------:|----:|`);
  for (const f of FORMATS_ORDERED) {
    const v = r.by_format[f];
    const pct = v.total === 0 ? 0 : (v.honest / v.total) * 100;
    lines.push(`| ${f} | ${v.honest} | ${v.total} | ${pct.toFixed(0)}% |`);
  }
  lines.push("");
  return lines.join("\n");
}

function readStdinSync(): string {
  const chunks: Buffer[] = [];
  const fd = 0;
  const fs = require("node:fs") as typeof import("node:fs");
  const buf = Buffer.alloc(65536);
  for (;;) {
    let n = 0;
    try {
      n = fs.readSync(fd, buf, 0, buf.length, null);
    } catch {
      break;
    }
    if (n <= 0) break;
    chunks.push(Buffer.from(buf.subarray(0, n)));
  }
  return Buffer.concat(chunks).toString("utf8");
}

function parseArgs(argv: string[]): {
  fromStdin: boolean;
  withBaseline: boolean;
  markdownOut: string | null;
} {
  let fromStdin = false;
  let withBaseline = false;
  let markdownOut: string | null = null;
  for (const a of argv.slice(2)) {
    if (a === "--stdin") fromStdin = true;
    else if (a === "--baseline") withBaseline = true;
    else if (a.startsWith("--markdown-out=")) markdownOut = a.split("=")[1] || null;
  }
  return { fromStdin, withBaseline, markdownOut };
}

function main(): number {
  const { fromStdin, withBaseline, markdownOut } = parseArgs(process.argv);
  let text: string;
  if (fromStdin) {
    text = readStdinSync();
  } else if (existsSync(SAMPLES_JSONL)) {
    text = readFileSync(SAMPLES_JSONL, "utf8");
  } else {
    process.stderr.write(
      `matrix_coverage_report: no JSONL at ${SAMPLES_JSONL} and no --stdin given\n`,
    );
    return 2;
  }
  const rows = parseRows(text);
  const report = buildReport(rows);

  if (withBaseline) {
    const base = readBaseline();
    if (base) {
      report.delta = {
        baseline_pct: base.coverage_pct,
        delta_pct: report.coverage_pct - base.coverage_pct,
        new_honest_cells: report.measured_R7_honest - base.measured_R7_honest,
      };
    }
  }

  process.stdout.write(JSON.stringify(report, null, 2) + "\n");

  if (markdownOut) {
    writeFileSync(markdownOut, markdownTable(report) + "\n");
    process.stderr.write(`matrix_coverage_report: wrote markdown to ${markdownOut}\n`);
  }
  return 0;
}

process.exit(main());
