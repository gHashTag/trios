# Trinity Agent Bridge - Chrome Extension

Direct human↔agent communication via trios MCP WebSocket server on port 7474.

## Features

- **Real-time agent board**: View all connected agents with live status
- **Quick actions**: Start, sync, or stop agents with one click
- **Command input**: Send commands to specific agents with command history
- **Issue tracking**: Track GitHub issues and claim them for work
- **Auto-inject**: Automatically inject and submit agent output on claude.ai
- **GitHub sidebar**: Show agent status sidebar on issue #30

## Agent Status

- **Idle**: Agent waiting for work
- **Claiming**: Agent claiming an issue
- **Working**: Agent actively working
- **Blocked**: Agent blocked, waiting for something
- **Done**: Agent completed its task

## Development

```bash
npm install
npm run dev          # Build with HMR
npm run build        # Production build
npm run check        # TypeScript check
```

## Installation

1. Build the extension: `npm run build`
2. Load in Chrome: `chrome://extensions/` → Load unpacked → Select `extension/dist`
3. Or package as ZIP: Build and upload to Chrome Web Store

## Architecture

```
Chrome Extension ←→ WebSocket (port 7474) → trios-bridge (Rust)
     │                                          │
     │                                          ├── Agent Router
     │                                          ├── GitHub API
     │                                          └── CLI (tri bridge)
```

## Files

- `manifest.json`: Chrome Extension Manifest V3
- `src/background/service-worker.ts`: WebSocket client and message handling
- `src/content/`: Platform-specific injectors (claude, github, cursor)
- `src/popup/`: React UI components
- `src/shared/`: TypeScript types matching Rust protocol
