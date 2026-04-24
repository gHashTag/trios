//! UR-08 — Trinity Theme Stylesheet

pub const GOLD: &str = "#D1AD72";
pub const BLACK: &str = "#000000";
pub const GOLD_DIM: &str = "#8a6f3e";
pub const SURFACE: &str = "#0d0d0d";
pub const SURFACE2: &str = "#161616";
pub const BORDER: &str = "#2a2310";

pub const STYLESHEET: &str = r#"
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

html, body {
  background: #000000;
  color: #D1AD72;
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 13px;
  height: 100%;
  overflow: hidden;
}

#main {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #000000;
}

.header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-bottom: 1px solid #2a2310;
  background: #000000;
  flex-shrink: 0;
}

.header h1 {
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.18em;
  color: #D1AD72;
  text-transform: uppercase;
  flex: 1;
}

.phi-icon {
  font-size: 22px;
  color: #D1AD72;
  line-height: 1;
}

.status {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.06em;
  padding: 3px 8px;
  border-radius: 10px;
}

.status.connected {
  color: #4ade80;
  background: rgba(74,222,128,0.08);
  border: 1px solid rgba(74,222,128,0.2);
}

.status.error {
  color: #f87171;
  background: rgba(248,113,113,0.08);
  border: 1px solid rgba(248,113,113,0.2);
}

.tabs {
  display: flex;
  border-bottom: 1px solid #2a2310;
  background: #000000;
  flex-shrink: 0;
}

.tab {
  flex: 1;
  padding: 10px 4px;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  color: #8a6f3e;
  font-family: inherit;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}

.tab:hover {
  color: #D1AD72;
}

.tab.active {
  color: #D1AD72;
  border-bottom: 2px solid #D1AD72;
}

.tab-content {
  display: none;
  flex: 1;
  overflow-y: auto;
  padding: 12px 14px;
  flex-direction: column;
}

.tab-content.active {
  display: flex;
}

/* CHAT */
#messages {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-bottom: 8px;
}

.message {
  max-width: 80%;
  padding: 8px 12px;
  border-radius: 10px;
  font-size: 13px;
  line-height: 1.5;
  word-break: break-word;
}

.message.you {
  align-self: flex-end;
  background: #1a150a;
  border: 1px solid #2a2310;
  color: #D1AD72;
  border-bottom-right-radius: 3px;
}

.message.agent {
  align-self: flex-start;
  background: #0d0d0d;
  border: 1px solid #2a2310;
  color: #c8a85e;
  border-bottom-left-radius: 3px;
}

.message.error {
  align-self: flex-start;
  background: rgba(248,113,113,0.06);
  border: 1px solid rgba(248,113,113,0.2);
  color: #f87171;
  border-bottom-left-radius: 3px;
}

#chat-input {
  width: 100%;
  background: #0d0d0d;
  border: 1px solid #2a2310;
  border-radius: 8px;
  color: #D1AD72;
  font-family: inherit;
  font-size: 13px;
  padding: 10px 14px;
  outline: none;
  transition: border-color 0.15s;
  margin-top: 8px;
  flex-shrink: 0;
}

#chat-input:focus {
  border-color: #D1AD72;
  background: #0a0900;
}

#chat-input::placeholder {
  color: #3d3020;
}

/* AGENTS & TOOLS lists */
#agent-list, #tool-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  color: #c8a85e;
  font-size: 12px;
  line-height: 1.7;
  white-space: pre-wrap;
}

.tool-item {
  padding: 5px 8px;
  border-radius: 5px;
  border-left: 2px solid #2a2310;
  color: #c8a85e;
  transition: border-color 0.1s;
}

.tool-item:hover {
  border-left-color: #D1AD72;
  background: #0d0d0d;
}

/* SETTINGS */
.settings-section {
  margin-bottom: 20px;
}

.settings-title {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.15em;
  text-transform: uppercase;
  color: #8a6f3e;
  margin-bottom: 10px;
  padding-bottom: 6px;
  border-bottom: 1px solid #2a2310;
}

.provider-card {
  background: #0d0d0d;
  border: 1px solid #2a2310;
  border-radius: 8px;
  padding: 12px 14px;
  margin-bottom: 8px;
  transition: border-color 0.15s;
}

.provider-card:hover {
  border-color: #D1AD72;
}

.provider-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.provider-name {
  font-size: 12px;
  font-weight: 700;
  color: #D1AD72;
  letter-spacing: 0.08em;
}

.provider-badge {
  font-size: 10px;
  padding: 2px 7px;
  border-radius: 8px;
  font-weight: 600;
}

.provider-badge.active {
  background: rgba(74,222,128,0.1);
  color: #4ade80;
  border: 1px solid rgba(74,222,128,0.25);
}

.provider-badge.inactive {
  background: rgba(209,173,114,0.06);
  color: #8a6f3e;
  border: 1px solid #2a2310;
}

.token-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.token-input {
  flex: 1;
  background: #000000;
  border: 1px solid #2a2310;
  border-radius: 6px;
  color: #c8a85e;
  font-family: inherit;
  font-size: 11px;
  padding: 6px 10px;
  outline: none;
  transition: border-color 0.15s;
}

.token-input:focus {
  border-color: #D1AD72;
}

.token-input::placeholder {
  color: #2a2310;
}

.btn {
  background: #1a150a;
  border: 1px solid #D1AD72;
  border-radius: 6px;
  color: #D1AD72;
  font-family: inherit;
  font-size: 11px;
  font-weight: 600;
  padding: 6px 12px;
  cursor: pointer;
  transition: background 0.15s;
  white-space: nowrap;
}

.btn:hover {
  background: #2a2310;
}

.btn-sm {
  padding: 4px 8px;
  font-size: 10px;
}

.env-hint {
  font-size: 10px;
  color: #3d3020;
  margin-top: 4px;
}

.env-hint span {
  color: #8a6f3e;
  font-family: monospace;
}

/* version stamp */
#ver {
  position: fixed;
  bottom: 7px;
  right: 11px;
  color: #3d3020;
  font-size: 10px;
  pointer-events: none;
  z-index: 9999;
}
"#;
