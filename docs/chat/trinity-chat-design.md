# Trinity Secure Chat — Design Document

**Document ID:** TRINITY-CHAT-001 · Rev 1.0 · 2026-05-09
**Anchor:** `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
**Parent EPIC:** trinity-fpga#19 (dePIN-Compute) → trinity-fpga#22 (Mesh Quality, ✅ closed)
**Honesty mode:** R5 — каждое утверждение помечено `[VERIFIED]`, `[CITED]`, `[DERIVED]`, `[ASPIRATIONAL]`

> **Цель:** построить **самый безопасный и приватный** чат для пары *юзеры ↔ агент-боты* поверх trios-mesh-node (X25519 + ChaCha20-Poly1305 + ETX routing, уже LANDED в `gHashTag/trios:main` после PR #629). Не «ещё один Signal», а единственный мессенджер, спроектированный под **смешанный трафик: люди и автономные агенты**, где prompt-injection — атака первого порядка, а не сноска.

---

## 0 · Резюме (TL;DR)

| Свойство | Trinity Chat | Signal | MLS-native (Element X) | SimpleX | Reticulum LXMF |
|---|---|---|---|---|---|
| FS (forward secrecy) | ✅ Double Ratchet + MLS | ✅ DR | ✅ TreeKEM | ✅ DR | ⚠ msg-level only |
| PCS (post-compromise) | ✅ MLS Update | ✅ DR | ✅ | ✅ | ❌ |
| PQ-secure handshake | ✅ Hybrid X25519+ML-KEM-768 | ⚠ PQXDH/Triple Ratchet | 🔬 draft | ❌ | ❌ |
| Метаданные (получатель) | ✅ sealed-sender + queue-id | ⚠ sealed-sender | ❌ rooms | ✅✅ no-id | ✅ dest-hash |
| Метаданные (отправитель) | ✅ ring-sig + cover traffic opt-in | ⚠ phone | ❌ | ✅ | ⚠ pubkey |
| **Bot/agent capability** | ✅ scope-attested keys + signed tools | ❌ | ❌ | ❌ | ❌ |
| **Prompt-injection guard** | ✅ dual-LLM + structured-output | ❌ | ❌ | ❌ | ❌ |
| Deniability | ✅ online + offline (RingXKEM) | ✅ offline | ⚠ partial | ✅ | ❌ |
| Mesh transport | ✅ trios-mesh ETX | ❌ | ❌ federated | ❌ | ✅ Reticulum |
| Coq-verified invariants | ✅ 7 theorems | ❌ | partial (Cryspen) | ❌ | ❌ |
| Open source | ✅ Apache 2.0 | ✅ | ✅ | ✅ | ✅ MIT |

**Уникальные дифференциаторы Trinity Chat (то, чего нет ни у одного конкурента):**

1. **Native agent threat model.** Капабилити-токены, scope-attested public keys, signed tool manifests, dual-LLM filter — встроены в протокол, не прикручены сбоку.
2. **Mesh-native transport.** Sealed-sender по ETX-routed dest_hash вместо центрального сервера. Унаследовано от уже LANDED `trios-mesh-node`.
3. **Post-quantum hybrid с первого дня.** ML-KEM-768 рядом с X25519 в каждом handshake; миграция на полностью PQ deniable ring signatures (RingXKEM-style) запланирована как ADR-009.
4. **Coq runtime invariants.** Те же 7 инвариантов, что и в `trinity-clara`, но для chat: `chat_no_plaintext_at_rest`, `agent_capability_bound`, `ratchet_no_replay`, `metadata_no_link`, `mls_epoch_monotone`, `pq_kem_present`, `signed_tool_only`.
5. **R5 honesty + R7 falsifier.** Каждый G-Cn gate имеет публичный falsifier-witness (см. §10).

---

## 1 · Глубокое исследование

### 1.1 Signal — каноническая база

Signal-protocol эволюционировал в три этапа [CITED [Signal PQXDH-to-RingXKEM slides 2025](https://gniot.fr/assets/slides/2025/2025-12-signal.pdf)]:

| Эпоха | Год | Handshake | PQ-FS | PQ-Auth | Deniability |
|---|---|---|---|---|---|
| X3DH | 2016 | 4×DH над Curve25519 | ❌ | ❌ | ✅ offline |
| **PQXDH** | 2023 | X3DH + Kyber768 KEM | ✅ initial-FS | ❌ | ✅ |
| **Triple Ratchet** | 2025 | + ML-KEM ratchet step | ✅ continuous | ❌ | ⚠ unresolved |
| RingXKEM (research) | 2025-12 | KEMs + deniable ring signatures | ✅ | ✅ | ✅ online+offline |

Ключевые свойства Double Ratchet, которые нужно сохранить:
- **KDF-цепочка:** `root_key, chain_key = HKDF(root_key, DH(...))`; компрометация одного `chain_key` не раскрывает прошлые/будущие.
- **Sealed sender** [CITED [Signal docs](https://signal.org/docs/)]: identity-key получателя расшифровывает «конверт» с identity-key отправителя; промежуточные узлы видят только dest_hash.
- **Safety numbers:** SHA-256(pubA ‖ pubB), 60 цифр, отображаются обоим — анти-MITM при out-of-band сравнении.

### 1.2 MLS (RFC 9420) — каноническая база для группового чата

[CITED [RFC 9420](https://datatracker.ietf.org/doc/rfc9420/)]:
- **TreeKEM**: log(N) шифрований при удалении/обновлении члена группы. Tree size = 2..thousands.
- **GroupContext**: `{version, cipher_suite, group_id, epoch:uint64, tree_hash, confirmed_transcript_hash, extensions}`. Каждый Commit инкрементит epoch.
- **PCS** через Update/Commit, который «обнуляет» direct path скомпрометированного члена.
- **Authentication binding** обязателен: external sender'ы должны быть подписаны и привязаны к GroupContext, иначе атака на импорт.

[CITED [draft-ietf-mls-partial-02](https://datatracker.ietf.org/doc/draft-ietf-mls-partial/), 2025-09]:
- **Partial MLS** позволяет клиентам не скачивать всё дерево (log-scale). Подходит для *агент-ботов*, которые могут быть в тысячах групп.
- Partial клиенты **не могут отправлять Commit** — естественный capability bound.

### 1.3 SimpleX — анти-метаданная архитектура

[CITED [simplex.chat docs](https://simplex.chat/docs/simplex.html)]:
- **Нет user-ID вообще.** Ни телефона, ни юзернейма, ни долгоживущего pubkey-as-identity. Идентификатор — *unidirectional queue address* per-contact.
- Сервер не знает, сколько у него юзеров — он видит только очереди.
- Sender/recipient unlinkability: на проводе нет общих идентификаторов между отправленным и принятым.
- **Trade-off:** обязателен out-of-band обмен queue-address (QR / link). Trinity Chat решит это поверх mesh — см. §6.

### 1.4 Reticulum LXMF — наш родственник

[CITED [github.com/markqvist/LXMF](https://github.com/markqvist/LXMF)]:
- Wire: `16B dest_hash ‖ 16B src_hash ‖ 64B Ed25519 sig ‖ msgpack(timestamp, content, title, fields)` = 111 B overhead.
- Propagation Nodes хранят зашифрованные сообщения для оффлайн-юзеров (≈ store-and-forward). Распределённое доска объявлений.
- **Слабость:** Ed25519-подпись на каждом сообщении даёт *non-repudiation* (анти-deniability). Trinity Chat заменит это на MAC-from-shared-secret + опциональную deniable ring sig.

### 1.5 Briar / Cwtch / Session — экстремальная анонимность, плохая агентность

| Проект | Идея | Минусы для chat-with-agents |
|---|---|---|
| Briar | Tor + Bluetooth + WiFi mesh, P2P без серверов | Очень высокая latency; нет push-уведомлений → агент не получит «звонок» |
| Cwtch | Tor onion services + group chats | Зависит от Tor; нет PQ; нет mesh |
| Session | Onion routing (Lokinet) + Signal-protocol fork | Централизованный servers-as-onions; нет MLS group; уже было раскрытие [CITED [discuss.privacyguides.net](https://discuss.privacyguides.net/t/any-e2ee-messenger-that-is-similar-to-session-messenger/34110)] |

Trinity Chat заимствует из них *идею onion-routed metadata*, но реализует её поверх trios-mesh ETX, а не Tor.

### 1.6 PingPong — metadata-private без координации

[CITED [arxiv 2504.19566](https://arxiv.org/html/2504.19566v1), 2025]:
- "Notify-before-retrieval" вместо "dial-before-converse".
- Oblivious hash tables в Intel SGX enclave. Глобальный пассивный/активный adversary.
- Trinity Chat **не зависит от SGX** (vendor-locked, BootHole-class supply-chain risk), но возьмёт идею **fixed-size sealed pings** + **carrier traffic для uniformity** как опциональный режим в L-CHAT-7.

### 1.7 Agent-specific угрозы (новизна 2026)

#### MCP (Model Context Protocol)
[CITED [stackoverflow.blog 2026-01-21](https://stackoverflow.blog/2026/01/21/is-that-allowed-authentication-and-authorization-in-model-context-protocol/), [workos.com 2026 MCP guide](https://workos.com/blog/everything-your-team-needs-to-know-about-mcp-in-2026)]:
- Спецификация **2025-11-25** — текущая. SSE deprecated, **Streamable HTTP** + OAuth 2.1.
- **Resource Indicators (RFC 8707)** обязательны с июня 2025: токен для server-A не валиден на server-B (anti-confused-deputy).
- **Session-scoped authorization** (нояб 2025): доступ агента живёт ровно столько, сколько задача; renew требует человека.
- **Gap:** static client secrets всё ещё распространены; нет SSO-интеграции по умолчанию.

#### Google A2A
[CITED [developers.googleblog.com](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/), [paz.ai A2A 2026 glossary](https://www.paz.ai/glossary/agent-to-agent-protocol-a2a)]:
- Запущен 2025-04-09. HTTP + SSE + JSON-RPC. 50+ enterprise партнёров.
- "Secure by default" — паритет с OpenAPI auth.
- **Gap:** нет E2E-шифрования между агентами; trust-on-first-use; нет capability-token-binding к conversation.

#### OWASP LLM Top-10 2026
[CITED [OWASP LLM Prompt Injection Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/LLM_Prompt_Injection_Prevention_Cheat_Sheet.html), [repello.ai/blog/owasp-llm-top-10-2026](https://repello.ai/blog/owasp-llm-top-10-2026), [atlan.com 2026 prompt injection on agents](https://atlan.com/know/prompt-injection-attacks-ai-agents/)]:
- **Direct prompt injection:** role-play hijack, instruction override, system-prompt extraction.
- **Indirect prompt injection:** ядовитые URL, документы, email, code comments — особенно опасно для RAG-агентов.
- Mitigations: input regex/fuzzy-match (Levenshtein/Jaro-Winkler), output validators (regex для system-prompt-leakage, API-key exposure), HITL для high-risk keywords (`password`, `api_key`, `admin`, `system`), structured output (JSON schema), tool-whitelisting, dual-LLM (одна модель санитизирует, другая исполняет), capability-token binding (агент может вызвать только tool X в session Y).

---

## 2 · Threat model (TM-1..TM-10)

| ID | Adversary | Capability | Mitigation |
|---|---|---|---|
| TM-1 | Passive network observer | sniff любого hop | E2E AEAD (G-C1) |
| TM-2 | Active MITM | inject/modify packets, replay | Double Ratchet (G-C2) + AEAD nonce monotone (G-C7) |
| TM-3 | Malicious mesh node | подменить next-hop, drop, harvest dest_hash | sealed-sender (G-C3) + ETX-quality (унаследовано от mesh-node) |
| TM-4 | Compromised client (long-term key leak, future) | расшифровать прошлое/будущее | FS+PCS через Triple-Ratchet + MLS Update (G-C2 + G-C5) |
| TM-5 | Malicious bot operator | агент по ту сторону зомбирует пользователя | scope-attested key + signed tool manifest (G-C6) |
| TM-6 | Prompt-injector (direct/indirect) | заставить агента вызвать tool вне scope, утечь system-prompt | dual-LLM filter + structured output + HITL high-risk (G-C8) |
| TM-7 | Metadata harvester (государство, ISP) | связать sender↔receiver↔time | sealed-sender + per-contact queue (G-C3) + opt-in cover traffic (G-C9) |
| TM-8 | Future quantum attacker (HNDL — harvest-now-decrypt-later) | хранит ciphertext, ждёт CRQC | hybrid X25519+ML-KEM-768 c day-1 (G-C4) |
| TM-9 | Court / legal compulsion | требовать non-repudiation подписи | deniable authentication: MAC-from-shared-secret, no per-message signature (G-C10) |
| TM-10 | Supply-chain / TEE-vendor | rollback или side-channel | НЕ зависим от TEE/SGX. Оптимизация — software-only crypto |

---

## 3 · Архитектура

### 3.1 Высокоуровневая схема

```
┌─────────────────┐       ┌─────────────────┐
│ User Alice (📱) │       │ Bot/Agent Bob   │
│  X25519+Ed25519 │       │ scope-attested  │
│  + ML-KEM-768   │       │ + signed tools  │
└────────┬────────┘       └────────┬────────┘
         │                         │
         │   Triple Ratchet (1:1)  │
         │  ←──────────────────→   │
         │   MLS TreeKEM (group)   │
         │  ←──────────────────→   │
         │                         │
         ▼                         ▼
   ┌─────────────────────────────────────────┐
   │  trios-mesh-node ETX routing layer     │
   │  ChaCha20-Poly1305 hop-by-hop overlay  │
   │  sealed-sender envelope                │
   │  Neon persistence (encrypted at rest)   │
   └─────────────────────────────────────────┘
                │
        ┌───────┴───────┐
        │ Reticulum     │  (optional bridge for off-grid)
        │ LXMF gateway  │
        └───────────────┘
