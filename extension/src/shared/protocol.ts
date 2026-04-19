/// Protocol helpers for Trinity Agent Bridge

import type { AgentStatus, AgentState, IssueStatus } from './types';

export type BridgeMessage =
  | { type: 'send_command', payload: SendCommandPayload }
  | { type: 'claim_issue', payload: ClaimIssuePayload }
  | { type: 'update_status', payload: UpdateStatusPayload }
  | { type: 'list_agents', payload: {} }
  | { type: 'board_state', payload: BoardStatePayload }
  | { type: 'agent_event', payload: AgentEventPayload }
  | { type: 'command_delivered', payload: CommandDeliveredPayload }
  | { type: 'error', payload: ErrorPayload };

export interface SendCommandPayload {
  target: string;
  command: string;
  issue_comment?: boolean;
}

export interface ClaimIssuePayload {
  agent_id: string;
  issue_number: number;
  branch: string;
}

export interface UpdateStatusPayload {
  agent_id: string;
  status: AgentStatus;
  message: string;
}

export interface BoardStatePayload {
  agents: AgentState[];
  issues?: IssueStatus[];
}

export interface AgentEventPayload {
  agent_id: string;
  event: 'claimed' | 'done' | 'blocked';
  issue_number: number;
  message: string;
}

export interface CommandDeliveredPayload {
  target: string;
  success: boolean;
  error?: string;
}

export interface ErrorPayload {
  code: string;
  message: string;
}

// Re-export IssueStatus from types.ts to avoid conflict
export type { IssueStatus } from './types';

export class MessageHandler {
  parse(data: unknown): BridgeMessage | null {
    if (typeof data !== 'object' || data === null) {
      return null;
    }

    const msg = data as Record<string, unknown>;
    if (typeof msg.type !== 'string' || !msg.payload) {
      return null;
    }

    const type = msg.type as BridgeMessage['type'];
    return { type, payload: msg.payload as any };
  }
}
