/// Trinity Agent Bridge - Chrome Extension Service Worker
/// WebSocket client connecting to Rust server on localhost:7474

import type { AgentState, AgentStatus } from '../shared/types';
import type { BridgeMessage } from '../shared/protocol';
import { MessageHandler } from '../shared/protocol';

const WS_URL = 'ws://localhost:7474';

let ws: WebSocket | null = null;
let reconnectTimer: number | null = null;

// Agent state cache
let agentStates: Map<string, AgentState> = new Map();
let listeners: Set<(state: Map<string, AgentState>) => void> = new Set();

export function getAgentStates(): Map<string, AgentState> {
  return new Map(agentStates);
}

export function subscribeToStateChanges(
  callback: (state: Map<string, AgentState>) => void
): () => void {
  listeners.add(callback);
  callback(new Map(agentStates));
  return () => listeners.delete(callback);
}

function notifyStateChange(): void {
  const currentState = new Map(agentStates);
  listeners.forEach(cb => cb(currentState));
}

// WebSocket connection management
function connect(): void {
  if (ws?.readyState === WebSocket.OPEN) {
    return;
  }

  console.log('[Trinity] Connecting to', WS_URL);

  ws = new WebSocket(WS_URL);

  ws.onopen = () => {
    console.log('[Trinity] Connected to bridge server');
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }

    // Request initial board state
    sendMessage({ type: 'list_agents', payload: {} });
  };

  ws.onclose = (event) => {
    console.log('[Trinity] Disconnected', event.code, event.reason);
    ws = null;
    reconnectTimer = window.setTimeout(() => {
      console.log('[Trinity] Reconnecting...');
      connect();
    }, 3000);
  };

  ws.onerror = (error) => {
    console.error('[Trinity] WebSocket error:', error);
  };

  ws.onmessage = async (event) => {
    try {
      const data = JSON.parse(event.data);
      const handler = new MessageHandler();
      const message = handler.parse(data);

      if (!message) {
        console.warn('[Trinity] Unknown message:', data);
        return;
      }

      await handleMessage(message);
    } catch (e) {
      console.error('[Trinity] Failed to handle message:', e);
    }
  };
}

function disconnect(): void {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }

  if (ws) {
    ws.close();
    ws = null;
  }
}

function sendMessage(msg: BridgeMessage): void {
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(msg));
  } else {
    console.warn('[Trinity] Not connected, cannot send:', msg);
  }
}

// Message handling
async function handleMessage(message: BridgeMessage): Promise<void> {
  switch (message.type) {
    case 'board_state':
      handleBoardState(message.payload);
      break;

    case 'agent_event':
      handleAgentEvent(message.payload);
      break;

    case 'command_delivered':
      console.log('[Trinity] Command delivered:', message.payload);
      break;

    case 'error':
      console.error('[Trinity] Server error:', message.payload);
      break;

    default:
      console.warn('[Trinity] Unhandled message type:', message.type);
  }
}

function handleBoardState(payload: any): void {
  if (payload && Array.isArray(payload.agents)) {
    agentStates.clear();
    payload.agents.forEach((agent: AgentState) => {
      agentStates.set(agent.id, agent);
    });
    notifyStateChange();
  }
}

function handleAgentEvent(payload: any): void {
  if (payload && payload.agent) {
    const agent: AgentState = payload.agent;
    agentStates.set(agent.id, agent);
    notifyStateChange();
  }
}

// API for popup/content scripts
export function sendCommand(agentId: string, command: string): void {
  sendMessage({
    type: 'send_command',
    payload: { target: agentId, command }
  });
}

export function claimIssue(issueNumber: number): void {
  sendMessage({
    type: 'claim_issue',
    payload: { agent_id: 'cli', issue_number: issueNumber, branch: 'feature/issue-' + issueNumber }
  });
}

export function updateAgentStatus(
  agentId: string,
  status: AgentStatus,
  message?: string
): void {
  sendMessage({
    type: 'update_status',
    payload: { agent_id: agentId, status, message: message || '' }
  });
}

// Initialize connection
connect();

// Keep service worker alive
chrome.runtime.onStartup.addListener(() => {
  console.log('[Trinity] Service worker starting up');
  connect();
});

chrome.runtime.onInstalled.addListener(() => {
  console.log('[Trinity] Extension installed/updated');
  connect();
});
