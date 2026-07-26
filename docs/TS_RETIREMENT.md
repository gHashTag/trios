# Вывод TS-бэкенда из эксплуатации (Волна 4)

Единая точка входа бэкенда trios — Rust-крейт `trios-server` (axum, порт
`TRIOS_PORT`, дефолт 9005). Он собирает все консолидированные доменные крейты
и является единственным серверным процессом, который нужно запускать.

## Что уже в Rust (единый язык бэкенда)

| Домен | Крейт | Источник в TS | Статус |
|---|---|---|---|
| Персистентность | `trios-store` | `db/schema.ts` (drizzle SQLite) | ✅ перенесено (Волна 0) |
| A2A-протокол | `trios-a2a` | `api/routes/a2a.ts` + Swift | ✅ паритет схем + persistence (Волна 1) |
| A2A REST+SSE для Swift | `trios-server/rest_a2a.rs` + SR-02 | `api/routes/a2a.ts`, `a2a-registry-service.ts` | ✅ wire-паритет (Волна 5): register/unregister/heartbeat/agents/message/task/stream, liveness-TTL 120s, очереди офлайн-сообщений, watchdog |
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
   - ✅ **Код-сторона готова и закоммичена:**
     - `trios-server` читает порт в порядке `TRIOS_PORT` → `TRIOS_MCP_PORT`
       → 9005 (единый процесс отдаёт MCP, A2A и `/health` на одном порту).
     - `trios/build.sh`: `TRIOS_A2A_PORT` схлопнут на `TRIOS_MCP_PORT`
       (9105), т.е. Swift-клиент (`agentHealthURL`, `mcpBaseURL`) бьёт в
       один бэкенд.
     - `trios-mcp-bridge` default `browserosMcpUrl`: `9200 → 9105`
       (+ help/тест обновлены, тест зелёный).
     - Осталось только операционное: пересобрать `.app` (`build.sh`) и
       запустить `trios-server` на 9105 вместо Hono. Кодом больше менять
       нечего.
4. ✅ Host-runtime адаптер браузера подключён end-to-end. SR-03
   (`BrowserCommandQueue`) реэкспортирован из `trios-a2a`; `AppState` держит
   `BrowserState`; в роутере добавлены `browser/enqueue` (сервер кладёт
   команду), `browser/poll` (host-CDP-агент забирает) и `browser/result`
   (агент отчитывается). CDP-драйвер и VM/контейнеры остаются тонкими
   исполнителями на хосте, вызываемыми через эту очередь.
5. ✅ `packages/browseros-agent/apps/server` помечен как **DEPRECATED**
   (баннер в его README). **Физическое удаление сознательно не делается
   сейчас:** Hono-сервер держит 51 роут (агентный чат-цикл, стриминг,
   файлы, OAuth, провайдеры, сессии, dashboard), а Rust `trios-server`
   покрывает пока только ядро (`/ws /sse /api/chat /api/status
   /api/adapters /health` + A2A/browser). Удалять роуты можно только
   после порта оставшихся host-runtime фичей — что по дизайну вне
   scope консолидации (тяжёлый host-bound код остаётся на хосте).
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

## Волна 5 — REST `/a2a/*` в Rust + перенос приложений (2026-07-25)

Закрыт главный пробел консолидации: Swift-клиент говорит REST
`/a2a/*` + SSE, а у `trios-server` таких роутов не было (только
JSON-RPC/WS) — переключение портов дало бы 404.

- `rings/SR-02`: liveness (heartbeat + TTL 120s, prune), wire-карточки
  клиента (хранятся вербатим), bounded-очереди недоставленного,
  `upsert_task`/`get_task` — чистая логика, время инжектируется
  (18 тестов).
- `trios-server/src/rest_a2a.rs`: 9 роутов (`register`, `unregister`,
  `heartbeat`, `agents`, `matrix`, `message`, `task/assign`, `task/update`,
  `stream` SSE) + `A2aHub` (live-подписчики, drop-guard отписка) +
  вотчдог-прунер раз в минуту (15 тестов, всего в крейте 56).
