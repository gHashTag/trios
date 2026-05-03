// Trinity Agent Bridge v0.4 — A2A Social Network
// Ring Architecture: BR-APP (WASM future) → BRONZE-RING-EXT (Chrome MV3)
// Connects to HITL-A2A HTTP Bridge (:9876) + trios-server (:9005 WS)
//
// UR-09 Social Panel → when WASM build works, this JS gets replaced by Dioxus
//
// Tabs: 🕸️ SOCIAL | 💬 CHAT | 🤖 AGENTS | 🔧 TOOLS

const $ = id => document.getElementById(id);

// ─── Ring Architecture Constants ──────────────────────────────
const RING_VERSION = 'v0.4.0-ring';

// ─── State (mirrors UR-00 atoms) ─────────────────────────────
const state = {
  bridgeUrl: 'http://127.0.0.1:9876',
  convId: 'trinity-ops-2026-05-03',
  messages: [],
  presence: new Map(),
  busConnected: false,
  wsConnected: false,
  interruptActive: false,
  autoScroll: true,
  activeFilter: null,
  lastMsgIds: new Set(),
  pollTimer: null,
  heartbeatTimer: null,
  activeTab: 'social',
};

// ─── Agent Profiles (mirrors UR-09 AgentBubble profiles) ─────
const PROFILES = {
  'PerplexityScarabs': { emoji: '🕷️', color: '#ff6b9d', label: 'Scarabs', desc: 'Cloud code agent — Rust + Neon + GitHub' },
  'BrowserOS-Agent':   { emoji: '🤖', color: '#4fc3f7', label: 'BOS', desc: 'Local browser agent — full web control' },
  'HumanOverlord':     { emoji: '👑', color: '#D4AF37', label: 'You', desc: 'Human-in-the-Loop — veto power' },
  'phi-t27':           { emoji: 'φ', color: '#FF6B6B', label: 't27', desc: 'Trinity compute agent' },
  'System':            { emoji: '⚡', color: '#888', label: 'System', desc: 'System messages' },
};

function getProfile(name) {
  return PROFILES[name] || { emoji: '❓', color: '#666', label: name || 'Unknown', desc: '' };
}

