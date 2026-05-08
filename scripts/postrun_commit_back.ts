#!/usr/bin/env -S npx tsx
/**
 * scripts/postrun_commit_back.ts — commit-back orchestrator for the
 * trios-mr-priority-runner Railway service (issue gHashTag/trios#598).
 *
 * Reads the local `assertions/matrix_samples.jsonl` produced inside the
 * runner container, computes a per-row content hash, fetches the current
 * `main` JSONL via the GitHub API, identifies new rows, and opens one PR
 * per batch (default 25 rows) on a fresh `data/matrix-runner-<ts>` branch.
 *
 * Idempotent: re-runs skip rows whose hash already exists upstream.
 *
 * Anchor: phi^2 + phi^-2 = 3 · DOI 10.5281/zenodo.19227877.
 *
 * L1 compliance: TypeScript only — no `.sh`. Run via `npx tsx`.
 *
 * Usage (any of):
 *   GITHUB_TOKEN=ghp_... npx tsx scripts/postrun_commit_back.ts
 *   GITHUB_TOKEN=ghp_... ./scripts/postrun_commit_back.ts
 *
 * Required env:
 *   GITHUB_TOKEN        PAT or fine-grained token with contents:write +
 *                       pull-requests:write on gHashTag/trios.
 *
 * Optional env:
 *   SAMPLES_JSONL       override input JSONL path (default
 *                       assertions/matrix_samples.jsonl).
 *   BATCH_SIZE          rows per PR (default 25).
 *   GH_OWNER            owner override (default gHashTag).
 *   GH_REPO             repo override  (default trios).
 *   GH_BASE             base branch    (default main).
 *   RUNNER_SHA          pinned image SHA for commit messages
 *                       (default 6cf0b5bd per the L-MR-POSTRUN brief).
 *   DRY_RUN=1           print plan without creating branches or PRs.
 *   PARENT_ISSUE        issue # for heartbeat (default 446).
 */

import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const REPO_ROOT = resolve(__dirname, "..");
const SAMPLES_JSONL = resolve(
  REPO_ROOT,
  process.env.SAMPLES_JSONL || "assertions/matrix_samples.jsonl",
);
const BATCH_SIZE = Math.max(1, Number(process.env.BATCH_SIZE || 25));
const OWNER = process.env.GH_OWNER || "gHashTag";
const REPO = process.env.GH_REPO || "trios";
const BASE = process.env.GH_BASE || "main";
const RUNNER_SHA = process.env.RUNNER_SHA || "6cf0b5bd";
const DRY_RUN = process.env.DRY_RUN === "1";
const PARENT_ISSUE = Number(process.env.PARENT_ISSUE || 446);
const TOKEN = process.env.GITHUB_TOKEN || "";
const ANCHOR = "phi^2 + phi^-2 = 3";
const DOI = "10.5281/zenodo.19227877";
const AUTHOR_NAME = "Dmitrii Vasilev";
const AUTHOR_EMAIL = "admin@t27.ai";

if (!TOKEN && !DRY_RUN) {
  process.stderr.write(
    "GITHUB_TOKEN required (or set DRY_RUN=1 to inspect the plan)\n",
  );
  process.exit(2);
}

// ---------------------------------------------------------------------------
// Row schema (matches scripts/run_priority_matrix.ts)
// ---------------------------------------------------------------------------

interface SampleRow {
  format: string;
  algo: string;
  seed_phi: number;
  step: number;
  bpb: number;
  sha: string;
  source: string;
  timestamp: string;
}

/**
 * Canonical content hash. Uses ONLY the (format, algo, seed_phi, step, bpb)
 * fields so that re-runs that produce identical numerical outputs are
 * deduplicated even if the runner-emitted source/timestamp/sha vary.
 */
function rowHash(r: SampleRow): string {
  const canonical = JSON.stringify({
    format: r.format,
    algo: r.algo,
    seed_phi: r.seed_phi,
    step: r.step,
    bpb: r.bpb,
  });
  return createHash("sha256").update(canonical).digest("hex");
}

