# Experience Log — trios-claude Claude Code process bridge (#106)

**Date:** 2026-04-21
**Agent:** ECHO
**Issue:** #106

## What was done
1. Created `crates/trios-claude/` with bridge.rs, process.rs, lib.rs
2. `ClaudeBridge` — spawn/send_task/get_status/list_agents/kill_agent/send_task_streaming
3. `ChildProcess` — agent state with id, name, model, status, pid
4. `AgentStatus` enum — Ready/Idle/Working/Error/Terminated
5. `SpawnConfig` with defaults (model=claude-opus-4-5)
6. Streaming task support with line-by-line callback
7. 10 unit tests (bridge + process)
8. clippy=0 warnings

## Lessons
- tokio::process::Command for async process spawning
- Arc<Mutex<HashMap>> for shared agent state
- `claude --model X --print "task"` is the CLI interface
