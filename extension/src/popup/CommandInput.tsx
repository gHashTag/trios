/// Trinity Agent Bridge - Command Input Component
import { useState } from 'react';
import { type AgentState } from '../shared/types';
import { sendCommand } from '../background/service-worker';

interface Props {
  agents: AgentState[];
}

export function CommandInput({ agents }: Props): JSX.Element {
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);
  const [command, setCommand] = useState('');
  const [history, setHistory] = useState<string[]>([]);

  const activeAgents = agents.filter(a => a.status === 'working' || a.status === 'idle');

  const handleSend = (): void => {
    if (!selectedAgent || !command.trim()) return;

    sendCommand(selectedAgent, command);

    setHistory(prev => [command, ...prev].slice(0, 20));
    setCommand('');
  };

  return (
    <div className="command-input">
      <div className="agent-selector">
        <label>Target Agent:</label>
        <select
          value={selectedAgent || ''}
          onChange={e => setSelectedAgent((e.target as HTMLSelectElement).value)}
        >
          <option value="">Select an agent...</option>
          {activeAgents.map(agent => (
            <option key={agent.id} value={agent.id}>
              {agent.name} ({agent.status})
            </option>
          ))}
        </select>
      </div>

      <div className="input-area">
        <textarea
          value={command}
          onChange={e => setCommand((e.target as HTMLTextAreaElement).value)}
          placeholder="Enter command to send to agent..."
          rows={8}
          disabled={!selectedAgent}
        />

        <div className="input-footer">
          <small className="hint">
            Press <kbd>Enter</kbd> to send
          </small>

          <div className="history-count">
            {history.length > 0 && (
              <small>{history.length} commands in history</small>
            )}
          </div>
        </div>
      </div>

      <div className="actions">
        <button
          onClick={handleSend}
          disabled={!selectedAgent || !command.trim()}
          className="btn-primary"
        >
          Send Command
        </button>

        {history.length > 0 && (
          <button
            onClick={() => setHistory([])}
            className="btn-secondary"
          >
            Clear History
          </button>
        )}
      </div>
    </div>
  );
}
