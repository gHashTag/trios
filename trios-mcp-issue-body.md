## 🎯 Миссия

Создать крейт `trios-mcp` — Rust клиентский адаптер который подключается к browser-tools-server (3025) и транслирует MCP протокол для Perplexity Agents через WebSocket туннель.

## 📋 Задачи

### Phase 1: Бронзовое кольцо (структура + workspace)

- [ ] Создать директорию `crates/trios-mcp/`
- [ ] Создать `Cargo.toml` с workspace dependencies (tokio, axum, serde, anyhow, tracing)
- [ ] Добавить `trios-mcp` в `workspace.members` корневого `Cargo.toml`
- [ ] Создать `RING.md` по R3

### Phase 2: Серебряные кольца (реализация)

- [ ] **SR-00** (`rings/SR-00/`): MCP protocol core
  - MCP типы (Message, Tool, Resource)
  - JSON-RPC 2.0 сериализация
- [ ] **SR-01** (`rings/SR-01/`): Auth + WebSocket
  - Basic Auth (perplexity:test123 → Base64)
  - WebSocket клиент для browser-tools-server
  - Автоматический reconnect

### Phase 3: Золотое кольцо (интеграция)

- [ ] Точка входа в `src/lib.rs` — агрегирует SR-00 + SR-01
- [ ] Конфигурация (host, port, auth)
- [ ] Health check к browser-tools-server

## ✅ LAWS.md compliance

- [ ] L2: PR закроет этот issue (Closes #\${ISSUE_ID})
- [ ] L3: `cargo clippy -- -D warnings` — чистый
- [ ] L4: `cargo test` — все зелёные
- [ ] L8: каждое изменение = коммит + push
- [ ] R3: `RING.md` создан

## 🔗 Связи

- browser-tools-server:3025 — целевой сервер
- Perplexity Agent MCP Config — будет использовать trios-mcp
- wss://playras-macbook-pro-1.tail01804b.ts.net — туннель

## 📦 Deliverables

1. Рабочий крейт `trios-mcp`
2. Интеграция в trios workspace
3. Документация в `RING.md`
4. Пример конфигурации для Perplexity

---

**Приоритет:** P0 | **Сложность:** Medium | **Время:** ~4h
