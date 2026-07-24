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
3. ⏭️ Перенаправить нативный/Swift-клиент и расширение на порт `trios-server`
   (9005) вместо Hono (9200).
4. ⏭️ Оставить host-runtime адаптеры (браузер CDP, VM/контейнеры) как тонкие
   исполнители, вызываемые из Rust по A2A.
5. ⏭️ Пометить `packages/browseros-agent/apps/server` как deprecated и удалить
   дублирующие HTTP-роуты после переключения клиентов.
6. ⏭️ Убрать хардкод macOS-пути из `ecosystem.config.js` (P4 из аудита).

Пункты 3–6 требуют координации с деплоем клиентов и выполняются отдельно от
консолидации бэкенда.
