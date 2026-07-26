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

## Linux (CI-артефакт → systemd или Docker)

CI (`ci.yml`, job Test) на каждом пуше собирает release-бинарь со
штампом коммита (`TRIOS_BUILD_SHA` → `GET /version`), гоняет smoke
(бинарь и Docker-контейнер) и публикует артефакт
**`trios-server-linux-x86_64`** (retention 14 дней).

> Бинарь собран на ubuntu-24.04 и требует glibc ≥ 2.39 (Ubuntu 24.04+,
> Debian trixie+). Для более старых хостов используйте Docker-образ ниже
> (база ubuntu:24.04).

### Забрать артефакт

```
gh run download --repo gHashTag/trios -n trios-server-linux-x86_64 -D /tmp/trios-bin
chmod +x /tmp/trios-bin/trios-server
/tmp/trios-bin/trios-server &   # TRIOS_PORT=9105 по умолчанию не задан → 9005
curl -s http://127.0.0.1:9005/version   # {"name":"trios-server","version":...,"git_sha":...}
```

### systemd

Юнит: `deploy/systemd/trios-server.service` (порт 9105, `TRIOS_A2A_DB`
в `/var/lib/trios/data.db`, hardening: ProtectSystem=strict и т.д.).
Инструкция по установке — в шапке самого юнита.

### Docker

Образ собирается из готового бинаря (без компиляции в контейнере):
`deploy/docker/Dockerfile.server` — debian slim, непривилегированный
пользователь `trios`, порт 9105, HEALTHCHECK по `/health`, OCI-label
с ревизией. Пример сборки — в шапке Dockerfile; проверка:

```
docker run -d -p 9105:9105 trios-server:<sha>
curl -s http://127.0.0.1:9105/version
```

Версионирование: источник истины — сам бинарь (`/version` отдаёт
`CARGO_PKG_VERSION` + git-sha, вшитые при сборке в CI), поэтому любой
артефакт/образ трассируется до коммита без внешних меток.

## Релизы (тег → GitHub Release)

Публичный релиз делается тегом, без ручной сборки:

```
# 1) при необходимости поднять [workspace.package] version в Cargo.toml
# 2) тег обязан совпадать с Cargo-версией (release.yml это проверяет)
git tag v0.1.0 && git push origin v0.1.0
```

`release.yml` собирает бинарь со штампом коммита, смокает `/health` и
`/version` (sha и версия обязаны совпасть с тегом), пакует
`trios-server-linux-x86_64` + systemd-unit + Dockerfile.server в tar.gz,
считает SHA256SUMS и публикует GitHub Release с changelog от предыдущего
тега. Проверка скачанного бинарника: `GET /version` → `git_sha` равен
коммиту тега.
