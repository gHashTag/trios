# Agent I · Wave 21 — Отчёт CI + PR

## Резюме (Russian)

Agent I завершил подготовку CI-пайплайна и PR для волн 16–21 (Sovereign Scarabs).

### Что сделано

| Задача | Статус | Артефакт |
|--------|--------|----------|
| CI workflow (`.github/workflows/wave21_ci.yml`) | ✅ | `agent_I_ci.yml` |
| PR body (RU+EN, gates G1-G7 + W21-G1..G5) | ✅ | `agent_I_pr_body.md` |
| Rust PR-prep binary (`cargo check` PASS) | ✅ | `agent_I_pr_prep.rs` + `agent_I_pr_prep/` |
| GitHub repo check (reachable, HEAD = d92feb6) | ✅ | см. ниже |
| DONE flag | ✅ | `agent_I_DONE.flag` |

### Состояние репозитория gHashTag/trios

- **Репозиторий:** https://github.com/gHashTag/trios
- **Default branch:** `main`
- **HEAD:** `d92feb6a9b9a5633ecc4662c927dc06a849caad0` ✅ (совпадает с контекстом)
- **Ветка `wave21/sovereign-scarabs`:** НЕ СУЩЕСТВУЕТ (будет создана бинарником)
- **Открытые PR:** 1 (несвязанный `feat/trios-chat-wave38` #955)
- **Конфликтов нет:** PR можно открыть

### CI Jobs (wave21_ci.yml)

```
on: pull_request→main + push→main

jobs:
  lint              — cargo fmt --check + cargo clippy -D warnings
  test              — cargo test --workspace --all-features (без #[ignore] DB-тестов)
  live_smoke        — только на push to main, RAILWAY_DSN secret, --ignored тесты
  matrix_sweep_artifact — upload sweep artifacts (30 дней)
  v3_migration_check    — SQL syntax check v3*.sql
```

### Rust Binary (`agent_I_pr_prep.rs`)

Бинарник на стандартной библиотеке Rust (`std::process::Command`):
- Клонирует `gHashTag/trios` по `d92feb6` в `/tmp/trios-w21`
- Создаёт ветку `wave21/sovereign-scarabs`
- Применяет `/home/user/workspace/wave20/wave16_20_combined.patch`
- Копирует Wave 21 артефакты (v3.sql, e2e.rs, sweep, ci.yml) в дерево
- Делает коммит с каноническим сообщением
- Печатает статус + инструкции по push (НЕ делает push автоматически)

```bash
# Запуск:
cargo run --manifest-path /home/user/workspace/wave21/agent_I_pr_prep/Cargo.toml
```

---

## Команда для создания PR (запускать только после явного одобрения пользователя)

### Шаг 1 — Запустить prep-бинарник (клон + патч + коммит):
```bash
cargo run --manifest-path /home/user/workspace/wave21/agent_I_pr_prep/Cargo.toml
```

### Шаг 2 — Push ветки (требует одобрения):
```bash
git -C /tmp/trios-w21 push -u origin wave21/sovereign-scarabs
```

### Шаг 3 — Создать PR (требует одобрения):
```bash
gh pr create \
  --repo gHashTag/trios \
  --base main \
  --head wave21/sovereign-scarabs \
  --title "Wave 16-21: sovereign scarabs, v3 control plane, matrix sweep +112" \
  --body-file /home/user/workspace/wave21/agent_I_pr_body.md
```

> ⚠ **КРИТИЧНО:** НЕ выполнять Шаг 2 и Шаг 3 без явной команды пользователя "push".
> Push и открытие PR — необратимые действия.

---

## Список всех артефактов Wave 21

| Файл | Описание |
|------|----------|
| `/home/user/workspace/wave21/agent_I_ci.yml` | GitHub Actions CI workflow |
| `/home/user/workspace/wave21/agent_I_pr_body.md` | PR description (RU+EN) |
| `/home/user/workspace/wave21/agent_I_pr_prep.rs` | Rust source (standalone) |
| `/home/user/workspace/wave21/agent_I_pr_prep/` | Cargo project (cargo check ✅) |
| `/home/user/workspace/wave21/agent_I_report.md` | Этот файл |
| `/home/user/workspace/wave21/agent_I_DONE.flag` | Completion marker |
| `/home/user/workspace/wave20/wave16_20_combined.patch` | Базовый патч W16-20 (4047 строк) |

---

## Технические детали

### Ограничения, соблюдённые в CI

1. `RAILWAY_DSN` — **только через GitHub Secret** (`${{ secrets.RAILWAY_DSN }}`), не хардкод
2. `live_smoke` — только на `push` to `main` (`if: github.event_name == 'push' && github.ref == 'refs/heads/main'`)
3. DB-зависимые тесты — `#[ignore]` тег, запускаются только в `live_smoke`
4. Toolchain — `actions-rust-lang/setup-rust-toolchain@v1` (stable + cache)

### PR: Breaking-but-Safe-Additive

`bump_strategy_v3` расширяет whitelist UPDATE-полей (`trainer_bin`, `w_jepa`, `w_nca`).
- `bump_strategy` (v1) — не тронут
- `bump_strategy_v2` — не тронут
- Миграция применяется напрямую на live Railway (без стейджинга, согласно решению Wave 20)

---

_Автоматически подготовлено Agent I · Wave 21 · 2026-05-22_