- **Исправлены два латентных бага TS-сервера** (не воспроизведены):
  1. `GET /a2a/agents` в TS заворачивал ответ в `{"agents": [...]}`, а
     Swift декодирует голый `[AgentCard]` — теперь голый массив.
  2. payload системных сообщений в TS шёл как plain-JSON-строка, а
     Swift `Data` ждёт base64 — теперь объектные payload кодируются
     в base64(JSON).
- **Перенос приложений в монорепо trios («всё в одном месте»):**
  Swift-клиент → `apps/trios-macos/`, MCP-мост → `apps/trios-mcp-bridge/`
  (см. MIGRATED.md в каждом). Копии в browseros помечены баннерами
  «ПЕРЕНЕСЕНО» и будут удалены после переключения macOS CI.

Вся кодовая часть консолидации закрыта. Остались чисто
операционные/продуктовые шаги, которые невозможно закрыть кодом:
1. **Деплой (п.3):** пересобрать `.app` (`build.sh`) и запустить
   `trios-server` на 9105 вместо Hono — теперь включая REST `/a2a/*`,
   который раньше был единственным недостающим куском.
2. **Порт host-runtime фичей (п.5):** после этого удалить 51 роут
   Hono-сервера — отдельная крупная задача, вне scope этой консолидации.
3. **Переключение macOS CI** на `apps/trios-macos/` в репо trios и
   удаление копий в browseros.

## Волна 7 (луп 3): TS-сервер физически удалён из browseros

- `apps/server` → `packages/agent-core` (`@browseros/agent-core`):
  оставлено только host-bound агентное ядро — tool-loop, CDP-драйвер
  браузера, MCP-инструменты, клиенты (`src/{agent,browser,tools,lib,
  skills,monitoring}`). Klavis strata-proxy/cache переехали из
  `api/services` в `lib/clients/klavis`.
- Удалена вся HTTP-поверхность TS: `src/api/` (51 роут Hono), `src/
  {index,main,rpc,config,types}.ts`, тесты API, серверная сборка
  (`scripts/build/server*`, `build:server*`, `release-server.yml`).
- `apps/eval` переключён на `@browseros/agent-core` (10 файлов);
  `apps/agent` больше не зависит от `@browseros/server` (RPC-тип
  локальный: контракт теперь обслуживает Rust `trios-server`).
- Dev-тулинг (Go `tools/dev`, `tools/dogfood`, `process-compose.yaml`,
  `scripts/dev/start.ts`) запускает `cargo run -p trios-server` из
  `$TRIOS_REPO` вместо `bun src/index.ts`.
- CI: `test.yml` — 7 матричных сьютов agent-core (agent, skills, tools,
  browser, integration, lib, root; server-api удалён), `eval-weekly.yml`
  — новые пути, `release-server.yml` удалён.
- Проверки: typecheck agent-core/eval чистые; тесты agent-core
  262+31+279+34+1 зелёные, eval 93/93, build-скрипты 6/6, biome чист.

Итог: в browseros не осталось TS-бэкенда. Единственный бэкенд —
Rust `trios-server` (этот репозиторий, `crates/trios-server`).

## Волна 8 (луп 4): агентный tool-loop перенесён в Rust

- Новый золотой крейт `crates/trios-agent-loop` (Ring Isolation):
  - **AL-00** — OpenAI-совместимый chat-контракт + HTTP-клиент
    (`LlmClient`/`HttpLlmClient`, парсинг tool_calls, конфиг из env
    `TRIOS_LLM_{BASE_URL,API_KEY,MODEL}`; дефолт — ollama).
  - **AL-01** — `Tool`-трейт, `ToolRegistry`, встроенные инструменты и
    6 браузерных (goto, content, screenshot, click, evaluate, list_pages)
    поверх BW-01 `BrowserCommand` через `BrowserBridge`-трейт.
  - **AL-02** — `AgentLoop`: system+user → чередование LLM-ходов и
    исполнения инструментов; стоп-условия (финальный ответ / max_steps,
    TS-паритет `MAX_TURNS=100`), события шагов, транскрипт, усечение
    длинных результатов, учёт токенов.
