# Вывод TS-бэкенда из эксплуатации (Волна 4)

Единая точка входа бэкенда trios — Rust-крейт `trios-server` (axum, порт
`TRIOS_PORT`, дефолт 9005). Он собирает все консолидированные доменные крейты
и является единственным серверным процессом, который нужно запускать.

## Что уже в Rust (единый язык бэкенда)

| Домен | Крейт | Источник в TS | Статус |
|---|---|---|---|
| Персистентность | `trios-store` | `db/schema.ts` (drizzle SQLite) | ✅ перенесено (Волна 0) |
| A2A-протокол | `trios-a2a` | `api/routes/a2a.ts` + Swift | ✅ паритет схем + persistence (Волна 1) |
| Безопасность | `trios-server/security.rs` | — | ✅ Origin-guard (Волна 1) |
| Агентный слой | `trios-agent-harness` | `lib/agents/*` | ✅ типы/каталог/очередь/turn-registry (Волна 2) |
| Браузер | `trios-browser` | `browser/*` | ✅ контракты + протокол действий (Волна 2) |
| OpenClaw-шлюз | `trios-openclaw` | `lib/agents/openclaw/*`, `hermes/*` | ✅ ACP-команда + provider-map (Волна 3) |
| HTTP-слой | `trios-http`, `trios-server` | `api/server.ts` | ✅ axum |
| Чат | `trios-chat` | `api/routes/chat.ts` | ✅ |
| MCP | `trios-mcp`, `trios-server/mcp.rs` | `api/routes/mcp.ts` | ✅ |

Единая доменная точка проверена: `GET /api/adapters` отдаёт каталог адаптеров
(из `trios-agent-harness`) + Hermes-провайдеры и порт шлюза (из
`trios-openclaw`) — прямо из перенесённых крейтов, без TS.

## Что осознанно остаётся вне Rust (host-runtime рядом с машиной)

Эти компоненты управляют внешними процессами и живут там, где они исполняются;
переносить их в Rust нецелесообразно. Rust хранит их **контракты**, а исполнение
проксируется по A2A.

- **CDP-драйвер браузера** (`browser/browser.ts`, 1683 LOC) — живёт рядом с
  процессом Chrome. Rust: `trios-browser` (данные + `BrowserCommand`/`Response`).
- **VM/контейнеры** (`lib/vm/lima-cli.ts`, `lib/container/*`,
  `api/services/openclaw/*`) — управляют Lima/nerdctl на хосте. Rust:
  `trios-openclaw` (сборка ACP-команды + provider-map).
- **Swift-приложение** (`trios/`) — нативный клиент; общается с бэкендом по A2A
  (schema-паритет обеспечен в Волне 1).

## План отключения TS-сервера (checklist)

1. ✅ Все доменные контракты и чистая логика перенесены в Rust-крейты.
2. ✅ `trios-server` собирает доменные крейты и отдаёт консолидированный
   surface (`/api/adapters`, A2A, chat, MCP, health).
3. ⏭️ **Переключение клиентов на `trios-server`** — требует деплой-решения,
   не простой замены порта. Фактическая топология портов (проверено в коде):
   - Клиенты **уже config-driven**, порты не захардкожены:
     - Swift `ProjectPaths.swift` читает порты из `Info.plist` на этапе сборки:
       `TRIOS_MCP_PORT=9105`, `TRIOS_A2A_PORT=9200`, `TRIOS_CANARY_MCP_PORT=9205`,
       `TRIOS_MESH_PORT=9505`.
     - MCP-bridge (`trios-mcp-bridge/src/config.ts`) — `http://127.0.0.1:9200/mcp`,
       переопределяется флагом `--browserclaw-url`.
   - **Важно:** `9005` — это CDP-порт Chrome (`ports.ts`, `ServerManager.swift`),
     а не A2A-порт. Формулировка «9200→9005» была неточной. `trios-server`
     по умолчанию слушает `TRIOS_PORT` (дефолт 9005), но клиенты ходят на
     9105 (MCP) и 9200 (A2A).
   - **Решение к деплою:** выбрать, на каком порту публикует свой A2A/MCP
     surface единый `trios-server`.
   - ✅ **Код-сторона готова:** `trios-server` теперь читает порт в порядке
     `TRIOS_PORT` → `TRIOS_MCP_PORT` → 9005. Поскольку Swift-клиент уже
     инжектит `TRIOS_MCP_PORT` через `Info.plist`, перенаправление на единый
     сервер не требует правок в коде клиента — только деплой-решение, какой
     процесс слушает этот порт.
4. ✅ Host-runtime адаптер браузера подключён end-to-end. SR-03
   (`BrowserCommandQueue`) реэкспортирован из `trios-a2a`; `AppState` держит
   `BrowserState`; в роутере добавлены `browser/enqueue` (сервер кладёт
   команду), `browser/poll` (host-CDP-агент забирает) и `browser/result`
   (агент отчитывается). CDP-драйвер и VM/контейнеры остаются тонкими
   исполнителями на хосте, вызываемыми через эту очередь.
5. ✅ `packages/browseros-agent/apps/server` помечен как **DEPRECATED**
   (баннер в его README). Физическое удаление дублирующих HTTP-роутов —
   после того, как деплой переключит клиентов (п.3).
6. ✅ Убран хардкод macOS-пути из `ecosystem.config.js` (P4 из аудита):
   `TRIOS_ROOT = process.env.TRIOS_ROOT || __dirname`, все порты теперь
   переопределяемы через env, дефолты сохранены.

## Статус аудита (P0–P4)

| ID | Проблема | Статус |
|---|---|---|
| P0 | Origin-guard отсутствует | ✅ Закрыт (Волна 1) |
| P1 | Расхождение схем | ✅ Закрыт (Волна 1) |
| P2 | Сообщение/задача несуществующему агенту копится вечно | ✅ Закрыт (SR-02: проверка получателя/ассайни) |
| P3 | Нет лимита регистраций / TTL очереди | ✅ Закрыт (SR-02: MAX_AGENTS + bounded message log) |
| P4 | Хардкод macOS-пути в `ecosystem.config.js` | ✅ Закрыт (env-параметризация) |

Осталась **только деплой-часть** пункта 3 (выбор порта + рестарт
клиентов) и финальное удаление дублирующего Hono-сервера (п.5) после
переключения. Всё остальное по консолидации бэкенда закрыто.
