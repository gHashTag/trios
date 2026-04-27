/**
 * TypeScript types for the trios MCP server.
 */

/**
 * Result of running a CLI command (R5 - honest passthrough)
 */
export interface CliResult {
  stdout: string;      // Verbatim stdout from CLI
  stderr: string;      // Verbatim stderr from CLI
  exitCode: number;    // Exact exit code from CLI (0, 1, 2)
  success: boolean;    // Derived from exitCode === 0
  triplets?: string[]; // Extracted R7 triplets (if any)
}

/**
 * Paths to the Rust binaries
 */
export interface BinaryPaths {
  tri: string;
  igla: string;
  iglaRace: string;
}

/**
 * Result of binary validation
 */
export interface ValidationResult {
  allPresent: boolean;
  missing: string[];
  results: Record<string, boolean>;
}

/**
 * MCP Tool response format
 */
export interface ToolResponse {
  content: Array<{
    type: 'text';
    text: string;
  }>;
  isError?: boolean;
}

/**
 * Common arguments for trios-igla tools
 */
export interface IglaCommonArgs {
  ledger?: string;
}

/**
 * Common arguments for trios-igla-race tools
 */
export interface IglaRaceCommonArgs {
  neonUrl?: string;
}
