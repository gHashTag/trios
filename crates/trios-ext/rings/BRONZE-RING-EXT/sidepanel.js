// Trinity Agent Chat — Sidepanel Client v2
// Ring Architecture: BR-APP (WASM future) → BRONZE-RING-EXT (Chrome MV3)
// Connects to HITL-A2A HTTP Bridge (:9876) + Cloudflare Tunnel + trios-server (:9005 WS)
//
// Ring migration path:
//   v1 (NOW):     HTML/JS sidepanel → HTTP bridge polling
//   v2 (next):    WASM sidepanel (BR-APP) → Dioxus UR-00..09 atoms
//   v3 (future):  Full Ring stack — UR-09 Social + UR-07 WS + Neon persistence

const _$ = id => document.getElementById(id);

// ─── Ring Architecture Constants ──────────────────────────────
const RING_VERSION = 'v0.4.0-ring';
const RING_PATH = {
  BR_APP: 'crates/trios-ui/rings/BR-APP',      // WASM entry
  UR_00: 'crates/trios-ui/rings/UR-00',          // State atoms
  UR_07: 'crates/trios-ui/rings/UR-07',          // WS client
  UR_09: 'crates/trios-ui/rings/UR-09',          // Social feed (NEW)
  BRONZE_EXT: 'crates/trios-ext/rings/BRONZE-RING-EXT',  // Chrome ext
};

// ─── State (mirrors UR-00 atoms) ─────────────────────────────
const state = {
  bridgeUrl: 'http://127.0.0.1:9876',
  tunnelUrl: '',
  convId: 'trinity-ops-2026-05-03',
  pollInterval: 3000,
  messages: [],         // A2ASocialMessage[] (mirrors UR-00 ChatState.messages)
  presence: new Map(),  // AgentPresence[]    (mirrors UR-00 AgentsAtom)
  busConnected: false,
  wsConnected: false,
  interruptActive: false,
  autoScroll: true,
  activeFilter: null,   // agent name filter (mirrors UR-09 A2ASocialState)
  lastMsgTs: 0,
  pollTimer: null,
  heartbeatTimer: null,
};

// ─── Agent Profiles (mirrors UR-09 AgentBubble profiles) ─────
const AGENT_PROFILES = {
  'PerplexityScarabs': { emoji: '🕷️', color: '#ff6b9d', label: 'Scarabs', desc: 'Cloud code agent — Rust + Neon + GitHub' },
  'BrowserOS-Agent':   { emoji: '🤖', color: '#4fc3f7', label: 'BOS', desc: 'Local browser agent — full web control' },
  'HumanOverlord':     { emoji: '👑', color: '#D4AF37', label: 'You', desc: 'Human-in-the-Loop — veto power' },
  'phi-t27':           { emoji: 'φ', color: '#FF6B6B', label: 't27', desc: 'Trinity compute agent' },
  'System':            { emoji: '⚡', color: '#888', label: 'System', desc: 'System messages' },
};

function getProfile(name) {
  return AGENT_PROFILES[name] || { emoji: '❓', color: '#666', label: name || 'Unknown', desc: '' };
}

// ─── Settings (mirrors UR-00 SettingsAtom) ────────────────────
function loadSettings() {
  try {
    const s = JSON.parse(localStorage.getItem('trinity-chat-settings') || '{}');
    if (s.bridgeUrl) state.bridgeUrl = s.bridgeUrl;
    if (s.convId) state.convId = s.convId;
    if (s.pollInterval) state.pollInterval = s.pollInterval;
    if (s.tunnelUrl) state.tunnelUrl = s.tunnelUrl;
  } catch {}
  _$('bridge-url').value = state.bridgeUrl;
  _$('conv-id').value = state.convId;
  _$('poll-interval').value = state.pollInterval;
}

function saveSettings() {
  state.bridgeUrl = _$('bridge-url').value.replace(/\/+$/, '');
  state.convId = _$('conv-id').value;
  state.pollInterval = parseInt(_$('poll-interval').value) || 3000;
  localStorage.setItem('trinity-chat-settings', JSON.stringify({
    bridgeUrl: state.bridgeUrl,
    convId: state.convId,
    pollInterval: state.pollInterval,
    tunnelUrl: state.tunnelUrl,
  }));
  restart();
}

function toggleSettings() {
  _$('settings').classList.toggle('open');
}

// ─── API (mirrors UR-07 ApiClient) ───────────────────────────
function getBusUrl() {
  return state.tunnelUrl || state.bridgeUrl;
}

async function api(path, opts = {}) {
  const url = `${getBusUrl()}/bus/${state.convId}${path}`;
  try {
    const r = await fetch(url, { ...opts, signal: AbortSignal.timeout(5000) });
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    return await r.json();
  } catch (e) {
    state.busConnected = false;
    setBusStatus('disconnected');
    return null;
  }
}

