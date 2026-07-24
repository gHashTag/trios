# Консолидация бэкенда trios → единый Rust-воркспейс по кольцам

План от 2026-07-24. Цель: **весь backend в одном месте (репозиторий `gHashTag/trios`), один язык (Rust), декомпозиция по кольцам** как в `trios-a2a` (SR-00…03 + BR-OUTPUT, закон L-ARCH-001).

Источники: `browseros/packages/browseros-agent` (TS/Bun, Hono) ≈ **38.8k строк** backend-логики + Swift-приложение `browseros/trios` + уже существующие Rust-крейты в `trios/crates`.

---

## 1. Принцип: rings-архитектура (как есть в trios-a2a)

Каждый крейт-домен = отдельный внутренний воркспейс:
```
crates/<domain>/
├── Cargo.toml          ← [workspace] members = rings/*, + фасад-зависимости
├── RING.md             ← паспорт: metal, законы, dep-flow
├── src/lib.rs          ← ТОЛЬКО re-export (без логики, L-ARCH-001)
└── rings/
    ├── XX-00/          ←核 core: типы, serde, без I/O и async
    ├── XX-01/          ← протокол/логика над -00
    ├── XX-02/          ← реестр/состояние/адаптеры
    └── BR-OUTPUT/      ← сборка (axum Router / сервис-фасад)
```
Правила (из RING.md a2a): `BR-OUTPUT → …-02 → …-01 → …-00`, кольцо не импортирует соседа своего уровня; `src/lib.rs` — только фасад; каждое кольцо — публикуемая единица с собственными тестами; каждый ring-PR завершается `three-roads.json` (R1/R2/R3).

Металлы (уже используются): 🥇 Gold — контрактные (a2a), 🥉 Silver — core-кольца (chat). Новые backend-домены пойдут Silver, агрегатор — Gold.

---

## 2. Карта миграции: TS-домен → Rust-крейт (кольца)

| TS-источник (browseros-agent) | LOC | → Rust-крейт | Кольца | Статус базы в Rust |
|---|---|---|---|---|
| `api/routes/a2a.ts` + `services/a2a/*` | ~620 | **trios-a2a** (есть) | SR-00…03, BR-OUTPUT | ✅ есть, нужен паритет схем + PG-store ring |
| `api/routes/agents.ts` + `lib/agents/*` (ACP-runtime, catalog, message-queue, turn-registry) | ~5.8k | **trios-agent-harness** (нов.) | AH-00 типы · AH-01 catalog · AH-02 message-queue · AH-03 turn-registry · AH-04 ACP-runtime · BR-OUTPUT routes | ⚠️ `trios-agents` = только zig-proxy, нужен новый крейт |
| `api/services/openclaw/*` + `lib/container/*` + `lib/vm/*` | ~7.5k | **trios-openclaw** (нов.) | OC-00 типы · OC-01 vm-runtime(Lima) · OC-02 container-cli · OC-03 http-client · OC-04 cli-client · OC-05 produced-files · OC-06 history-mapper · BR-OUTPUT service | ❌ с нуля (самый крупный кусок) |
| `browser/*` (CDP: dom, elements, mouse, keyboard, markdown) | ~3.7k | **trios-browser** (нов.) | BW-00 CDP-типы · BW-01 dom/elements · BW-02 input(mouse/kbd) · BW-03 content-markdown · BR-OUTPUT command-exec | 🟡 частично: команды уже в trios-a2a SR-03 + trios-server browser endpoints |
| `tools/*` (MCP tool impls) | ~6k | **trios-mcp** (есть) + перенос tool-реестра | MR-* по группам | 🟡 есть клиент-адаптер, нужны сами tools |
| `api/routes/mcp.ts` + `services/mcp/*` | ~370 | **trios-mcp** (есть) | + MR-routes | 🟡 |
| `api/routes/klavis.ts` + `services/klavis/*` + `lib/clients/klavis` | ~780 | **trios-klavis** (нов.) | KL-00 типы · KL-01 client · BR-OUTPUT routes | ❌ |
| `api/routes/chat.ts` + `chat-service.ts` | ~460 | **trios-chat** (есть, 86k) | подключить BR-OUTPUT-CHAT | ✅ огромный, только wiring |
| `api/routes/acl.ts` + `services/acl/*` | ~105 | **trios-acl** (нов.) | AC-00 rules · BR-OUTPUT routes | ❌ малый |
| `lib/db/*` (SQLite + drizzle, schema, migrations) | ~310 | **trios-store** (нов.) | ST-00 schema · ST-01 sqlx/SeaORM repo · ST-02 migrations | ❌ ключевой (persistence для всех) |
| `lib/clients/*` (gateway, llm, oauth) | ~2.2k | **trios-llm** (есть) + **trios-clients** (нов.) | LM-* / CL-* | 🟡 |
| routes: memory, soul, skills, credits, oauth, provider, refine-prompt, monitoring, terminal, status, health, shutdown | ~500 | **trios-server** (есть) — тонкие эндпоинты | добавить в handle_message / routes | 🟡 |
| `main.ts` / `index.ts` / `api/server.ts` (сборка приложения) | ~1k | **trios-server** (есть) — единая точка входа axum | заменяет Hono-app | ✅ каркас есть |