function parseJsonl(text: string): SampleRow[] {
  const rows: SampleRow[] = [];
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    let obj: unknown;
    try {
      obj = JSON.parse(trimmed);
    } catch {
      continue;
    }
    if (typeof obj !== "object" || obj === null) continue;
    const o = obj as Record<string, unknown>;
    // Skip the schema header line that carries underscore-prefixed metadata
    // (e.g. _schema, _comment, _anchor) instead of sample fields.
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
        sha: typeof o.sha === "string" ? o.sha : "",
        source: typeof o.source === "string" ? o.source : "",
        timestamp:
          typeof o.timestamp === "string" ? o.timestamp : new Date().toISOString(),
      });
    }
  }
  return rows;
}

// ---------------------------------------------------------------------------
// GitHub API helpers (REST v3, no third-party deps)
// ---------------------------------------------------------------------------

const GH_API = "https://api.github.com";

interface GhResponse {
  status: number;
  body: unknown;
}

async function gh(
  method: "GET" | "POST" | "PATCH" | "PUT" | "DELETE",
  path: string,
  body?: unknown,
): Promise<GhResponse> {
  const url = path.startsWith("http") ? path : `${GH_API}${path}`;
  const init: RequestInit = {
    method,
    headers: {
      Authorization: `Bearer ${TOKEN}`,
      Accept: "application/vnd.github+json",
      "X-GitHub-Api-Version": "2022-11-28",
      "User-Agent": "trios-postrun-orchestrator/1.0",
      ...(body ? { "Content-Type": "application/json" } : {}),
    },
    body: body ? JSON.stringify(body) : undefined,
  };
  const res = await fetch(url, init);
  let parsed: unknown = null;
  const text = await res.text();
  if (text) {
    try {
      parsed = JSON.parse(text);
    } catch {
      parsed = text;
    }
  }
  return { status: res.status, body: parsed };
}

// ---------------------------------------------------------------------------
// Remote JSONL fetch (raw contents at HEAD of base branch)
// ---------------------------------------------------------------------------

async function fetchRemoteSamples(): Promise<SampleRow[]> {
  if (DRY_RUN && !TOKEN) return [];
  const path = "assertions/matrix_samples.jsonl";
  const res = await gh(
    "GET",
    `/repos/${OWNER}/${REPO}/contents/${path}?ref=${BASE}`,
  );
  if (res.status === 404) return [];
  if (res.status !== 200 || typeof res.body !== "object" || res.body === null) {
    throw new Error(`fetchRemoteSamples: GH ${res.status} for ${path}`);
  }
  const obj = res.body as Record<string, unknown>;
  if (typeof obj.content !== "string") {
    throw new Error("fetchRemoteSamples: missing content field");
  }
  const decoded = Buffer.from(obj.content, "base64").toString("utf8");
  return parseJsonl(decoded);
}

async function fetchRemoteRaw(): Promise<{ sha: string; text: string }> {
  const path = "assertions/matrix_samples.jsonl";
  const res = await gh(
    "GET",
    `/repos/${OWNER}/${REPO}/contents/${path}?ref=${BASE}`,
  );
  if (res.status !== 200 || typeof res.body !== "object" || res.body === null) {
    throw new Error(`fetchRemoteRaw: GH ${res.status} for ${path}`);
  }
  const obj = res.body as Record<string, unknown>;
  if (typeof obj.content !== "string" || typeof obj.sha !== "string") {
    throw new Error("fetchRemoteRaw: missing fields");
  }
  return {
    sha: obj.sha,
    text: Buffer.from(obj.content, "base64").toString("utf8"),
  };
}

// ---------------------------------------------------------------------------
// Branch + commit + PR creation
// ---------------------------------------------------------------------------

async function getBaseHeadSha(): Promise<string> {
  const res = await gh("GET", `/repos/${OWNER}/${REPO}/git/ref/heads/${BASE}`);
  if (res.status !== 200 || typeof res.body !== "object" || res.body === null) {
    throw new Error(`getBaseHeadSha: GH ${res.status}`);
  }
  const obj = res.body as { object?: { sha?: string } };
  if (!obj.object || typeof obj.object.sha !== "string") {
    throw new Error("getBaseHeadSha: missing object.sha");
  }
  return obj.object.sha;
}