- REST-поверхность в trios-server (`rest_agent.rs`): `GET /agent/tools`,
  `POST /agent/run`, `POST /agent/run/stream` (SSE: assistant_text /
  tool_call / tool_result / done).
- Браузерные инструменты замкнуты на существующую SR-03 host-runtime
  очередь: сервер кладёт команду, хост-CDP-агент поллит и репортит
  (`QueueBrowserBridge`, таймаут < TTL команды 30s).
- Тесты: 18 в кольцах (вкл. интеграцию HttpLlmClient с мок-эндпоинтом),
  6 в rest_agent (вкл. полный мост через SR-03-очередь с фейковым
  хост-агентом); live-проверка `/agent/run` и SSE против mock-LLM.

## Волна 9 (луп 5): хост-CDP-агент на Rust

- Новый крейт `crates/trios-host-cdp` (кольца HC-00..02) — бинарник,
  который поллит SR-03-очередь trios-server по WS (`browser/poll` →
  `browser/result`) и исполняет команды в Chrome через сырой CDP
  (`/json/list` discovery, id-коррелированный клиент, без тяжёлых стеков).
- Все 12 хост-инструментов SR-03 реализованы: navigate/get_url/get_title/
  get_dom/query_selector/click/type/scroll/eval/screenshot/open_tab/close_tab
  (DOM-действия через Runtime.evaluate c JSON-экранированием селекторов).
- `trios-a2a`: ре-экспорт `BrowserCommandType`.
- Тесты: 12 (мок-CDP WS-сервер с шумовыми событиями, фейковый исполнитель,
  фейковый trios-server); сквозной e2e: /agent/run → tool-loop → очередь →
  бинарник trios-host-cdp → мок-Chrome → ответ модели.

## Волна 10 (луп 6): боевой деплой (macOS)

- Деплой-тулинг на Rust (закон L1): xtask-бинарник `trios-deploy`
  (render / install / uninstall / status / smoke) + launchd-шаблоны
  `deploy/launchd/*.plist.tmpl` (server на 9105, host-cdp → CDP 9102,
  KeepAlive, логи в ~/Library/Logs/trios/).
- CI `trios-macos-binaries.yml` (macos-15): release-сборка trios-server +
  trios-host-cdp + trios-deploy, smoke против живого сервера, plutil-линт
  плистов, публикация деплой-бандла (tar.gz + sha256, 30 дней).
- docs/DEPLOY.md: установка из бандла и из исходников, предусловия
  (BrowserOS CDP 9102, LLM env), проверка и откат.
- Дочистка L1 (та же волна): 4 shell-скрипта, приехавшие с миграцией
  Swift-приложения (волна 5), портированы в Rust-бинарник `trios-app`
  (apps/trios-macos/xtask): build / chat-sse-e2e / mesh-chat-e2e / e2e-flow.
  Laws Guard и основной CI (cargo test) были красными с волны 5 — теперь
  проходят закон L1.

## Волна 11 (луп 7): зелёный CI, наблюдаемость, финал eval-харнесса

- **A. Легаси-тесты igla-race (221/221 + бинари):** исправлены 5 тестов,
  красных с волны 5: off-by-one в drive-циклах hive-автомата (полный цикл
  Boot→…→Done = 7 переходов); знак-инвертированный нижнехвостовой CDF
  t-распределения (df=2): F(t) = 0.5 + t/(2·√(2+t²)) (старый код ещё и
  давал NaN при t<0); защита нулевой дисперсии (p=1.0 — гейт отказывает,
  а не делит на ноль). Пре-регистрированный baseline μ₀ = 1.55
  (= BPB_VICTORY_TARGET + TTEST_EFFECT_SIZE_MIN) восстановлен литералом и
  теперь реально используется stat_strength (исходный баг — константа была
  объявлена, но не применялась). Канарейка ledger_check 14/14.
