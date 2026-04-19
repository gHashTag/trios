/**
 * Wire protocol types for Trinity Agent Bridge.
 *
 * This module defines the message types that flow between:
 * - Chrome Extension (service-worker, content scripts, popup)
 * - Rust WebSocket server (trios-bridge)
 * - CLI commands (tri bridge)
 *
 * All types must be kept in sync with the Rust types in
 * crates/trios-bridge/src/protocol.rs.
 */

/// Agent ID type.
export type AgentId = string;

/// Agent ID or "broadcast" for sending to all agents.
export type AgentIdOrBroadcast = string;

/// Agent status - MUST match Rust protocol.rs exactly.
export type AgentStatus = 'idle' | 'claiming' | 'working' | 'blocked' | 'done';

/// Get the emoji for this status.
export function getAgentStatusEmoji(status: AgentStatus): string {
  switch (status) {
    case 'idle':
      return "🟢";
    case 'claiming':
      return "🟡";
    case 'working':
      return "🔵";
    case 'blocked':
      return "🔴";
    case 'done':
      return "✅";
  }
}

/// Agent state - represents an agent's current status.
export interface AgentState {
  /// Unique agent identifier
  id: AgentId;
  /// Human-readable agent name
  name: string;
  /// GitHub issue number currently claimed (if any)
  issue?: number;
  /// Current status of the agent
  status: AgentStatus;
  /// Git branch the agent is working on (if any)
  branch?: string;
  /// ISO timestamp of last update
  lastUpdate: string;
  /// Last status message or task description
  message: string;
}

/// Issue status for the child issues tracker.
export interface IssueStatus {
  /// Issue number
  number: number;
  /// Issue title
  title: string;
  /// Current status: "todo", "in_progress", "blocked", "done"
  status: string;
}

/// Agent event type.
export type AgentEvent = 'claimed' | 'done' | 'blocked';

// ============================================================================
// Messages: Client → Server
// ============================================================================

/// Send a command to a specific agent or broadcast to all.
export interface SendCommandMsg {
  /// Target agent ID or "broadcast" for all agents
  target: AgentIdOrBroadcast;
  /// Command text to send
  command: string;
  /// Also post this as a comment to GitHub issue?
  issueComment?: boolean;
}

/// Claim a GitHub issue for work.
export interface ClaimIssueMsg {
  /// Agent ID claiming the issue
  agentId: AgentId;
  /// GitHub issue number to claim
  issueNumber: number;
  /// Git branch to work on
  branch: string;
}

/// Update agent status.
export interface UpdateStatusMsg {
  /// Agent ID to update
  agentId: AgentId;
  /// New status
  status: AgentStatus;
  /// Status message
  message: string;
}

/// List connected agents.
export interface ListAgentsMsg {}

// ============================================================================
// Messages: Server → Client
// ============================================================================

/// Full board state - all agents and issues.
export interface BoardStateMsg {
  /// List of all connected agents
  agents: AgentState[];
  /// List of issue #30 child issues status
  issues: IssueStatus[];
}

/// An agent event occurred.
export interface AgentEventMsg {
  /// Agent that triggered the event
  agentId: AgentId;
  /// Event type
  event: AgentEvent;
  /// Related issue number
  issueNumber: number;
  /// Event message
  message: string;
}

/// Command was delivered to agent(s).
export interface CommandDeliveredMsg {
  /// Target agent ID
  target: AgentId;
  /// Whether delivery succeeded
  success: boolean;
  /// Error message if success is false
  error?: string;
}

/// Error response from server.
export interface ErrorMsg {
  /// Error code
  code: string;
  /// Human-readable error message
  message: string;
}

// ============================================================================
// Unified message type for WebSocket communication
// ============================================================================

/// Unified message type for WebSocket communication.
export type BridgeMessage =
  | { type: "send_command"; data: SendCommandMsg }
  | { type: "claim_issue"; data: ClaimIssueMsg }
  | { type: "update_status"; data: UpdateStatusMsg }
  | { type: "list_agents"; data: ListAgentsMsg }
  | { type: "board_state"; data: BoardStateMsg }
  | { type: "agent_event"; data: AgentEventMsg }
  | { type: "command_delivered"; data: CommandDeliveredMsg }
  | { type: "error"; data: ErrorMsg };

// ============================================================================
// Constants
// ============================================================================

/// Default WebSocket port: 7474 (T-R-I-N in phone digits).
export const DEFAULT_PORT = 7474;

/// WebSocket URL for local connection.
export const LOCAL_WS_URL = `ws://localhost:${DEFAULT_PORT}`;
