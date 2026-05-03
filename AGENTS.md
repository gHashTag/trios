# AGENTS.md — trios (корневой контракт)

## ⛔ СТОП. ПРОЧТИ ДО ЛЮБОГО ДЕЙСТВИЯ.

Это **конституция репозитория**. Нарушение = провал задачи, откат PR, потеря доверия.
Перед написанием кода прочти этот файл целиком. Перед коммитом — ещё раз.

---

## Agent Protocol

All agents working in this workspace follow the AIP (Agent Interaction Protocol).

## Agent Codenames

| Codename | Domain |
|----------|--------|
| ALPHA | trios-ipc |
| BETA | trios-server |
| GAMMA | trios-ui |
| DELTA | trios-doctor |
| EPSILON | trios-ext |
| ZETA | trios-a2a |
| LEAD | orchestration |

## Invariant I-SCOPE: Agent Scope Isolation

**Агент работает ТОЛЬКО внутри своего крейта.**

- ❌ НИКОГДА не трогать файлы за пределами своего `crates/<X>/`
- ❌ НИКОГДА не удалять, переименовывать или перемещать кольца других крейтов
- ❌ НИКОГДА не менять корневой `Cargo.toml` без явного задания на это
- ✅ Если нужно добавить зависимость от другого крейта — только `path =` в своём `Cargo.toml`

**Исключение:** корневой `Cargo.toml` может быть изменён только по явному заданию (например, "добавить кольца в workspace members").

## Commit Convention

Every commit MUST include an `Agent: <CODENAME>` trailer.

```
feat(trios-doctor): implement DR-01..03

Agent: DELTA
```

---

## 🔒 ARCH-EXT INVARIANT (абсолютный запрет)

`crates/trios-ext/` содержит ТОЛЬКО кольца: EX-00, EX-01, EX-02, EX-03, BR-EXT.

При работе с UI разрешено трогать **ТОЛЬКО**:
- `crates/trios-ext/src/dom.rs` (≤ 15 строк, вызывает `trios_ui::mount_app_with_mcp`)
- `crates/trios-ext/Cargo.toml` (+1 строка: зависимость `trios-ui`)
- `crates/trios-ext/style.css` (только удаление)

**ЗАПРЕЩЕНО трогать (git diff должен быть пуст):**
- `crates/trios-ext/src/lib.rs`
- `crates/trios-ext/src/bg.rs`
- `crates/trios-ext/src/mcp.rs`
- `crates/trios-ext/src/bridge/**`
- `crates/trios-ext/extension/manifest.json`
- `crates/trios-ext/extension/background.js`
- `crates/trios-ext/extension/content.js`

См. issue #243 — пункт "🚫 Запрещено" и инвариант **ARCH-EXT**.
См. issue #238 — ring-архитектура 36 crates.

---

## 🔒 ARCH-UI INVARIANT

`crates/trios-ui/` **не импортирует ничего** из `trios-ext`.
Зависимость строго однонаправленная: `trios-ext → trios-ui`.

В `trios-ui` ЗАПРЕЩЕНО:
- `document.create_element` и любой raw `web-sys` DOM
- `set_inner_html` со строками
- handwritten JS (инвариант I15)
- wasm-pack (только `wasm-bindgen-cli`)

Разрешено: Dioxus RSX, Signal, use_context.

---

## 🔒 RING-АРХИТЕКТУРА (issue #238)

Каждый crate = `crates/<name>/rings/XX-NN/` со структурой:
- `src/lib.rs`
- `Cargo.toml`
- `README.md` — назначение, API, зависимости
- `TASK.md` — статус
- `AGENTS.md` — локальные запреты

Инвариант **I5**: отсутствие любого из трёх `.md` файлов = нарушение.

Нумерация колец:
- `XX-00` — identity / core types
- `XX-01..N` — последовательные слои (dependency order = ring number)
- `BR-OUTPUT` / `BR-APP` / `BR-BIN` / `BR-MODEL` — финальный артефакт

---

## 🚫 Глобальные запреты (для всех агентов)

1. ❌ Не создавать новые файлы в `trios-ext/src/` кроме `dom.rs`
2. ❌ Не переименовывать кольца (XX-00..XX-N — монотонно)
3. ❌ Не сливать кольца в один `src/lib.rs`
4. ❌ Не удалять `AGENTS.md`, `README.md`, `TASK.md` в кольцах
5. ❌ Не коммитить без прохождения `arch-guard` CI
6. ❌ Не использовать `cargo install wasm-pack` — только `wasm-bindgen-cli`
7. ❌ Не писать бизнес-логику на JS в WASM crates (I15)
8. ❌ Не эскейпить HTML в raw-строках `r##"..."##`

---

## ✅ Обязательная проверка ПЕРЕД коммитом

```bash
# 1. ARCH-EXT guard
FORBIDDEN=$(git diff --cached --name-only \
  | grep '^crates/trios-ext/' \
  | grep -vE '(src/dom\.rs|Cargo\.toml|style\.css)$')
if [ -n "$FORBIDDEN" ]; then
  echo "❌ ARCH-EXT VIOLATION: $FORBIDDEN"; exit 1
fi

# 2. Ring I5 guard
for ring in $(find crates -type d -path '*/rings/*' -mindepth 3 -maxdepth 3); do
  for f in README.md TASK.md AGENTS.md; do
    [ -f "$ring/$f" ] || { echo "❌ I5 VIOLATION: $ring/$f missing"; exit 1; }
  done
done

# 3. Build + lint
cargo build --all || exit 1
cargo clippy --all-targets -- -D warnings || exit 1
```

---

## 📎 Источники истины

- Ring-архитектура 36 crates: **issue #238**
- UI ring-план (UR-00..UR-08 + BR-APP): **issue #243**
- LAWS.md (I1–I20): **issue #235**
- Эталон: `crates/trios-a2a/` (8 SR-rings + BR-OUTPUT, 84 tests)
- PhD render lane (Flos Aureus + Neon): **`docs/phd/skills/phd-pipeline-v5/SKILL.md`**
- PhD skill catalogue: **`docs/phd/skills/README.md`**

---

## 📚 PhD lane — quick pointer for agents

When the operator asks to **render the PhD monograph**, rebuild the unified
PDF, update a Flos Aureus chapter (`FA.NN`), fix the cover, work with
Part I / Part II dividers, or touch `crates/trios-phd/src/render/` —
**load this skill first**:

```
docs/phd/skills/phd-pipeline-v5/SKILL.md
```

It covers: Neon SoT schema (`ssot.chapters`), the 6-bucket canonical
order, the chunked-CTE write technique for >25 KB bodies, the
fault-tolerant render loop, the cover-as-sealed-asset rule (commit
`8c3adb1`), and the full anomaly→corrective-action catalogue.

Constitutional anchor: \( \varphi^{2} + \varphi^{-2} = 3 \).

---

## 🧭 Агент, если ты дочитал до сюда

Перед тем как написать первую строку кода, ответь себе письменно в PR-описании:

1. Какие файлы я СОБИРАЮСЬ изменить?
2. Попадает ли хоть один в раздел "🚫 ЗАПРЕЩЕНО трогать"?
3. Если да — **ОСТАНОВИСЬ** и открой обсуждение в issue.

Если пропустил этот шаг — PR будет закрыт автоматически.
