/**
 * tri CLI tool definitions and handlers
 *
 * Tools:
 * - tri_railway_deploy: Deploy N Railway instances
 * - tri_railway_status: Show Railway deployment status
 * - tri_train: Train CPU n-gram model locally
 * - tri_race_init: Initialize IGLA RACE with Optuna study
 * - tri_race_status: Show live leaderboard
 */

import { runCli, buildToolResponse } from './runner.js';
import type { Tool, ToolHandler } from '../tools/index.js';

/**
 * Build tri railway deploy command args
 */
function buildRailwayDeployArgs(args: Record<string, unknown>): string[] {
  const cmdArgs: string[] = ['railway', 'deploy'];

  if (args.seeds && typeof args.seeds === 'number') {
    cmdArgs.push('--seeds', String(args.seeds));
  }
  if (args.start_seed && typeof args.start_seed === 'number') {
    cmdArgs.push('--start-seed', String(args.start_seed));
  }
  if (args.dry_run === true) {
    cmdArgs.push('--dry-run');
  }

  return cmdArgs;
}

/**
 * Build tri train command args
 */
function buildTrainArgs(args: Record<string, unknown>): string[] {
  const cmdArgs: string[] = ['train'];

  if (args.steps && typeof args.steps === 'number') {
    cmdArgs.push('--steps', String(args.steps));
  }
  if (args.hidden && typeof args.hidden === 'number') {
    cmdArgs.push('--hidden', String(args.hidden));
  }
  if (args.lr && typeof args.lr === 'number') {
    cmdArgs.push('--lr', String(args.lr));
  }
  if (args.seeds && typeof args.seeds === 'string') {
    cmdArgs.push('--seeds', args.seeds);
  }
  if (args.activation && typeof args.activation === 'string') {
    cmdArgs.push('--activation', args.activation);
  }
  if (typeof args.parallel === 'boolean') {
    cmdArgs.push('--parallel', String(args.parallel));
  }
  if (typeof args.residual === 'boolean') {
    cmdArgs.push('--residual', String(args.residual));
  }
  if (args.dropout && typeof args.dropout === 'string') {
    cmdArgs.push('--dropout', args.dropout);
  }
  if (args.warmup && typeof args.warmup === 'string') {
    cmdArgs.push('--warmup', args.warmup);
  }
  if (args.wd && typeof args.wd === 'string') {
    cmdArgs.push('--wd', args.wd);
  }

  return cmdArgs;
}

/**
 * Build tri race init command args
 */
function buildRaceInitArgs(args: Record<string, unknown>): string[] {
  const cmdArgs: string[] = ['race', 'init'];

  if (args.study && typeof args.study === 'string') {
    cmdArgs.push('--study', args.study);
  } else {
    cmdArgs.push('--study', 'igla-race');
  }
  if (args.neon_url && typeof args.neon_url === 'string') {
    cmdArgs.push('--neon-url', args.neon_url);
  }

  return cmdArgs;
}

/**
 * Build tri race status command args
 */
function buildRaceStatusArgs(args: Record<string, unknown>): string[] {
  const cmdArgs: string[] = ['race', 'status'];

  if (args.limit && typeof args.limit === 'number') {
    cmdArgs.push('--limit', String(args.limit));
  } else {
    cmdArgs.push('--limit', '10');
  }

  return cmdArgs;
}

/**
 * Create a tool handler for tri commands
 */
function createTriHandler(
  commandBuilder: (args: Record<string, unknown>) => string[],
): ToolHandler {
  return async (args: Record<string, unknown>, binaryPath: string) => {
    const cliArgs = commandBuilder(args);
    const result = await runCli(binaryPath, cliArgs);
    return buildToolResponse(result);
  };
}

/**
 * Tool definitions for tri CLI
 */
export const triTools: Tool[] = [
  {
    name: 'tri_railway_deploy',
    description:
      'Deploy N Railway instances with unique seeds for IGLA training. Max 4 instances per Railway law (L-R1).',
    inputSchema: {
      type: 'object',
      properties: {
        seeds: {
          type: 'number',
          description:
            'Number of Railway instances to deploy (1-4, default: 1)',
          default: 1,
          minimum: 1,
          maximum: 4,
        },
        start_seed: {
          type: 'number',
          description: 'Starting seed value (default: 42)',
          default: 42,
        },
        dry_run: {
          type: 'boolean',
          description: 'Show what would be deployed without deploying',
          default: false,
        },
      },
      required: [],
    },
    handler: createTriHandler(buildRailwayDeployArgs),
  },
  {
    name: 'tri_railway_status',
    description:
      'Show Railway deployment status by calling the Railway CLI status command.',
    inputSchema: {
      type: 'object',
      properties: {},
      required: [],
    },
    handler: async (_args, binaryPath) => {
      const result = await runCli(binaryPath, ['railway', 'status']);
      return buildToolResponse(result);
    },
  },
  {
    name: 'tri_train',
    description:
      'Train CPU n-gram model with specified hyperparameters. Runs training across multiple seeds in parallel by default.',
    inputSchema: {
      type: 'object',
      properties: {
        steps: {
          type: 'number',
          description: 'Number of training steps (default: 12000)',
          default: 12000,
        },
        hidden: {
          type: 'number',
          description: 'Hidden layer size (default: 128)',
          default: 128,
        },
        lr: {
          type: 'number',
          description: 'Learning rate (default: 0.004)',
          default: 0.004,
        },
        seeds: {
          type: 'string',
          description: 'Comma-separated seed values (default: 42,43,44)',
          default: '42,43,44',
        },
        activation: {
          type: 'string',
          description: 'Activation function (default: relu)',
          default: 'relu',
          enum: ['relu', 'gelu', 'tanh', 'sigmoid'],
        },
        parallel: {
          type: 'boolean',
          description: 'Run seeds in parallel (default: true)',
          default: true,
        },
        residual: {
          type: 'boolean',
          description: 'Use residual connections (default: false)',
          default: false,
        },
        dropout: {
          type: 'string',
          description: 'Dropout rate (default: 0.0)',
          default: '0.0',
        },
        warmup: {
          type: 'string',
          description: 'Warmup steps (default: 0)',
          default: '0',
        },
        wd: {
          type: 'string',
          description: 'Weight decay (default: 0.04)',
          default: '0.04',
        },
      },
      required: [],
    },
    handler: createTriHandler(buildTrainArgs),
  },
  {
    name: 'tri_race_init',
    description:
      'Initialize IGLA RACE with Optuna study. Sets up the distributed hyperparameter optimization race in Neon PostgreSQL.',
    inputSchema: {
      type: 'object',
      properties: {
        study: {
          type: 'string',
          description: 'Optuna study name (default: igla-race)',
          default: 'igla-race',
        },
        neon_url: {
          type: 'string',
          description:
            'Neon PostgreSQL connection URL (optional, overrides NEON_DATABASE_URL env var)',
        },
      },
      required: [],
    },
    handler: createTriHandler(buildRaceInitArgs),
  },
  {
    name: 'tri_race_status',
    description:
      'Show live leaderboard from the IGLA RACE. Displays top trials, their BPB scores, agents, and race statistics.',
    inputSchema: {
      type: 'object',
      properties: {
        limit: {
          type: 'number',
          description: 'Number of leaderboard entries to show (default: 10)',
          default: 10,
          minimum: 1,
        },
      },
      required: [],
    },
    handler: createTriHandler(buildRaceStatusArgs),
  },
];
