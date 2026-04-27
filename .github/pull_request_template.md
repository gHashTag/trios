## Что сделано

<!-- Краткое описание изменений -->

## 🔒 ARCH checklist (обязательно)

> Агент: заполни ДО отправки PR. Незаполненный чеклист = PR закрыт.

- [ ] Я прочитал корневой `AGENTS.md` перед написанием кода
- [ ] `crates/trios-ext/src/lib.rs` — **не тронут** (git diff пуст)
- [ ] `crates/trios-ext/src/bg.rs` — **не тронут**
- [ ] `crates/trios-ext/src/mcp.rs` — **не тронут**
- [ ] `crates/trios-ext/src/bridge/` — **не тронут**
- [ ] `extension/manifest.json`, `background.js`, `content.js` — **не тронуты**
- [ ] Все новые кольца содержат `README.md` + `TASK.md` + `AGENTS.md` (I5)
- [ ] `cargo build --all` exits 0 (I1)
- [ ] `cargo clippy --all-targets -- -D warnings` = 0 warnings (I3)
- [ ] Нет `wasm-pack` в коде (I15)
- [ ] Нет handwritten JS в WASM crates (I15)

## Список изменённых файлов

```
# вставь сюда: git diff --name-only main...HEAD
```

## Ссылки

- Closes #
- Ref #238 (ring-архитектура)
- Ref #243 (trios-ui / EX-01)
