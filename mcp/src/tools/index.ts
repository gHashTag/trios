/**
 * Tool registry with JSON schemas for all MCP tools
 */

import { triTools } from '../cli/tri.js';
import { iglaTools } from '../cli/igla.js';
import { iglaRaceTools } from '../cli/igla-race.js';
import type { BinaryPaths } from '../types.js';

/**
 * Tool definition interface
 */
export interface Tool {
  name: string;
  description: string;
  inputSchema: {
    type: 'object';
    properties: Record<string, unknown>;
    required: string[];
  };
  handler: (
    args: Record<string, unknown>,
    binaryPath: string,
  ) => Promise<{ content: Array<{ type: 'text'; text: string }> }>;
}

/**
 * Type for tool handler function
 */
export type ToolHandler = (
  args: Record<string, unknown>,
  binaryPath: string,
) => Promise<{ content: Array<{ type: 'text'; text: string }> }>;

/**
 * All tools mapped by name
 */
const toolMap = new Map<string, Tool>();

/**
 * Register all tools
 */
function registerTools(tools: Tool[]): void {
  for (const tool of tools) {
    toolMap.set(tool.name, tool);
  }
}

// Register all CLI tools
registerTools(triTools);
registerTools(iglaTools);
registerTools(iglaRaceTools);

/**
 * Get all tool definitions for tools/list
 */
export function getToolDefinitions(): Array<{
  name: string;
  description: string;
  inputSchema: {
    type: 'object';
    properties: Record<string, unknown>;
    required: string[];
  };
}> {
  return Array.from(toolMap.values()).map((tool) => ({
    name: tool.name,
    description: tool.description,
    inputSchema: tool.inputSchema,
  }));
}

/**
 * Get a tool handler by name
 */
export function getToolHandler(
  name: string,
): ((args: Record<string, unknown>) => Promise<{
  content: Array<{ type: 'text'; text: string }>;
}>) | null {
  const tool = toolMap.get(name);
  if (!tool) {
    return null;
  }

  // Determine which binary to use based on tool name prefix
  const getBinaryPath = (paths: BinaryPaths): string => {
    if (name.startsWith('tri_')) {
      return paths.tri;
    } else if (name.startsWith('igla_')) {
      return name.startsWith('igla_race') ? paths.iglaRace : paths.igla;
    }
    throw new Error(`Unknown tool prefix: ${name}`);
  };

  return async (args: Record<string, unknown>) => {
    const { getBinaryPaths } = await import('../config.js');
    const paths = getBinaryPaths();
    const binaryPath = getBinaryPath(paths);
    return tool.handler(args, binaryPath);
  };
}
