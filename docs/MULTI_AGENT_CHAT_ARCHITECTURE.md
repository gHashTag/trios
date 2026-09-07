# Multi-Agent & Chat History Architecture

## 🎯 Problem Statement

Current BrowserOS limitations:
- ❌ No access to other chat sessions (isolated per-session)
- ❌ Cannot spawn new agents or chat sessions
- ❌ No inter-agent communication protocol
- ❌ Memory is limited to CORE.md + daily notes (not full transcripts)

## 🏗️ Proposed Architecture

### Layer 1: Chat History Storage

```
/Users/playra/trios/.trios/sessions/
├── YYYY-MM-DD/
│   ├── session-{uuid}.json        # Full transcript
│   ├── session-{uuid}.summary.md  # Auto-generated summary
│   └── session-{uuid}.meta.json   # Metadata (agent, duration, topics)
└── index.json                      # Searchable index of all sessions
```

**Session Schema:**
```json
{
  "id": "uuid",
  "timestamp": "2026-01-15T14:30:00Z",
  "agent_id": "doctor-001",
  "agent_name": "🔬 Doctor",
  "duration_seconds": 3600,
  "messages": [
    {
      "role": "user|assistant|system|tool",
      "content": "...",
      "timestamp": "...",
      "tool_calls": [],
      "snapshots": []
    }
  ],
  "topics": ["architecture", "multi-agent", "browseros"],
  "summary": "Designed multi-agent architecture...",
  "artifacts": ["/path/to/generated/files"]
}
```

### Layer 2: Session Index (Search Engine)

**Location:** `/Users/playra/trios/.trios/sessions/index.json`

```json
{
  "sessions": [
    {
      "id": "uuid",
      "date": "2026-01-15",
      "agent": "🔬 Doctor",
      "topics": ["architecture", "multi-agent"],
      "summary": "Brief summary...",
      "path": "2026-01-15/session-uuid.json"
    }
  ],
  "topic_index": {
    "architecture": ["uuid1", "uuid2"],
    "multi-agent": ["uuid1", "uuid3"]
  }
}
```

### Layer 3: Agent Registry

**Location:** `/Users/playra/trios/.trios/agents/registry.json`

```json
{
  "agents": [
    {
      "id": "doctor-001",
      "name": "🔬 Doctor",
      "role": "Code surgeon, architect",
      "tools": ["BrowserOS", "GitHub", "A2A"],
      "soul_path": ".trios/agents/doctor/SOUL.md",
      "status": "active|idle|busy",
      "current_session": "uuid-or-null"
    },
    {
      "id": "guard-001",
      "name": "🛡️ Guard",
      "role": "Code quality, tests, linting",
      "tools": ["GitButler", "test-suite"],
      "soul_path": ".trios/agents/guard/SOUL.md",
      "status": "active|idle|busy"
    },
    {
      "id": "scout-001",
      "name": "🔍 Scout",
      "role": "Research, context gathering",
      "tools": ["RAG", "search", "NotebookLM"],
      "soul_path": ".trios/agents/scout/SOUL.md",
      "status": "active|idle|busy"
    },
    {
      "id": "deployer-001",
      "name": "🚀 Deployer",
      "role": "Deployment, scaling, health",
      "tools": ["Railway", "Docker", "health-checks"],
      "soul_path": ".trios/agents/deployer/SOUL.md",
      "status": "active|idle|busy"
    }
  ]
}
```

### Layer 4: Task Queue

**Location:** `/Users/playra/trios/.trios/run/task-queue.json`

```json
{
  "queue": [
    {
      "id": "task-001",
      "created_at": "2026-01-15T14:30:00Z",
      "assigned_to": "scout-001",
      "status": "pending|in-progress|completed|failed",
      "type": "research|code|deploy|test",
      "description": "Research A2A protocols",
      "context": {
        "source_session": "uuid",
        "related_files": [],
        "priority": "high"
      },
      "result": {
        "output": "...",
        "artifacts": [],
        "completed_at": "..."
      }
    }
  ]
}
```

### Layer 5: Inter-Agent Communication Protocol (A2A v2)

**Protocol:** SSE + HTTP POST

```
Agent A → Task Queue → Agent B
         (HTTP POST /tasks)

Agent B → Result → Agent A
         (HTTP POST /results or SSE stream)
```

**Message Schema:**
```json
{
  "type": "task|result|status|heartbeat",
  "from_agent": "doctor-001",
  "to_agent": "scout-001",
  "payload": {
    "task_id": "task-001",
    "action": "research",
    "query": "A2A protocols",
    "context": {...},
    "deadline": "2026-01-15T15:00:00Z"
  }
}
```

## 🔧 Implementation Plan

### Phase 1: Chat History Storage (Week 1)

**Goal:** Persist all chat sessions to disk

**Tasks:**
1. Create session storage structure
2. Modify BrowserOS server to write transcripts
3. Add session metadata extraction (topics, summary)
4. Create session viewer UI in Queen

