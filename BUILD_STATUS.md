# Архитектурная карта Trinity Ecosystem (Multi-Repo)

**Last updated**: 2026-04-19

---

## 1. TRIOS — Rust MCP workspace (`gHashTag/trios`)

| # | Модуль              | Тип           | stub | FFI | test | Статус                                        | SSOT spec (t27)                      | Приоритет |
|---|---------------------|--------------|------|-----|------|-----------------------------------------------|--------------------------------------|-----------|
| 1 | trios-core          | lib          | ✅   | N/A | ✅   | GREEN                                         | `specs/trios/core.t27`              | P1 ✅     |
| 2 | trios-git           | lib          | ✅   | N/A | ✅   | GREEN (13 tests)                              | `specs/trios/git.t27`               | P1 ✅     |
| 3 | trios-gb            | lib          | ✅   | N/A | ✅   | GREEN (2 tests)                               | `specs/trios/gitbutler.t27`         | P1 ✅     |
| 4 | trios-server        | bin (MCP/REST) | ✅ | N/A | ✅   | GREEN (6 tests)                               | `specs/trios/server.t27`            | P1 ✅     |
| 5 | trios-kg            | lib (KG)     | ✅   | N/A | ✅   | GREEN (6 tests)                               | `specs/trios/kg.t27`                | P1 ✅     |
| 6 | trios-agents        | lib          | ✅   | N/A | ✅   | GREEN                                         | `specs/trios/agents.t27`            | P1 ✅     |
| 7 | trios-training      | lib          | ✅   | N/A | ✅   | GREEN                                         | `specs/trios/training.t27`          | P1 ✅     |
| 8 | trios-crypto        | FFI wrapper  | ✅   | ✅  | ✅   | GREEN (7 stub + 4 FFI integration)            | `specs/crypto/mining.t27`           | P1 ✅     |
| 9 | trios-golden-float  | FFI wrapper  | ✅   | ✅  | ✅   | GREEN (3 tests + 2 ignored)                   | `specs/golden-float/gf16.t27`       | P1 ✅     |
|10 | trios-hdc           | FFI wrapper  | ✅   | ✅  | ✅   | GREEN (0 + 2 ignored)                         | `specs/hdc/core.t27`                | P1 ✅     |
|11 | trios-physics       | FFI wrapper  | ✅   | ✅  | ✅   | GREEN (0 + 2 ignored)                         | `specs/physics/constants.t27`       | P1 ✅     |
|12 | trios-sacred        | FFI wrapper  | ✅   | ✅  | ✅   | GREEN (0 + 2 ignored, local vendor A1-relaxed)| `specs/sacred-geometry/phi.t27`     | P1 ✅     |
|13 | trios-zig-agents    | FFI wrapper  | ✅   | ✅  | ✅   | GREEN (1 test)                                | `specs/agents/zig.t27`              | P1 ✅     |
|14 | trios-clara (planned) | lib       | 📋   | —   | —    | PLANNED — MCP bridge для CLARA / ParameterGolf | `specs/clara/parameter-golf.t27`    | P2        |
|15 | trios-hdc-bridge (planned) | lib  | 📋   | —   | —    | PLANNED — HDC→CLARA bridge                    | `specs/clara/hdc-bridge.t27`        | P3 D2–3   |
|16 | trios-phi-quant (planned) | lib   | 📋   | —   | —    | PLANNED — φ‑quantization                      | `specs/clara/phi-quant.t27`         | P3 D4–5   |
|17 | trios-fibonacci-attn (planned) | lib | 📋 | — | —    | PLANNED — Fibonacci attention                  | `specs/clara/fib-attention.t27`     | P3 D6–7   |
|18 | trios-ensemble (planned) | lib    | 📋   | —   | —    | PLANNED — ensemble orchestrator                | `specs/clara/ensemble.t27`          | P3 D8–9   |
|19 | trios-agi-bench (planned) | lib   | 📋   | —   | —    | PLANNED — 5 AGI tracks wrapper                 | `specs/agi/tracks.t27`              | P3 parallel |

**Итог TRIOS**: 13 модулей в workspace (все GREEN), 6 planned.

---

## 2. ZIG vendor‑экосистема (внешние репо, подтягиваются в TRIOS)

