# 🥈 SILVER RING — `trios-ext`

> **Metal tier**: SILVER — рабочий слой, браузерная интеграция
> **Ring level**: зависит от GOLD (core φ + codec), обеспечивает BRONZE (scripts/CI)

---

## ⬡ Назначение

`trios-ext` — WASM-расширение браузера Trinity. Соединяет φ-математическое ядро с браузерной средой через MCP-протокол. Инжектирует Trinity UI в GitHub и Claude.ai, отображает IssueTracker, управляет агентами прямо из браузера.

---

## 🔗 Топология зависимостей

```
🥇 GOLD (core, codec)           ← trios-ext НЕ СОБИРАЕТСЯ без этого
        │
        ▼
🥈 SILVER (trios-ext)           ← ЭТОТ RING
        │
        ▼
🥉 BRONZE (scripts, CI, docs)   ← зависит от нас
```

**Requires (GOLD):**
- `trios-core` — φ-константы, GF16 типы
- MCP протокол (issues/list, chat/send)

**Provides (для BRONZE и агентов):**
- WASM build artifact: `extension/dist/trios_ext_bg.wasm`
- JS bindings: `extension/dist/trios_ext.js`
- Content scripts: `github-bootstrap.js`, `claude-bootstrap.js`
- Manifest: `extension/manifest.json`

---

## 📐 Инварианты (LAWS)

| ID | Закон | Проверка |
|----|-------|----------|
| I4 | Ноль WebSocket в WASM | `grep -r WebSocket src/` = пусто |
| I5 | Extension только в `crates/trios-ext/extension/` | Нет `/extension` в корне репо |
| I7 | `wasm-unsafe-eval` в CSP | `manifest.json` содержит флаг |
| I13 | Accept: application/json во всех MCP запросах | `mcp.rs` headers |
| I14 | REST /chat endpoint (не WebSocket) | `mcp.rs` transport |
| I15 | Ноль handwritten JS (кроме bootstrap loader) | Весь логик в Rust/WASM |
| I16 | `cargo clippy -p trios-ext` = 0 warnings | CI обязателен |

---

## 🏗️ Архитектура

```
crates/trios-ext/
├── src/
│   ├── lib.rs          — WASM entry point, #[wasm_bindgen(start)]
│   ├── dom.rs          — UI: build_ui(), set_issue_list(), tabs
│   ├── mcp.rs          — MCP клиент: list_issues(), send_chat()
│   └── injector/
│       ├── github.rs   — github_inject_button() на PR/Issue страницах
│       └── claude.rs   — claude_dispatch() в ProseMirror
├── extension/
│   ├── manifest.json   — MV3, content_scripts, CSP
│   └── dist/           — cargo build artifacts (НЕ КОММИТИТЬ .wasm вручную)
└── xtask/              — build helpers
```

---

## 🚫 Запрещено

- Импортировать модули из BRONZE уровня
- Создавать WebSocket соединения в WASM
- Писать бизнес-логику на JavaScript (только bootstrap loader)
- Дублировать `#[wasm_bindgen(start)]` (только один на модуль)
- Размещать extension файлы вне `crates/trios-ext/extension/`
- Мержить PR с clippy warnings

---

## ✅ Верификация перед PR

```bash
# Обязательно
cargo clippy --all-targets -p trios-ext  # 0 warnings
cargo test -p trios-ext                  # все тесты зелёные
cargo build -p trios-ext --target wasm32-unknown-unknown --release  # сборка проходит

# Проверка инвариантов
grep -r "WebSocket" crates/trios-ext/src/  # должно быть пусто
ls extension/ 2>/dev/null && echo 'VIOLATION I5' || echo 'I5 OK'
```

---

## 🤖 Контекст для агентов

Агент, работающий в этом RING:
1. **Прочитай этот файл полностью** до начала работы
2. Проверь что GOLD ring собирается: `cargo build -p trios-core`
3. Читай `TASK.md` для текущих задач
4. Каждый PR = один таск из TASK.md
5. После изменений запускай верификацию выше
6. Не трогай файлы из GOLD ring без явного указания

> φ² + 1/φ² = 3 — математика определяет архитектуру