**Files to create:**
- `trios/scripts/session-persister.js` — Auto-save sessions
- `trios/scripts/session-summarizer.js` — Generate summaries
- `trios/apps/queen/views/SessionHistory.swift` — UI

### Phase 2: Session Index & Search (Week 2)

**Goal:** Make sessions searchable

**Tasks:**
1. Build session index (JSON + optional SQLite)
2. Add topic extraction (keyword + LLM-based)
3. Create search API (`GET /sessions/search?q=...`)
4. Add search UI in Queen

**Files to create:**
- `trios/scripts/session-indexer.js` — Build/maintain index
- `trios/api/sessions.js` — REST API for sessions
- `trios/apps/queen/views/SessionSearch.swift` — Search UI

### Phase 3: Agent Registry (Week 3)

**Goal:** Register and track multiple agents

**Tasks:**
1. Create agent registry schema
2. Define SOUL.md templates for each agent type
3. Add agent status tracking (active/idle/busy)
4. Create agent management UI

**Files to create:**
- `trios/.trios/agents/doctor/SOUL.md`
- `trios/.trios/agents/guard/SOUL.md`
- `trios/.trios/agents/scout/SOUL.md`
- `trios/.trios/agents/deployer/SOUL.md`
- `trios/apps/queen/views/AgentRegistry.swift`

### Phase 4: Task Queue (Week 4)

**Goal:** Enable task assignment between agents

**Tasks:**
1. Implement task queue data structure
2. Add task creation API
3. Add task polling for agents
4. Create task dashboard UI

**Files to create:**
- `trios/scripts/task-queue-manager.js`
- `trios/api/tasks.js` — REST API
- `trios/apps/queen/views/TaskQueue.swift`

### Phase 5: Inter-Agent Communication (Week 5-6)

**Goal:** Full A2A protocol implementation

**Tasks:**
1. Define SSE streaming protocol
2. Implement agent-to-agent messaging
3. Add task result callbacks
4. Create communication logs

**Files to create:**
- `trios/crates/a2a-protocol/` — Rust library
- `trios/scripts/agent-orchestrator.js`
- `trios/apps/queen/views/AgentCommunication.swift`

## 📊 Current State vs Target

| Feature | Current | Target |
|---------|---------|--------|
| Chat persistence | ❌ None | ✅ JSON files + index |
| Session search | ❌ None | ✅ Full-text + topics |
| Multiple agents | ⚠️ Manual | ✅ Registry + status |
| Task assignment | ❌ None | ✅ Queue + auto-assign |
| Inter-agent comms | ❌ None | ✅ SSE + HTTP |
| Cross-session context | ❌ Memory only | ✅ Full transcript access |

## 🚀 Quick Wins (Can implement today)

### 1. Manual Session Export Script

```bash
#!/bin/bash
# trios/scripts/export-session.sh
# Export current session to file

SESSION_ID=$(uuidgen)
DATE=$(date +%Y-%m-%d)
mkdir -p /Users/playra/trios/.trios/sessions/$DATE

# Export from BrowserOS memory
cp ~/.browseros/memory/daily/*.md \
   /Users/playra/trios/.trios/sessions/$DATE/session-$SESSION_ID.md

echo "Session exported to /Users/playra/trios/.trios/sessions/$DATE/session-$SESSION_ID.md"
```

### 2. Simple Agent Status File

```json
// /Users/playra/trios/.trios/run/agent-status.json
{
  "doctor": {"status": "active", "session": "current-session-id"},
  "guard": {"status": "idle"},
  "scout": {"status": "idle"},
  "deployer": {"status": "idle"}
}
```

### 3. Manual Task Assignment

```bash
# Create task manually
echo '{"task": "research A2A", "assigned_to": "scout"}' >> /Users/playra/trios/.trios/run/task-queue.json

# Agent polls for tasks
cat /Users/playra/trios/.trios/run/task-queue.json | jq '.queue[] | select(.assigned_to == "scout")'
```

## 🔐 Security Considerations

1. **Session isolation** — Agents should only access sessions they participated in (unless explicitly granted)
2. **Task permissions** — Some tasks require user confirmation (deploy, delete, send messages)
3. **Rate limiting** — Prevent agent spam in task queue
4. **Audit log** — Log all inter-agent communications

## 📈 Scaling Considerations

- **SQLite vs JSON** — For >1000 sessions, migrate index to SQLite
- **Agent pools** — Multiple instances of same agent type (scout-001, scout-002)
- **Task priorities** — Add priority queue for urgent tasks
- **Distributed agents** — Agents running on different machines (requires network protocol)

## 🎯 Success Metrics

- [ ] Can search and retrieve any past session by topic/keyword
- [ ] Can spawn a new agent instance via API
- [ ] Can assign a task to another agent
- [ ] Can retrieve task results asynchronously
- [ ] Can view all agent statuses in Queen UI

---

**Status:** 📝 Design Phase  
**Next Action:** User review → Phase 1 implementation  
**Owner:** 🔬 Doctor (coordinating)