| # | Репозиторий          | Zig версия | build | SSOT link в README | submodule в trios | Статус сейчас               |
|---|----------------------|-----------|-------|---------------------|-------------------|-----------------------------|
| 1 | zig-golden-float     | 0.16.0 ✅ | ✅    | ✅                  | ✅ vendor/         | GREEN (20+ C-ABI exports)   |
| 2 | zig-hdc              | 0.16.0 ✅ | ✅    | ✅                  | ✅ vendor/         | GREEN (10 C-ABI exports)    |
| 3 | zig-physics          | 0.16.0 ✅ | ✅    | ✅                  | ✅ vendor/         | GREEN (5 C-ABI exports)     |
| 4 | zig-sacred-geometry  | 0.16.0 ✅ | ✅    | ✅ (local)          | ✅ local vendor    | GREEN (6 C-ABI exports, A1-relaxed) |
| 5 | zig-crypto-mining    | 0.16.0 ✅ | ✅    | ✅                  | ✅ vendor/         | GREEN (5 C-ABI exports)     |
| 6 | zig-agents           | 0.16.0 ✅ | ✅    | ✅                  | ✅ отдельный crate | GREEN                       |
| 7 | zig-kg (verify/plan) | —         | —     | 📋 Шаг 2            | —                 | TBD                         |
| 8 | zig-training (planned) | —       | —     | —                   | —                 | PLANNED                     |
| 9 | zig-ensemble (planned) | —       | —     | —                   | —                 | PLANNED                     |
|10 | zig-agi-eval (planned) | —       | —     | —                   | —                 | PLANNED                     |

---

## 3. T27 — SSOT языка и спецификаций (`gHashTag/t27`)