// ─── Styles ───────────────────────────────────────────────────
const style = document.createElement('style');
style.textContent = `
  * { box-sizing: border-box; margin: 0; padding: 0; }
  :root {
    --gold: #D4AF37; --bg: #0a0a0f; --surface: #12121a; --surface2: #1a1a26;
    --border: #252535; --text: #e8e8f0; --muted: #666680;
    --green: #4caf50; --red: #e74c3c; --orange: #ff9800;
    --scarabs: #ff6b9d; --bos: #4fc3f7; --human: #D4AF37;
  }
  body { background: var(--bg); color: var(--text); font-family: 'SF Mono','Fira Code',monospace; height: 100vh; display: flex; flex-direction: column; font-size: 12px; }

  /* Header */
  #header { padding: 8px 14px; border-bottom: 1px solid var(--border); display: flex; align-items: center; gap: 8px; background: var(--surface); }
  #header .logo { font-size: 14px; color: var(--gold); }
  #header .title { font-weight: 700; color: var(--gold); font-size: 12px; letter-spacing: 0.5px; }
  #header .spacer { flex: 1; }
  #bus-status { font-size: 9px; padding: 2px 8px; border-radius: 10px; border: 1px solid var(--border); text-transform: uppercase; letter-spacing: 0.5px; }
  #bus-status.connected { border-color: #2d6a2d; color: var(--green); background: #0a1a0a; }
  #bus-status.disconnected { border-color: #6a2d2d; color: var(--red); background: #1a0a0a; }

  /* Tabs */
  #tabs { display: flex; border-bottom: 1px solid var(--border); background: var(--surface); }
  .tab { flex: 1; padding: 7px; text-align: center; font-size: 10px; cursor: pointer; color: var(--muted); border-bottom: 2px solid transparent; transition: all 0.15s; }
  .tab.active { color: var(--gold); border-bottom-color: var(--gold); }
  .tab:hover { color: var(--text); }

  /* Presence bar */
  #presence-bar { display: flex; gap: 6px; padding: 5px 14px; border-bottom: 1px solid var(--border); background: var(--surface); overflow-x: auto; }
  .agent-chip { display: flex; align-items: center; gap: 4px; padding: 2px 8px; border-radius: 12px; font-size: 10px; border: 1px solid var(--border); white-space: nowrap; cursor: pointer; }
  .agent-chip .dot { width: 6px; height: 6px; border-radius: 50%; }
  .agent-chip.online .dot { background: var(--green); box-shadow: 0 0 4px var(--green); }
  .agent-chip.offline .dot { background: var(--muted); }
  .agent-chip.filtered { border-color: var(--gold); background: #1a1a0a; }

  /* Chat area */
  #content { flex: 1; overflow-y: auto; padding: 8px 14px; display: flex; flex-direction: column; gap: 4px; }
  #content::-webkit-scrollbar { width: 4px; }
  #content::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }

  /* Messages */
  .msg { padding: 6px 8px; border-radius: 8px; font-size: 11px; line-height: 1.5; max-width: 95%; position: relative; word-break: break-word; }
  .msg-header { display: flex; align-items: center; gap: 5px; margin-bottom: 2px; font-size: 9px; text-transform: uppercase; letter-spacing: 0.5px; }
  .msg-time { color: var(--muted); font-size: 9px; margin-left: auto; }
  .msg.chat { background: var(--surface2); border: 1px solid var(--border); }
  .msg.presence { background: #0a0a15; border: 1px solid #1a1a2a; font-size: 10px; color: var(--muted); }
  .msg.interrupt { background: #1a0a0a; border: 1px solid #4a1a1a; }
  .msg.abort { background: #150a0a; border: 1px solid #3a1515; font-size: 10px; }
  .msg.interrupted { background: #1a1a0a; border: 1px solid #4a4a1a; }

  /* Agent colors */
  .msg[data-agent="PerplexityScarabs"] .msg-agent { color: var(--scarabs); }
  .msg[data-agent="PerplexityScarabs"] { border-left: 2px solid var(--scarabs); }
  .msg[data-agent="BrowserOS-Agent"] .msg-agent { color: var(--bos); }
  .msg[data-agent="BrowserOS-Agent"] { border-left: 2px solid var(--bos); }
  .msg[data-agent="HumanOverlord"] .msg-agent { color: var(--human); }
  .msg[data-agent="HumanOverlord"] { border-left: 2px solid var(--human); }

  /* Input area */
  #input-area { border-top: 1px solid var(--border); background: var(--surface); padding: 6px 14px; display: flex; flex-direction: column; gap: 5px; }
  #input-row { display: flex; gap: 6px; }
  #msg-input { flex: 1; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; padding: 6px 10px; color: var(--text); font-family: inherit; font-size: 11px; outline: none; }
  #msg-input:focus { border-color: var(--gold); }
  #send-btn { background: var(--gold); color: #000; border: none; border-radius: 6px; padding: 6px 12px; font-size: 11px; font-weight: 700; cursor: pointer; font-family: inherit; }
  #send-btn:hover { background: #e8c44a; }
  #action-row { display: flex; gap: 6px; }
  .action-btn { flex: 1; background: var(--surface2); border: 1px solid var(--border); border-radius: 6px; padding: 4px 8px; font-size: 10px; color: var(--muted); cursor: pointer; font-family: inherit; text-align: center; }
  .action-btn:hover { border-color: var(--gold); color: var(--text); }
  .action-btn.interrupt { color: var(--red); border-color: #3a1515; }
  .action-btn.interrupt:hover { background: #1a0a0a; border-color: var(--red); }
  .action-btn.interrupt.active { background: #2a0a0a; border-color: var(--red); }

  /* Agents list */
  .agent-card { padding: 8px 12px; border: 1px solid var(--border); border-radius: 8px; margin-bottom: 6px; display: flex; align-items: center; justify-content: space-between; }
  .agent-card .name { font-weight: 600; font-size: 12px; }
  .agent-card .desc { color: var(--muted); font-size: 10px; margin-top: 2px; }
  .agent-card .status-badge { font-size: 9px; padding: 2px 6px; border-radius: 10px; }

  /* Empty state */
  .empty { color: var(--muted); font-size: 11px; text-align: center; padding: 40px 20px; }
`;
document.head.appendChild(style);

