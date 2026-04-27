# TASK — SR-02 (MCP stdio Server)

## Статус
- [x] Структура создана
- [x] `rings/SR-02/src/lib.rs` — ре-экспорты из SR-00 + SR-01
- [ ] Написан `rings/SR-02/Cargo.toml` — с зависимостями
- [ ] Написан `rings/SR-02/TASK.md` — статус задач
- [ ] Написан `rings/SR-02/RING.md` — описание уровня

## Варианты
- [ ] SR-03 (MCP Tools) можно начать параллельно
- [ ] SR-04 (Tunnel) можно начать параллельно
- [ ] BR-XTASK (Launcher) можно начать параллельно

## Зависимости
- SR-02 зависит от SR-00 (Identity & Config)
- SR-03 зависит от SR-02 (MCP stdio)
- SR-04 зависит от SR-02 (MCP stdio)
- BR-XTASK зависит от всех SR

## Следующие шаги
- [ ] Написать SR-03/src/main.rs — HTTP клиент + stdio loop (JSON-RPC 2.0)
- [ ] Написать SR-03/Cargo.toml — workspace зависимостями
- [ ] Написать SR-03/TASK.md — статус задач
- [ ] Создать SR-03/RING.md — описание уровня
