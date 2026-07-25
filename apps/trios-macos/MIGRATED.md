# trios-macos — перенесено из browseros

Каноническое расположение Swift-клиента trios (macOS) — **этот каталог**:
`trios/apps/trios-macos/`.

- Источник: `BrowserOS/trios/` (репо browseros, ветка dev, снимок от 2026-07-25).
- Копия в browseros временно сохранена, т.к. на неё ссылается macOS CI
  workflow (`.github/workflows`, коммит 9cf3077) и там активно работает
  другой агент. После переключения CI на этот репозиторий копию в
  browseros нужно удалить.
- Бэкенд для клиента — единый Rust-сервер `crates/trios-server`
  (REST `/a2a/*` + SSE, wire-совместим с `A2ARegistryClient.swift`).

Правки вносить здесь; синхронизацию в browseros не поддерживаем.