- **A. I5-доки:** 57 файлов для 47 легаси-колец (README для 42 колец с
  RING.md; честные README/TASK/AGENTS для trios-store ST-00..02/BR-OUTPUT
  и SR-02-скаффолда пайплайна). arch-guard впервые зелёный на main.
- **B. Наблюдаемость SR-03:** счётчики жизненного цикла очереди
  (enqueued/polled/results/rejected), бэкпрешер MAX_PENDING_COMMANDS=256
  (try_enqueue → QueueFull, дубликат идемпотентен), QueueStats;
  `GET /metrics` — Prometheus text 0.0.4 без новых зависимостей
  (9 метрик: глубина/executing/capacity, 4 counters, агенты, задачи).
  При сатурации enqueue отвечает `{error, queue_full:true, depth, capacity}`.
- **C. Финал eval-харнесса (browseros, dev):** тонкий типизированный клиент
  `@browseros/agent-core/lib/clients/trios-server` (POST /agent/run,
  GET /agent/tools; fetch, ноль зависимостей) + новый executor-бэкенд
  `rust-server` в apps/eval — eval-прогоны идут через боевой Rust-цикл
  вместо локального TS tool-loop (max_steps→timeout, ошибки транспорта→
  blocked, toolsUsed из транскрипта). TS tool-loop остаётся только как
  легаси-бейзлайн eval. README agent-core зафиксировал роль пакета.
- **Статус CI (main):** CI ✅, Laws Guard ✅, arch-guard ✅, No-JS ✅,
  Path A ✅, Leaderboard ✅, macOS binaries ✅. Легаси-красным остаётся
  rainbow-bridge (assertions/hive_state.json — пустой файл с волны сквоша;
  падает с мая) — кандидат на следующую волну. В browseros/dev два
  до-волновых красных сьюта: agent-core-tools (tests/__helpers__/server.ts
  удалён вместе с TS-сервером в волну 7 — сьюту нужен спавн Rust-сервера)
  и agent (fetchMcpTools: 14 компиляций схем вместо 0).

## Волна 12 — rainbow-bridge, tools-сьют на Rust-сервере, SeaORM

- **A. rainbow-bridge:** реконструирован `assertions/hive_state.json`
  (пустой с сквоша 8984fa0) — состояние миссии IGLA RACE v2 (L1–L12) и
  `pre_registration.INV-8` с реальным blake3-пином
  `docs/infrastructure/preregistration_rainbow.md`. Воркфлоу зелёный
  впервые с мая 2026; на main зелёны все воркфлоу.
- **B. tools-сьют agent-core (browseros/dev):** сьют, красный с волны 7
  (импорт удалённого TS server.ts), поднят на боевом Rust `trios-server`:
  хелпер спавна (TRIOS_SERVER_BIN/PATH, /health, graceful shutdown,
  опционален — без бинаря browser-only), гейтед HTTP-тест
  (/health, /agent/tools, /metrics), CI-джоба собирает trios-server из
  checkout trios с кешем. Добиты: self-kill раннера в killProcessOnPort
  (теперь lsof -sTCP:LISTEN + фильтр pid) и graceful-skip hidden-window
  тестов на headless (Ozone ≠ x11). Мёртвый withMcpServer удалён.
- **C. trios-store: sqlx → SeaORM.** ST-01 — SeaORM-энтити и Store поверх
  DatabaseConnection (API без изменений, upsert через OnConflict, PRAGMA
  идентичны: WAL, foreign_keys=ON); ST-02 — тот же SCHEMA_DDL через
  execute_unprepared в транзакции; BR-OUTPUT — файловый smoke-тест.
  Тесты вложенного workspace store добавлены в ci.yml (раньше не гонялись
  нигде). 7/7 зелёные.

## Волна 13 (луп 9)

