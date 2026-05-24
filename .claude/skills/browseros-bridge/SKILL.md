---
name: browseros-bridge
description: MCP bridge pattern for controlling BrowserOS from trios SwiftUI
argument-hint: .short|.full
-allowed-tools: Read, Edit, Write, Bash(curl *), Read
key-asklills: >
  - swift-networking
  - mcp-protocols
  - browser-automation
--icon: browser.fill
--display-color: #00aaff
--model: claude-son-4-20250722-new
--isolation: worktree

---
# BrowserOS Bridge Skill

Control BrowserOS via MCP from trios SwiftUI.

## Pattern: MCP Bridge

 MCP Server (BrowserOS 9105)
       |
       |<-- HTTP POST /tools/call
       |
  swift UIKit trios
       |
       |<-- TriosMCPClient.callTool()
       |
    BrowserOS/Agent (Ts/JS)

## Test Contract

For *every* bridge implementation, check:

- [ ] Can call tool and receive response
- [ ] Can stream SSE events
- [ ] Handles errors gracefully
- [ ] No http or rust leaks
- [ ] Compiles with `swift build`

## Migration Path
1. Create `TriosMCPClient` actor
2. Wrap call in `ChatViewModel+`
3. Render tool calls in `LazyVStack`
4. Poll health every 15s