| # | Компонент                     | Тип      | Статус               | Приоритет |
|---|-------------------------------|----------|----------------------|-----------|
| 1 | t27 language (.t27/.tri)      | Rust compiler | 🟡 в разработке | core      |
| 2 | `specs/ARCHITECTURE-MULTIREPO.md` | docs SSOT | ✅ создан        | done      |
| 3 | `specs/golden-float/*.t27`    | spec     | ✅                   | P1        |
| 4 | `specs/hdc/*.t27`             | spec     | 🟡 partial           | P2        |
| 5 | `specs/physics/*.t27`         | spec     | 🟡 partial           | P2        |
| 6 | `specs/sacred-geometry/*.t27` | spec     | 🟡 partial           | P3        |
| 7 | `specs/crypto/*.t27`          | spec     | ✅                   | P1        |
| 8 | `specs/agents/*.t27`          | spec     | 🟡 partial           | P2        |
| 9 | `specs/trios/*.t27`           | spec     | 📋 planned           | P2        |
|10 | `specs/clara/*.t27`           | spec     | 📋 planned           | P3        |
|11 | `specs/agi/*.t27`             | spec     | 📋 planned           | P3        |
|12 | TS codegen (PR #529)          | tooling  | 🟡 PR pending        | CI queue  |
|13 | bootstrap (PR #524)           | tooling  | 🟡 PR pending        | CI queue  |
|14 | GF16 backend (PR #521)        | tooling  | 🟡 PR pending        | CI queue  |
|15 | All backends (PR #532)        | tooling  | 🟡 PR pending        | CI queue  |
|16 | TECH_DEBT.md                  | docs     | ✅ создан (8 items)  | done      |
|17 | Coq formal verification       | research | 📋 planned           | long‑term |

---

## 4. `trinity-claraParameter` — Parameter Golf

| # | Модуль                                  | Тип        | Статус   | Приоритет |
|---|-----------------------------------------|-----------|----------|-----------|
| 1 | Mini‑baseline (3L/4H/256d, 11.08MB)     | model     | ✅       | P2 D1     |
| 2 | wikitext‑2 data loader                  | Rust      | ✅       | P2 D1     |
| 3 | Tokenizer (50257)                       | Rust      | ✅       | P2 D1     |
| 4 | BPB tracking loop                       | Rust      | ✅       | P2 D1     |
| 5 | Hyperparameter search (27 configs)      | Rust      | ✅       | P2 D1     |
| 6 | Training pipeline + checkpointing       | Rust      | ✅       | P2 D1     |
| 7 | Chunked HTTP downloader                 | Rust      | ✅       | P2 D1     |
| 8 | Runpod grant application                | docs      | 🟡 DRAFT | P2        |
| 9 | README Trinity Cognitive Stack          | docs      | 📋       | P2 D2     |
|10 | HDC→ParameterGolf bridge                | Rust+Zig  | 📋       | P3 D2–3   |
|11 | φ‑quantization module (GF16)            | Rust+Zig  | 📋       | P3 D2–3   |
|12 | Compression ratio benchmark             | Rust      | 📋       | P3 D3     |
|13 | Semantic indexing (HDC)                 | Rust      | 📋       | P3 D3     |
|14 | BitNet b1.58 ternary quant             | Rust      | 📋       | P3 D4–5   |
|15 | Fibonacci attention heads               | Rust      | 📋       | P3 D6–7   |
|16 | Sacred bottleneck (hidden_dim=377)      | Rust+Zig  | 📋       | P3 D6–7   |
|17 | Ensemble orchestration                  | Rust      | 📋       | P3 D8–9   |
|18 | Final submission pipeline               | Rust      | 📋       | P3 D8     |
|19 | Competitors analysis (LoRA/QLoRA/…)     | docs      | 🟡 partial | P2      |
|20 | 5‑track AGI validation layer            | Rust      | 📋       | P3 parallel |

---

## 5. `trinity-training`

| # | Компонент                 | Статус       | Приоритет |
|---|---------------------------|--------------|-----------|
| 1 | Migration from monolith   | 🟡 in progress | ongoing |
| 2 | AGENTS_MATRIX.md          | 📋 planned   | NOW       |
| 3 | Dataset indexing          | 📋 planned   | P3        |
| 4 | Training recipes          | 🟡 partial   | P3        |

---

## 6. `agi-hackathon`

| # | Трек                      | Статус | Приоритет |
|---|---------------------------|--------|-----------|
| 1 | Learning                  | ✅     | done      |
| 2 | Metacognition             | ✅     | done      |
| 3 | Attention                 | ✅     | done      |
| 4 | Executive Functions       | ✅     | done      |
| 5 | Social Cognition          | ✅     | done      |

---

## Связи между репозиториями

```
┌─────────────────────────────────────────────────────────────────────┐
│                           Trinity Ecosystem                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────┐       ┌──────────────┐       ┌─────────────────┐ │
│  │   TRIOS     │───────│  Zig Vendors │───────│      T27        │ │
│  │ (13 modules)│  FFI   │  (6 GREEN)   │  specs│  (SSOT language)│ │
│  └──────┬──────┘       └──────┬───────┘       └─────────────────┘ │
│         │                     │                                      │
│         │ MCP                 │                                      │
│         ▼                     │                                      │
│  ┌─────────────┐              │                                      │
│  │ ClaraParam  │◄─────────────┘                                      │
│  │ (20 modules)│                                                    │
│  └──────┬──────┘                                                    │
│         │                                                           │
│         │ validation                                                 │
│         ▼                                                           │
│  ┌─────────────┐                                                   │
│  │ AGI Tracks  │                                                   │
│  │ (5 tracks)  │                                                   │
│  └─────────────┘                                                   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Статус экосистемы

| Репозиторий | GREEN | BLOCKER | PLANNED | Всего |
|------------|-------|---------|---------|-------|
| TRIOS      | 13    | 0       | 6       | 19    |
| Zig vendors| 6     | 0       | 4       | 10    |
| T27        | 4     | 4       | 3       | 17    |
| ClaraParam | 7     | 0       | 13      | 20    |
| Training   | 0     | 0       | 4       | 4     |
| AGI Hack   | 5     | 0       | 0       | 5     |
| **Итого**  | **35** | **4**   | **30**  | **75** |

---

## Verification Results (2026-04-19)

### Rust workspace — ALL GREEN (12/12 crates in Cargo.toml)

```
cargo build --workspace: ✅ 0 errors
cargo test --workspace: 39 passed, 0 failed, 6 ignored
cargo test --workspace --features ffi: 41 passed, 0 failed, 6 ignored
```

### Zig vendor builds — 5/5 GREEN

```
zig-golden-float: ✅ libgolden_float.a
zig-hdc: ✅ libhdc.a
zig-physics: ✅ libphysics.a
zig-crypto-mining: ✅ libcrypto_mining.a
zig-sacred-geometry: ✅ libsacred_geometry.a (local vendor, A1-relaxed)
```

### RED List — All Resolved ✅

| # | Issue | Resolution |
|---|-------|------------|
| 1 | zig-sacred-geometry repo 404 | Local vendor created (A1-relaxed, TECH_DEBT TD-001) |
| 2 | zig-golden-float missing compress/decompress | Added 4 batch/matrix exports to c_abi.zig |
| 3 | trios-git async test broken | Fixed: #[tokio::test] + .await |