- **A. agent-сьют (browseros): zod-дедуп.** Корень 14 Function-компиляций
  схем — вторая инстанция zod (4.3.5) у @modelcontextprotocol/sdk по
  scoped-записи bun.lock; jitless конфигурировал не ту копию. Постоянный
  `z.config({ jitless: true })` в lib/mcp/client.ts + удаление
  scoped-записи (SDK на hoisted 4.3.6). Сьют 80/0.
- **B. Мёртвая TS-персистентность удалена (2585 строк).** lib/db
  (drizzle: клиент, схема, миграции), db-agent-store + AgentStore,
  серверный OAuth-стек (token-store/manager, callback-server, providers)
  и их тесты; deps drizzle-orm/drizzle-kit, скрипт db:generate,
  протухший bun.lockb. Роль слоя выполняет trios-store (SeaORM/SQLite).
  Массовое удаление остального agent-core по-прежнему отложено: это
  легаси-бейзлайн eval и живой сьют CI-матрицы.
- **C. Release-конвейер trios-server.** ci.yml: после release-сборки —
  smoke бинаря (:9207, /health==ok, /agent/tools ⊇ {echo, browser_goto},
  /metrics ⊇ trios_agents_registered) и артефакт
  trios-server-linux-x86_64 (retention 14 дней). Подтверждено в CI:
  smoke success, артефакт 6.25 МБ.
- **CI: browseros dev @ ac69508 — первый полностью зелёный прогон всей
  матрицы test.yml (12/12, включая легаси agent). trios main @ 9610b73 —
  дашборд зелёный.**

## Волна 14 — деплой-цепочка и полный браузерный e2e

- `/version` (git sha из CI), `deploy/docker/Dockerfile.server` (образ из
  готового релизного бинарника, non-root, healthcheck),
  `deploy/systemd/trios-server.service` (hardened), docker-смок в CI,
  Linux-раздел в DEPLOY.md.
- Новый бинарь `trios-e2e-llm` (скриптованный фейк-LLM, L1) и CI-шаг:
  headless Chrome + trios-server + trios-host-cdp + `/agent/run` —
  впервые весь производственный контур (agent-loop → SR-03 → WS → CDP →
  DOM) проверяется в CI на каждый пуш.
- browseros: apps/agent typecheck 42 ошибки → 0, сьют `agent-typecheck`
  в матрице test.yml.
- Коммиты: trios `a3d3181`, browseros/dev `4708f51`.
- Итог CI: фикс базы образа (glibc 2.39 → ubuntu:24.04, `6309897`);
  run 30167869032 — весь дашборд зелёный, e2e подтверждён транскриптом
  (реальный DOM через SR-03, E2E_OK). browseros: 13/13 c agent-typecheck.

## Волна 15 — релизный пайплайн

- `release.yml`: тег `v*` → штампованный бинарь, смок `/version`
  (sha + версия == тег), tar.gz с деплой-файлами, SHA256SUMS,
  GitHub Release с changelog от предыдущего тега.
- Guard: тег == `[workspace.package] version` (одна точка истины).
- Первый релиз: **trios-server v0.1.0** (`59a4112`, run 30185841316) —
  ассеты: бинарь 17.4 МБ, tar.gz 6.1 МБ, SHA256SUMS.
- Историческая заметка: старые теги v5.x без релизов; позже — поднять
  версию выше 5.x или scoped-теги `server-v*`.

## Волна 16 — чат сайдпанели вернулся (на Rust) + 6.0.0

- Найдена причина «не работает чат»: `POST /chat` умер с TS-сервером,
  панель получала 404. Новый `rest_chat.rs` отдаёт AI SDK v6 UI Message
  Stream поверх агент-лупа; провайдеры — общий маппинг с /test-provider;
  браузерные тулы автоподключаются к активному поллеру SR-03 (60с окно,
  `BrowserState.pollers`).
- Версия воркспейса 0.1.0 → 6.0.0 (продолжение линии v5.x);
  релиз **v6.0.0** опубликован релизным пайплайном (Latest).
- trios-server: 76/76 тестов.
