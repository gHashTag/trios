//! UR-08 — Trinity Brand Theme
//!
//! CSS constants and theme values following Trinity Brand Kit.
//! Total Black palette with gold accent.

/// Trinity brand colors — Total Black Palette
pub mod colors {
    pub const BLACK: &str = "#000000";
    pub const DARK: &str = "#161616";
    pub const GRAY: &str = "#2a2a2a";
    pub const ACCENT: &str = "#D4AF37";
    pub const PINK: &str = "#F5D3F2";
    pub const PURPLE: &str = "#5D3FF2";
    pub const TEXT: &str = "#ffffff";
    pub const TEXT_MUTED: &str = "#888888";
    pub const ERROR: &str = "#FF6B6B";
    pub const SUCCESS: &str = "#4CAF50";
}

/// Full CSS stylesheet for the Trinity sidebar
pub const STYLESHEET: &str = r#"
:root {
    --trinity-black: #000000;
    --trinity-dark: #161616;
    --trinity-gray: #2a2a2a;
    --trinity-accent: #D4AF37;
    --trinity-pink: #F5D3F2;
    --trinity-purple: #5D3FF2;
    --trinity-text: #ffffff;
    --trinity-text-muted: #888888;
    --trinity-error: #FF6B6B;
    --trinity-success: #4CAF50;
}

* { margin: 0; padding: 0; box-sizing: border-box; }

body {
    background-color: var(--trinity-black);
    color: var(--trinity-text);
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
}

#main {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
}

/* Header */
.header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    background: var(--trinity-dark);
    border-bottom: 1px solid var(--trinity-accent);
}

.header h1 {
    font-size: 16px;
    color: var(--trinity-accent);
    letter-spacing: 2px;
    text-transform: uppercase;
}

.header .status {
    font-size: 11px;
    color: var(--trinity-text-muted);
    margin-left: auto;
}

.header .status.connected {
    color: var(--trinity-success);
}

.header .status.error {
    color: var(--trinity-error);
}

/* Tabs */
.tabs {
    display: flex;
    background: var(--trinity-dark);
    border-bottom: 1px solid var(--trinity-gray);
}

.tab {
    flex: 1;
    padding: 10px;
    border: none;
    background: transparent;
    color: var(--trinity-text-muted);
    cursor: pointer;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 1px;
    transition: color 0.2s, border-bottom 0.2s;
}

.tab:hover { color: #999; }
.tab.active { color: var(--trinity-accent); border-bottom: 2px solid var(--trinity-accent); }

/* Tab Content */
.tab-content { display: none; flex: 1; padding: 16px; overflow-y: auto; }
.tab-content.active { display: flex; flex-direction: column; }

/* Chat */
#messages {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.message {
    padding: 8px 12px;
    margin: 4px 0;
    border-radius: 6px;
    font-size: 13px;
    line-height: 1.5;
}

.message.you {
    background: var(--trinity-gray);
    color: var(--trinity-text);
    align-self: flex-end;
    border: 1px solid #333;
}

.message.agent {
    background: #0D1117;
    color: var(--trinity-accent);
    align-self: flex-start;
    border: 1px solid #1A3A5C;
}

.message.error {
    background: #2A0A0A;
    color: var(--trinity-error);
}

#chat-input {
    width: 100%;
    padding: 10px 14px;
    border: 1px solid var(--trinity-gray);
    border-radius: 6px;
    background: var(--trinity-dark);
    color: var(--trinity-text);
    font-size: 13px;
    margin-top: 12px;
}

#chat-input:focus {
    outline: none;
    border-color: var(--trinity-accent);
}

#chat-input::placeholder { color: #555; }

/* Agent/Tool lists */
#agent-list, #tool-list {
    font-size: 13px;
    font-family: monospace;
    white-space: pre-wrap;
}
"#;
