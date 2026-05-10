#!/usr/bin/env -S npx tsx
/**
 * scripts/render_matrix_446.ts — full 312-cell Format×Algorithm matrix renderer
 *
 * Closes the work begun in github.com/gHashTag/trios/issues/446#issuecomment-4370442020
 * (38/312 measured · 12.2%) by:
 *   1. reading frozen legacy snapshot from assertions/matrix_legacy_snapshot_2026-05-04.jsonl
 *   2. overlaying live ssot.bpb_samples (best BPB per (format, algo) cell, ≥3 rows × ≥2 seeds × step≥3000)
 *   3. rendering the full Format×Algorithm matrix (39 formats × 8 algos = 312 cells)
 *   4. emitting structured Markdown with marker block <!-- matrix_bot:auto:begin --> ... <!-- matrix_bot:auto:end -->
 *
 * R5-honest: live > legacy. R7 thresholds enforced for live promotion. Legacy cells
 * are flagged with `(legacy)` annotation. Empty cells render as `🔲`.
 *
 * Anchor: phi^2 + phi^-2 = 3 · DOI 10.5281/zenodo.19227877
 */
import * as fs from "fs";
import * as path from "path";

type Cell = {
  best_bpb: number;
  source: "live" | "legacy";
  n_rows?: number;
  distinct_seeds?: number;
  max_step?: number;
};

const FORMATS_ORDER = [
  // 16-bit headline triplet first
  "gf16", "bf16", "fp16",
  // 32-bit
  "fp32", "gf32", "tf32", "int32",
  // 8-bit
  "fp8_e4m3", "fp8_e5m2", "int8", "uint8", "gf8",
  // 4/12/20/24-bit GF family
  "gf4", "gf12", "gf20", "gf24", "gf64",
  // 16-bit signed int
  "int16",
  // 64-bit
  "fp64",
  // posit / NF / FP6 / FP4 / MXFP / decimal / binary / FP80 / VAX / Cray / IBM / Unum
  "posit8", "posit16", "posit32", "posit64",
  "nf4",
  "fp6_e3m2", "fp6_e2m3",
  "fp4_e2m1",
  "mxfp4", "mxfp6", "mxfp8",
  "decimal32", "decimal64", "decimal128",
  "binary128", "binary256",
  "fp80",
  "lns",
  "unum_i", "unum_ii",
];

const ALGOS_ORDER = [
  "adamw", "muon", "sgdm", "lion", "adafactor", "lamb", "soap", "schedulefree",
];

const FORMAT_ALIAS: Record<string, string> = {
  // legacy snapshot used `fp32`; rest of the system uses `f32` in some places
  f32: "fp32",
  f64: "fp64",
  fp32: "fp32",
  fp64: "fp64",
};

function loadLegacy(): Map<string, Cell> {
  const filePath = path.join(__dirname, "..", "assertions", "matrix_legacy_snapshot_2026-05-04.jsonl");
  const map = new Map<string, Cell>();
  if (!fs.existsSync(filePath)) return map;
  const lines = fs.readFileSync(filePath, "utf-8").split("\n").filter(l => l.trim());
  for (const line of lines) {
    const obj = JSON.parse(line);
    if (obj._schema || !obj.format) continue;
    const fmt = FORMAT_ALIAS[obj.format] ?? obj.format;
    const key = `${fmt}|${obj.algo}`;
    map.set(key, { best_bpb: obj.best_bpb, source: "legacy" });
  }
  return map;
}

async function loadLive(): Promise<Map<string, Cell>> {
  const map = new Map<string, Cell>();
  const dbUrl = process.env.RAILWAY_POSTGRES_URL ?? process.env.NEON_DATABASE_URL;
  if (!dbUrl) {
    console.error("[render_matrix_446] no DB URL — skipping live overlay");
    return map;
  }
  const { Client } = await import("pg");
  const client = new Client({ connectionString: dbUrl });
  await client.connect();
  try {
    const res = await client.query(`
      SELECT format, algo,
             MIN(bpb)::float AS best_bpb,
             COUNT(*)::int AS n_rows,
             COUNT(DISTINCT seed)::int AS distinct_seeds,
             MAX(steps)::int AS max_step
        FROM ssot.bpb_samples
       GROUP BY format, algo
      HAVING COUNT(*) >= 3
         AND COUNT(DISTINCT seed) >= 2
         AND MAX(steps) >= 3000;
    `);
    for (const r of res.rows) {
      const fmt = FORMAT_ALIAS[r.format] ?? r.format;
      map.set(`${fmt}|${r.algo}`, {
        best_bpb: r.best_bpb,
        source: "live",
        n_rows: r.n_rows,
        distinct_seeds: r.distinct_seeds,
        max_step: r.max_step,
      });
    }
  } finally {
    await client.end();
  }
  return map;
}

