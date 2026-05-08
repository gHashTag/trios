#!/usr/bin/env -S npx tsx
/**
 * scripts/postrun_sidecar.ts — long-running sidecar daemon for the
 * trios-mr-priority-runner JSONL retrieval lane (issue gHashTag/trios#598).
 *
 * Wakes every `INTERVAL_MIN` minutes (default 30), invokes
 * `scripts/postrun_commit_back.ts` as a child process, logs the result, and
 * sleeps. On `SIGTERM` / `SIGINT` performs one final flush before exiting.
 *
 * Designed to run as a separate Railway service alongside
 * `trios-mr-priority-runner` (service id 71f5aac2-d4d5-4640-8895-90ced5d4ea63),
 * with the same volume mount so it can read the runner's
 * `assertions/matrix_samples.jsonl`.
 *
 * Anchor: phi^2 + phi^-2 = 3 · DOI 10.5281/zenodo.19227877.
 *
 * L1 compliance: TypeScript only — no `.sh`. Run via `npx tsx` (or `tsx`
 * inside the container that has tsx pre-installed).
 *
 * Usage:
 *   GITHUB_TOKEN=ghp_... npx tsx scripts/postrun_sidecar.ts
 *
 * Required env:
 *   GITHUB_TOKEN        forwarded to the child commit-back process.
 *
 * Optional env:
 *   INTERVAL_MIN        sleep between iterations (default 30 min).
 *   MAX_ITERATIONS      cap on iterations for tests / smoke runs (default
 *                       0 = unbounded).
 *   COMMIT_BACK_PATH    override path to commit-back script (default
 *                       scripts/postrun_commit_back.ts).
 *   DRY_RUN=1           forwarded to the child; sidecar still runs the
 *                       wake loop normally.
 */

import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve } from "node:path";

const REPO_ROOT = resolve(__dirname, "..");
const COMMIT_BACK = resolve(
  REPO_ROOT,
  process.env.COMMIT_BACK_PATH || "scripts/postrun_commit_back.ts",
);
const INTERVAL_MIN = Math.max(1, Number(process.env.INTERVAL_MIN || 30));
const INTERVAL_MS = INTERVAL_MIN * 60 * 1000;
const MAX_ITERATIONS = Math.max(0, Number(process.env.MAX_ITERATIONS || 0));
const ANCHOR = "phi^2 + phi^-2 = 3";

if (!existsSync(COMMIT_BACK)) {
  process.stderr.write(`[sidecar] commit-back script not found at ${COMMIT_BACK}\n`);
  process.exit(2);
}

let stopRequested = false;
let inFlight = false;
let timer: NodeJS.Timeout | null = null;

function log(msg: string): void {
  const ts = new Date().toISOString();
  process.stdout.write(`[sidecar ${ts}] ${msg}\n`);
}

function runCommitBack(reason: string): Promise<number> {
  return new Promise((resolveFn) => {
    if (inFlight) {
      log(`runCommitBack: skipping (${reason}); previous run still in-flight`);
      resolveFn(0);
      return;
    }
    inFlight = true;
    log(`runCommitBack: spawning (${reason}) — anchor ${ANCHOR}`);
    const child = spawn("npx", ["tsx", COMMIT_BACK], {
      env: process.env,
      stdio: "inherit",
    });
    child.on("exit", (code) => {
      inFlight = false;
      log(`runCommitBack: exit code ${code ?? "null"} (${reason})`);
      resolveFn(code ?? 0);
    });
    child.on("error", (err) => {
      inFlight = false;
      log(`runCommitBack: spawn error (${reason}): ${err.message}`);
      resolveFn(1);
    });
  });
}

async function flushAndExit(signal: string): Promise<never> {
  log(`signal ${signal}: requesting graceful flush`);
  stopRequested = true;
  if (timer) clearTimeout(timer);
  // Wait for any in-flight run, then perform one final commit-back.
  // If the in-flight run is still going, give it a chance; commit-back is
  // itself idempotent so a final flush after it is safe.
  while (inFlight) {
    await new Promise((r) => setTimeout(r, 250));
  }
  await runCommitBack(`final-flush:${signal}`);
  log("flush complete; exiting");
  process.exit(0);
}

process.on("SIGTERM", () => {
  void flushAndExit("SIGTERM");
});
process.on("SIGINT", () => {
  void flushAndExit("SIGINT");
});

async function main(): Promise<void> {
  log(
    `started — interval=${INTERVAL_MIN}min commit_back=${COMMIT_BACK} max_iter=${MAX_ITERATIONS || "∞"}`,
  );

  // First iteration runs immediately; subsequent ones honour INTERVAL.
  let iter = 0;
  while (!stopRequested) {
    iter += 1;
    await runCommitBack(`iter=${iter}`);
    if (MAX_ITERATIONS > 0 && iter >= MAX_ITERATIONS) {
      log(`reached MAX_ITERATIONS=${MAX_ITERATIONS}; exiting`);
      break;
    }
    if (stopRequested) break;
    await new Promise<void>((resolveFn) => {
      timer = setTimeout(() => {
        timer = null;
        resolveFn();
      }, INTERVAL_MS);
    });
  }
}

main().catch((err) => {
  process.stderr.write(`[sidecar] FATAL: ${(err as Error).stack || err}\n`);
  process.exit(1);
});
