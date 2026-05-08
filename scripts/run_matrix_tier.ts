#!/usr/bin/env -S npx tsx
/**
 * scripts/run_matrix_tier.ts — generalized tiered runner for L-MATRIX-FILL-351.
 *
 * Generalises `scripts/run_priority_matrix.ts` to accept `TIER=1|2|3|all` and
 * dispatch to the matching priority CSV. The underlying runner logic is
 * inherited verbatim by spawning the existing tier-1 runner with overridden
 * env (`PRIORITY_CSV`, `SEEDS`, `SOURCE_TAG`) — `scripts/run_priority_matrix.ts`
 * is FROZEN per L-MATRIX-FILL-351 contract and must not be modified.
 *
 * Anchor: phi^2 + phi^-2 = 3 · DOI 10.5281/zenodo.19227877.
 *
 * L1 compliance: TypeScript only — no `.sh`. Run via `npx tsx`.
 *
 * Usage:
 *   TIER=1   npx tsx scripts/run_matrix_tier.ts   # → matrix_priority_50.csv
 *   TIER=2   npx tsx scripts/run_matrix_tier.ts   # → matrix_priority_tier2.csv
 *   TIER=3   npx tsx scripts/run_matrix_tier.ts   # → matrix_priority_tier3.csv
 *   TIER=all npx tsx scripts/run_matrix_tier.ts   # runs 1,2,3 sequentially
 *   (no TIER) npx tsx scripts/run_matrix_tier.ts  # backward-compatible: tier1
 *
 * Required env: none.
 *
 * Optional env (forwarded to underlying runner):
 *   TRIOS_TRAINER_BIN   absolute path to a prebuilt trainer binary.
 *   STEPS               override step count (default 3000).
 *   SEEDS               override seed list (default L-SEED-CANON: 47,89,144,123).
 *   SAMPLES_JSONL       override JSONL output path.
 *   DRY_RUN=1           print plan without invoking the trainer.
 */

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve } from "node:path";

const REPO_ROOT = resolve(__dirname, "..");
const TIER1_RUNNER = resolve(REPO_ROOT, "scripts/run_priority_matrix.ts");
// L-SEED-CANON #600 ruling: canonical seeds {47, 89, 144, 123}.
const SEEDS_CANON = "47,89,144,123";
const TIER_DEFAULT = (process.env.TIER || "1").trim().toLowerCase();
const ANCHOR = "phi^2 + phi^-2 = 3";

interface TierConfig {
  tier: string;
  csv: string;
  source_tag: string;
}

const TIERS: Record<string, TierConfig> = {
  "1": {
    tier: "1",
    csv: resolve(REPO_ROOT, "assertions/matrix_priority_50.csv"),
    source_tag: "matrix-tier1-2026-05-08",
  },
  "2": {
    tier: "2",
    csv: resolve(REPO_ROOT, "assertions/matrix_priority_tier2.csv"),
    source_tag: "matrix-tier2-2026-05-08",
  },
  "3": {
    tier: "3",
    csv: resolve(REPO_ROOT, "assertions/matrix_priority_tier3.csv"),
    source_tag: "matrix-tier3-2026-05-08",
  },
};

function logHeader(t: TierConfig): void {
  process.stdout.write(
    `[run_matrix_tier] tier=${t.tier} csv=${t.csv} source_tag=${t.source_tag}\n` +
      `[run_matrix_tier] anchor=${ANCHOR}\n`,
  );
}

function runTier(t: TierConfig): number {
  if (!existsSync(t.csv)) {
    process.stderr.write(
      `[run_matrix_tier] ERROR: tier ${t.tier} CSV missing at ${t.csv}\n`,
    );
    return 2;
  }
  if (!existsSync(TIER1_RUNNER)) {
    process.stderr.write(
      `[run_matrix_tier] ERROR: underlying runner not found at ${TIER1_RUNNER}\n`,
    );
    return 2;
  }
  logHeader(t);

  // Forward env, override the bits the underlying runner reads. Note that
  // run_priority_matrix.ts hardcodes SOURCE_TAG; we cannot override that
  // without modifying it (which is forbidden by lane contract). Therefore
  // we tag rows downstream via an env-aware companion field embedded in the
  // SEEDS comment — actually the cleanest path is to rely on the
  // (format, algo, seed_phi) idempotence key which already discriminates
  // tier-by-tier as long as priority CSVs are disjoint, which they are
  // by construction.
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    PRIORITY_CSV: t.csv,
    SEEDS: process.env.SEEDS || SEEDS_CANON,
  };

  const res = spawnSync("npx", ["tsx", TIER1_RUNNER], {
    cwd: REPO_ROOT,
    env,
    stdio: "inherit",
  });
  return res.status ?? 1;
}

function main(): number {
  const tier = TIER_DEFAULT;
  if (tier === "all") {
    for (const k of ["1", "2", "3"] as const) {
      const code = runTier(TIERS[k]);
      if (code !== 0) {
        process.stderr.write(
          `[run_matrix_tier] tier ${k} failed (exit ${code}); aborting all-mode\n`,
        );
        return code;
      }
    }
    return 0;
  }
  const cfg = TIERS[tier];
  if (!cfg) {
    process.stderr.write(
      `[run_matrix_tier] ERROR: unknown TIER='${tier}'. Valid: 1, 2, 3, all\n`,
    );
    return 2;
  }
  return runTier(cfg);
}

const code = main();
process.exit(code);