async function apiHealth() {
  try {
    const r = await fetch(`${getBusUrl()}/health`, { signal: AbortSignal.timeout(3000) });
    return r.ok;
  } catch { return false; }
}

// ─── Bus Status ───────────────────────────────────────────────
function setBusStatus(status) {
  const el = _$('bus-status');
  el.className = status;
  el.textContent = status === 'connected' ? 'online' : status === 'connecting' ? '...' : 'offline';
}

// ─── Polling (mirrors UR-07 WS fallback) ─────────────────────
async function poll() {
  // Health check first
  const health = await apiHealth();
  if (!health) {
    // Try tunnel URL fallback
    if (state.tunnelUrl && state.tunnelUrl !== state.bridgeUrl) {
      // Already tried tunnel, mark offline
    }
    state.busConnected = false;
    setBusStatus('disconnected');
    return;
  }

  state.busConnected = true;
  setBusStatus('connected');

  // Fetch all messages (dedup by timestamp)
  const data = await api('/messages');
  if (!data || !data.messages) return;

  // Merge new messages
  const existingTs = new Set(state.messages.map(m => m.timestamp));
  let hasNew = false;
  
  for (const m of data.messages) {
    if (!existingTs.has(m.timestamp)) {
      state.messages.push(m);
      existingTs.add(m.timestamp);
      renderMessage(m);
      hasNew = true;
    }
  }

  // Sort by timestamp
  state.messages.sort((a, b) => (a.timestamp || 0) - (b.timestamp || 0));

  // Cap at 500
  if (state.messages.length > 500) {
    state.messages = state.messages.slice(-500);
  }

  if (hasNew && state.autoScroll) scrollToBottom();

  // Check interrupt state
  const intData = await api('/interrupt');
  if (intData) {
    const wasActive = state.interruptActive;
    state.interruptActive = !!intData.hasInterrupt;
    if (wasActive !== state.interruptActive) updateInterruptUI();
  }

  // Update presence
  const pres = await api('/presence');
  if (pres) renderPresence(pres);
}

function restart() {
  if (state.pollTimer) clearInterval(state.pollTimer);
  if (state.heartbeatTimer) clearInterval(state.heartbeatTimer);
  state.messages = [];
  state.lastMsgTs = 0;
  _$('chat').innerHTML = '';
  state.presence.clear();
  setBusStatus('connecting');
  state.pollTimer = setInterval(poll, state.pollInterval);
  state.heartbeatTimer = setInterval(sendHeartbeat, 30000);
  poll();
  sendHeartbeat();
}

// ─── Heartbeat (mirrors UR-09 presence) ───────────────────────
async function sendHeartbeat() {
  if (!state.busConnected) return;
  try {
    await fetch(`${getBusUrl()}/bus/${state.convId}/presence`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ role: 'human', agentName: 'HumanOverlord', action: 'heartbeat' }),
    });
  } catch { /* silent */ }
}

// ─── Render Presence (mirrors UR-09 PresenceBar) ─────────────
function renderPresence(pres) {
  const bar = _$('presence-bar');
  
  // Merge known agents
  for (const [name, info] of Object.entries(pres.agents || {})) {
    state.presence.set(name, { ...info, lastSeen: Date.now() });
  }

  bar.innerHTML = '';

  // Core agents always shown
  const coreAgents = ['HumanOverlord', 'BrowserOS-Agent', 'PerplexityScarabs'];
  const allAgents = [...coreAgents, ...[...state.presence.keys()].filter(n => !coreAgents.includes(n))];

  for (const name of allAgents) {
    const info = state.presence.get(name);
    const profile = getProfile(name);
    const online = info && (Date.now() - (info.lastSeen || 0) < 120000);
    const isActive = state.activeFilter === name;

    const chip = document.createElement('div');
    chip.className = `agent-chip ${online ? 'online' : 'offline'} ${isActive ? 'active-filter' : ''}`;
    chip.innerHTML = `<span class="dot"></span>${profile.emoji} ${profile.label}`;
    chip.onclick = () => toggleFilter(name);
    bar.appendChild(chip);
  }
}

// ─── Filter (mirrors UR-09 active_filter) ─────────────────────
function toggleFilter(agentName) {
  state.activeFilter = state.activeFilter === agentName ? null : agentName;
  rerenderAll();
}

function rerenderAll() {
  const chat = _$('chat');
  chat.innerHTML = '';
  const filtered = state.activeFilter
    ? state.messages.filter(m => m.agentName === state.activeFilter)
    : state.messages;
  filtered.forEach(m => renderMessage(m));
  if (state.autoScroll) scrollToBottom();
  // Re-render presence to show active filter
  renderPresence({ agents: Object.fromEntries(state.presence) });
}

