# trios-host-cdp — хост-CDP-агент (кольца HC-00..02)

Rust-замена хостового браузерного раннера: поллит SR-03-очередь
`browser_commands` в trios-server и исполняет команды в реальном Chrome
через сырой CDP (без chromiumoxide — один WS, id-корреляция).

| Кольцо | Крейт | Ответственность |
|--------|-------|-----------------|
| HC-00 | `trios-host-cdp-hc00` | CDP-клиент: discovery `/json/list`, id-коррелированный `call()`, пропуск протокольных событий |
| HC-01 | `trios-host-cdp-hc01` | Исполнитель всех 12 SR-03 `browser_*` команд поверх `CdpCall`-трейта (Page.navigate / Runtime.evaluate / Page.captureScreenshot / Target.*) |
| HC-02 | `trios-host-cdp-hc02` | Поллинг-цикл: WS `{method,params}`→`{result}`, `browser/poll` → исполнение → `browser/result`, реконнект с бэкоффом, пропуск broadcast-событий |

Бинарник: `cargo run -p trios-host-cdp`. Env:
`TRIOS_SERVER_WS` (ws://127.0.0.1:9005/ws), `TRIOS_CDP_HTTP`
(http://127.0.0.1:9102), `TRIOS_BROWSER_AGENT_ID` (host-cdp),
`TRIOS_POLL_INTERVAL_MS` (1000).

Правила колец: HC-N зависит только от HC-(N-1) и внешних крейтов;
семантика команд — только в HC-01; транспорт — только в HC-00/02.