// ─── Build UI ─────────────────────────────────────────────────
$('main').innerHTML = `
  <div id="header">
    <span class="logo">🕸️</span>
    <span class="title">Trinity Agent Bridge</span>
    <span class="spacer"></span>
    <span id="bus-status" class="disconnected">offline</span>
  </div>

  <div id="tabs">
    <div class="tab active" data-tab="social">🕸️ SOCIAL</div>
    <div class="tab" data-tab="chat">💬 CHAT</div>
    <div class="tab" data-tab="agents">🤖 AGENTS</div>
    <div class="tab" data-tab="tools">🔧 TOOLS</div>
  </div>

  <div id="presence-bar"></div>
  <div id="content"></div>

  <div id="input-area">
    <div id="input-row">
      <input id="msg-input" placeholder="👑 Message agents..." />
      <button id="send-btn">↵</button>
    </div>
    <div id="action-row">
      <button class="action-btn interrupt" id="interrupt-btn">⛔ INTERRUPT</button>
      <button class="action-btn" id="resume-btn">✅ RESUME</button>
      <button class="action-btn" id="bottom-btn">↓ BOTTOM</button>
    </div>
  </div>
`;

// ─── Tab switching ────────────────────────────────────────────
document.querySelectorAll('.tab').forEach(t => {
  t.addEventListener('click', () => {
    document.querySelectorAll('.tab').forEach(x => x.classList.remove('active'));
    t.classList.add('active');
    state.activeTab = t.dataset.tab;
    renderTab();
  });
});

// ─── API ──────────────────────────────────────────────────────
function busUrl(path) { return `${state.bridgeUrl}/bus/${state.convId}${path}`; }

async function apiGet(path) {
  try {
    const r = await fetch(busUrl(path), { signal: AbortSignal.timeout(3000) });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    return await r.json();
  } catch { return null; }
}

async function apiPost(path, body) {
  try {
    await fetch(busUrl(path), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(5000),
    });
  } catch { /* optimistic */ }
}

async function apiDelete(path, body) {
  try {
    await fetch(busUrl(path), {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(5000),
    });
  } catch {}
}

// ─── Polling ──────────────────────────────────────────────────
async function poll() {
  // Health check
  try {
    const hr = await fetch(`${state.bridgeUrl}/health`, { signal: AbortSignal.timeout(2000) });
    state.busConnected = hr.ok;
  } catch {
    state.busConnected = false;
  }

  const statusEl = $('bus-status');
  if (state.busConnected) {
    statusEl.textContent = 'online';
    statusEl.className = 'connected';
  } else {
    statusEl.textContent = 'offline';
    statusEl.className = 'disconnected';
    return;
  }

  // Fetch messages
  const data = await apiGet('/messages');
  if (data?.messages) {
    for (const m of data.messages) {
      if (!state.lastMsgIds.has(m.id)) {
        state.lastMsgIds.add(m.id);
        state.messages.push(m);
      }
    }
    state.messages.sort((a, b) => (a.timestamp || 0) - (b.timestamp || 0));
    if (state.messages.length > 300) state.messages = state.messages.slice(-300);
    if (state.activeTab === 'social') renderTab();
  }

  // Check interrupt
  const intData = await apiGet('/interrupt');
  if (intData) {
    state.interruptActive = !!intData.hasInterrupt;
    updateInterruptUI();
  }

  // Update presence
  const pres = await apiGet('/presence');
  if (pres?.agents) {
    for (const [name, info] of Object.entries(pres.agents)) {
      state.presence.set(name, { ...info, lastSeen: info.lastSeen || Date.now() });
    }
    renderPresence();
  }
}

// ─── Heartbeat ────────────────────────────────────────────────
async function sendHeartbeat() {
  if (!state.busConnected) return;
  await apiPost('/presence', { role: 'human', agentName: 'HumanOverlord', action: 'heartbeat' });
}

// ─── Presence bar ─────────────────────────────────────────────
function renderPresence() {
  const bar = $('presence-bar');
  if (!bar) return;
  const coreAgents = ['HumanOverlord', 'BrowserOS-Agent', 'PerplexityScarabs', 'phi-t27'];
  const all = [...coreAgents, ...[...state.presence.keys()].filter(n => !coreAgents.includes(n))];
  bar.innerHTML = all.map(name => {
    const profile = getProfile(name);
    const info = state.presence.get(name);
    const online = info && (Date.now() - (info.lastSeen || 0) < 120000);
    const filtered = state.activeFilter === name;
    return `<div class="agent-chip ${online ? 'online' : 'offline'} ${filtered ? 'filtered' : ''}" data-agent="${name}">
      <span class="dot"></span><span style="color:${profile.color}">${profile.emoji} ${profile.label}</span>
    </div>`;
  }).join('');

  // Click to filter
  bar.querySelectorAll('.agent-chip').forEach(chip => {
    chip.onclick = () => {
      const name = chip.dataset.agent;
      state.activeFilter = state.activeFilter === name ? null : name;
      renderPresence();
      if (state.activeTab === 'social') renderTab();
    };
  });
}

