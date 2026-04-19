/// Trinity Agent Bridge - Agent Board Component
import { type AgentState } from '../shared/types';
import { sendCommand, claimIssue, updateAgentStatus } from '../background/service-worker';

interface Props {
  agents: AgentState[];
}

export function AgentBoard({ agents }: Props): JSX.Element {
  const getStatusColor = (status: string): string => {
    const colors: Record<string, string> = {
      idle: '#3fb950',
      claiming: '#d29922',
      working: '#a371f7',
      blocked: '#f85149',
      done: '#238636',
    };
    return colors[status] || '#8b949e';
  };

  const getStatusIcon = (status: string): string => {
    const icons: Record<string, string> = {
      idle: '😴',
      claiming: '🔄',
      working: '⚙️',
      blocked: '⛔',
      done: '✅',
    };
    return icons[status] || '❓';
  };

  return (
    <div className="agent-board">
      <div className="quick-actions">
        <button>🚀 Start Agent</button>
        <button>🔄 Sync</button>
        <button>⚡ STOP</button>
      </div>

      {agents.length === 0 ? (
        <div className="empty-state">
          <p>No active agents</p>
          <small>Agents will appear here when they connect</small>
        </div>
      ) : (
        <div className="agent-list">
          {agents.map(agent => (
            <div
              key={agent.id}
              className="agent-card"
              style={{ borderLeft: `3px solid ${getStatusColor(agent.status)}` }}
            >
              <div className="agent-header">
                <span className="agent-name">{agent.name}</span>
                <span
                  className="agent-status"
                  style={{ color: getStatusColor(agent.status) }}
                >
                  {getStatusIcon(agent.status)} {agent.status.toUpperCase()}
                </span>
              </div>

              {agent.issue && (
                <div className="agent-issue">
                  Issue: <span>#{agent.issue}</span>
                  {agent.branch && <span className="branch">{agent.branch}</span>}
                </div>
              )}

              {agent.message && (
                <div className="agent-message">{agent.message}</div>
              )}

              <div className="agent-actions">
                {agent.status === 'idle' && agent.issue && (
                  <button
                    onClick={() => claimIssue(agent.issue!)}
                    className="btn-primary"
                  >
                    Claim Issue
                  </button>
                )}
                {agent.status === 'working' && (
                  <button
                    onClick={() => updateAgentStatus(agent.id, 'blocked')}
                    className="btn-warning"
                  >
                    Mark Blocked
                  </button>
                )}
                {agent.status === 'blocked' && (
                  <button
                    onClick={() => updateAgentStatus(agent.id, 'working')}
                    className="btn-primary"
                  >
                    Resume Work
                  </button>
                )}
                {agent.status === 'working' && (
                  <button
                    onClick={() => updateAgentStatus(agent.id, 'done')}
                    className="btn-success"
                  >
                    Mark Done
                  </button>
                )}
              </div>

              <div className="agent-footer">
                <small className="timestamp">
                  {new Date(agent.lastUpdate).toLocaleString()}
                </small>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
