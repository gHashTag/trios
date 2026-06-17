// Trinity Agent Bridge — stub UI (no WASM build yet)
// Replace with: import init from './dist/trios_ui_br_app.js'; await init();
// after running: cargo xtask build-all

const $ = id => document.getElementById(id);

const style = document.createElement('style');
style.textContent = `
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { background: #0d0d0d; color: #f0f0f0; font-family: -apple-system, sans-serif; height: 100vh; display: flex; flex-direction: column; }
  #header { padding: 12px 16px; border-bottom: 1px solid #222; display: flex; align-items: center; gap: 8px; }
  #header .logo { color: #D1AD72; font-size: 18px; }
  #header .title { font-size: 13px; font-weight: 600; color: #D1AD72; }
  #status { font-size: 10px; padding: 2px 8px; border-radius: 10px; background: #1a1a1a; border: 1px solid #333; }
  #status.connected { border-color: #2d6a2d; color: #5cb85c; }
  #status.disconnected { border-color: #6a2d2d; color: #d9534f; }
  #tabs { display: flex; border-bottom: 1px solid #222; }
  .tab { flex: 1; padding: 8px; text-align: center; font-size: 11px; cursor: pointer; color: #666; border-bottom: 2px solid transparent; }
  .tab.active { color: #D1AD72; border-bottom-color: #D1AD72; }
  #content { flex: 1; overflow-y: auto; padding: 12px; }
  #chat-log { display: flex; flex-direction: column; gap: 8px; margin-bottom: 12px; min-height: 200px; }
  .msg { padding: 8px 12px; border-radius: 8px; font-size: 12px; line-height: 1.4; max-width: 90%; }
  .msg.user { background: #1a2a1a; border: 1px solid #2d4a2d; align-self: flex-end; }
  .msg.agent { background: #1a1a2a; border: 1px solid #2d2d4a; align-self: flex-start; }
  .msg .label { font-size: 10px; color: #666; margin-bottom: 4px; }
  #input-row { display: flex; gap: 8px; padding: 12px 16px; border-top: 1px solid #222; }
  #msg-input { flex: 1; background: #1a1a1a; border: 1px solid #333; border-radius: 6px; padding: 8px 10px; color: #f0f0f0; font-size: 12px; outline: none; }
  #msg-input:focus { border-color: #D1AD72; }
  #send-btn { background: #D1AD72; color: #000; border: none; border-radius: 6px; padding: 8px 14px; font-size: 12px; font-weight: 600; cursor: pointer; }
  #send-btn:hover { background: #e8c882; }
  .agent-item { padding: 8px 12px; border: 1px solid #222; border-radius: 6px; margin-bottom: 6px; font-size: 11px; }
  .agent-item .name { color: #D1AD72; font-weight: 600; }
  .agent-item .cap { color: #666; font-size: 10px; margin-top: 2px; }
  #tools-list { font-size: 11px; }
  .tool-item { padding: 6px 10px; border-bottom: 1px solid #1a1a1a; }
  .tool-item .tname { color: #a0c8ff; }
  .tool-item .tdesc { color: #555; font-size: 10px; }
`;
document.head.appendChild(style);

// State
let ws = null;
let activeTab = 'chat';
const messages = [];

// Build UI
$('main').innerHTML = `
  <div id="header">
    <span class="logo">▽</span>
    <span class="title">Trinity Agent Bridge</span>
    <span id="status" class="disconnected">disconnected</span>
  </div>
  <div id="tabs">
    <div class="tab active" data-tab="chat">CHAT</div>
    <div class="tab" data-tab="agents">AGENTS</div>
    <div class="tab" data-tab="tools">TOOLS</div>
  </div>
  <div id="content">
    <div id="chat-log"></div>
  </div>
  <div id="input-row">
    <input id="msg-input" placeholder="Message to agents..." />
    <button id="send-btn">SEND</button>
  </div>
`;

// Tab switching
document.querySelectorAll('.tab').forEach(t => {
  t.addEventListener('click', () => {
    document.querySelectorAll('.tab').forEach(x => x.classList.remove('active'));
    t.classList.add('active');
    activeTab = t.dataset.tab;
    renderTab();
  });
});

function addMsg(role, text) {
  messages.push({ role, text, ts: Date.now() });
  if (activeTab === 'chat') renderTab();
}

function renderTab() {
  const c = $('content');
  if (activeTab === 'chat') {
    c.innerHTML = '<div id="chat-log"></div>';
    const log = $('chat-log');
    messages.forEach(m => {
      const d = document.createElement('div');
      d.className = `msg ${m.role}`;
      d.innerHTML = `<div class="label">${m.role === 'user' ? 'You' : '⚡ Agent'}</div>${m.text}`;
      log.appendChild(d);
    });
    log.scrollTop = log.scrollHeight;
  } else if (activeTab === 'agents') {
    c.innerHTML = '<div id="agents-list"><div style="color:#555;font-size:11px;padding:8px">Loading agents...</div></div>';
    if (ws?.readyState === 1) ws.send(JSON.stringify({ jsonrpc:'2.0', method:'a2a_list_agents', params:{}, id: Date.now() }));
  } else if (activeTab === 'tools') {
    c.innerHTML = '<div id="tools-list"><div style="color:#555;font-size:11px;padding:8px">Loading tools...</div></div>';
    if (ws?.readyState === 1) ws.send(JSON.stringify({ jsonrpc:'2.0', method:'tools/list', params:{}, id: Date.now() }));
  }
}

// WebSocket
function connect() {
  ws = new WebSocket('ws://localhost:9005/ws');
  ws.onopen = () => {
    $('status').textContent = 'connected';
    $('status').className = 'connected';
    addMsg('agent', '✅ Connected to Trinity server at :9005');
  };
  ws.onclose = () => {
    $('status').textContent = 'disconnected';
    $('status').className = 'disconnected';
    setTimeout(connect, 3000);
  };
  ws.onerror = () => {
    $('status').textContent = 'error';
  };
  ws.onmessage = (e) => {
    try {
      const data = JSON.parse(e.data);
      if (data.result?.agents) {
        const list = document.getElementById('agents-list');
        if (list) list.innerHTML = data.result.agents.map(a =>
          `<div class="agent-item"><div class="name">${a.name || a.id}</div><div class="cap">${(a.capabilities||[]).join(', ')}</div></div>`
        ).join('') || '<div style="color:#555;font-size:11px;padding:8px">No agents registered</div>';
      } else if (data.result?.tools) {
        const tl = document.getElementById('tools-list');
        if (tl) tl.innerHTML = data.result.tools.map(t =>
          `<div class="tool-item"><div class="tname">${t.name}</div><div class="tdesc">${t.description||''}</div></div>`
        ).join('');
      } else if (data.result) {
        addMsg('agent', JSON.stringify(data.result, null, 2));
      } else if (data.error) {
        addMsg('agent', '❌ ' + data.error.message);
      }
    } catch {}
  };
}

// Send
$('send-btn').addEventListener('click', send);
$('msg-input').addEventListener('keydown', e => { if (e.key === 'Enter') send(); });

function send() {
  const input = $('msg-input');
  const text = input.value.trim();
  if (!text) return;
  addMsg('user', text);
  input.value = '';
  if (ws?.readyState === 1) {
    ws.send(JSON.stringify({ jsonrpc:'2.0', method:'chat', params:{ message: text }, id: Date.now() }));
  } else {
    addMsg('agent', '⚠️ Not connected. Server at :9005 unreachable.');
  }
}

connect();