// ─── Render Tab ───────────────────────────────────────────────
function renderTab() {
  const c = $('content');
  if (state.activeTab === 'social') {
    renderSocialFeed(c);
  } else if (state.activeTab === 'chat') {
    renderChatTab(c);
  } else if (state.activeTab === 'agents') {
    renderAgentsTab(c);
  } else if (state.activeTab === 'tools') {
    renderToolsTab(c);
  }
}

function renderSocialFeed(container) {
  let msgs = state.activeFilter
    ? state.messages.filter(m => m.agentName === state.activeFilter)
    : state.messages;

  // Filter out heartbeat/presence noise — keep only meaningful messages
  msgs = msgs.filter(m => {
    if (m.type === 'presence' && (m.content === 'heartbeat' || m.content === 'join' || m.content === 'leave')) return false;
    return true;
  });

  if (msgs.length === 0) {
    container.innerHTML = '<div class="empty">🕸️ No messages yet.<br>Agents will appear here when they connect to the bus.</div>';
    return;
  }

  container.innerHTML = msgs.map(m => {
    const profile = getProfile(m.agentName);
    const time = m.timestamp ? new Date(m.timestamp).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' }) : '';
    let typeTag = '';
    switch (m.type) {
      case 'interrupt': typeTag = '⛔ INTERRUPT'; break;
      case 'abort': typeTag = '🛑 ABORT'; break;
      case 'interrupted': typeTag = '✅ ACK'; break;
      case 'presence': typeTag = '📡'; break;
    }
    const content = formatContent(m.content || '');
    return `<div class="msg ${m.type || 'chat'}" data-agent="${m.agentName}">
      <div class="msg-header">
        <span class="msg-agent">${profile.emoji} ${profile.label}</span>
        ${typeTag ? `<span style="color:var(--muted);font-size:9px">${typeTag}</span>` : ''}
        <span class="msg-time">${time}</span>
      </div>
      <div class="msg-body">${content}</div>
    </div>`;
  }).join('');

  if (state.autoScroll) container.scrollTop = container.scrollHeight;
}

function renderChatTab(c) {
  const chatMsgs = state.messages.filter(m => m.type === 'chat');
  if (chatMsgs.length === 0) {
    c.innerHTML = '<div class="empty">💬 No chat messages.</div>';
    return;
  }
  c.innerHTML = chatMsgs.map(m => {
    const profile = getProfile(m.agentName);
    const time = m.timestamp ? new Date(m.timestamp).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' }) : '';
    return `<div class="msg chat" data-agent="${m.agentName}" style="align-self:${m.role === 'human' ? 'flex-end' : 'flex-start'};max-width:80%">
      <div class="msg-header">
        <span class="msg-agent">${profile.emoji} ${profile.label}</span>
        <span class="msg-time">${time}</span>
      </div>
      <div class="msg-body">${formatContent(m.content || '')}</div>
    </div>`;
  }).join('');
}

function renderAgentsTab(c) {
  const agents = [
    { name: 'HumanOverlord', ...PROFILES.HumanOverlord },
    { name: 'BrowserOS-Agent', ...PROFILES.BrowserOS-Agent },
    { name: 'PerplexityScarabs', ...PROFILES.PerplexityScarabs },
    { name: 'phi-t27', ...PROFILES['phi-t27'] },
  ];
  c.innerHTML = agents.map(a => {
    const info = state.presence.get(a.name);
    const online = info && (Date.now() - (info.lastSeen || 0) < 120000);
    return `<div class="agent-card">
      <div>
        <div class="name" style="color:${a.color}">${a.emoji} ${a.label}</div>
        <div class="desc">${a.desc}</div>
      </div>
      <span class="status-badge" style="background:${online ? '#0a1a0a' : '#1a1a1a'};color:${online ? 'var(--green)' : 'var(--muted)'};border:1px solid ${online ? '#2d6a2d' : '#333'}">
        ${online ? '● online' : '○ offline'}
      </span>
    </div>`;
  }).join('');
}

