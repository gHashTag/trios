#!/usr/bin/env -S npx tsx
/**
 * scripts/run_priority_matrix.ts — local matrix runner for L-MR-MATRIX-PRIORITY.
 *
 * Reads `assertions/matrix_priority_50.csv` and, for each (format, algo) cell,
 * invokes `trios-trainer-igla` once per Lucas/Fibonacci seed in {47, 89, 144}
 * for `--steps 3000`, appending one JSON line per run to
 * `assertions/matrix_samples.jsonl`.
 *
 * Idempotent: a (format, algo, seed_phi) triple already present in the JSONL
 * is skipped on re-runs.
 *
 * Anchor: phi^2 + phi^-2 = 3 · DOI 10.5281/zenodo.19227877.
 *
 * L1 compliance note: this runner is TypeScript (executed via `npx tsx` or
 * `bun run`) — repo law L1 forbids `.sh` files, so the original bash spec
 * from the L-MR-MATRIX-PRIORITY brief was retargeted to a typed runner.
 *
 * Usage (any of):
 *   npx tsx scripts/run_priority_matrix.ts
 *   bun run scripts/run_priority_matrix.ts
 *   ./scripts/run_priority_matrix.ts                  # via shebang
 *
 * Optional env:
 *   TRIOS_TRAINER_BIN   absolute path to a prebuilt `trios-trainer-igla`
 *                       binary; otherwise the runner falls back to
 *                       `cargo run --release -p trios-trainer-igla --`.
 *   PRIORITY_CSV        override the priority CSV path (default
 *                       assertions/matrix_priority_50.csv).
 *   SAMPLES_JSONL       override the output JSONL path (default
 *                       assertions/matrix_samples.jsonl).
 *   SEEDS               comma-separated override for {47,89,144}.
 *   STEPS               override step count (default 3000).
 *   DRY_RUN=1           print the plan without invoking the trainer.
 */

import { spawnSync } from "node:child_process";
import { appendFileSync, existsSync, readFileSync, statSync } from "node:fs";
import { resolve } from "node:path";

// ---------------------------------------------------------------------------
// Constants and tiny utilities
// ---------------------------------------------------------------------------

const REPO_ROOT = resolve(__dirname, "..");
const PRIORITY_CSV = resolve(
  REPO_ROOT,
  process.env.PRIORITY_CSV || "assertions/matrix_priority_50.csv",
);
const SAMPLES_JSONL = resolve(
  REPO_ROOT,
  process.env.SAMPLES_JSONL || "assertions/matrix_samples.jsonl",
);
const SEEDS: number[] = (process.env.SEEDS || "47,89,144")
  .split(",")
  .map((s) => Number.parseInt(s.trim(), 10))
  .filter((n) => Number.isFinite(n));
const STEPS = Number.parseInt(process.env.STEPS || "3000", 10);
const SOURCE_TAG = "local-priority-2026-05-08";
const DRY_RUN = process.env.DRY_RUN === "1";

interface Cell {
  format: string;
  algo: string;
  rank: number;
  reason: string;
}

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

// ---------------------------------------------------------------------------
// CSV / JSONL parsing (header-aware, no third-party deps)
// ---------------------------------------------------------------------------

function readPriorityCsv(path: string): Cell[] {
  const text = readFileSync(path, "utf8");
  const lines = text.split(/\r?\n/).filter((l) => l.length > 0);
  if (lines.length < 2) {
    throw new Error(`priority CSV is empty: ${path}`);
  }
  const header = lines[0].split(",");
  const idx = (col: string) => {
    const i = header.indexOf(col);
    if (i < 0) throw new Error(`column ${col} missing in ${path}`);
    return i;
  };
  const fF = idx("format");
  const fA = idx("algo");
  const fR = idx("priority_rank");
  const fReason = idx("priority_reason");
  const out: Cell[] = [];
  for (let i = 1; i < lines.length; i++) {
    const row = lines[i].split(",");
    out.push({
      format: row[fF],
      algo: row[fA],
      rank: Number.parseInt(row[fR], 10),
      reason: row[fReason],
    });
  }
  return out;
}

function readDoneTriples(path: string): Set<string> {
  if (!existsSync(path)) return new Set();
  const text = readFileSync(path, "utf8");
  const done = new Set<string>();
  for (const line of text.split(/\r?\n/)) {
    if (!line || line.startsWith("{\"_schema\"")) continue;
    try {
      const r = JSON.parse(line) as Partial<SampleRow>;
      if (r.format && r.algo && r.seed_phi !== undefined) {
        done.add(`${r.format}::${r.algo}::${r.seed_phi}`);
      }
    } catch {
      /* skip malformed lines silently — header is the only legit non-row */
    }
  }
  return done;
}

// ---------------------------------------------------------------------------
// Trainer invocation (with binary→cargo fallback)
// ---------------------------------------------------------------------------

interface TrainerResult {
  step: number;
  bpb: number;
  sha: string;
}

function trainerArgs(format: string, algo: string, seed: number): string[] {
  return [
    "--steps",
    String(STEPS),
    "--format",
    format,
    "--algo",
    algo,
    "--seed",
    String(seed),
  ];
}

function gitShortSha(): string {
  const r = spawnSync("git", ["rev-parse", "--short=7", "HEAD"], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  });
  if (r.status === 0) return (r.stdout || "").trim() || "unknown";
  return "unknown";
}

