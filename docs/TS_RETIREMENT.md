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
     surface единый `trios-server`, и обновить `Info.plist` вариантов +
     `trios-mcp-bridge` config. Код-правка минимальна (только дефолты), но не
     может быть залита в одиночку без согласования портового плана.
4. ⏭️ Оставить host-runtime адаптеры (браузер CDP, VM/контейнеры) как тонкие
   исполнители, вызываемые из Rust по A2A.
5. ⏭️ Пометить `packages/browseros-agent/apps/server` как deprecated и удалить
   дублирующие HTTP-роуты после переключения клиентов.
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

Пункты 3–5 требуют координации с деплоем клиентов и выполняются отдельно от
консолидации бэкенда.
