/**
 * trios-igla-race CLI tool definitions and handlers
 *
 * Tools:
 * - igla_race_start: Start ASHA worker for hyperparameter optimization
 * - igla_race_status: Show status from Neon PostgreSQL
 * - igla_race_best: Show best trial from Neon PostgreSQL
 */

import { runCli, buildToolResponse } from './runner.js';
import type { Tool } from '../tools/index.js';

/**
 * Build trios-igla-race start command args
 */
function buildRaceStartArgs(args: Record<string, unknown>): string[] {
  const cmdArgs: string[] = ['start'];

  const machine = args.machine && typeof args.machine === 'string' ? args.machine : 'local';
  cmdArgs.push('--machine', machine);

  const workers = args.workers && typeof args.workers === 'number' ? args.workers : 4;
  cmdArgs.push('--workers', String(workers));

  return cmdArgs;
}

/**
 * Create a tool handler for trios-igla-race commands
 */
function createIglaRaceHandler(
  commandBuilder: (args: Record<string, unknown>) => string[],
): (args: Record<string, unknown>, binaryPath: string) => Promise<{
  content: Array<{ type: 'text'; text: string }>;
}> {
  return async (args: Record<string, unknown>, binaryPath: string) => {
    const cliArgs = commandBuilder(args);
    const result = await runCli(binaryPath, cliArgs);
    return buildToolResponse(result);
  };
}

/**
 * Tool definitions for trios-igla-race CLI
 */
export const iglaRaceTools: Tool[] = [
  {
    name: 'igla_race_start',
    description:
      'Start ASHA worker for IGLA RACE hyperparameter optimization. Requires NEON_URL env var to be set.',
    inputSchema: {
      type: 'object',
      properties: {
        machine: {
          type: 'string',
          description: 'Machine ID for worker identification (default: local)',
          default: 'local',
        },
        workers: {
          type: 'number',
          description: 'Number of worker processes (default: 4)',
          default: 4,
          minimum: 1,
        },
      },
      required: [],
    },
    handler: createIglaRaceHandler(buildRaceStartArgs),
  },
  {
    name: 'igla_race_status',
    description:
      'Show IGLA RACE status from Neon PostgreSQL database. Requires NEON_URL env var.',
    inputSchema: {
      type: 'object',
      properties: {},
      required: [],
    },
    handler: async (_args, binaryPath) => {
      const result = await runCli(binaryPath, ['status']);
      return buildToolResponse(result);
    },
  },
  {
    name: 'igla_race_best',
    description:
      'Show the best trial in the IGLA RACE from Neon PostgreSQL database. Requires NEON_URL env var.',
    inputSchema: {
      type: 'object',
      properties: {},
      required: [],
    },
    handler: async (_args, binaryPath) => {
      const result = await runCli(binaryPath, ['best']);
      return buildToolResponse(result);
    },
  },
];
