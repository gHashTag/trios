/// Trinity Agent Bridge - Main Popup Application
import { useState, useEffect } from 'react';
import { AgentBoard } from './AgentBoard';
import { CommandInput } from './CommandInput';
import { IssueTracker } from './IssueTracker';
import { getAgentStates, subscribeToStateChanges } from '../background/service-worker';
import { type AgentState } from '../shared/types';

export function App(): JSX.Element {
  const [agents, setAgents] = useState<Map<string, AgentState>>(new Map());
  const [activeTab, setActiveTab] = useState<'agents' | 'command' | 'issues'>('agents');

  useEffect(() => {
    const unsubscribe = subscribeToStateChanges((newStates) => {
      setAgents(new Map(newStates));
    });

    return () => unsubscribe();
  }, []);

  const tabs = [
    { id: 'agents' as const, label: 'Agents', icon: '🤖' },
    { id: 'command' as const, label: 'Command', icon: '⌨️' },
    { id: 'issues' as const, label: 'Issues', icon: '📋' },
  ];

  return (
    <div className="trinity-popup">
      <header>
        <h1>Trinity Bridge</h1>
        <div className="status-indicator">
          <span className="dot online"></span>
          <span className="status-text">Connected</span>
        </div>
      </header>

      <nav>
        {tabs.map(tab => (
          <button
            key={tab.id}
            className={activeTab === tab.id ? 'active' : ''}
            onClick={() => setActiveTab(tab.id)}
          >
            <span className="icon">{tab.icon}</span>
            <span>{tab.label}</span>
          </button>
        ))}
      </nav>

      <main>
        {activeTab === 'agents' && (
          <AgentBoard agents={Array.from(agents.values())} />
        )}
        {activeTab === 'command' && <CommandInput agents={Array.from(agents.values())} />}
        {activeTab === 'issues' && <IssueTracker />}
      </main>
    </div>
  );
}
