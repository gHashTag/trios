# Chat Context Access — Implementation Plan

**Goal:** Enable BrowserOS agents to access and read chat history from other sessions.

**Date:** 2026-07-24  
**Status:** In Progress  
**Priority:** P0

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     BrowserOS Agent Server                       │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  /api/chats  │  │  /api/a2a    │  │  /api/agents │          │
│  │   (NEW)      │  │  (existing)  │  │  (existing)  │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                 │                 │                   │
│         ▼                 ▼                 ▼                   │
│  ┌─────────────────────────────────────────────────┐           │
│  │           ChatHistoryService (NEW)              │           │
│  │  - listConversations(profileId)                 │           │
│  │  - getConversation(conversationId)              │           │
│  │  - searchConversations(query, profileId)        │           │
│  └─────────────────────┬───────────────────────────┘           │
│                        │                                        │
│                        ▼                                        │
│  ┌─────────────────────────────────────────────────┐           │
│  │        GraphQL Client (existing)                │           │
│  │  - GetConversationsForHistoryDocument           │           │
│  │  - ConversationMessages                         │           │
│  └─────────────────────┬───────────────────────────┘           │
│                        │                                        │
└────────────────────────┼────────────────────────────────────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │   PostgreSQL DB     │
              │   (Neon/Railway)    │
              │  - conversations    │
              │  - conversation_    │
              │    messages         │
              │  - profiles         │
              └─────────────────────┘
```

---

## Phase 1: Chat History API

### 1.1 Endpoint: GET /api/chats

**Purpose:** List all conversations for a profile

**Request:**
```http
GET /api/chats?profileId={id}&limit=50&offset=0
```

**Response:**
```json
{
  "conversations": [
    {
      "id": "conv-001",
      "profileId": "user-123",
      "lastMessagedAt": "2026-07-24T10:30:00Z",
      "preview": "Last 2 messages...",
      "messageCount": 42
    }
  ],
  "totalCount": 150,
  "hasMore": true
}
```

### 1.2 Endpoint: GET /api/chats/:conversationId

**Purpose:** Get full transcript of a specific conversation

**Request:**
```http
GET /api/chats/conv-001?limit=100&offset=0
```

**Response:**
```json
{
  "conversation": {
    "id": "conv-001",
    "profileId": "user-123",
    "createdAt": "2026-07-20T08:00:00Z",
    "lastMessagedAt": "2026-07-24T10:30:00Z",
    "messages": [
      {
        "id": "msg-001",
        "role": "user",
        "content": "Hello...",
        "timestamp": "2026-07-24T10:28:00Z"
      },
      {
        "id": "msg-002",
        "role": "assistant",
        "content": "Hi there...",
        "timestamp": "2026-07-24T10:29:00Z"
      }
    ]
  }
}
```

### 1.3 Endpoint: GET /api/chats/search

**Purpose:** Search across all conversations by text query

**Request:**
```http
GET /api/chats/search?q=A2A architecture&profileId=user-123&limit=20
```

**Response:**
```json
{
  "results": [
    {
      "conversationId": "conv-005",
      "messageId": "msg-042",
      "content": "...discussing A2A architecture...",
      "timestamp": "2026-07-23T14:00:00Z",
      "context": ["preceding message", "following message"]
    }
  ],
  "totalMatches": 15
}
```

---

## Phase 2: Session Persistence

### 2.1 Problem
Current `SessionStore` uses in-memory `Map<string, AgentSession>` — sessions lost on restart.

### 2.2 Solution
Add SQLite persistence layer for session state:

```typescript
interface PersistedSession {
  conversationId: string
  profileId: string
  createdAt: string
  lastAccessedAt: string
  state: SerializedAgentState  // JSON-serializable
  browserContext?: BrowserContextState
}
```

**Storage:** `/Users/playra/BrowserOS/packages/browseros-agent/data/sessions.db`

### 2.3 Implementation
- On session create → write to DB
- On session access → update lastAccessedAt
- On session delete → soft delete (archive)
- On server start → restore active sessions

---

## Phase 3: Agent Chat Access Control

### 3.1 ACL Model
```typescript
interface ChatAccessRule {
  agentId: string
  conversationId?: string  // undefined = all conversations
  permissions: ['read', 'write', 'delete']
  scope: 'profile' | 'global' | 'specific'
}
```

### 3.2 Default Rules
- Doctor (orchestrator): read/write all
- Guard: read-only (code review context)
- Scout: read-only (research context)
- Deployer: read-only (deployment context)

---

## Implementation Checklist

- [ ] **Phase 1: Chat History API**
  - [ ] Create ChatHistoryService class
  - [ ] Implement GraphQL client integration
  - [ ] Add GET /api/chats endpoint
  - [ ] Add GET /api/chats/:id endpoint
  - [ ] Add GET /api/chats/search endpoint
  - [ ] Add TypeScript types/schemas
  - [ ] Write unit tests
  - [ ] Integration test with running server

- [ ] **Phase 2: Session Persistence**
  - [ ] Create SQLite database schema
  - [ ] Add session serialization/deserialization
  - [ ] Modify SessionStore to use persistence
  - [ ] Add session restoration on startup
  - [ ] Add cleanup job for stale sessions

- [ ] **Phase 3: Access Control**
  - [ ] Define ACL schema
  - [ ] Add ACL middleware to chat routes
  - [ ] Add default rules for built-in agents
  - [ ] Add admin endpoint to manage ACL

- [ ] **Phase 4: Documentation**
  - [ ] API documentation
  - [ ] Usage examples
  - [ ] Migration guide

---

## Technical Notes

### GraphQL Integration
Use existing GraphQL client from:
`packages/browseros-agent/apps/agent/entrypoints/sidepanel/history/graphql/chatHistoryDocument.ts`

### Database Schema (existing)
```graphql
type Conversation {
  rowId: String!
  profileId: String!
  createdAt: Datetime!
  lastMessagedAt: Datetime!
  conversationMessages: ConversationMessageConnection!
}

type ConversationMessage {
  rowId: String!
  conversationId: String!
  message: String!
  role: String!  # user | assistant | system
  orderIndex: Int!
  createdAt: Datetime!
}
```

### Dependencies
- `@graphql-request` — for GraphQL queries
- `better-sqlite3` — for session persistence (if not already present)

---

## Success Criteria

1. ✅ Agent can list all user conversations via API
2. ✅ Agent can read full transcript of any conversation
3. ✅ Agent can search across conversations by keyword
4. ✅ Sessions survive server restart
5. ✅ No breaking changes to existing chat functionality
6. ✅ All endpoints have tests

---

## Rollback Plan

If issues arise:
1. Disable new endpoints via feature flag
2. Revert to in-memory SessionStore
3. Archive SQLite DB for debugging

---

## Next Steps

1. Implement ChatHistoryService
2. Add /api/chats routes
3. Test with existing data
4. Document API usage