```

### 3.2 Слои (от низа к верху)

| Слой | Технология | Источник |
|---|---|---|
| **Transport** | trios-mesh-node ETX, sealed_envelope | LANDED после PR #629 [VERIFIED] |
| **Hop-by-hop** | ChaCha20-Poly1305 (уже есть) | LANDED [VERIFIED] |
| **Identity** | Ed25519 long-term + X25519 ephemeral + ML-KEM-768 | новый, L-CHAT-1 |
| **1:1 session** | Triple Ratchet (X3DH+ML-KEM init, DH+ML-KEM ratchet) | новый, L-CHAT-2 |
| **Group session** | MLS RFC 9420 + Partial MLS extension для bot'ов | новый, L-CHAT-3 |
| **Sealed sender** | identity-key получателя «конверт» вокруг ratchet payload | новый, L-CHAT-4 |
| **Persistence** | Neon Postgres, encrypted-at-rest, ratchet state на клиенте | новый, L-CHAT-5 |
| **Agent capability** | scope tokens (RFC 8707-style), signed tool manifests | новый, L-CHAT-6 |
| **Anti-injection** | dual-LLM filter, output validator, HITL | новый, L-CHAT-6 |
| **Anti-metadata** | fixed-size padding, opt-in cover traffic, queue rotation | новый, L-CHAT-7 |
| **PQ migration** | ML-KEM-768 hybrid с day-1; план миграции на RingXKEM | новый, L-CHAT-8 |

### 3.3 R-CHAT правила (R-CHAT-1..R-CHAT-12)

1. **R-CHAT-1 — NO PLAINTEXT AT REST.** Ни Neon, ни Reticulum propagation node, ни локальный диск не хранят расшифрованный контент.
2. **R-CHAT-2 — HYBRID PQ FROM DAY ONE.** Каждый handshake имеет KDF input от X25519 ⊕ ML-KEM-768. Опциональность ML-KEM запрещена.
3. **R-CHAT-3 — SEALED SENDER MANDATORY.** Mesh-routing видит только `dest_hash` (16 B). `src_pub` зашифрован identity-key получателя.
4. **R-CHAT-4 — DENIABLE AUTHENTICATION.** Никаких per-message Ed25519. MAC берётся из shared symmetric secret. Подпись только на prekey-bundle при онбординге.
5. **R-CHAT-5 — AGENT KEY ≠ USER KEY.** Bot-keys имеют scope-extension `bot_capability=[...]` и обязаны быть подписаны operator-CA, который пользователь явно auth'нул через HITL.
6. **R-CHAT-6 — TOOLS ARE SIGNED PROMPTS.** Любой tool-call от агента сопровождается JSON Schema-validated structured-output, подписанным под капабилити-токен сессии.
7. **R-CHAT-7 — DUAL-LLM ISOLATION.** Если агент обрабатывает входящий контент (RAG, web, document) — он сначала проходит через **filter-LLM** в read-only режиме без tool-access; только санитизированный summary попадает в **executor-LLM**.
8. **R-CHAT-8 — SESSION-SCOPED CAPABILITY.** Капабилити-токен живёт ровно один epoch (для группы) или одну Triple-Ratchet chain (для 1:1). Renew — новый HITL approval (наследство MCP nov-2025).
9. **R-CHAT-9 — FIXED-SIZE PADDING.** Все сообщения паддятся до фиксированных классов: 256, 1024, 4096, 16384 B. Размер файла не утекает.
10. **R-CHAT-10 — ZERO BACKGROUND CHATTER.** Унаследовано от trios-mesh (Art. IV из EPIC #22). Cover traffic — opt-in per-conversation.
11. **R-CHAT-11 — COQ-VERIFIED INVARIANTS.** 7 теорем (см. §9) обязаны компилироваться зелёно перед merge любого PR в `trios-chat`.
12. **R-CHAT-12 — R5 HONESTY + R7 FALSIFIER.** Любой G-Cn gate в § 8 имеет attached falsifier_witness; gate считается зелёным только если falsifier-corpus прогоняется и выдаёт *FAIL* на негативных кейсах.

---

## 4 · Декомпоз — 10 lanes

### L-CHAT-1 — Identity & Onboarding · *5 days*
- Ed25519 long-term identity + X25519 prekey + ML-KEM-768 PQ prekey.
- Prekey bundle публикуется в Neon (или mesh DHT) с подписью; **только prekey-bundle подписан**, message — нет (R-CHAT-4).
- **Safety numbers** SHA-256(pub_A ‖ pub_B), отображаются как 60-digit + emoji-grid для UX.
- **Verification UX**: QR + NFC + TAILSCALE-MagicDNS-link.
- Acceptance G-C1: prekey bundle валидируется тестом против **5 mutation falsifiers** (flipped sig, swapped order, expired, replay, foreign CA).

### L-CHAT-2 — Triple Ratchet 1:1 · *7 days*
- Initial: PQXDH-style — `ss = HKDF(ss_X3DH ‖ ss_KEM)`.
- Ratchet step: DH + ML-KEM (Triple Ratchet 2025).
- Replay protection: monotone nonce + per-chain message_number; reject if seen.
- **Acceptance G-C2**: forward-secrecy test = compromise current key, decrypt past ciphertext → **MUST FAIL**. PCS test = recover after compromise → **MUST PASS** after one full ratchet cycle.

### L-CHAT-3 — MLS Group + Partial-MLS for bots · *10 days*
- Группы используют RFC 9420 cipher-suite **MLS_256_DHKEMP384_AES256GCM_SHA384_P384** ⊕ patched с ML-KEM-768.
- Боты подключаются как **partial clients** — не качают всё дерево, не могут Commit (естественный capability bound).
- Bot welcome содержит `bot_capability` extension, видимый всем юзерам в комнате.
- **Acceptance G-C5**: после Update удалённый член не расшифровывает следующий epoch (PCS).

### L-CHAT-4 — Sealed Sender over Mesh · *4 days*
- Расширяем mesh-node `crypto.rs` функцией `seal_envelope(recipient_pub, src_pub, ratchet_payload) -> bytes`.
- Mesh видит только `(dest_hash[16], encrypted_envelope, padded_size_class)`.
- **Acceptance G-C3**: статистический тест — 10 000 сообщений между 5 парами; mesh-логи не позволяют отделить пары лучше random-guess.

### L-CHAT-5 — Persistence (Neon + client) · *5 days*
- **Серверная сторона:** только зашифрованный envelope + dest_hash + size_class + TTL. Recovery <5 s (унаследовано от L-E2E-4 mesh).
- **Клиентская сторона:** ratchet-state в SQLCipher (mobile) / encrypted SQLite (desktop), key derived from passphrase + Argon2id.
- **Acceptance G-C7**: дамп Neon БД не содержит plaintext; full-text grep на 10K сообщений → 0 утечек.

### L-CHAT-6 — Agent capability + Anti-injection · *14 days*
- **Capability tokens** (RFC 8707 Resource Indicators inspired): `{aud: chat://room/<id>, scope: [send, read, tool:*], exp: <epoch+ratchet>, nonce}`. Подписан operator-CA.
- **Signed tool manifest:** агент публикует `tools.json` (имя, JSON Schema input/output, requires_hitl: bool). Подпись Ed25519 операторского ключа.
- **Dual-LLM:** входящий untrusted контент (web, document) → filter-LLM (no tools, read-only) → структурированный summary → executor-LLM (с tools).
- **Output validator** [CITED OWASP cheat sheet]: regex для system-prompt-leakage, API-key exposure, длина ≤ 5000 → fallback "I cannot provide that information for security reasons."
- **HITL gate** для high-risk keywords: `password`, `api_key`, `admin`, `system`, `sudo`, `delete`, `transfer` — обязательное подтверждение пользователя.
- **Acceptance G-C8**: prompt-injection corpus (см. §10.2) — 200 атак, ≥ 95 % blocked, 0 % false-execute on tools.

### L-CHAT-7 — Anti-metadata · *7 days*
- **Padding:** все сообщения → класс 256/1024/4096/16384 B (PKCS-7-style); итоговый ciphertext — точно класс.
- **Cover traffic (opt-in):** Poisson process λ=0.1 msg/s per active conversation; carrier-messages indistinguishable from real (по PingPong идее, без SGX).
- **Queue rotation** (от SimpleX): per-contact dest_hash меняется каждые N epoch'ов; старый ещё принимает T_grace.
- **No read receipts by default.**
- **Acceptance G-C9**: t-test на латентности sender↔receiver coupling: p > 0.05 (нельзя отличить от шума).

### L-CHAT-8 — Post-Quantum Migration Path · *parallel, ongoing*
- **Day 1:** hybrid X25519 + ML-KEM-768.
- **Day 90 (ADR-009):** добавить deniable ring signatures (RingXKEM-style) — закрывает PQ-Auth gap, который Triple Ratchet 2025 не решил.
- **Day 180:** drop classical fallback, keep only PQ when ≥ 95 % installed base поддерживает.
- **Acceptance G-C4**: handshake passes Cryspen ProVerif model для PQ-FS + PQ-Auth.

### L-CHAT-9 — Coq invariants · *6 days*
См. §9. Семь теорем в `trinity-chat-clara/proofs/chat/`. CI-блокер на любой PR.

### L-CHAT-10 — Falsifier corpus + Test suite · *7 days*
- 25 unit-тестов (parallel к e2e_25 от mesh-node).
- 200 prompt-injection corpus — open dataset.
- 10 MITM attack scenarios.
- 5 PQ-HNDL simulations.
- **Acceptance G-C6 (R7 honesty):** 100 % corpus must produce expected verdict.

---

## 5 · Acceptance gates G-C1..G-C10

| Gate | Lane | Criterion | Falsifier witness |
|---|---|---|---|
| **G-C1** | L-CHAT-1 | Prekey bundle validates → mutation tests fail | `tests/identity_mutation.rs` |
| **G-C2** | L-CHAT-2 | FS: past undecryptable after compromise · PCS: recovery in 1 cycle | `tests/ratchet_fs_pcs.rs` |
| **G-C3** | L-CHAT-4 | Mesh-side observer cannot link sender↔receiver (statistical) | `tests/sealed_sender_link.rs` |
| **G-C4** | L-CHAT-8 | ProVerif model PQ-FS + PQ-Auth green | `proofs/proverif/chat.pv` |
| **G-C5** | L-CHAT-3 | Removed MLS member cannot decrypt next epoch | `tests/mls_pcs.rs` |
| **G-C6** | L-CHAT-10 | Falsifier corpus 100 % expected verdicts | `tests/falsifier_runner.rs` |
| **G-C7** | L-CHAT-5 | DB dump grep on 10K msg → 0 plaintext leaks | `tests/persist_no_leak.rs` |
| **G-C8** | L-CHAT-6 | 200-attack prompt-injection corpus ≥ 95 % blocked, 0 % false-tool-exec | `tests/prompt_injection.rs` |
| **G-C9** | L-CHAT-7 | t-test sender-receiver coupling p > 0.05 | `tests/metadata_ttest.rs` |
| **G-C10** | L-CHAT-2 | No per-message digital signature in wire dump | `tests/deniability.rs` |
| **G-EPIC** | — | EPIC closes when ≥ 8/10 lanes DONE и G-C8 ≥ 95 % | gates aggregator |

---

## 6 · Onboarding UX (без user-ID, по уроку SimpleX)

```
1. Пользователь Alice открывает Trinity Chat → генерируется
   {ed25519_lt_pub, x25519_pre_pub, mlkem768_pre_pub}.
2. Чтобы пригласить Bob:
   - QR-код кодирует prekey-bundle + одноразовый queue_address.
   - Или mesh-DHT publish + share short-link `trinity://invite/<base32>`.
3. Bob сканирует → его клиент шлёт первое сообщение по queue,
   handshake ratchets, queue_address пересоздаётся (queue rotation).
4. Safety numbers (60 цифр + emoji 8×8 grid) — out-of-band проверка.
5. Bot:
   - Operator публикует bot_pub + signed tool manifest на mesh-DHT.
   - Alice видит "Add bot @weather" → HITL диалог:
     "Этот бот запрашивает scope=[send, read, tool:fetch_url].
      Operator: weather.example, signed by CA fp:abc123. Принять?"
   - Только при явном Accept формируется capability_token.
```

---

## 7 · Сравнительная матрица (полная)

| Параметр | Trinity Chat | Signal/PQXDH | MLS-native | SimpleX | Briar | Cwtch | Session | Reticulum LXMF | Matrix |
|---|---|---|---|---|---|---|---|---|---|
| 1:1 FS | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠ | ✅ Olm |
| Group FS+PCS | ✅ MLS | ⚠ pairwise | ✅ | ⚠ | ❌ | ⚠ | ⚠ | ❌ | ✅ Megolm |
| PQ handshake | ✅ X25519+MLKEM | ✅ PQXDH | 🔬 draft | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| PQ ratchet | ✅ Triple | ✅ Triple 2025 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| PQ deniable auth | 🟡 ADR-009 | 🟡 RingXKEM research | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Sealed sender | ✅ | ✅ | ❌ | ✅✅ no-id | ✅ | ✅ Tor | ✅ Lokinet | ⚠ dest-hash | ❌ |
| No user-ID | ⚠ pubkey | ❌ phone | ⚠ | ✅✅ | ⚠ pub | ⚠ pub | ⚠ pub | ⚠ pub | ❌ |
| Mesh transport | ✅ ETX | ❌ HTTPS | ❌ | ❌ | ✅ Bt/Wifi | ❌ Tor | ❌ Lokinet | ✅✅ | ❌ federated |
| Padding fixed-size | ✅ R-CHAT-9 | partial | ❌ | ⚠ | partial | ⚠ | ⚠ | ❌ | ❌ |
| Cover traffic | ✅ opt-in | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Agent capability** | ✅✅ scope-attested | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Prompt-injection guard** | ✅✅ dual-LLM | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Tool manifest signing** | ✅✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Coq-verified | ✅ 7 theorems | ❌ | partial Cryspen | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

«✅✅» = уникальная фича Trinity Chat.

---

## 8 · Coq invariants (L-CHAT-9)

Файл `trinity-chat-clara/proofs/chat/`:

| Theorem | Inv-id | Statement (informal) |
|---|---|---|
| `chat_no_plaintext_at_rest` | INV-CHAT-1 | ∀ msg ∈ persist_log → ¬ plaintext_visible(msg) |
| `agent_capability_bound` | INV-CHAT-2 | ∀ tool_call · tool_call.scope ⊆ session.capability_token.scope |
| `ratchet_no_replay` | INV-CHAT-3 | ∀ (m1, m2) ∈ chain · m1.nonce = m2.nonce → m1 = m2 |
| `metadata_no_link` | INV-CHAT-4 | ∀ obs ∈ MeshObserver · Pr[link(s, r) | obs] − Pr[link(s, r)] ≤ ν |
| `mls_epoch_monotone` | INV-CHAT-5 | ∀ commits c1<c2 · c1.epoch < c2.epoch |
| `pq_kem_present` | INV-CHAT-6 | ∀ handshake · ∃ kem ∈ {ML-KEM-768} ∧ kem ∈ KDF.input |
| `signed_tool_only` | INV-CHAT-7 | ∀ tool_call · verify(tool_manifest.sig, operator_ca) = true |

Бюджет admitted = 1 (только `metadata_no_link` — статистическая аппроксимация, runtime contract = `tests/metadata_ttest.rs`).

---

## 9 · Falsifier corpus (R7 honesty)

### 9.1 Crypto falsifiers (≥ 5 per gate)
- G-C1: prekey-flip-bit, prekey-swap-order, prekey-expired, prekey-replay, prekey-foreign-CA.
- G-C2: ratchet-skip-step, ratchet-reuse-nonce, ratchet-rollback-chain.
- G-C3: dest-hash-collision, sender-pub-leak-via-timing, queue-rotation-failure.
- G-C5: removed-member-decrypts, epoch-rollback, commit-without-update.
- G-C7: persist-plaintext-fixture, sql-grep-keyword-leak.
- G-C10: ed25519-per-msg-found, mac-derived-from-keypair (deniable) check.

### 9.2 Prompt-injection corpus (200 attacks)
- 50 direct: role-play hijack, instruction override, system-prompt-extraction, jailbreak-known.
- 50 indirect: poisoned URL, poisoned PDF, poisoned email, poisoned code-comment, poisoned RAG snippet.
- 50 multi-turn: gradual-coercion, persona-drift, encoded-payload (base64, leetspeak, typoglycemia, Unicode-confusable).
- 50 capability-abuse: tool-out-of-scope, tool-with-spoofed-manifest, expired-token-replay, cross-session leak.

Эти 200 формируют open dataset `trinity-chat-falsifier-corpus.jsonl` (Apache 2.0). Каждая атака имеет `expected_verdict: BLOCK | SANITIZE | HITL | ALLOW`.

---

## 10 · 6-week roadmap

| Неделя | Lanes (parallel) | Milestones |
|---|---|---|
| 1 | L-CHAT-1, L-CHAT-9 (proof skeleton) | issue в `gHashTag/trios-chat`; identity-bundle + Coq stubs |
| 2 | L-CHAT-2, L-CHAT-4 | Triple Ratchet + sealed-sender PR |
| 3 | L-CHAT-3, L-CHAT-5 | MLS + Neon persistence integration |
| 4 | L-CHAT-6 (capability + tool manifest) | bot HITL flow, capability-token verifier |
| 5 | L-CHAT-6 (anti-injection), L-CHAT-7 | dual-LLM, padding, queue rotation |
| 6 | L-CHAT-8, L-CHAT-10, freeze | ProVerif model, falsifier corpus, beta release |

---

## 11 · ADRs (готовые шаблоны)

- **ADR-CHAT-001** — Использовать MLS вместо n-pairwise Double Ratchet для группы > 2.
- **ADR-CHAT-002** — Hybrid X25519 + ML-KEM-768 как нижняя граница; не делать ML-KEM опциональным.
- **ADR-CHAT-003** — Запретить per-message Ed25519 (deniability over non-repudiation).
- **ADR-CHAT-004** — Мессадж-padding фиксированных классов; `(256, 1024, 4096, 16384)` как φ-аналог { 1, 4, 16, 64 } × 256.
- **ADR-CHAT-005** — Cover traffic — opt-in per conversation, не общесистемный (UX).
- **ADR-CHAT-006** — Bot capability tokens — session-scoped (наследие MCP nov-2025); renew = HITL.
- **ADR-CHAT-007** — Dual-LLM filter обязателен для любого untrusted ingest (RAG, web, document).
- **ADR-CHAT-008** — Не использовать Intel SGX/AMD SEV: vendor-locked, supply-chain-risk; software-only.
- **ADR-CHAT-009** — Day-90 миграция к RingXKEM-style deniable PQ auth.
- **ADR-CHAT-010** — Reticulum LXMF — gateway, не основной транспорт.

---

## 12 · Acceptance summary

| EPIC gate | Status |
|---|:---:|
| G-C1..G-C10 individually | 🟡 design (this doc) |
| ≥ 8/10 lanes DONE | 🟡 plan |
| Coq 7 theorems Qed | 🟡 plan (1 admitted) |
| 200-attack corpus ≥ 95 % | 🟡 plan |
| ProVerif PQ-FS+PQ-Auth | 🟡 plan |
| Constitutional laws (Art. I-V) preserved | ✅ design respects |

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · NEVER STOP`

---

## Sources (полный список)

- [draft-ietf-mls-partial-02 (2025-09)](https://datatracker.ietf.org/doc/draft-ietf-mls-partial/)
- [RFC 9420 — MLS Protocol](https://datatracker.ietf.org/doc/rfc9420/)
- [Signal Documentation](https://signal.org/docs/)
- [Signal PQXDH-to-RingXKEM slides 2025-12](https://gniot.fr/assets/slides/2025/2025-12-signal.pdf)
- [SimpleX platform docs](https://simplex.chat/docs/simplex.html)
- [LXMF — markqvist/LXMF](https://github.com/markqvist/LXMF)
- [Element X / Matrix E2EE](https://element.io/features/end-to-end-encryption)
- [Cwtch — privacy preserving messaging](https://news.ycombinator.com/item?id=43367012)
- [PingPong: Metadata-private messaging without coordination (arXiv 2504.19566, 2025)](https://arxiv.org/html/2504.19566v1)
- [TEEMS — TEE-based metadata-private (PoPETs 2025-0119)](https://petsymposium.org/popets/2025/popets-2025-0119.pdf)
- [Real-World Deniability in Messaging (PoPETs 2025-0018)](https://petsymposium.org/popets/2025/popets-2025-0018.pdf)
- [MCP authentication 2026 — stackoverflow.blog](https://stackoverflow.blog/2026/01/21/is-that-allowed-authentication-and-authorization-in-model-context-protocol/)
- [MCP 2026 status — workos.com](https://workos.com/blog/everything-your-team-needs-to-know-about-mcp-in-2026)
- [Google A2A announcement](https://developers.googleblog.com/en/a2a-a-new-era-of-agent-interoperability/)
- [A2A 2026 glossary — paz.ai](https://www.paz.ai/glossary/agent-to-agent-protocol-a2a)
- [OWASP LLM Prompt Injection Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/LLM_Prompt_Injection_Prevention_Cheat_Sheet.html)
- [OWASP LLM Top-10 2026 — repello.ai](https://repello.ai/blog/owasp-llm-top-10-2026)
- [Prompt injection on AI agents 2026 — atlan.com](https://atlan.com/know/prompt-injection-attacks-ai-agents/)
- [Reticulum FOSDEM 2026 slides](https://fosdem.org/2026/events/attachments/9NCWUR-reticulum_community_meetup_implementations_migration_and_future/slides/267005/reticulum_dimz1j8.pdf)
- [Metadata Protection in IM (Pass-the-SALT 2025)](https://cfp.pass-the-salt.org/pts2025/talk/7K9MEV/)
- [Trinity mesh-node EPIC #22 (closed) — gHashTag/trinity-fpga#22](https://github.com/gHashTag/trinity-fpga/issues/22)
- [PR #629 (LANDED) — gHashTag/trios#629](https://github.com/gHashTag/trios/pull/629)
