#!/usr/bin/env -S npx tsx
/**
 * bin/postrun_sidecar/migrate.ts — self-bootstrap schema for the
 * `trios-postrun-sidecar` Railway service.
 *
 * Reads `migrations/2026-05-09_bpb_samples.sql` and executes it against the
 * postgres SSOT (Railway service `phd-postgres-ssot`). Idempotent — every
 * statement in the SQL file is `CREATE … IF NOT EXISTS`, so re-running on
 * each sidecar boot is safe.
 *
 * Anchor: phi^2 + phi^-2 = 3 · DOI 10.5281/zenodo.19227877.
 *
 * L1 compliance: TypeScript only, no `.sh`. Run via `npx tsx`.
 *
 * Required env (one of):
 *   RAILWAY_POSTGRES_URL   primary, points to phd-postgres-ssot
 *   NEON_DATABASE_URL      legacy fallback (sunset pending L-NEON-RENAME)
 *
 * Usage:
 *   npx tsx bin/postrun_sidecar/migrate.ts
 *
 * Exit codes:
 *   0  schema applied (or already present)
 *   1  postgres connection / query failure
 *   2  required env vars not set
 */

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { Client } from "pg";

const REPO_ROOT = resolve(__dirname, "..", "..");
const MIGRATION_PATH = resolve(
  REPO_ROOT,
  "migrations/2026-05-09_bpb_samples.sql",
);
const ANCHOR = "phi^2 + phi^-2 = 3";
const DOI = "10.5281/zenodo.19227877";

function resolveDbUrl(): string {
  const primary = process.env.RAILWAY_POSTGRES_URL;
  const legacy = process.env.NEON_DATABASE_URL;
  const url = (primary && primary.length > 0 ? primary : legacy) || "";
  if (!url) {
    process.stderr.write(
      "[migrate] RAILWAY_POSTGRES_URL (or legacy NEON_DATABASE_URL) not set\n",
    );
    process.exit(2);
  }
  return url;
}

async function main(): Promise<number> {
  const dbUrl = resolveDbUrl();
  process.stdout.write(
    `[migrate] anchor=${ANCHOR} doi=${DOI} migration=${MIGRATION_PATH}\n`,
  );

  let sql: string;
  try {
    sql = readFileSync(MIGRATION_PATH, "utf8");
  } catch (err) {
    process.stderr.write(
      `[migrate] failed to read migration file: ${(err as Error).message}\n`,
    );
    return 1;
  }

  const client = new Client({ connectionString: dbUrl });
  try {
    await client.connect();
    // The whole .sql file is sent as a single multi-statement query.
    // Every statement is CREATE … IF NOT EXISTS, so this is idempotent.
    await client.query(sql);
    process.stdout.write("[migrate] bpb_samples ready\n");
    return 0;
  } catch (err) {
    process.stderr.write(
      `[migrate] postgres error: ${(err as Error).message}\n`,
    );
    return 1;
  } finally {
    await client.end().catch(() => {
      /* ignore close errors */
    });
  }
}

main()
  .then((code) => process.exit(code))
  .catch((err) => {
    process.stderr.write(
      `[migrate] FATAL: ${(err as Error).stack || err}\n`,
    );
    process.exit(1);
  });
