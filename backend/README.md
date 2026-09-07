# TRIOS Backend

Agent Network Server для TRIOS — управление агентами, задачами и чатами.

## 🚀 Быстрый старт

### 1. Установка зависимостей

```bash
cd /Users/playra/trios/backend
bun install
```

### 2. Настройка переменных окружения

Создай `.env` файл:

```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/trios
PORT=3000
```

Или используй существующий Neon/Railway:

```bash
export DATABASE_URL="postgresql://..."
```

### 3. Применение миграций

```bash
bun run scripts/migrate.ts
```

### 4. Запуск сервера

```bash
# Development (auto-reload)
bun run dev

# Production
bun run start
```

Сервер запустится на `http://localhost:3000`

---

## 📡 API Endpoints

### Health Check
- `GET /health` — проверка статуса сервера

### Task Queue
- `POST /api/tasks` — создать задачу для агента
- `GET /api/tasks/queue/:agentId` — получить следующую задачу (dequeue)
- `GET /api/tasks/:taskId` — получить задачу по ID
- `PUT /api/tasks/:taskId` — обновить статус задачи
- `POST /api/tasks/:taskId/retry` — повторить неудачную задачу
- `POST /api/tasks/:taskId/cancel` — отменить задачу
- `DELETE /api/tasks/:taskId` — удалить задачу
- `GET /api/tasks/stats` — статистика очереди

### A2A Messaging
- `POST /api/a2a/register` — зарегистрировать агента
- `POST /api/a2a/message` — отправить сообщение агенту
- `GET /api/a2a/matrix` — матрица агентов
- `POST /api/a2a/heartbeat` — обновить heartbeat агента

### Chats
- `GET /api/chats` — список чатов
- `POST /api/chats` — создать новый чат
- `GET /api/chats/:id` — получить чат с сообщениями
- `POST /api/chats/:id/messages` — добавить сообщение в чат

---

## 🗄️ Database Schema

### agent_tasks
```sql
- id UUID PRIMARY KEY
- agent_id VARCHAR(255)
- task_type VARCHAR(255)
- payload JSONB
- priority INTEGER
- status VARCHAR (pending/running/completed/failed/cancelled)
- retry_count INTEGER
- max_retries INTEGER
- created_at, started_at, completed_at TIMESTAMPTZ
- error_message TEXT
- result JSONB
- assigned_by VARCHAR(255)
- metadata JSONB
```

### conversations (существующая)
```sql
- id UUID
- profile_id VARCHAR(255)
- title VARCHAR(500)  -- добавлено миграцией 002
- metadata JSONB     -- добавлено миграцией 002
- created_at, updated_at TIMESTAMPTZ
```

---

## 🧪 Тестирование

### Создать задачу
```bash
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "agentId": "scout-001",
    "taskType": "research",
    "payload": {"type": "search", "data": {"query": "Trinity"}},
    "priority": 10
  }'
```

### Получить задачу (dequeue)
```bash
curl http://localhost:3000/api/tasks/queue/scout-001
```

### Создать чат
```bash
curl -X POST http://localhost:3000/api/chats \
  -H "Content-Type: application/json" \
  -d '{
    "profileId": "doctor-001",
    "title": "Scout Mission #42"
  }'
```

---

## 📦 Структура проекта

```
trios/backend/
├── src/
│   ├── index.ts                 # Точка входа
│   ├── api/
│   │   ├── routes/
│   │   │   ├── tasks.ts         # Task Queue endpoints
│   │   │   ├── a2a.ts           # A2A messaging endpoints
│   │   │   ├── chat-history.ts  # Chat history endpoints
│   │   │   └── chat.ts          # Chat endpoints
│   │   └── services/
│   │       ├── task-queue-service.ts
│   │       ├── chat-history-service.ts
│   │       └── chat-service.ts
│   └── lib/
│       └── logger.ts            # Логгер (нужно создать)
├── migrations/
│   ├── 001-agent-tasks.sql
│   └── 002-chat-schema.sql
├── scripts/
│   └── migrate.ts               # Migration runner
├── package.json
└── README.md
```

---

## 🔧 Интеграция с BrowserOS (Swift UI)

BrowserOS использует этот сервер через HTTP API:

```swift
// AgentNetworkClient.swift
class AgentNetworkClient {
    static let shared = AgentNetworkClient(baseURL: "http://localhost:3000")
    
    func createTask(agentId: String, taskType: String, payload: TaskPayload) async throws -> Task
    func dequeueTask(agentId: String) async throws -> Task?
    func createChat(profileId: String, title: String) async throws -> Chat
    func sendMessage(agentId: String, message: A2aMessage) async throws -> Message
}
```

---

## 📝 License

AGPL-3.0-or-later © 2025 TRIOS
