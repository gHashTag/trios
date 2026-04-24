# RUNBOOK — trios Infrastructure

> Source of truth for all machines, services, and operational procedures.
> Related: [issue #143](https://github.com/gHashTag/trios/issues/143) · [tailscale-funnel.md](./tailscale-funnel.md)

---

## Machines

| Name | Role | OS | Tailnet IP | Status |
|------|------|----|------------|--------|
| `playras-macbook-pro-1` | Dev / MCP server | macOS | `100.66.38.103` | ✅ Active |
| RunPod GPU pod | Training (IGLA RACE) | Ubuntu 22.04 | dynamic | ⏳ Grant pending |

---

## Machine 1 — MacBook Pro (Dev)

### Quick Start

```bash
# 1. MCP server
cargo run -p trios-server

# 2. Tailscale Funnel (App Store CLI — ОБЯЗАТЕЛЬНО)
/Applications/Tailscale.app/Contents/MacOS/Tailscale funnel --bg 9005
```

### Endpoints

| Method | URL | Expected |
|--------|-----|----------|
| `GET` | `https://playras-macbook-pro-1.tail01804b.ts.net/api/status` | `{"status":"ok"}` |
| `GET` | `http://100.66.38.103:9005/api/status` | внутри tailnet |
| `GET` | `/health` | `ok` |
| `WS` | `/ws` | MCP WebSocket |

### Health Check

```bash
curl http://100.66.38.103:9005/api/status
# → {"agents":0,"status":"ok","tools":19}
```

### ⚠️ Tailscale: два CLI на машине

```bash
# ПРАВИЛЬНЫЙ (App Store)
/Applications/Tailscale.app/Contents/MacOS/Tailscale

# СЛОМАННЫЙ (brew — не использовать)
/opt/homebrew/bin/tailscale
```

Добавь алиас в `~/.zshrc`:
```bash
alias tailscale="/Applications/Tailscale.app/Contents/MacOS/Tailscale"
```

### Остановка Funnel

```bash
/Applications/Tailscale.app/Contents/MacOS/Tailscale funnel --https=443 off
```

---

## Machine 2 — RunPod GPU (Training)

> **Статус:** Grant не отправлен (см. [RUNPOD_GRANT_STATUS.md](../../RUNPOD_GRANT_STATUS.md))
> **Action required:** Отправить заявку → обновить этот файл

### После получения пода

```bash
# SSH
ssh root@<RUNPOD_IP> -p <PORT> -i ~/.ssh/id_ed25519

# Клонировать репо
git clone https://github.com/gHashTag/trios.git
cd trios

# Установить Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Проверить сборку
cargo build -p trios-train-cpu --release
cargo clippy -p trios-train-cpu -- -D warnings
cargo test -p trios-train-cpu
```

### Запуск тренировки IGLA RACE

```bash
# Wave 9 training (ASHA, 3000+ steps на Rung-1)
cargo run -p trios-train-cpu --release --bin igla_train -- \
  --config configs/wave9.json \
  --output results/wave9/

# Мониторинг BPB
tail -f results/wave9/metrics.jsonl | jq '.bpb'
```

### Целевые метрики (из issue #143 TASK-5)

| Метрика | Цель |
|---------|------|
| BPB vs N-gram | −0.3 … −0.5 |
| MSE loss | < 0.35 (J-001: 0.30 достигнут) |
| EMA decay | 0.996 → 1.0 линейно |
| ASHA Rung-1 min steps | 3000 |

---

## CI/CD

### GitHub Actions — ветки

| Ветка | Триггер | Статус |
|-------|---------|--------|
| `main` | push / PR | `cargo test` + `cargo clippy -D warnings` |

### Локальная проверка перед пушем

```bash
# Обязательно перед любым коммитом (LAWS.md L-R4 + L-R5)
cargo clippy -- -D warnings
cargo test
```

### Если CI красный

```bash
# Посмотреть последний провалившийся тест
cargo test 2>&1 | grep FAILED

# Проверить конкретный крейт
cargo clippy -p trios-train-cpu -- -D warnings
cargo test -p trios-train-cpu -- --nocapture
```

---

## Ключевые файлы

| Файл | Роль |
|------|------|
| `LAWS.md` | Конституция проекта — читать первым |
| `CLAUDE.md` | Доктрина репо + PHI LOOP |
| `AGENTS.md` | Роли агентов, claiming, handoff |
| `NOW.json` | Append-only heartbeat лог (не редактировать!) |
| `docs/infrastructure/tailscale-funnel.md` | Tailscale детали |
| `RUNPOD_GRANT_STATUS.md` | Статус GPU гранта |
| `crates/trios-train-cpu/src/gf16.rs` | GF16 арифметика (clippy clean) |
| `crates/trios-train-cpu/src/tjepa.rs` | T-JEPA scaffold (TASK-5) |

---

## Trinity Research References

| Модель | Docs |
|--------|------|
| JEPA-T | https://github.com/gHashTag/trinity/tree/main/docs/research/models/JEPA-T/ |
| NCA | https://github.com/gHashTag/trinity/tree/main/docs/research/models/NCA/ |
| Hybrid | https://github.com/gHashTag/trinity/tree/main/docs/research/models/Hybrid/ |
| VSA | https://github.com/gHashTag/trinity/tree/main/docs/research/models/VSA/ |
| Ternary | https://github.com/gHashTag/trinity/tree/main/docs/research/models/Ternary/ |

---

## Troubleshooting

### `cargo clippy` упал с warnings

```bash
cargo clippy -p <crate> -- -D warnings 2>&1 | grep "warning\["
# Исправить → коммитить → пушить
```

### Tailscale funnel не работает

```bash
# Проверить какой daemon активен
ps aux | grep -i tailscale
# Использовать App Store CLI (IPNExtension, PID ~5646)
/Applications/Tailscale.app/Contents/MacOS/Tailscale status
```

### Порт 9005 занят

```bash
lsof -i :9005
kill -9 <PID>
cargo run -p trios-server
```
