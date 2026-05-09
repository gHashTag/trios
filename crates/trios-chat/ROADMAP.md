# Trinity Secure Chat — ROADMAP

> Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · POST-QUANTUM · UNLINKABLE · COVER-TIMING · AT-REST-AEAD · BOT-PARTIAL-MLS · KEM-KEY-CONFUSION · AAD-CONTEXT · RATCHET-FS · MLS-REORDER · SKIPPED-KEYS-DOS · MLS-WELCOME-REPLAY`
>
> Parent EPIC: [trinity-fpga#28](https://github.com/gHashTag/trinity-fpga/issues/28)
> Crate: [`crates/trios-chat`](./)
> Status as of Wave-13: **197 tests · 25/25 e2e · 1200/1200 falsifier · 24 categories · 90 Coq Qed / 0 Admitted · 0 unsafe · 0 monoliths**

This document tracks the wave-by-wave evolution of the privacy-first
chat protocol that powers user ↔ agent-bot communication on top of
`trios-mesh-node`. Every wave ships:

1. **Two new falsifier lanes** in distinct rings, each with 5 deterministic
   tests pinning a specific threat-model invariant.
2. **+50 / +50 falsifier corpus** in two new categories at 100% coverage
   and a `≥95%` threshold lane in `falsifier_runner` (`indirect ≥90%`).
3. **+~10 Coq `Qed.`** closing the new invariants with no new admissions.
4. **A small, runnable scaffold** — every claim is one of
   `[VERIFIED]` / `[CITED]` / `[DERIVED]` / `[ASPIRATIONAL]` per
   Article I + R5 of the Trinity Constitution.

Architectural rules **never** broken across waves:

- **L-ARCH-001** — No monoliths. Each lane lives in a ring under
  `crates/trios-chat/rings/` and may only depend on lower-numbered rings.
- **L1** — No `.sh` files anywhere in `crates/trios-chat/`.
- **L2** — Every PR body starts with `Closes #N` (Refs alone fails Laws Guard CI).
- **R3** — Forbid `unsafe`, deny clippy `-D warnings`, `coqc` must be silent.
- **R5** — Honesty mode: no claim without a verification tag.
- **SeaORM** is the only ORM. No `.sh`. No `unsafe`.

---

## Threat-model lanes (`L-CHAT-*`) and rings (`R-CHAT-*`)

| Lane code | Ring(s) | What it pins down |
| :-- | :-- | :-- |
| L-CHAT-1 | R-CHAT-1 / CR-CHAT-01 | Identity + sealed sender (Signal-style) |
| L-CHAT-2 | R-CHAT-2 / CR-CHAT-02 | Triple Ratchet (DH + KEM hybrid PQXDH-style) |
| L-CHAT-3 | R-CHAT-11 / CR-CHAT-03 | MLS group state + epoch monotonicity |
| L-CHAT-4 | R-CHAT-1 / CR-CHAT-01 | Sealed sender (sender unlinkability) |
| L-CHAT-5 | R-CHAT-1 / CR-CHAT-05 | Persistence at-rest AEAD (no plaintext at rest) |
| L-CHAT-6 | R-CHAT-9 / CR-CHAT-04 | Fixed-size padding classes `{256,1024,4096,16384}` |
| L-CHAT-7 | R-CHAT-10 / CR-CHAT-07 | Cover traffic + timing uniformity |
| L-CHAT-8 | R-CHAT-2 / CR-CHAT-02 | PQ hybrid `(X25519 ‖ ML-KEM-768)` mix into root |
| L-CHAT-9 | R-CHAT-12 / CR-CHAT-06 | Capability + injection guardrails (deny patterns) |

Threat-model invariants are formalised in `proofs/chat/Trinity_Chat.v`
(Coq 8.20.1) and bound to runtime guards in the rings via the
`coq-runtime-invariants` skill (assertions / Result / process_*).

---

## Waves shipped (W1–W13)

Every wave is one merged PR landing on `main`. Wave-N+1 always branches
from `origin/main` immediately after Wave-N is merged. The cadence is
strict: pick **two** uncovered threat classes, ship 5 deterministic
tests per lane, +50 falsifier per lane, +~10 Coq Qed, all gates green.

