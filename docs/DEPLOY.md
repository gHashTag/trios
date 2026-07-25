# Боевой деплой trios-server + trios-host-cdp (macOS)

Связка из двух launchd-юнитов (закон L1 — никаких .sh, весь тулинг на Rust):

| Юнит | Бинарник | Порт/цель |
|------|----------|-----------|
| `com.trios.server` | `trios-server` | HTTP/WS/SSE на **9105** (инвариант I8) |
| `com.trios.host-cdp` | `trios-host-cdp` | поллит `ws://127.0.0.1:9105/ws`, CDP BrowserOS на **9102** |

## Быстрый старт (из деплой-бандла CI)

Workflow `trios-macos-binaries.yml` на каждом пуше в `main` собирает бандл
`trios-macos-deploy.tar.gz` (бинарники + плисты + этот файл):

```
mkdir -p ~/trios && tar -xzf trios-macos-deploy.tar.gz -C ~/trios
cd ~/trios
./bin/trios-deploy install --repo ~/trios --bin-dir ~/trios/bin
./bin/trios-deploy status
./bin/trios-deploy smoke            # против 9105
```

## Из исходников

```
cargo build --release -p trios-server -p trios-host-cdp -p trios-server-xtask
cargo run -p trios-server-xtask --bin trios-deploy -- install
```

`install` рендерит плисты в `~/Library/LaunchAgents`, делает
`launchctl bootstrap gui/$UID` + `kickstart -k` для обоих юнитов.
Логи: `~/Library/Logs/trios/{server,host-cdp}{,.err}.log`.

## Подкоманды trios-deploy

- `render [--repo P] [--bin-dir P] [--port N] [--cdp-http URL] [--out DIR]`
  — рендер плистов без установки (для проверки или не-macOS систем);
- `install` / `uninstall` / `status` — управление юнитами (только macOS);
- `smoke [--host H] [--port N]` — проверка живого сервера:
  `/health`, `/agent/tools` (builtin+browser инструменты),
  WS `browser/poll`, смонтированность `/agent/run`.

## Предусловия на Mac

1. BrowserOS запущен с CDP на 9102 (`--remote-debugging-port=9102`
   или конфиг `cdp_port` в browser-runtime; см. Swift
   `CompanionServerConfig.fallbackCDPPort`).
2. Для реальных прогонов агента: `TRIOS_LLM_BASE_URL` / `TRIOS_LLM_MODEL`
   (иначе дефолт — локальная ollama `http://127.0.0.1:11434/v1`).
3. Порт 9105 свободен (инвариант I8: это MCP/HTTP-порт trios).

## Проверка после установки

```
./bin/trios-deploy smoke --port 9105
curl -s http://127.0.0.1:9105/agent/tools | head
launchctl print gui/$(id -u)/com.trios.server | grep state
```

Откат: `./bin/trios-deploy uninstall`.
