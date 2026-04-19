/// Content script for github.com
/// Parses issue #30 comments and shows sidebar with agent status

function getIssueNumber(): number | null {
  const match = window.location.pathname.match(/\/issues\/(\d+)/);
  return match ? parseInt(match[1], 10) : null;
}

function isIssue30(): boolean {
  return getIssueNumber() === 30;
}

const AGENT_STATUS_MARKERS: Record<string, RegExp> = {
  claiming: /🔄\s*claiming/i,
  working: /⚙️\s*working/i,
  blocked: /⛔\s*blocked/i,
  done: /✅\s*done/i,
};

function parseAgentStatus(comment: HTMLElement): { status: string; agent: string; message: string } | null {
  const text = comment.textContent || '';
  const author = comment.querySelector('.author')?.textContent?.trim() || 'Unknown';

  for (const [status, regex] of Object.entries(AGENT_STATUS_MARKERS)) {
    const match = text.match(regex);
    if (match) {
      const messageParts = text.split(match[0]);
      const message = messageParts[1]?.trim() || '';
      return { status, agent: author, message };
    }
  }

  return null;
}

function getAllIssueComments(): HTMLElement[] {
  const timeline = document.querySelector('.js-issue-timeline');
  if (!timeline) return [];

  return Array.from(timeline.querySelectorAll('.timeline-comment'));
}

function createSidebar(): HTMLElement {
  const sidebar = document.createElement('div');
  sidebar.id = 'trinity-agent-sidebar';
  sidebar.style.cssText = `
    position: fixed;
    right: 20px;
    top: 20px;
    width: 280px;
    max-height: calc(100vh - 40px);
    background: #0d1117;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 16px;
    overflow-y: auto;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-size: 14px;
    color: #c9d1d9;
    z-index: 9999;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
  `;

  const header = document.createElement('div');
  header.style.cssText = `
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
    border-bottom: 1px solid #30363d;
    padding-bottom: 8px;
  `;
  header.innerHTML = `
    <strong style="color: #58a6ff;">🤖 Trinity Agents</strong>
    <button id="trinity-close" style="background:none;border:none;color:#8b949e;cursor:pointer;font-size:16px;">×</button>
  `;
  sidebar.appendChild(header);

  const list = document.createElement('div');
  list.id = 'trinity-agent-list';
  list.style.cssText = `
    display: flex;
    flex-direction: column;
    gap: 8px;
  `;
  sidebar.appendChild(list);

  const actions = document.createElement('div');
  actions.style.cssText = `
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid #30363d;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  `;

  const actionButtons = [
    { emoji: '🚀', label: 'Start', action: 'start' },
    { emoji: '🔄', label: 'Sync', action: 'sync' },
    { emoji: '⚡', label: 'STOP', action: 'stop' },
  ];

  actionButtons.forEach(({ emoji, label, action }) => {
    const btn = document.createElement('button');
    btn.textContent = `${emoji} ${label}`;
    btn.dataset.action = action;
    btn.style.cssText = `
      padding: 6px 12px;
      border: 1px solid #30363d;
      border-radius: 4px;
      background: #21262d;
      color: #c9d1d9;
      cursor: pointer;
      font-size: 12px;
      font-weight: 600;
    `;
    actions.appendChild(btn);
  });

  sidebar.appendChild(actions);

  sidebar.querySelector('#trinity-close')?.addEventListener('click', () => {
    sidebar.remove();
  });

  return sidebar;
}

function showSidebar(): void {
  if (document.getElementById('trinity-agent-sidebar')) return;

  const sidebar = createSidebar();
  document.body.appendChild(sidebar);
}

if (isIssue30()) {
  console.log('[Trinity-GitHub] Issue #30 detected, showing agent sidebar');

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', showSidebar);
  } else {
    showSidebar();
  }
}
