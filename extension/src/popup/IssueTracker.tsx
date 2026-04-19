/// Trinity Agent Bridge - Issue Tracker Component
import { useState } from 'react';
import { claimIssue } from '../background/service-worker';

export function IssueTracker(): JSX.Element {
  const [issueNumber, setIssueNumber] = useState('');

  const handleTrack = (): void => {
    const num = parseInt(issueNumber, 10);
    if (!isNaN(num) && num > 0) {
      setIssueNumber('');
      console.log('Tracking issue:', num);
    }
  };

  return (
    <div className="issue-tracker">
      <div className="track-form">
        <input
          type="number"
          value={issueNumber}
          onChange={e => setIssueNumber((e.target as HTMLInputElement).value)}
          placeholder="Issue number..."
          min={1}
        />

        <button
          onClick={handleTrack}
          disabled={!issueNumber}
          className="btn-primary"
        >
          Track Issue
        </button>
      </div>

      <div className="empty-state">
        <p>No tracked issues</p>
        <small>Add issue numbers to track their status</small>
      </div>
    </div>
  );
}
