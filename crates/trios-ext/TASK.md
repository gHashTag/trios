# 📋 TASK.md — SILVER RING `trios-ext`

> **Агент**: прочитай `RING.md` до начала любой работы.
> **Закон**: один PR = один таск. clippy 0 warnings обязателен.

---

## 🟢 Статус: ACTIVE

**Последнее обновление**: 2026-04-22
**Текущий блокер**: нет

---

## ✅ Выполнено

- [x] **#232-T1** — IssueTracker tab в `dom.rs` + `mcp.rs`
  - `[Issues]` 4й таб в `build_ui()`
  - `set_issue_list()` парсит JSON, рендерит `.issue-item`
  - `append_issue()` добавляет одиночный issue
  - `list_issues()` → MCP `issues/list` запрос
  - CSS стили в константе `STYLE`

- [x] **#232-T2** — Content injectors (pure Rust/WASM)
  - `injector/github.rs`: `github_inject_button()` — кнопка `⬡ Trinity` на PR/Issue
  - `injector/claude.rs`: `claude_dispatch()` + `claude_inject_text()` + `claude_auto_submit()`
  - `content_scripts` зарегистрированы в `manifest.json`
  - Bootstrap scripts: `github-bootstrap.js`, `claude-bootstrap.js`

- [x] **#232-T3** — Инвариант I5 соблюдён (нет `/extension` в корне)

- [x] **#232-FIX** — Model `glm-5.1`/`zai`, убраны дубли `#[wasm_bindgen(start)]`

---

## 🔵 Очередь (следующие задачи)

- [ ] **EXT-001** — `dom.rs`: live-update IssueTracker через MCP polling (30s interval)
  - Приоритет: HIGH
  - Блокер: нет
  - Инварианты: I4 (no WebSocket), I15 (no JS logic)

- [ ] **EXT-002** — `injector/github.rs`: парсить и показывать статус issue прямо в кнопке
  - `⬡ Trinity [open: 5]` вместо просто `⬡ Trinity`
  - Приоритет: MEDIUM

- [ ] **EXT-003** — `mcp.rs`: обработка ошибок MCP (timeout, 5xx) с retry логикой
  - Приоритет: HIGH
  - Максимум 3 попытки, exponential backoff

- [ ] **EXT-004** — `dom.rs`: Agents tab — список активных агентов из MCP `agents/list`
  - Приоритет: MEDIUM
  - Зависит от: MCP endpoint `agents/list` (GOLD ring)

- [ ] **EXT-005** — `extension/manifest.json`: добавить `linear.app/*` в content_scripts
  - `injector/linear.rs`: inject Trinity кнопку на Linear issues
  - Приоритет: LOW

---

## 🔴 Блокеры

_нет активных блокеров_

---

## 📐 Правила работы в этом RING

```
1. cargo clippy -p trios-ext = 0 warnings  → обязательно перед PR
2. wasm-pack build --target web             → проверить что собирается
3. Изменения в manifest.json               → проверить все content_scripts
4. Новый injector                          → добавить в TASK.md + README.md архитектуру
5. Изменение MCP эндпоинта                → обновить инварианты в RING.md
6. Нет handwritten JS                      → всё в Rust, bootstrap только loader
```

---

## 🔗 Связанные RINGS

| Ring | Металл | Отношение |
|------|--------|----------|
| `trios-core` | 🥇 GOLD | мы зависим — не трогать без согласования |
| `trios-codec` | 🥇 GOLD | мы зависим — не трогать без согласования |
| `scripts/` | 🥉 BRONZE | они зависят от нас — наш output их input |
| `docs/` | 🥉 BRONZE | они зависят от нас — обновлять после API changes |