async function createBranch(name: string, fromSha: string): Promise<void> {
  const res = await gh("POST", `/repos/${OWNER}/${REPO}/git/refs`, {
    ref: `refs/heads/${name}`,
    sha: fromSha,
  });
  if (res.status !== 201) {
    throw new Error(`createBranch: GH ${res.status} for ${name}`);
  }
}

async function putFile(
  branch: string,
  path: string,
  content: string,
  fileSha: string,
  message: string,
): Promise<void> {
  const res = await gh(
    "PUT",
    `/repos/${OWNER}/${REPO}/contents/${path}`,
    {
      message,
      content: Buffer.from(content, "utf8").toString("base64"),
      branch,
      sha: fileSha,
      committer: { name: AUTHOR_NAME, email: AUTHOR_EMAIL },
      author: { name: AUTHOR_NAME, email: AUTHOR_EMAIL },
    },
  );
  if (res.status < 200 || res.status >= 300) {
    throw new Error(`putFile: GH ${res.status} for ${path} on ${branch}`);
  }
}

async function openPullRequest(
  branch: string,
  title: string,
  body: string,
): Promise<number> {
  const res = await gh("POST", `/repos/${OWNER}/${REPO}/pulls`, {
    title,
    head: branch,
    base: BASE,
    body,
    maintainer_can_modify: true,
  });
  if (res.status !== 201 || typeof res.body !== "object" || res.body === null) {
    throw new Error(`openPullRequest: GH ${res.status}`);
  }
  const obj = res.body as { number?: number };
  if (typeof obj.number !== "number") {
    throw new Error("openPullRequest: missing number");
  }
  return obj.number;
}