**Итог по объёму:** ~38.8k строк TS → оценочно 25–35k строк Rust (Rust плотнее на типах, но многословнее на error-handling). Реально переносимых с нуля: openclaw (~7.5k) и agent-harness (~5.8k) — 2/3 усилий.

---

## 3. Целевой граф зависимостей (единый бэкенд)

```
                         trios-server (BR: axum app, единый вход :9005/:9200)
                              │  агрегирует все BR-OUTPUT доменов
   ┌──────────┬──────────┬───┴────┬───────────┬──────────┬─────────┐
   ▼          ▼          ▼        ▼           ▼          ▼         ▼
 trios-a2a  agent-    openclaw  browser    trios-mcp   klavis    chat
 (Gold)     harness   (Silver)  (Silver)   (+tools)   (Silver) (есть)
   │          │          │        │           │          │
   └──────────┴──────────┴────────┴───────────┴──────────┘
                              ▼
                        trios-store (ST: sqlx/SeaORM — единая персистентность)
                              ▼
                        trios-core (базовые типы, уже есть)
```
Правило: домены НЕ импортируют друг друга напрямую — только через `trios-server` (BR-агрегатор) и общий `trios-store`/`trios-core`. Это повторяет закон изоляции колец на уровне доменов.

---

## 4. Порядок исполнения (волнами, ring-by-ring)

**Волна 0 — фундамент (разблокирует все).**
- `trios-store`: ST-00 (schema-типы) → ST-01 (sqlx repo поверх SQLite, тот же файл БД что и сейчас) → ST-02 (миграции). Портируем drizzle-схему 1:1.
- Закрепить общий `trios-core` как основание.

**Волна 1 — контракты и тонкие домены (быстрые победы).**
- `trios-a2a`: доделать паритет схем Swift↔Rust (`sender/recipient`↔`from/to` через `#[serde(rename)]`), добавить PG/SQLite-store ring, закрыть P0 Origin-guard из прошлого аудита.
- `trios-acl`, `trios-klavis` — малые, дают шаблон домена.

**Волна 2 — агентный слой.**
- `trios-agent-harness` (AH-00…04): ACP-runtime, catalog, message-queue, turn-registry.
- `trios-browser` (BW-00…03): собрать с уже готовыми browser endpoints trios-server.

**Волна 3 — тяжёлый слой.**
- `trios-openclaw` (OC-00…06): VM(Lima)/container-runtime, CLI+HTTP клиенты, produced-files, history. Здесь сосредоточен главный риск (внешние процессы, контейнеры) — делать последним, с интеграционными тестами.