function runTrainer(
  format: string,
  algo: string,
  seed: number,
): TrainerResult | null {
  const args = trainerArgs(format, algo, seed);
  const binEnv = process.env.TRIOS_TRAINER_BIN;
  let cmd: string;
  let argv: string[];
  if (binEnv && existsSync(binEnv)) {
    cmd = binEnv;
    argv = args;
  } else {
    cmd = "cargo";
    argv = ["run", "--release", "-p", "trios-trainer-igla", "--", ...args];
  }
  const res = spawnSync(cmd, argv, {
    cwd: REPO_ROOT,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  if (res.status !== 0) {
    process.stderr.write(
      `  ⚠️  trainer failed (status=${res.status}) for ${format}/${algo}/${seed}\n`,
    );
    if (res.stderr) {
      process.stderr.write(res.stderr.split("\n").slice(-5).join("\n") + "\n");
    }
    return null;
  }
  // The trainer is expected to emit a final line of the form
  //   {"step": N, "bpb": F, "sha": "...", ...}
  // on stdout; if not, we accept the last well-formed JSON object on stdout.
  const lines = (res.stdout || "")
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l.startsWith("{") && l.endsWith("}"));
  for (let i = lines.length - 1; i >= 0; i--) {
    try {
      const j = JSON.parse(lines[i]) as Partial<TrainerResult>;
      if (
        typeof j.step === "number" &&
        typeof j.bpb === "number" &&
        typeof j.sha === "string"
      ) {
        return { step: j.step, bpb: j.bpb, sha: j.sha };
      }
    } catch {
      /* keep scanning back */
    }
  }
  process.stderr.write(
    `  ⚠️  trainer produced no parseable JSON line for ${format}/${algo}/${seed}\n`,
  );
  return null;
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

function nowIso(): string {
  return new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
}

function fmtDuration(ms: number): string {
  const s = Math.round(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}h${String(m).padStart(2, "0")}m${String(sec).padStart(2, "0")}s`;
  if (m > 0) return `${m}m${String(sec).padStart(2, "0")}s`;
  return `${sec}s`;
}

function main(): number {
  if (!existsSync(PRIORITY_CSV)) {
    process.stderr.write(`error: priority CSV missing at ${PRIORITY_CSV}\n`);
    return 2;
  }
  // Read existing JSONL byte size so we know whether the header was present.
  const cells = readPriorityCsv(PRIORITY_CSV);
  const done = readDoneTriples(SAMPLES_JSONL);
  const totalRuns = cells.length * SEEDS.length;
  const remaining = totalRuns - done.size;

  process.stdout.write(`L-MR-MATRIX-PRIORITY runner — ${nowIso()}\n`);
  process.stdout.write(`  priority CSV : ${PRIORITY_CSV}\n`);
  process.stdout.write(`  samples JSONL: ${SAMPLES_JSONL}\n`);
  process.stdout.write(
    `  cells=${cells.length} × seeds=${SEEDS.length} → planned runs=${totalRuns}\n`,
  );
  process.stdout.write(
    `  already done: ${done.size}; remaining: ${remaining}\n`,
  );
  if (DRY_RUN) {
    process.stdout.write("  DRY_RUN=1 — printing plan only, no trainer calls.\n");
  }

  // Append-only sentinel: ensure the JSONL has at least the header line.
  if (!existsSync(SAMPLES_JSONL) || statSync(SAMPLES_JSONL).size === 0) {
    process.stderr.write(
      `error: ${SAMPLES_JSONL} is missing or empty; create the schema header first.\n`,
    );
    return 2;
  }

  const t0 = Date.now();
  let completed = 0;
  let failed = 0;
  const sha = gitShortSha();

  for (const cell of cells) {
    for (const seed of SEEDS) {
      const key = `${cell.format}::${cell.algo}::${seed}`;
      if (done.has(key)) {
        process.stdout.write(
          `  [skip] ${cell.format}/${cell.algo}/seed=${seed} (already in JSONL)\n`,
        );
        continue;
      }
      const idx = completed + failed + 1;
      const elapsed = Date.now() - t0;
      const avg = idx > 1 ? elapsed / (idx - 1) : 0;
      const eta = avg * (remaining - (idx - 1));
      process.stdout.write(
        `  [${idx}/${remaining}] ${cell.format}/${cell.algo}/seed=${seed} ` +
          `(rank=${cell.rank}, ${cell.reason}) — elapsed=${fmtDuration(
            elapsed,
          )} eta=${fmtDuration(eta)}\n`,
      );
      if (DRY_RUN) {
        completed += 1;
        continue;
      }
      const result = runTrainer(cell.format, cell.algo, seed);
      if (!result) {
        failed += 1;
        continue;
      }
      const row: SampleRow = {
        format: cell.format,
        algo: cell.algo,
        seed_phi: seed,
        step: result.step,
        bpb: result.bpb,
        sha: result.sha || sha,
        source: SOURCE_TAG,
        timestamp: nowIso(),
      };
      appendFileSync(SAMPLES_JSONL, JSON.stringify(row) + "\n");
      done.add(key);
      completed += 1;
    }
  }

  const dt = Date.now() - t0;
  process.stdout.write(
    `\nL-MR-MATRIX-PRIORITY runner — DONE\n` +
      `  cells_completed_runs : ${completed}\n` +
      `  cells_failed_runs    : ${failed}\n` +
      `  total_runtime        : ${fmtDuration(dt)}\n` +
      `  output               : ${SAMPLES_JSONL}\n`,
  );
  return failed > 0 ? 1 : 0;
}

process.exit(main());
