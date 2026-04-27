/**
 * Main entry point for the trios MCP server
 *
 * Implements:
 * - R1: Server is TypeScript on stdio; no Python
 * - R5: Honest passthrough of CLI exit codes (0, 1, 2)
 * - R7: R7 triplet emitted by wrapped binaries is forwarded byte-for-byte
 * - R9: igla_check exposes the embargo predicate
 */

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from '@modelcontextprotocol/sdk/types.js';
import { getToolDefinitions, getToolHandler } from './tools/index.js';
import { getBinaryPaths, validateBinaries } from './config.js';

async function main() {
  const server = new Server(
    {
      name: 'trios-mcp-server',
      version: '0.1.0',
    },
    {
      capabilities: {
        tools: {},
      },
    },
  );

  // Validate binaries on startup
  const paths = getBinaryPaths();
  const validation = await validateBinaries(paths);

  if (!validation.allPresent) {
    console.error(
      `trios-mcp: Warning - Missing binaries: ${validation.missing.join(', ')}`,
    );
    console.error('Build binaries with: cargo build --release');
    console.error(
      '  - tri: cargo build --release -p trios-cli --bin tri',
    );
    console.error(
      '  - trios-igla: cd trios-trainer-igla && cargo build --release --bin trios-igla',
    );
    console.error(
      '  - trios-igla-race: cargo build --release -p trios-igla-race --bin trios-igla-race',
    );
    // Continue anyway - tools will fail at runtime
  }

  // List tools handler
  server.setRequestHandler(ListToolsRequestSchema, async () => {
    return { tools: getToolDefinitions() };
  });

  // Call tool handler
  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;

    if (!name) {
      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(
              {
                stdout: '',
                stderr: 'Tool name is required',
                exitCode: 1,
                success: false,
              },
              null,
              2,
            ),
          },
        ],
      };
    }

    const handler = getToolHandler(name);

    if (!handler) {
      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(
              {
                stdout: '',
                stderr: `Unknown tool: ${name}`,
                exitCode: 1,
                success: false,
              },
              null,
              2,
            ),
          },
        ],
      };
    }

    try {
      return await handler(args || {});
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      return {
        content: [
          {
            type: 'text',
            text: JSON.stringify(
              {
                stdout: '',
                stderr: message,
                exitCode: 1,
                success: false,
              },
              null,
              2,
            ),
          },
        ],
      };
    }
  });

  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error('trios MCP server running on stdio');
}

main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
