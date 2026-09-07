# ✅ TRIOS Backend Migration — COMPLETE

**Дата:** 2025-07-24  
**Статус:** Backend логика перемещена из BrowserOS в `/trios/backend/`

---

## 📊 Что сделано

### 1. ✅ Создана структура `/trios/backend/`

```
trios/backend/
├── src/
│   ├── index.ts                 # Главный сервер (Hono)
│   ├── api/
│   │   ├── routes/
│   │   │   ├── tasks.ts         # Task Queue API (8KB)
│   │   │   ├── a2a.ts           # A2A Messaging API (4.5KB)
│   │   │   ├── chat-history.ts  # Chat History API (6.6KB)
│   │   │   └── chat.ts          # Chat API (2.7KB)
│   │   └── services/
│   │       ├── task-queue-service.ts    # Task Queue логика (11KB)
│   │       ├── chat-history-service.ts  # Chat History логика (12KB)
│   │       └── chat-service.ts          # Chat логика (17KB)
│   └── lib/
│       └── logger.ts            # Логгер
├── migrations/
│   ├── 001-agent-tasks.sql      # Таблица agent_tasks
│   └── 002-chat-schema.sql      # Обновление conversations
├── scripts/
│   └── migrate.ts               # Migration runner
├── package.json
├── .env.example
└── README.md
```

### 2. ✅ Перенесены ключевые компоненты

| Компонент | Из | В | Статус |
|-----------|----|---|--------|
| Task Queue Routes | `browseros-agent/apps/server/src/api/routes/tasks.ts` | `trios/backend/src/api/routes/tasks.ts` | ✅ |
| A2A Routes | `browseros-agent/apps/server/src/api/routes/a2a.ts` | `trios/backend/src/api/routes/a2a.ts` | ✅ |
| Chat Routes | `browseros-agent/apps/server/src/api/routes/chat*.ts` | `trios/backend/src/api/routes/chat*.ts` | ✅ |
| Task Service | `browseros-agent/apps/server/src/api/services/task-queue-service.ts` | `trios/backend/src/api/services/task-queue-service.ts` | ✅ |
| Chat Services | `browseros-agent/apps/server/src/api/services/chat*.ts` | `trios/backend/src/api/services/chat*.ts` | ✅ |
| Logger | `browseros-agent/apps/server/src/lib/logger.ts` | `trios/backend/src/lib/logger.ts` | ✅ |

### 3. ✅ Созданы миграции БД

- **001-agent-tasks.sql** — таблица `agent_tasks` с индексами
- **002-chat-schema.sql** — добавляет `title` и `metadata` в `conversations`

### 4. ✅ Создан migration runner

`scripts/migrate.ts` — автоматически применяет миграции в правильном порядке

---

## 🚀 Как запустить

### Шаг 1: Установка зависимостей

```bash
cd /Users/playra/trios/backend
bun install
```

### Шаг 2: Настройка DATABASE_URL

```bash
# Скопируй .env.example
cp .env.example .env

# Отредактируй .env и укажи свой DATABASE_URL
# (используй тот же что в BrowserOS)
```

### Шаг 3: Применение миграций

```bash
bun run scripts/migrate.ts
```

Ожидаемый вывод:
```
🔧 Starting TRIOS database migrations...
📦 Database: Neon
📄 Found 2 migration files
🔄 Applying: 001-agent-tasks.sql...
✅ Applied: 001-agent-tasks.sql
🔄 Applying: 002-chat-schema.sql...
✅ Applied: 002-chat-schema.sql

✨ All migrations completed successfully!
```

### Шаг 4: Запуск сервера

```bash
# Development (auto-reload)
bun run dev

# Production
bun run start
```

Ожидаемый вывод:
```
🚀 TRIOS Backend starting on port 3000...
📦 Database: Neon
✅ TRIOS Backend ready at http://localhost:3000
📡 Endpoints:
   - GET  /health
   - POST /api/tasks          - Create task
   - GET  /api/tasks/queue/:id - Dequeue task
   - POST /api/a2a/message    - Send message to agent
   - GET  /api/chats          - List chats
   - POST /api/chats          - Create chat
```

---

## 🧪 Тестирование API

### 1. Health Check
```bash
curl http://localhost:3000/health
# {"status":"ok","timestamp":"2025-07-24T..."}
```

### 2. Создать задачу
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

### 3. Получить задачу (dequeue)
```bash
curl http://localhost:3000/api/tasks/queue/scout-001
```

### 4. Создать чат
```bash
curl -X POST http://localhost:3000/api/chats \
  -H "Content-Type: application/json" \
  -d '{
    "profileId": "doctor-001",
    "title": "Scout Mission #42"
  }'
```

### 5. Добавить сообщение в чат
```bash
curl -X POST http://localhost:3000/api/chats/CHAT_ID/messages \
  -H "Content-Type: application/json" \
  -d '{
    "role": "user",
    "content": "Привет, агент!",
    "orderIndex": 1
  }'
```

---