function fmtCell(c: Cell | undefined): string {
  if (!c) return "🔲";
  const v = c.best_bpb.toFixed(4);
  return c.source === "live" ? `**${v}**` : `${v} ⓛ`;
}

function render(legacy: Map<string, Cell>, live: Map<string, Cell>): string {
  // live takes precedence
  const merged = new Map<string, Cell>(legacy);
  for (const [k, v] of live) merged.set(k, v);

  let live_n = 0, legacy_n = 0;
  const winners: Record<string, { algo: string; bpb: number; src: "live" | "legacy" }> = {};

  let table = `| Format ↓ \\ Algo → |`;
  for (const a of ALGOS_ORDER) table += ` **${a}** |`;
  table += `\n|---|`;
  for (const _ of ALGOS_ORDER) table += `---:|`;
  table += `\n`;

  for (const fmt of FORMATS_ORDER) {
    table += `| **${fmt}** |`;
    for (const algo of ALGOS_ORDER) {
      const c = merged.get(`${fmt}|${algo}`);
      table += ` ${fmtCell(c)} |`;
      if (c) {
        if (c.source === "live") live_n++; else legacy_n++;
        const cur = winners[fmt];
        if (!cur || c.best_bpb < cur.bpb) winners[fmt] = { algo, bpb: c.best_bpb, src: c.source };
      }
    }
    table += `\n`;
  }

  const total_cells = FORMATS_ORDER.length * ALGOS_ORDER.length;
  const measured = live_n + legacy_n;
  const pct = ((measured / total_cells) * 100).toFixed(1);
  const live_pct = ((live_n / total_cells) * 100).toFixed(1);

  // 16-bit anchor row (the PhD thesis claim)
  const anchor16 = ["gf16", "bf16", "fp16"]
    .map(f => {
      const w = winners[f];
      return w ? `| **${f}** | ${w.bpb.toFixed(4)} ${w.src === "live" ? "(live)" : "ⓛ"} | ${w.algo} |` : `| **${f}** | — | — |`;
    })
    .join("\n");

  const ts = new Date().toISOString();
  const ANCHOR_ROW = anchor16 ? `\n### 🔑 16-bit anchor (PhD thesis claim)\n\n| Format | Best BPB | Algo |\n|---|---:|---|\n${anchor16}\n\n` : "";

  return [
    `<!-- matrix_bot:auto:begin -->`,
    `## 📐 PhD Coordinate Matrix — Format × Algorithm — auto-regenerated ${ts}`,
    ``,
    `**Coverage:** ${measured}/${total_cells} cells (${pct}%) · live=${live_n} (${live_pct}%) · legacy ⓛ=${legacy_n}`,
    ``,
    `Legend: \`**val**\` = live SSOT (R7-honest) · \`val ⓛ\` = legacy snapshot from [issuecomment-4370442020](https://github.com/gHashTag/trios/issues/446#issuecomment-4370442020) · \`🔲\` = TODO`,
    ``,
    table,
    ANCHOR_ROW,
    `**R7 thresholds for live promotion**: \`COUNT(*) ≥ 3 · COUNT(DISTINCT seed) ≥ 2 · MAX(step) ≥ 3000\` (\`.github/scripts/closure_gate.py\`).`,
    ``,
    `_Anchor: \`phi^2 + phi^-2 = 3\` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)_`,
    `<!-- matrix_bot:auto:end -->`,
  ].join("\n");
}

async function main() {
  const legacy = loadLegacy();
  const live = await loadLive();
  const md = render(legacy, live);
  const outArg = process.argv.find(a => a.startsWith("--output="));
  if (outArg) {
    const out = outArg.split("=")[1];
    fs.writeFileSync(out, md + "\n");
    console.error(`[render_matrix_446] wrote ${out}`);
  } else {
    process.stdout.write(md + "\n");
  }
}

main().catch(e => { console.error("[render_matrix_446] fatal:", e); process.exit(1); });
