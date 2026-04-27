/**
 * Configuration and binary path resolution for the trios MCP server.
 */

import path from 'node:path';
import { promises as fs } from 'node:fs';
import type { BinaryPaths, ValidationResult } from './types.js';

/**
 * Get binary paths from environment variables or defaults
 */
export function getBinaryPaths(): BinaryPaths {
  const repoRoot = process.env.TRIOS_REPO_ROOT || process.cwd();
  const releaseDir = path.join(repoRoot, 'target', 'release');

  return {
    tri: process.env.TRIOS_TRI_BIN || path.join(releaseDir, 'tri'),
    igla: process.env.TRIOS_IGLA_BIN || path.join(releaseDir, 'trios-igla'),
    iglaRace:
      process.env.TRIOS_IGLA_RACE_BIN ||
      path.join(releaseDir, 'trios-igla-race'),
  };
}

/**
 * Check if a file exists
 */
async function fileExists(filePath: string): Promise<boolean> {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

/**
 * Validate that all required binaries exist
 */
export async function validateBinaries(
  paths: BinaryPaths,
): Promise<ValidationResult> {
  const results = {
    tri: await fileExists(paths.tri),
    igla: await fileExists(paths.igla),
    iglaRace: await fileExists(paths.iglaRace),
  };

  const missing = Object.entries(results)
    .filter(([, exists]) => !exists)
    .map(([name]) => name);

  return {
    allPresent: missing.length === 0,
    missing,
    results,
  };
}

/**
 * Default ledger path for trios-igla tools
 */
export function getDefaultLedgerPath(): string {
  const repoRoot = process.env.TRIOS_REPO_ROOT || process.cwd();
  return path.join(repoRoot, 'assertions', 'seed_results.jsonl');
}

/**
 * Default embargo path for trios-igla check
 */
export function getDefaultEmbargoPath(): string {
  const repoRoot = process.env.TRIOS_REPO_ROOT || process.cwd();
  return path.join(repoRoot, 'assertions', 'embargo.txt');
}