function renderToolsTab(c) {
  if (!state.wsConnected) {
    c.innerHTML = '<div class="empty">🔧 MCP tools require trios-server at :9005<br><span style="color:var(--muted)">Start: bun run trios-server</span></div>';
    return;
  }
  c.innerHTML = '<div class="empty">🔧 Loading MCP tools...</div>';
  if (ws?.readyState === 1) ws.send(JSON.stringify({ jsonrpc: '2.0', method: 'tools/list', params: {}, id: Date.now() }));
}

function formatContent(text) {
  return text
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/`([^`]+)`/g, '<code style="background:#1a1a26;padding:1px 4px;border-radius:3px;font-size:10px">$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/(https?:\/\/[^\s<]+)/g, '<a href="$1" target="_blank" style="color:var(--bos)">$1</a>')
    .replace(/\n/g, '<br>');
}

// ─── Send Message ─────────────────────────────────────────────
function sendMsg() {
  const input = $('msg-input');
  const text = input.value.trim();
  if (!text) return;
  input.value = '';

  const msg = {
    type: 'chat', role: 'human', agentName: 'HumanOverlord',
    content: text, conversationId: state.convId, timestamp: Date.now(),
  };

  // Optimistic render
  state.messages.push({ ...msg, id: `local-${Date.now()}` });
  if (state.activeTab === 'social') renderTab();

  // Send to bus
  apiPost('/messages', msg);
  setTimeout(poll, 500);
}

$('send-btn').addEventListener('click', sendMsg);
$('msg-input').addEventListener('keydown', e => { if (e.key === 'Enter') sendMsg(); });

// ─── Interrupt / Resume ───────────────────────────────────────
$('interrupt-btn').addEventListener('click', async () => {
  state.interruptActive = true;
  updateInterruptUI();
  await apiPost('/interrupt', {
    role: 'human', agentName: 'HumanOverlord',
    reason: '⛔ Human veto — STOP all agents', scope: 'all_agents', priority: 'P0',
  });
  state.messages.push({ id: `int-${Date.now()}`, type: 'interrupt', role: 'human', agentName: 'HumanOverlord', content: '⛔ INTERRUPT ALL — human veto', conversationId: state.convId, timestamp: Date.now() });
  if (state.activeTab === 'social') renderTab();
});

$('resume-btn').addEventListener('click', async () => {
  state.interruptActive = false;
  updateInterruptUI();
  await apiDelete('/interrupt', { role: 'human', agentName: 'HumanOverlord', partialOutput: 'Human lifted veto' });
  const msg = { type: 'chat', role: 'human', agentName: 'HumanOverlord', content: '✅ Resume — all agents may continue.', conversationId: state.convId, timestamp: Date.now() };
  await apiPost('/messages', msg);
  state.messages.push({ ...msg, id: `resume-${Date.now()}` });
  if (state.activeTab === 'social') renderTab();
});

$('bottom-btn').addEventListener('click', () => {
  const c = $('content');
  c.scrollTop = c.scrollHeight;
});

function updateInterruptUI() {
  const btn = $('interrupt-btn');
  if (state.interruptActive) {
    btn.classList.add('active');
    btn.textContent = '⛔ ACTIVE';
  } else {
    btn.classList.remove('active');
    btn.textContent = '⛔ INTERRUPT';
  }
}

// ─── WS to trios-server (:9005) for MCP tools ────────────────
let ws = null;
function connectWS() {
  ws = new WebSocket('ws://localhost:9005/ws');
  ws.onopen = () => { state.wsConnected = true; };
  ws.onclose = () => { state.wsConnected = false; setTimeout(connectWS, 5000); };
  ws.onerror = () => { state.wsConnected = false; };
  ws.onmessage = (e) => {
    try {
      const data = JSON.parse(e.data);
      if (data.result?.tools && state.activeTab === 'tools') {
        const c = $('content');
        c.innerHTML = data.result.tools.map(t =>
          `<div style="padding:6px 10px;border-bottom:1px solid var(--border)"><div style="color:#a0c8ff">${t.name}</div><div style="color:var(--muted);font-size:10px">${t.description || ''}</div></div>`
        ).join('') || '<div class="empty">No tools registered</div>';
      }
    } catch {}
  };
}
connectWS();

// ─── Init ─────────────────────────────────────────────────────
renderTab();
state.pollTimer = setInterval(poll, 3000);
state.heartbeatTimer = setInterval(sendHeartbeat, 30000);
poll();
sendHeartbeat();

// Periodic presence staleness check
setInterval(() => { renderPresence(); }, 10000);

console.log(`[Trinity Agent Bridge] ${RING_VERSION} initialized. UR-09 Social → Bridge :9876`);