| Wave | Merge SHA | Tests | Coq Qed | Falsifier | Cats | Lanes shipped | PR |
| :-- | :-- | :-- | :-- | :-- | :-- | :-- | :-- |
| W1  | (scaffold) | — | INV-CHAT-1..3 | — | — | L-CHAT-1, L-CHAT-2 skeletons | EPIC [trinity-fpga#28](https://github.com/gHashTag/trinity-fpga/issues/28) |
| W2  | (scaffold) | — | INV-CHAT-4..8 | 100 | 4 | L-CHAT-3 (MLS skeleton) + L-CHAT-2 ratchet | — |
| W3  | (scaffold) | — | INV-CHAT-9..12 | 200 | 6 | L-CHAT-4 sealed-sender + L-CHAT-5 at-rest | — |
| W4  | (scaffold) | — | INV-CHAT-12 | 300 | 8 | L-CHAT-6 padding + indirect-injection | — |
| W5  | (scaffold) | — | INV-CHAT-13..15 | 400 | 9 | L-CHAT-8 PQ hybrid (X25519‖ML-KEM-768) | — |
| W6  | (scaffold) | — | INV-CHAT-16..21 | 500 | 10 | L-CHAT-7 cover traffic + timing uniformity | — |
| W7  | `8787f25`  | 125 | INV-CHAT-22..27 | 600 | 12 | sender_unlinkability + traffic_analysis | [#646](https://github.com/gHashTag/trios/pull/646) |
| W8  | `e991cec`  | 137 | INV-CHAT-28..33 (43 Qed total) | 700 | 14 | partial_mls_bot + envelope_padding_leak | [#648](https://github.com/gHashTag/trios/pull/648) |
| W9  | `7340d24`  | 149 | INV-CHAT-34..39 (51 Qed total) | 800 | 16 | kem_key_confusion + aad_context_confusion | [#651](https://github.com/gHashTag/trios/pull/651) |
| W10 | (PR open)  | 161 | INV-CHAT-40..46 (60 Qed total) | 900 | 18 | ratchet_forward_secrecy + mls_commit_reorder | [#665](https://github.com/gHashTag/trios/pull/665) |
| W11 | (PR open)  | 173 | INV-CHAT-47..53 (70 Qed total) | 1000 | 20 | skipped_keys_dos + mls_welcome_replay | [#689](https://github.com/gHashTag/trios/pull/689) |
| W12 | (open) | 185 | INV-CHAT-54..60 (79 Qed total) | 1100 | 22 | prekey_exhaustion + mls_leaf_compromise | #695 (open) |
| **W13** | **(this PR)** | **197** | **INV-CHAT-61..67 (90 Qed total)** | **1200** | **24** | **deniability_break + confused_deputy** | **(open)** |

> Notes on Coq counting: pre-Wave-10 the team used `grep -cE "^Qed\.$"`
> (standalone-line count). The new standard since Wave-10 is the
> **total `Qed.` occurrence count** (`grep -cE "Qed\."`), which captures
> inline `Proof. ... Qed.` lemmas too. All historical totals in this
> table are restated under the new standard.

---

## Detailed wave summaries

### Wave-7 — sender unlinkability + traffic-analysis resistance

- **L-CHAT-4-unlink** (R-CHAT-1) — sealed-sender unlinkability invariants.
- **L-CHAT-7-traffic** (R-CHAT-10) — wire-level traffic-analysis resistance.
- 6 deterministic falsifier tests per lane in `CR-CHAT-01` and `CR-CHAT-07`.
- Falsifier 500 → 600 (`sender_unlinkability` + `traffic_analysis`).
- Coq INV-CHAT-22..27 — 6 new theorems.
- PR [#646](https://github.com/gHashTag/trios/pull/646), merged as `8787f25`.

### Wave-8 — partial MLS bot + envelope-padding leak

- **L-CHAT-3-bot** (R-CHAT-11) — bot is partial MLS member; cannot read
  human-only sub-conversations; cannot exfiltrate group state.
- **L-CHAT-6-padlk** (R-CHAT-9) — envelope-padding-leak: padding class
  must not depend on plaintext length parity / mod-class side-channels.
- Coq INV-CHAT-28..33.
- Falsifier 600 → 700.
- PR [#648](https://github.com/gHashTag/trios/pull/648), merged as `e991cec`.

### Wave-9 — KEM key confusion + AAD context confusion

- **L-CHAT-8-kem** (R-CHAT-2) — ML-KEM-768 key confusion: shared secret
  derived from one ciphertext must not validate under another keypair
  (`ss_kp_injective` axiom; Wave-9 is the first wave to formalise an
  axiom about KEM injectivity).
- **L-CHAT-9-aad** (R-CHAT-12) — AAD context confusion: AEAD nonce/AAD
  must bind `(epoch, sender, counter, group_id)` so a ciphertext from
  one context cannot be replayed in another.
- Coq INV-CHAT-34..39 (51 Qed total under the new counting standard).
- Falsifier 700 → 800.
- PR [#651](https://github.com/gHashTag/trios/pull/651), merged as `7340d24`.

### Wave-10 — ratchet forward-secrecy + MLS commit-reorder

- **L-CHAT-2-rfs** (R-CHAT-2) — RFS-01..05 in `CR-CHAT-02`:
  - chain step rotates message key; chain diverges after many steps;
    DH step breaks chain continuity; post-compromise root is independent
    of pre-chain history; hybrid KEM contribution is non-degenerate.
- **L-CHAT-3-mls** (R-CHAT-11) — MCR-01..05 in `CR-CHAT-03`:
  - future commit rejected; swapped pair rejected; epoch replay rejected;
    parallel fork rejected; cross-group splice rejected.
- Coq INV-CHAT-40..46 + 2 helpers (`chain_step_increases`,
  `process_commit_advances_one`); 9 new Qed → **60 Qed total**.
- 3 new axioms: `dh_step_fresh`, `dh_post_history_independent`,
  `hybrid_kem_non_degenerate`. All concretely instantiated in
  `CR-CHAT-02` `chain.rs` (Wave-5+10 RFS suite).
- Falsifier 800 → 900 (PI-RFS-001..050 + PI-MCR-001..050).
- PR [#665](https://github.com/gHashTag/trios/pull/665) (open, awaiting Laws Guard).

### Wave-11 — skipped-keys DoS + MLS Welcome replay/forge

- **L-CHAT-2-skip** (R-CHAT-2) — SKP-01..05 in `CR-CHAT-02`:
  - skipped-key cache bounded by `SKIPPED_KEYS_CAP=1024`;
  - DH-ratchet step bounds the skipped cache (no cross-epoch leak);
  - huge counter jump does not blow past the cap;
  - replay of an already-consumed counter is rejected;
  - `take_skipped` is one-shot (second take returns `None`).
- **L-CHAT-3-welcome** (R-CHAT-11) — WLR-01..05 in `CR-CHAT-03`,
  with new `Group::process_welcome` API + `consumed_welcomes: BTreeSet`:
  - cross-group welcome rejected;
  - future-epoch welcome rejected;
  - replayed `(epoch, leaf)` welcome rejected;
  - non-member-leaf welcome rejected;
  - stale-epoch welcome (after re-key) rejected.
- Coq INV-CHAT-47..53 + 2 helpers (`bounded_insert_le_cap`,
  `process_welcome_marks_consumed`); 10 new Qed → **70 Qed total**.
- No new axioms — both lanes prove constructively.
- Falsifier 900 → 1000 (PI-SKP-001..050 + PI-WLR-001..050).
- 18 → 20 threshold lanes in `falsifier_runner` (all ≥ 0.95 except
  `indirect ≥ 0.90`).

### Wave-13 — cryptographic deniability + confused-deputy capability

- **L-CHAT-5-deniable** (R-CHAT-4) — DEN-01..06 in `CR-CHAT-02`, with new
  `crates/trios-chat/rings/CR-CHAT-02/src/deniable.rs` shipping a
  `DeniableMacKey` / `Tag` / `mac` / `verify` / `forge_transcript` API:
  - well-formed HMAC-SHA-256 MAC verifies (DEN-01);
  - flipping any plaintext byte invalidates the tag (DEN-02);
  - flipping any AAD byte invalidates the tag (DEN-03);
  - MAC under a different key is rejected (DEN-04);
  - transcript-forgery is bit-indistinguishable from honest MAC under
    the same key — the formal deniability witness (DEN-05);
  - `Tag` is exactly 32 bytes — carries no per-message public-key
    signature (DEN-06).
- **L-CHAT-9-cap** (R-CHAT-6/8) — CAP-01..06 in `CR-CHAT-06`, with new
  `crates/trios-chat/rings/CR-CHAT-06/src/confused_deputy.rs` shipping
  `Invocation` / `NonceLedger` / `check_invocation` / `DeputyError`:
  - session binding (CAP-01) — `tok.session_id == inv.session_id`;
  - deputy binding (CAP-02) — `tok.agent_id == inv.deputy_id`;
  - scope coverage (CAP-03) — `inv.action ∈ tok.scopes`;
  - caller/deputy structural separation (CAP-04) — `caller_id` and
    `deputy_id` are distinct fields, audited via `same_principal()`;
  - nonce-replay rejection (CAP-05) — per-deputy `(deputy, nonce)`
    ledger is one-shot;
  - ttl coverage (CAP-06) — `inv.now_unix < tok.expires_at`.
- Coq INV-CHAT-61..67 + 4 helpers (`mac_functional`, `cap_scope_in_cons`,
  `ttl_failure_short_circuits`, `seen_nonce_empty`); 11 new Qed →
  **90 Qed total**.
- No new axioms — both lanes prove constructively. The MAC
  collision-resistance hypothesis in INV-CHAT-64 is a *bound variable*
  in the theorem statement, not an axiom.
- Falsifier 1100 → 1200 (PI-DEN-001..050 + PI-CAP-001..050).
- 22 → 24 threshold lanes in `falsifier_runner` (all ≥ 0.95 except
  `indirect ≥ 0.90`).

### Wave-12 — prekey-bundle exhaustion + MLS leaf-key compromise

- **L-CHAT-1-prekey** (R-CHAT-1) — PEX-01..05 in `CR-CHAT-01`,
  with new `crates/trios-chat/rings/CR-CHAT-01/src/otpk.rs` shipping
  `OtpkPool` / `Otpk` / `JoinStrategy::{OneTime, SignedFallback}`:
  - one-time prekey pool drains to empty over N takes;
  - exhausted pool forces `JoinStrategy::SignedFallback`;
  - replayed (already-consumed) OTPK index rejected;
  - refill restores `OneTime` strategy with fresh indices;
  - single-use guarantee on `pool_take` is one-shot.
- **L-CHAT-3-leaf** (R-CHAT-11) — LCO-01..05 in `CR-CHAT-03`,
  with new `Group::process_leaf_resync` API + `leaf_keys: BTreeMap<u32, [u8;32]>`:
  - leaf-resync from non-member rejected;
  - legitimate resync rotates stored leaf key + advances epoch;
  - pre-resync packet rejected after rotation (epoch monotone);
  - replay of captured resync at older `from_epoch` rejected;
  - concurrent resync at same `from_epoch` — only first applies.
- Coq INV-CHAT-54..60 + 2 helpers (`pool_take_decreases`,
  `process_leaf_resync_advances_one`); 9 new Qed → **79 Qed total**.
- No new axioms — both lanes prove constructively.
- Falsifier 1000 → 1100 (PI-PEX-001..050 + PI-LCO-001..050).
- 20 → 22 threshold lanes in `falsifier_runner` (all ≥ 0.95 except
  `indirect ≥ 0.90`).

---

## Falsifier-corpus categories (W1–W13) — 24 total

| # | Category | First wave | Threshold |
| :-- | :-- | :-- | :-- |
|  1 | direct                       | W2  | 0.95 |
|  2 | indirect                     | W4  | **0.90** |
|  3 | multi_turn                   | W3  | 0.95 |
|  4 | capability_abuse             | W3  | 0.95 |
|  5 | metadata_leak                | W3  | 0.95 |
|  6 | replay                       | W4  | 0.95 |
|  7 | pq_downgrade                 | W5  | 0.95 |
|  8 | group_state_rollback         | W5  | 0.95 |
|  9 | persistence_at_rest          | W6  | 0.95 |
| 10 | cover_traffic_correlation    | W6  | 0.95 |
| 11 | sender_unlinkability         | W7  | 0.95 |
| 12 | traffic_analysis             | W7  | 0.95 |
| 13 | partial_mls_bot              | W8  | 0.95 |
| 14 | envelope_padding_leak        | W8  | 0.95 |
| 15 | kem_key_confusion            | W9  | 0.95 |
| 16 | aad_context_confusion        | W9  | 0.95 |
| 17 | ratchet_forward_secrecy      | W10 | 0.95 |
| 18 | mls_commit_reorder           | W10 | 0.95 |
| 19 | skipped_keys_dos             | W11 | 0.95 |
| 20 | mls_welcome_replay           | W11 | 0.95 |
| 21 | prekey_exhaustion    | W12 | 0.95 |
| 22 | mls_leaf_compromise  | W12 | 0.95 |
| **23** | **deniability_break**  | **W13** | **0.95** |
| **24** | **confused_deputy**    | **W13** | **0.95** |

`falsifier_runner` is the gate: it loads `corpus/prompt_injection.jsonl`,
runs `validate_output` on each entry, and exits non-zero if any threshold
lane drops below its bound. Wave-13 ships 1200/1200 blocked across 24 lanes.

---

## Coq invariant index (INV-CHAT-1..67)

Cumulative `Qed.` count: **90 / 0 Admitted**. R5 admission budget: **0/10 used**.

| Range | Wave | Theme |
| :-- | :-- | :-- |
| INV-CHAT-1..12  | W1–W3 | identity, ratchet skeleton, MLS skeleton |
| INV-CHAT-13..15 | W5    | PQ hybrid mix |
| INV-CHAT-16..21 | W6    | cover traffic / timing uniformity |
| INV-CHAT-22..27 | W7    | sender unlinkability + traffic analysis |
| INV-CHAT-28..33 | W8    | partial-MLS bot + envelope padding |
| INV-CHAT-34..39 | W9    | KEM key confusion + AAD context confusion |
| INV-CHAT-40..46 | W10   | ratchet FS / PCS + MLS commit reorder |
| INV-CHAT-47..53 | W11   | skipped-keys cap + Welcome replay/forge |
| INV-CHAT-54..60 | W12 | prekey-bundle exhaustion + MLS leaf-key compromise |
| **INV-CHAT-61..67** | **W13** | **cryptographic deniability + confused-deputy capability** |

Cumulative axioms: `ss_kp_injective` (W9), `dh_step_fresh` (W10),
`dh_post_history_independent` (W10), `hybrid_kem_non_degenerate` (W10).
Wave-11, Wave-12, and Wave-13 all introduce **zero** new axioms — every proof is constructive.

---

## Future waves (W14–W20) — `[ASPIRATIONAL]`

The plan below is `[ASPIRATIONAL]` per R5 — none of these have shipped
yet. Each row picks **two** uncovered or under-pinned threat classes
following the established cadence (5 tests/lane, +50/+50 corpus,
+~10 Coq Qed, all gates green, PR closes a sub-tracker issue).

| Wave | Lane A (ring) | Lane B (ring) | New corpus categories | Coq target | Tests target | Falsifier target |
| :-- | :-- | :-- | :-- | :-- | :-- | :-- |
| ~~W12~~ — SHIPPED (see Wave-12 detail above) | | | | | | |
| ~~W13~~ — SHIPPED in this PR (see Wave-13 detail above) | | | | | | |
| **W14** | L-CHAT-2-oob (R-CHAT-2) — out-of-band identity verification + safety-number mismatch | L-CHAT-3-extern (R-CHAT-11) — MLS external-commit / external-join forgery | `safety_number_swap`, `mls_external_commit` | INV-CHAT-68..74 (≥100 Qed) | ≈209 | 1300 / 26 cats |
| **W15** | L-CHAT-7-funnel (R-CHAT-10) — Tailscale-funnel egress fingerprinting / TLS-fingerprint | L-CHAT-1-revoke (R-CHAT-1) — identity-key revocation + grace-window | `egress_fingerprint`, `identity_revoke` | INV-CHAT-75..81 (≥110 Qed) | ≈221 | 1400 / 28 cats |
| **W16** | L-CHAT-2-clock (R-CHAT-2) — clock-skew / replay-window edge cases | L-CHAT-5-rotate (R-CHAT-5) — at-rest key rotation / re-encryption ordering | `clock_skew_replay`, `at_rest_rotation` | INV-CHAT-82..88 (≥120 Qed) | ≈233 | 1500 / 30 cats |
| **W17** | L-CHAT-9-tool (R-CHAT-12) — tool-call argument confusion / type-confusion injection | L-CHAT-3-pcs (R-CHAT-11) — group-PCS healing after device compromise | `tool_arg_confusion`, `group_pcs_break` | INV-CHAT-89..95 (≥130 Qed) | ≈245 | 1600 / 32 cats |
| **W18** | L-CHAT-6-cls (R-CHAT-9) — padding-class oracle (timing-class leak) | L-CHAT-7-jitter (R-CHAT-10) — jitter-injection / inter-arrival side-channel | `padding_class_oracle`, `jitter_side_channel` | INV-CHAT-96..102 (≥140 Qed) | ≈257 | 1700 / 34 cats |
| **W19** | L-CHAT-8-decap (R-CHAT-2) — ML-KEM-768 decapsulation oracle / Fujisaki–Okamoto re-encryption | L-CHAT-9-tagsplit (R-CHAT-12) — tag-stripping / structured-output split | `kem_decap_oracle`, `tag_stripping` | INV-CHAT-103..109 (≥150 Qed) | ≈269 | 1800 / 36 cats |
| **W20** | L-CHAT-1-handshake (R-CHAT-1) — handshake fingerprinting + transcript-binding | L-CHAT-3-add (R-CHAT-11) — concurrent Add/Remove ordering + ghost-member | `handshake_fingerprint`, `concurrent_add_remove` | INV-CHAT-110..116 (≥160 Qed) | ≈281 | 1900 / 38 cats |

After W20 the corpus crosses **1900 entries / 38 categories** and Coq
crosses **160 closed proofs / 0 admissions**, exhausting the planned
threat surface for the EPIC #28 scaffold. From W21+ the work shifts
from **adding** lanes to **deepening** existing ones (replacing
axioms with constructive proofs, retiring `[ASPIRATIONAL]` tags,
wiring lanes through the real `openmls` / `pqcrypto-mlkem` paths).

---

## Operational invariants — never broken

The following are **not** lanes; they are fixed contracts every wave
reverifies. A wave PR must keep all of them green.

| Gate | Command (run from `/home/user/workspace/trios`) | Expected |
| :-- | :-- | :-- |
| Chat unit tests | `cargo test -q -p trios-chat-cr-chat-* -p trios-chat-br-* -p trios-chat-cr-chat-laws -p trios-chat` | `N / 0` (N grows by ~12 per wave) |
| End-to-end smoke | `cargo run -q -p trios-chat --bin e2e_chat_25` | `25/25 pass` |
| Falsifier corpus | `cargo run -q -p trios-chat --bin falsifier_runner` | `1200/1200 blocked` (W13) at 24 thresholds |
| Clippy           | `cargo clippy -p trios-chat -p trios-chat-cr-chat-* --all-targets -- -D warnings` | clean |
| Coq              | `coqc crates/trios-chat/proofs/chat/Trinity_Chat.v` | silent, exit 0 |
| Laws Guard CI    | PR body opens with `Closes \|Fixes \|Resolves #N` | green |
| L-ARCH-001       | New code lives under `crates/trios-chat/rings/CR-CHAT-NN/` only | enforced by build graph |
| L1               | `find crates/trios-chat -name '*.sh'` | empty |

---

## Cross-wave conventions

- **Branch naming**: `feat/trios-chat-wave<N>` from the latest `origin/main`.
- **Commit identity**: `Trinity Chat Wave-N <trinity-chat@gHashTag.io>` per wave.
- **Sub-tracker issue**: every wave opens a fresh issue (`Wave-N sub-tracker`)
  closed by the wave PR (`Closes gHashTag/trios#NNN`).
- **PR body format**: starts with `Closes #NNN` on the very first line,
  then a brief lane summary, then the verification block:
  ```
  Verified [VERIFIED]: <N> tests, 25/25 e2e, <M>/<M> falsifier (<K> cats),
  clippy clean, coqc silent.
  ```
- **Wave-N tests gain ~12** (+5 lane A, +1 green A, +5 lane B, +1 green B).
- **Wave-N corpus gains exactly 100** (+50 per lane).
- **Wave-N Coq gains ~10 Qed** (3–4 INV theorems per lane + helpers).

---

## Honesty tags ([cite:R5])

This document is itself tagged per R5:

- All wave SHAs `8787f25`, `e991cec`, `7340d24` are **[VERIFIED]** by
  `git log` on `feat/trios-chat-wave10`.
- Wave-10 PR **#665** and Wave-11 PR are **[VERIFIED]** by `gh pr view`.
- All Coq Qed counts are **[VERIFIED]** by `grep -cE "Qed\." Trinity_Chat.v`.
- Test counts and falsifier counts are **[VERIFIED]** by the cargo
  output captured in each wave PR body.
- W14..W20 lane definitions are **[ASPIRATIONAL]** — they constitute the
  forward plan and have not been validated by tests/Coq yet.

---

## See also

- `crates/trios-chat/README.md` — crate overview, build & run
- `crates/trios-chat/proofs/chat/Trinity_Chat.v` — Coq invariant source
- `crates/trios-chat/corpus/prompt_injection.jsonl` — canonical falsifier corpus
- `crates/trios-chat/src/bin/falsifier_runner.rs` — threshold gate
- `crates/trios-chat/src/bin/e2e_chat_25.rs` — 25-step end-to-end smoke test
- EPIC [trinity-fpga#28](https://github.com/gHashTag/trinity-fpga/issues/28)
- Trinity Constitution Article I (R5 honesty mode)