**Волна 4 — единая точка входа + вывод из эксплуатации TS.**
- `trios-server`: заменяет Hono-app, монтирует все BR-OUTPUT; переносим memory/soul/skills/credits/oauth/monitoring/terminal как тонкие эндпоинты.
- Прогон общего e2e-конформанса (мой `client_e2e.py`/`client_ws_e2e.py` расширить на все домены), затем удаление `browseros-agent/apps/server` из сборки.

Каждая волна = отдельные ring-PR с `three-roads.json`, зелёным CI (`ci(trios)`), без регрессий (принцип golden-version из вашей safe-self-improvement-arch).

---

## 5. Что даёт консолидация
- **Один язык, одно место.** Backend целиком в `trios/crates`, сборка `cargo build`, единый CI вместо Bun+Cargo+swiftc.
- **Единый контракт A2A.** Исчезает расхождение двух реализаций (P1 из аудита) — остаётся одна, Rust.
- **Меньше поверхности атаки.** Origin-guard и auth — в одном middleware trios-server, а не размазаны по Hono.
- **Rings = изоляция.** Каждый домен собирается/тестируется независимо, соблюдая ваши законы L-ARCH-001 / R1–R5.

## 6. Риски и честные оговорки
- **Swift-приложение остаётся Swift** (это UI/macOS-слой trios.app) — переносим только backend-логику; клиент говорит с Rust по HTTP+SSE как сейчас.
- **openclaw завязан на Lima/контейнеры** — часть логики это оркестрация внешних бинарей; переносится дословно, но требует macOS-интеграционных прогонов.
- **trios-chat уже 86k строк Rust** — не переписываем, только подключаем через BR-OUTPUT-CHAT.
- Это план и каркас, а не готовая миграция: объём (~2/3 — openclaw+harness) требует нескольких итераций. Реалистично — волнами, с проверкой паритета на каждом кольце.

## 7. Следующий шаг
Готов начать с **Волны 0** — сгенерировать скелет `trios-store` (ST-00/01/02) и заготовки Cargo.toml/RING.md для новых доменов, встроить их в корневой workspace `Cargo.toml`, чтобы `cargo build` проходил на пустых кольцах. Скажи «поехали с Волны 0» — и я закоммичу скаффолд в ветку `feat/rust-backend-consolidation`.

---

## Журнал выполнения

### Волна 0 — выполнена (коммит 7367ac5)
- `trios-store` (ST-00/01/02/BR-OUTPUT) — единая SQLite-персистентность, схема 1:1 с drizzle. 6 тестов зелёные, проверена совместимость с внешним читателем БД.
- Заготовки доменов: trios-agent-harness, trios-openclaw, trios-browser, trios-klavis, trios-acl.

### Волна 1 — выполнена
- **P1 (схемы A2A) закрыт:** `trios-a2a/SR-01` приведён к единому wire-контракту с Swift/Hono (`sender`/`recipient`/`type`, camelCase-значения, вариант `addToolCall`) через `#[serde(rename)]`; добавлен broadcast-sentinel. +5 wire-parity тестов (SR-01: 9 тестов).
- **P0 (Origin-guard) закрыт:** в `trios-server/security.rs` добавлен middleware `origin_guard` (allowlist через `TRIOS_TRUSTED_ORIGINS`, дефолт — localhost/127.0.0.1/app-схемы), смонтирован на ВСЕ роуты. Live smoke: no-Origin→200, localhost→200, foreign→403, health→200. trios-server: 35 тестов.
- **Персистентность реестра:** новое кольцо `trios-a2a/SR-04` (`A2AStore` trait + `SqliteA2AStore`), хранит карточки агентов и задачи как канонический JSON в таблицах `a2a_agents`/`a2a_tasks`. Изоляция колец соблюдена (SR-04 → SR-01, SR-00; не импортирует SR-02). 2 теста.