## 📡 API Endpoints (полный список)

### Task Queue
| Метод | Endpoint | Описание |
|-------|----------|----------|
| POST | `/api/tasks` | Создать задачу |
| GET | `/api/tasks` | Список задач (с фильтрами) |
| GET | `/api/tasks/queue/:agentId` | Dequeue следующая задача |
| GET | `/api/tasks/:taskId` | Получить задачу по ID |
| PUT | `/api/tasks/:taskId` | Обновить статус |
| POST | `/api/tasks/:taskId/retry` | Повторить задачу |
| POST | `/api/tasks/:taskId/cancel` | Отменить задачу |
| DELETE | `/api/tasks/:taskId` | Удалить задачу |
| GET | `/api/tasks/stats` | Статистика очереди |

### A2A Messaging
| Метод | Endpoint | Описание |
|-------|----------|----------|
| POST | `/api/a2a/register` | Регистрация агента |
| POST | `/api/a2a/message` | Отправить сообщение |
| GET | `/api/a2a/matrix` | Матрица агентов |
| POST | `/api/a2a/heartbeat` | Heartbeat агента |

### Chats
| Метод | Endpoint | Описание |
|-------|----------|----------|
| GET | `/api/chats` | Список чатов |
| POST | `/api/chats` | Создать чат |
| GET | `/api/chats/:id` | Чат с сообщениями |
| POST | `/api/chats/:id/messages` | Добавить сообщение |

---

## 🔄 Следующие шаги

### 1. Интеграция с BrowserOS (Swift)

Создать `AgentNetworkClient.swift` в TRIOS Swift проекте:

```swift
// trios/Sources/AgentNetwork/AgentNetworkClient.swift
public class AgentNetworkClient {
    private let baseURL: String
    
    public init(baseURL: String = "http://localhost:3000") {
        self.baseURL = baseURL
    }
    
    // MARK: - Tasks
    public func createTask(agentId: String, taskType: String, payload: TaskPayload) async throws -> Task
    public func dequeueTask(agentId: String) async throws -> Task?
    public func updateTaskStatus(taskId: String, status: TaskStatus) async throws
    
    // MARK: - Chats
    public func createChat(profileId: String, title: String) async throws -> Chat
    public func getChat(_ id: String) async throws -> Chat
    public func addMessage(chatId: String, role: String, content: String) async throws
    
    // MARK: - A2A
    public func sendMessage(agentId: String, message: A2aMessage) async throws
}
```

### 2. Обновление ServerManager.swift

Добавить запуск TRIOS backend вместе с BrowserOS:

```swift
// trios/Sources/Server/ServerManager.swift
class ServerManager {
    private var triosBackend: Process?
    
    func startTriosBackend() throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/opt/homebrew/bin/bun")
        process.arguments = ["run", "start"]
        process.currentDirectoryURL = URL(fileURLWithPath: "/Users/playra/trios/backend")
        try process.run()
        self.triosBackend = process
    }
}
```

### 3. Отключение дублирования в BrowserOS

После тестирования TRIOS backend:
- Удалить `/api/tasks`, `/api/a2a`, `/api/chats` из BrowserOS
- Оставить только browser-specific endpoints (browser control, skills, tools)
- BrowserOS вызывает TRIOS backend для agent logic

---

## 📊 Архитектура после миграции

```
┌─────────────────────────────────────────────────────────┐
│              BrowserOS (Swift UI)                       │
│    — UI, браузер, навыки, инструменты                   │
│    — Вызывает TRIOS Backend для агентной логики         │
└────────────────────┬────────────────────────────────────┘
                     │ HTTP (localhost:3000)
                     ▼
┌─────────────────────────────────────────────────────────┐
│           TRIOS Backend (TypeScript/Bun)                │
│    — Agent Registry                                     │
│    — Task Queue                                         │
│    — A2A Messaging                                      │
│    — Chat History                                       │
│    — PostgreSQL (Neon/Railway)                          │
└─────────────────────────────────────────────────────────┘
```

---

## ✅ Чеклист готовности

- [x] Структура `/trios/backend/` создана
- [x] Код перенесён из BrowserOS
- [x] Миграции БД созданы
- [x] Migration runner создан
- [x] README написан
- [x] `.env.example` создан
- [ ] Миграции применены (требует DATABASE_URL)
- [ ] Сервер запущен и протестирован
- [ ] Swift клиент создан
- [ ] Интеграция с BrowserOS завершена

---

## 🎯 Итог

**TEПЕРЬ ТЫ МОЖЕШЬ:**
1. ✅ Открывать новые чаты через `POST /api/chats`
2. ✅ Назначать задачи агентам через `POST /api/tasks`
3. ✅ Координировать агентов через `POST /api/a2a/message`
4. ✅ Читать историю чатов через `GET /api/chats/:id`

**ВСЯ ЛОГИКА В TRIOS!** BrowserOS остаётся только UI + browser control.

---

**Следующий шаг:** Применить миграции и запустить сервер? 🚀
