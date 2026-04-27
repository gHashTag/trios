/**
 * Generic CLI execution wrapper with exit code handling (R5 - honest passthrough)
 */

import { spawn } from 'node:child_process';
import type { CliResult } from '../types.js';

/**
 * Run a CLI command and return stdout, stderr, and exit code
 * Implements R5: Honest passthrough of CLI exit codes
 */
export async function runCli(
  binary: string,
  args: string[],
  env: Record<string, string> = {},
): Promise<CliResult> {
  return new Promise((resolve) => {
    let stdout = '';
    let stderr = '';

    const child = spawn(binary, args, {
      env: { ...process.env, ...env },
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    child.stdout?.on('data', (data) => {
      stdout += data.toString();
    });

    child.stderr?.on('data', (data) => {
      stderr += data.toString();
    });

    child.on('close', (code) => {
      const exitCode = code ?? 1;
      resolve({
        stdout,
        stderr,
        exitCode,
        success: exitCode === 0,
      });
    });

    child.on('error', (err) => {
      resolve({
        stdout: '',
        stderr: err.message,
        exitCode: 1,
        success: false,
      });
    });
  });
}

/**
 * Extract R7 triplet lines from stdout
 * Triplet format: BPB=<v> @ step=<N> seed=<S> sha=<7c> jsonl_row=<L> gate_status=<g>
 * Implements R7: R7 triplet stays property of wrapped binaries
 */
export function extractTriplets(stdout: string): string[] {
  const lines = stdout.split('\n');
  return lines.filter((line) => line.startsWith('BPB='));
}

/**
 * Build tool response from CLI result
 */
export function buildToolResponse(
  result: CliResult,
): { content: Array<{ type: 'text'; text: string }> } {
  const triplets = extractTriplets(result.stdout);

  const responseData: CliResult & { triplets?: string[] } = {
    stdout: result.stdout,
    stderr: result.stderr,
    exitCode: result.exitCode,
    success: result.success,
  };

  // Only include triplets if found
  if (triplets.length > 0) {
    responseData.triplets = triplets;
  }

  return {
    content: [
      {
        type: 'text',
        text: JSON.stringify(responseData, null, 2),
      },
    ],
  };
}
