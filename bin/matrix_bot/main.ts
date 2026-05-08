// matrix_bot — hourly regenerator for trios#446 coverage matrix
// Anchor: phi^2 + phi^-2 = 3 · DOI 10.5281/zenodo.19227877
import { Client } from "pg";
import { Octokit } from "@octokit/rest";

const DB_URL = process.env.RAILWAY_POSTGRES_URL ?? process.env.NEON_DATABASE_URL;
const GH_TOKEN = process.env.GITHUB_TOKEN!;
const ISSUE = parseInt(process.env.ISSUE ?? "446", 10);
const OWNER = process.env.GH_OWNER ?? "gHashTag";
const REPO = process.env.GH_REPO ?? "trios";
const POLL_S = parseInt(process.env.POLL_INTERVAL_S ?? "3600", 10);
const SEEDS = (process.env.SEEDS ?? "47,89,144,123").split(",").map(s => parseInt(s.trim(), 10));
const TOTAL_CELLS = parseInt(process.env.TOTAL_CELLS ?? "351", 10);
const MARKER_BEGIN = "<!-- matrix_bot:auto:begin -->";
const MARKER_END = "<!-- matrix_bot:auto:end -->";

async function tick() {
  if (!DB_URL) {
    console.error("[matrix_bot] no DB URL configured");
    return;
  }
  const client = new Client({ connectionString: DB_URL });
  await client.connect();
  try {
    const totalSamples = TOTAL_CELLS * SEEDS.length;
    const tierQ = await client.query(`
      SELECT tier,
             COUNT(*)::int AS rows,
             COUNT(DISTINCT cell_id)::int AS cells,
             COUNT(DISTINCT seed)::int AS seeds,
             MIN(bpb)::float AS bpb_min,
             AVG(bpb)::float AS bpb_avg,
             MAX(bpb)::float AS bpb_max
      FROM ssot.bpb_samples
      GROUP BY tier
      ORDER BY tier;
    `);
    const total = await client.query(`SELECT COUNT(*)::int AS c FROM ssot.bpb_samples`);
    const totalRows = total.rows[0]?.c ?? 0;
    const pct = totalSamples > 0 ? ((totalRows / totalSamples) * 100).toFixed(1) : "0.0";
    const ts = new Date().toISOString();

    let table = "| tier | rows | cells | seeds | bpb_min | bpb_avg | bpb_max |\n|---|---:|---:|---:|---:|---:|---:|\n";
    for (const r of tierQ.rows) {
      table += `| ${r.tier} | ${r.rows} | ${r.cells} | ${r.seeds} | ${(r.bpb_min ?? 0).toFixed(4)} | ${(r.bpb_avg ?? 0).toFixed(4)} | ${(r.bpb_max ?? 0).toFixed(4)} |\n`;
    }

    const block = [
      MARKER_BEGIN,
      `## Matrix Coverage — auto-regenerated ${ts}`,
      ``,
      `**Total**: ${totalRows} / ${totalSamples} samples (${pct}%) — ${TOTAL_CELLS} cells × ${SEEDS.length} seeds = ${totalSamples}`,
      ``,
      table,
      ``,
      `_Anchor: \`phi^2 + phi^-2 = 3\` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)_`,
      MARKER_END,
    ].join("\n");

    const octo = new Octokit({ auth: GH_TOKEN });
    const issue = await octo.issues.get({ owner: OWNER, repo: REPO, issue_number: ISSUE });
    const body = issue.data.body ?? "";
    let newBody: string;
    if (body.includes(MARKER_BEGIN) && body.includes(MARKER_END)) {
      newBody = body.replace(new RegExp(`${MARKER_BEGIN}[\\s\\S]*?${MARKER_END}`), block);
    } else {
      newBody = body + `\n\n${block}\n`;
    }
    await octo.issues.update({ owner: OWNER, repo: REPO, issue_number: ISSUE, body: newBody });
    console.log(`[matrix_bot] updated #${ISSUE}: ${totalRows}/${totalSamples} (${pct}%)`);
  } finally {
    await client.end();
  }
}

async function main() {
  console.log(`[matrix_bot] starting · poll=${POLL_S}s · issue=${ISSUE} · seeds=${SEEDS.join(",")}`);
  while (true) {
    try { await tick(); } catch (e) { console.error("[matrix_bot] tick error:", e); }
    await new Promise(r => setTimeout(r, POLL_S * 1000));
  }
}

main().catch(e => { console.error("[matrix_bot] fatal:", e); process.exit(1); });
