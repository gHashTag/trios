/**
 * trios-igla CLI tool definitions and handlers
 *
 * Tools:
 * - igla_search: Filter ledger rows, emit R7 triplets (exit 2 if no matches)
 * - igla_list: Last N rows as R7 triplets
 * - igla_gate: Gate-2 quorum check (exit 2 if NOT YET)
 * - igla_check: Embargo refusal R9 (exit 1 if embargoed)
 * - igla_triplet: Get R7 triplet by row index
 *
 * Exit codes (R5 - honest passthrough):
 * - 0: Success
 * - 1: Embargo refused (igla check) or general error
 * - 2: No match (igla search) or NOT YET (igla gate)
 *
 * R7: R7 triplet stays property of wrapped binaries; MCP layer never invents one
 * R9: igla_check exposes the embargo predicate
 */

import { runCli, buildToolResponse } from './runner.js';
import {
  getDefaultLedgerPath,
  getDefaultEmbargoPath,
} from '../config.js';
import type { Tool } from '../tools/index.js';

/**
 * Build trios-igla search command args
 */
function buildSearchArgs(args: Record<string, unknown>): string[] {
  const cmdArgs: string[] = ['search'];

  if (args.seed && typeof args.seed === 'number') {
    cmdArgs.push('--seed', String(args.seed));
  }
  if (args.bpb_max && typeof args.bpb_max === 'number') {
    cmdArgs.push('--bpb-max', String(args.bpb_max));
  }
  if (args.step_min && typeof args.step_min === 'number') {
    cmdArgs.push('--step-min', String(args.step_min));
  }
  if (args.sha && typeof args.sha === 'string') {
    cmdArgs.push('--sha', args.sha);
  }
  if (args.gate_status && typeof args.gate_status === 'string') {
    cmdArgs.push('--gate-status', args.gate_status);
  }
  const ledger = args.ledger
    ? String(args.ledger)
    : getDefaultLedgerPath();
  cmdArgs.push('--ledger', ledger);

  return cmdArgs;
}

/**
 * Build trios-igla list command args
 */
function buildListArgs(args: Record<string, unknown>): string[] {
  const cmdArgs: string[] = ['list'];

  const last = args.last && typeof args.last === 'number' ? args.last : 10;
  cmdArgs.push('--last', String(last));

  const ledger = args.ledger
    ? String(args.ledger)
    : getDefaultLedgerPath();
  cmdArgs.push('--ledger', ledger);

  return cmdArgs;
}

/**
 * Build trios-igla gate command args
 */
function buildGateArgs(args: Record<string, unknown>): string[] {
  const cmdArgs: string[] = ['gate'];

  const target =
    args.target && typeof args.target === 'number' ? args.target : 1.85;
  cmdArgs.push('--target', String(target));

  const ledger = args.ledger
    ? String(args.ledger)
    : getDefaultLedgerPath();
  cmdArgs.push('--ledger', ledger);

  return cmdArgs;
}

/**
 * Build trios-igla check command args
 */
function buildCheckArgs(args: Record<string, unknown>): string[] {
  const cmdArgs: string[] = ['check'];

  if (!args.sha || typeof args.sha !== 'string') {
    throw new Error('igla_check requires sha argument');
  }
  cmdArgs.push(args.sha);

  const embargo = args.embargo
    ? String(args.embargo)
    : getDefaultEmbargoPath();
  cmdArgs.push('--embargo', embargo);

  return cmdArgs;
}

/**
 * Build trios-igla triplet command args
 */
function buildTripletArgs(args: Record<string, unknown>): string[] {
  const cmdArgs: string[] = ['triplet'];

  if (args.row_index === undefined || typeof args.row_index !== 'number') {
    throw new Error('igla_triplet requires row_index argument');
  }
  cmdArgs.push(String(args.row_index));

  const ledger = args.ledger
    ? String(args.ledger)
    : getDefaultLedgerPath();
  cmdArgs.push('--ledger', ledger);

  return cmdArgs;
}

/**
 * Create a tool handler for trios-igla commands
 */
function createIglaHandler(
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
 * Tool definitions for trios-igla CLI
 */
export const iglaTools: Tool[] = [
  {
    name: 'igla_search',
    description:
      'Search the IGLA RACE ledger for matching rows. Emits R7 triplets (BPB=<v> @ step=<N> seed=<S> sha=<7c> jsonl_row=<L> gate_status=<g>). Exit code 2 if no matches found.',
    inputSchema: {
      type: 'object',
      properties: {
        seed: {
          type: 'number',
          description: 'Filter by seed value',
        },
        bpb_max: {
          type: 'number',
          description: 'Filter by maximum BPB value',
        },
        step_min: {
          type: 'number',
          description: 'Filter by minimum step count',
        },
        sha: {
          type: 'string',
          description: 'Filter by SHA prefix',
        },
        gate_status: {
          type: 'string',
          description: 'Filter by gate status',
        },
        ledger: {
          type: 'string',
          description:
            'Path to ledger file (default: assertions/seed_results.jsonl)',
        },
      },
      required: [],
    },
    handler: createIglaHandler(buildSearchArgs),
  },
  {
    name: 'igla_list',
    description:
      'Print the last N rows from the IGLA RACE ledger in canonical R7 triplet form.',
    inputSchema: {
      type: 'object',
      properties: {
        last: {
          type: 'number',
          description: 'Number of rows to show (default: 10)',
          default: 10,
          minimum: 1,
        },
        ledger: {
          type: 'string',
          description:
            'Path to ledger file (default: assertions/seed_results.jsonl)',
        },
      },
      required: [],
    },
    handler: createIglaHandler(buildListArgs),
  },
  {
    name: 'igla_gate',
    description:
      'Gate-2 quorum check. PASS iff >=3 distinct seeds satisfy bpb<target AND step>=4000. Exit code 2 if NOT YET (quorum not reached).',
    inputSchema: {
      type: 'object',
      properties: {
        target: {
          type: 'number',
          description: 'Target BPB threshold (default: 1.85)',
          default: 1.85,
        },
        ledger: {
          type: 'string',
          description:
            'Path to ledger file (default: assertions/seed_results.jsonl)',
        },
      },
      required: [],
    },
    handler: createIglaHandler(buildGateArgs),
  },
  {
    name: 'igla_check',
    description:
      'Embargo refusal check (R9). Exit code 1 if the SHA is on the embargo list, 0 if clean. This implements the embargo predicate for Gate-2 compliance.',
    inputSchema: {
      type: 'object',
      properties: {
        sha: {
          type: 'string',
          description: 'SHA to check against embargo list',
        },
        embargo: {
          type: 'string',
          description:
            'Path to embargo file (default: assertions/embargo.txt)',
        },
      },
      required: ['sha'],
    },
    handler: createIglaHandler(buildCheckArgs),
  },
  {
    name: 'igla_triplet',
    description:
      'Print the canonical R7 triplet for a specific row index (0-based).',
    inputSchema: {
      type: 'object',
      properties: {
        row_index: {
          type: 'number',
          description: 'Row index (0-based)',
        },
        ledger: {
          type: 'string',
          description:
            'Path to ledger file (default: assertions/seed_results.jsonl)',
        },
      },
      required: ['row_index'],
    },
    handler: createIglaHandler(buildTripletArgs),
  },
];