async function postIssueComment(issue: number, body: string): Promise<void> {
  const res = await gh(
    "POST",
    `/repos/${OWNER}/${REPO}/issues/${issue}/comments`,
    { body },
  );
  if (res.status < 200 || res.status >= 300) {
    process.stderr.write(`postIssueComment: GH ${res.status} (non-fatal)\n`);
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

interface BatchResult {
  branch: string;
  pr: number | null;
  rows: number;
}

function chunk<T>(arr: T[], size: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < arr.length; i += size) {
    out.push(arr.slice(i, i + size));
  }
  return out;
}

function isoNowCompact(): string {
  // 2026-05-08T16-37-12Z (filesystem-safe form of ISO-8601 UTC)
  return new Date().toISOString().replace(/[:.]/g, "-").replace(/-\d{3}/, "");
}

async function main(): Promise<number> {
  process.stdout.write(
    `[postrun_commit_back] anchor=${ANCHOR} doi=${DOI} runner_sha=${RUNNER_SHA}\n`,
  );

  if (!existsSync(SAMPLES_JSONL)) {
    process.stdout.write(
      `[postrun_commit_back] no local JSONL at ${SAMPLES_JSONL}; nothing to do.\n`,
    );
    return 0;
  }

  const localText = readFileSync(SAMPLES_JSONL, "utf8");
  const localRows = parseJsonl(localText);
  process.stdout.write(
    `[postrun_commit_back] local rows: ${localRows.length}\n`,
  );

  if (localRows.length === 0) {
    process.stdout.write(
      "[postrun_commit_back] local JSONL has no sample rows; nothing to commit.\n",
    );
    return 0;
  }

  const remoteRows = DRY_RUN && !TOKEN ? [] : await fetchRemoteSamples();
  process.stdout.write(
    `[postrun_commit_back] remote rows: ${remoteRows.length}\n`,
  );

  const remoteHashes = new Set(remoteRows.map(rowHash));

  // De-duplicate locally too — same hash within local JSONL counts once.
  const seenLocal = new Set<string>();
  const newRows: SampleRow[] = [];
  for (const r of localRows) {
    const h = rowHash(r);
    if (remoteHashes.has(h)) continue;
    if (seenLocal.has(h)) continue;
    seenLocal.add(h);
    newRows.push(r);
  }

  process.stdout.write(
    `[postrun_commit_back] candidates after dedup: ${newRows.length} (batch=${BATCH_SIZE})\n`,
  );

  if (newRows.length === 0) {
    process.stdout.write(
      "[postrun_commit_back] no new rows; remote is up to date.\n",
    );
    return 0;
  }

  const batches = chunk(newRows, BATCH_SIZE);
  process.stdout.write(
    `[postrun_commit_back] planned batches: ${batches.length}\n`,
  );

  if (DRY_RUN) {
    for (const [i, b] of batches.entries()) {
      process.stdout.write(
        `  batch ${i + 1}/${batches.length}: ${b.length} rows\n`,
      );
    }
    process.stdout.write(
      "[postrun_commit_back] DRY_RUN=1 — exiting without remote writes.\n",
    );
    return 0;
  }

  // Sequential — each batch sees the previous batch's effect on remote sha
  // by re-fetching contents/{path} before its PUT. PR creation does not
  // wait for queen merge.
  const baseSha = await getBaseHeadSha();
  const ts = isoNowCompact();
  const results: BatchResult[] = [];

  for (const [i, batch] of batches.entries()) {
    const branch = `data/matrix-runner-${ts}-batch-${i + 1}`;
    await createBranch(branch, baseSha);

    // Fetch current file sha on BASE; PUT will overwrite on the new branch.
    const remote = await fetchRemoteRaw();
    const appended =
      (remote.text.endsWith("\n") || remote.text === "" ? remote.text : remote.text + "\n") +
      batch.map((r) => JSON.stringify(r)).join("\n") +
      "\n";

    const message = `data(matrix): commit-back batch ${i + 1}/${batches.length} (${batch.length} rows) from runner SHA ${RUNNER_SHA} · ${ANCHOR}`;
    await putFile(
      branch,
      "assertions/matrix_samples.jsonl",
      appended,
      remote.sha,
      message,
    );

    const prTitle = `data(matrix): commit-back batch ${i + 1}/${batches.length} from runner SHA ${RUNNER_SHA}`;
    const prBody = [
      `Closes-part #${PARENT_ISSUE} (matrix #446 commit-back, lane #598).`,
      "",
      `Batch ${i + 1} of ${batches.length} from \`trios-mr-priority-runner\` (Railway service \`71f5aac2-d4d5-4640-8895-90ced5d4ea63\`, image SHA \`${RUNNER_SHA}\`).`,
      "",
      `**Rows added:** ${batch.length}`,
      "",
      "**Hashes (sha256 of canonical {format,algo,seed_phi,step,bpb}):**",
      ...batch.map(
        (r) =>
          `- \`${rowHash(r).slice(0, 16)}\` · ${r.format}/${r.algo}/seed=${r.seed_phi} · step=${r.step} · bpb=${r.bpb}`,
      ),
      "",
      `Anchor: \`${ANCHOR}\` · DOI [${DOI}](https://doi.org/${DOI})`,
    ].join("\n");

    let prNum: number | null = null;
    try {
      prNum = await openPullRequest(branch, prTitle, prBody);
    } catch (err) {
      process.stderr.write(
        `[postrun_commit_back] PR open failed for ${branch}: ${(err as Error).message}\n`,
      );
    }
    results.push({ branch, pr: prNum, rows: batch.length });

    process.stdout.write(
      `  batch ${i + 1}/${batches.length}: branch=${branch} pr=${prNum ?? "?"} rows=${batch.length}\n`,
    );
  }

  // EOJ heartbeat on parent issue. Best-effort — non-fatal on failure.
  const heartbeat = [
    `<!-- postrun-heartbeat:${ts} -->`,
    `**postrun_commit_back** (lane #598) — runner SHA \`${RUNNER_SHA}\``,
    "",
    `- batches opened: ${results.length}`,
    `- rows committed: ${results.reduce((s, r) => s + r.rows, 0)}`,
    `- PRs: ${results.map((r) => (r.pr ? `#${r.pr}` : `${r.branch} (no PR)`)).join(", ")}`,
    "",
    `Anchor: \`${ANCHOR}\` · DOI ${DOI}`,
  ].join("\n");
  await postIssueComment(PARENT_ISSUE, heartbeat);

  return 0;
}

main()
  .then((code) => process.exit(code))
  .catch((err) => {
    process.stderr.write(
      `[postrun_commit_back] FATAL: ${(err as Error).stack || err}\n`,
    );
    process.exit(1);
  });