// ─── Render Message (mirrors UR-09 SocialFeed) ───────────────
function renderMessage(m) {
  const chat = _$('chat');
  const profile = getProfile(m.agentName);
  const time = m.timestamp
    ? new Date(m.timestamp).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' })
    : '';

  const div = document.createElement('div');
  div.className = `msg ${m.type || 'chat'}`;
  div.dataset.agent = m.agentName;

  let typeTag = '';
  switch (m.type) {
    case 'interrupt': typeTag = '⛔ INTERRUPT'; break;
    case 'abort': typeTag = '🛑 ABORT'; break;
    case 'interrupted': typeTag = '✅ ACK'; break;
    case 'presence': typeTag = '📡'; break;
    default: break;
  }

  const content = formatContent(m.content || '');

  div.innerHTML = `
    <div class="msg-header">
      <span class="msg-agent">${profile.emoji} ${profile.label}</span>
      ${typeTag ? `<span style="color:var(--muted);font-size:9px">${typeTag}</span>` : ''}
      <span class="msg-time">${time}</span>
    </div>
    <div class="msg-body">${content}</div>
  `;

  chat.appendChild(div);
}

function formatContent(text) {
  return text
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/`([^`]+)`/g, '<code style="background:#1a1a26;padding:1px 4px;border-radius:3px;font-size:10px">$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/(https?:\/\/[^\s<]+)/g, '<a href="$1" target="_blank" style="color:var(--bos)">$1</a>')
    .replace(/\n/g, '<br>');
}

// ─── Send (mirrors UR-09 HumanInput) ──────────────────────────
async function sendMsg() {
  const input = _$('msg-input');
  const text = input.value.trim();
  if (!text) return;
  input.value = '';

  // Optimistic render
  const msg = {
    type: 'chat',
    role: 'human',
    agentName: 'HumanOverlord',
    content: text,
    conversationId: state.convId,
    timestamp: Date.now(),
  };
  state.messages.push(msg);
  renderMessage(msg);
  if (state.autoScroll) scrollToBottom();

  // Send to bus
  try {
    await fetch(`${getBusUrl()}/bus/${state.convId}/messages`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(msg),
    });
  } catch { /* optimistic render already shown */ }

  setTimeout(poll, 500);
}

// ─── Interrupt (mirrors UR-09 InterruptButton) ────────────────
async function sendInterrupt() {
  state.interruptActive = true;
  updateInterruptUI();

  try {
    await fetch(`${getBusUrl()}/bus/${state.convId}/interrupt`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        role: 'human',
        agentName: 'HumanOverlord',
        reason: '⛔ Human veto — STOP all agents',
        scope: 'all_agents',
        priority: 'P0',
      }),
    });
  } catch {}

  // Optimistic render
  const msg = {
    type: 'interrupt', role: 'human', agentName: 'HumanOverlord',
    content: '⛔ INTERRUPT ALL — human veto', conversationId: state.convId,
    timestamp: Date.now(),
  };
  state.messages.push(msg);
  renderMessage(msg);
  if (state.autoScroll) scrollToBottom();
}

async function sendResume() {
  state.interruptActive = false;
  updateInterruptUI();

  // Clear interrupt on bus
  try {
    await fetch(`${getBusUrl()}/bus/${state.convId}/interrupt`, {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ role: 'human', agentName: 'HumanOverlord', partialOutput: 'Human lifted veto' }),
    });
  } catch {}

  // Send resume message
  const msg = {
    type: 'chat', role: 'human', agentName: 'HumanOverlord',
    content: '✅ Resume — all agents may continue.',
    conversationId: state.convId, timestamp: Date.now(),
  };
  state.messages.push(msg);
  renderMessage(msg);
  if (state.autoScroll) scrollToBottom();

  try {
    await fetch(`${getBusUrl()}/bus/${state.convId}/messages`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(msg),
    });
  } catch {}
}

function updateInterruptUI() {
  const btn = document.querySelector('.action-btn.interrupt');
  if (!btn) return;
  if (state.interruptActive) {
    btn.style.background = '#2a0a0a';
    btn.style.borderColor = 'var(--red)';
    btn.textContent = '⛔ ACTIVE';
  } else {
    btn.style.background = '';
    btn.style.borderColor = '';
    btn.textContent = '⛔ INTERRUPT';
  }
}

function scrollToBottom() {
  const chat = _$('chat');
  chat.scrollTop = chat.scrollHeight;
}

// ─── Init ─────────────────────────────────────────────────────
loadSettings();
setBusStatus('connecting');
restart();

// Periodic presence staleness check
setInterval(() => {
  if (state.presence.size > 0) {
    renderPresence({ agents: Object.fromEntries(state.presence) });
  }
}, 10000);

console.log(`[Trinity Agent Chat] ${RING_VERSION} initialized. Ring path: BR-APP → UR-09 → UR-07 → Bus`);