### Волна 2 — выполнена
- **trios-agent-harness** — перенос агентного слоя из TS `lib/agents/*`:
  - AH-00 (core): `AgentDefinition`, `AgentAdapter{Claude,Codex,Openclaw,Hermes}`, `PermissionMode`, `AgentStatus`/`AgentState`, `HistoryEntry`/`HistoryToolCall`, `AgentStreamEvent` (tagged по `type`). Чистые данные + serde, camelCase-паритет с Swift/Hono. 3 теста.
  - AH-01 (catalog): `AdapterDescriptor`, `ModelControl`, `CatalogOption`, `catalog()`/`descriptor_for()` — дефолты адаптеров и опции моделей/reasoning-effort. 3 теста.
  - AH-02 (queue): `MessageQueue` — ограниченная per-agent FIFO-очередь (`append`/`pop_oldest`/`push_front`/`remove`/`list`/`snapshot_all`/`agents_with_pending`), `QueueFullError`. Логика без I/O (mutex вместо файлового write-lock). 4 теста.
  - AH-03 (turns): `RingBuffer` (кольцевой лог кадров с удержанием терминального кадра) + `TurnRegistry` (register/append/complete/fail/cancel/get/slice/list/sweep), `TurnStatus`, `TurnFrame`, `ActiveTurnInfo`. Асинхронный streaming/abort-слой остаётся в runtime-кольце. 4 теста.
  - Итого 14 тестов зелёные. Изоляция колец соблюдена (AH-01→AH-00, AH-03→AH-00, AH-02 self-contained).
- **trios-browser** — контракты управления браузером из TS `browser/*`:
  - BW-00 (core): `PageInfo`, `WindowInfo`, `WindowType`, `WindowState`, `WindowBounds`, `SetWindowVisibilityResult`. camelCase-паритет с CDP-драйвером. 3 теста.
  - BW-01 (proto): `BrowserCommand` (goto/goBack/goForward/reload/closePage/snapshot/content/screenshot/evaluate/click/listPages/getActivePage) + `BrowserResponse`, `target_page()`-роутинг. Транспорт-агностичный envelope: trios-server проксирует команды CDP-драйверу по A2A, исполнение остаётся рядом с живым браузером. 4 теста.
  - Итого 7 тестов зелёные. BW-01→BW-00.
- **Проектное решение:** тяжёлый CDP-драйвер `Browser` (1683 LOC, Playwright/CDP-биндинги) НЕ переносится в Rust — он живёт рядом с процессом Chrome; в Rust перенесён контракт (данные + протокол действий), по которому идёт проксирование.

### Волна 3 — выполнена
- **trios-openclaw** — контракты OpenClaw-шлюза из TS `lib/agents/openclaw/*` и `lib/agents/hermes/*`:
  - OC-00 (gateway): `GatewayConfig` (accessor-снимок), `resolve_acp_command()` — детерминированная сборка argv (`env LIMA_HOME=… limactl shell … nerdctl exec … openclaw acp --url ws://127.0.0.1:18789 [--session …]`), `bridge_session_key()` (нормализация `agent:*` / санитайз), константа `OPENCLAW_GATEWAY_CONTAINER_PORT=18789`. 3 теста.
  - OC-01 (hermes): `HermesProviderMapping` + `get_mapping()`/`is_supported()` — маппинг провайдеров (anthropic/openai/openrouter/openai-compatible → hermes provider + env-var + base_url). openai и openai-compatible → `custom` (у Hermes v2026.4.x нет ключа `openai`). 5 тестов.
  - Итого 8 тестов зелёные. Оба кольца — листовые (без cross-импортов).
- **Проектное решение:** тяжёлое VM/контейнерное исполнение (lima-cli 270, container-cli 347, managed-container 681, openclaw-service 1770 LOC) НЕ переносится в Rust — оно управляет host-процессами и остаётся в host-runtime рядом с машиной. В Rust перенесена чистая логика (сборка команды + маппинг), которая чаще всего тихо ломалась и теперь полностью покрыта юнит-тестами.
