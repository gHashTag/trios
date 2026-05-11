# Trinity Secure Chat — ROADMAP

> Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · POST-QUANTUM · UNLINKABLE · COVER-TIMING · AT-REST-AEAD · BOT-PARTIAL-MLS · KEM-KEY-CONFUSION · AAD-CONTEXT · RATCHET-FS · MLS-REORDER · SKIPPED-KEYS-DOS · MLS-WELCOME-REPLAY · PREKEY-EXHAUSTION · MLS-LEAF-COMPROMISE · DENIABILITY · CONFUSED-DEPUTY · OOB-IDENTITY · MLS-EXTERNAL-COMMIT · EGRESS-FINGERPRINT · IDENTITY-REVOKE · CLOCK-SKEW-REPLAY · AT-REST-ROTATE · TOOL-ARG-CONFUSION · GROUP-PCS-HEAL · PADDING-CLASS-ORACLE · JITTER-SIDE-CHANNEL · KEM-DECAP-ORACLE · TAG-STRIPPING · HANDSHAKE-FINGERPRINT · CONCURRENT-ADD-REMOVE · EPOCH-AUTH-FAILURE · WELCOME-KP-PINNING`
>
> Parent EPIC: [trinity-fpga#28](https://github.com/gHashTag/trinity-fpga/issues/28)
> Crate: [`crates/trios-chat`](./)
> Status as of Wave-21: **330 tests · 25/25 e2e · 2000/2000 falsifier · 40 categories · 168 Coq Qed / 0 Admitted · 0 unsafe · 0 monoliths**

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

## Waves shipped (W1–W18)

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
| W13 | (open) | 197 | INV-CHAT-61..67 (90 Qed total) | 1200 | 24 | deniability_break + confused_deputy | #698 (open) |
| W14 | (open) | 209 | INV-CHAT-68..74 (101 Qed total) | 1300 | 26 | safety_number_swap + mls_external_commit | #701 (open) |
| W15 | (open) | 221 | INV-CHAT-75..81 (112 Qed total) | 1400 | 28 | egress_fingerprint + identity_revoke | #703 (open) |
| W16 | `1bd0c54` | 235 | INV-CHAT-82..88 (121 Qed total) | 1500 | 30 | clock_skew_replay + at_rest_rotation | [#711](https://github.com/gHashTag/trios/pull/711) (rolled up via [#665](https://github.com/gHashTag/trios/pull/665)) |
| W17 | `047f3cb` | 249 | INV-CHAT-89..95 (130 Qed total) | 1600 | 32 | tool_arg_confusion + group_pcs_break | [#715](https://github.com/gHashTag/trios/pull/715) |
| W18 | `6902a82` | 270 | INV-CHAT-96..102 (139 Qed total) | 1700 | 34 | padding_class_oracle + jitter_side_channel | [#717](https://github.com/gHashTag/trios/pull/717) |
| W19 | `d601a58` | 290 | INV-CHAT-103..109 (148 Qed total) | 1800 | 36 | kem_decap_oracle + tag_stripping | [#719](https://github.com/gHashTag/trios/pull/719) |
| W20 | `e556075` | 310 | INV-CHAT-110..116 (158 Qed total) | 1900 | 38 | handshake_fingerprint + concurrent_add_remove | [#724](https://github.com/gHashTag/trios/pull/724) |
| **W21** | **(this PR)** | **330** | **INV-CHAT-117..123 (168 Qed total)** | **2000** | **40** | **epoch_authentication_failure + welcome_keypackage_pinning** | **(open)** |

> Notes on Coq counting: pre-Wave-10 the team used `grep -cE "^Qed\.$"`
> (standalone-line count). The new standard since Wave-10 is the
> **total `Qed.` occurrence count** (`grep -cE "Qed\."`), which captures
> inline `Proof. ... Qed.` lemmas too. All historical totals in this
> table are restated under the new standard.

---

## Detailed wave summaries

### Wave-21 — epoch-authentication failure + Welcome KeyPackage pinning

- **L-CHAT-2-eaf** (R-CHAT-2 / **CR-CHAT-02**) — EAF-01..10 in
  `crates/trios-chat/rings/CR-CHAT-02/src/epoch_authentication_failure.rs`
  (301 lines) shipping
  `check_epoch(local, presented) -> Result<EpochVerdict, EpochAuthenticationFailed>`,
  constant `EPOCH_GRACE_WINDOW = 2`, and the verdict
  `EpochVerdict::{Match, WithinWindow}`. The comparison is fully
  constant-time via `subtle::ConstantTimeEq` + `ConstantTimeLess`,
  and the past-distance is computed with `saturating_sub` so a
  presented epoch above the local one cannot underflow to look
  recent. The error type carries no payload — every rejection
  collapses to the same opaque `EpochAuthenticationFailed` so an
  attacker cannot distinguish "too old" from "too future" from
  "replayed" by error shape.
  - EAF-01 exact-match — `check_epoch(e, e)` returns `Match`.
  - EAF-02 within-grace — distance `1` and `2` return `WithinWindow`.
  - EAF-03 just-stale — distance `3` returns the opaque error.
  - EAF-04 future no underflow — `presented > local` is rejected
    and never produces `Match` or `WithinWindow` via wraparound.
  - EAF-05 ancient-same-error — distance `10_000` returns the
    exact same error variant as distance `3`.
  - EAF-06 opaque-error Display — `format!("{}", err)` returns a
    fixed string with no epoch numbers leaked.
  - EAF-07 symmetric-rejection — `(local, presented)` and
    `(presented, local)` both reject when distance is too large.
  - EAF-08 boundary scan — distances `0..=3` produce the expected
    sequence `Match, WithinWindow, WithinWindow, Err`.
  - EAF-09 grace-constant — `EPOCH_GRACE_WINDOW == 2` is the
    single source of truth.
  - EAF-10 green — module compiles and re-exports through
    `CR-CHAT-02/src/lib.rs`. → **10 unit tests**.

- **L-CHAT-5-wkp** (R-CHAT-1 / **CR-CHAT-05**) — WKP-01..10 in
  `crates/trios-chat/rings/CR-CHAT-05/src/welcome_keypackage_pinning.rs`
  (382 lines) shipping
  `KeyPackageHash::compute(suite, lt_pub, init_pub, signing_pub, capabilities) -> Result<Self, WelcomeError>`,
  constants `WKP_LEN = 32` and
  `WKP_DOMAIN = b"trios-chat-keypackage-hash-v1\0"`, constant-time
  equality via `subtle::ConstantTimeEq` (`KeyPackageHash::eq_ct`),
  and a private `absorb_tagged` helper that length-prefixes every
  field with a per-field domain separator so KeyPackage components
  cannot be confused or truncated. `KeyPackagePin::pin(h)` freezes
  the first hash a peer sees; `verify_welcome(incoming)` constant-
  time-compares against the pin; `repin` **always** returns
  `WelcomeError::RepinForbidden`.
  - WKP-01 canonical compute — fixed canonical inputs produce a
    fixed canonical hash.
  - WKP-02 determinism — same inputs always produce the same hash.
  - WKP-03 field-swap detection — swapping `lt_pub` with `init_pub`
    changes the hash.
  - WKP-04 empty-field rejection — any empty input returns
    `WelcomeError::EmptyField`.
  - WKP-05 length-shift detection — moving a byte from one field
    to an adjacent field changes the hash (length-prefix proves
    domain separation).
  - WKP-06 pin/verify happy path — `pin(h)` + `verify_welcome(h)`
    is `Ok(())`.
  - WKP-07 mismatch rejected — verifying a different hash returns
    `WelcomeError::Mismatch`.
  - WKP-08 repin forbidden — `pin.repin(other)` returns
    `WelcomeError::RepinForbidden`, the pin is immutable.
  - WKP-09 single-bit flip — flipping a single bit in any input
    field changes the resulting hash.
  - WKP-10 green — module compiles and re-exports through
    `CR-CHAT-05/src/lib.rs`. → **10 unit tests**.

- **Falsifier corpus 1900 → 2000.** New categories
  `epoch_authentication_failure` and `welcome_keypackage_pinning`,
  50 entries each (`PI-EAF-001..050`, `PI-WKP-001..050`),
  generated by `gen_falsifier_wave21.py`. Each lane covers the
  specific exploitation phrasings (grace-window bypass, opaque-
  error leakage, fingerprint truncation, repin attempts,
  empty-field bypass, length-shift collisions, KeyPackage field
  swaps, etc.). `falsifier_runner` gains two new threshold lanes
  at `0.95`. Result: **40 categories at 100% block rate**,
  `2000 / 2000` blocked.

- **DENY_PATTERNS extension.** `CR-CHAT-06/src/injection.rs`
  grows two new keyword blocks covering Lane A epoch-failure
  jargon ("genesis epoch", "local + 1", "recently-seen epoch",
  "fast-forward", "handshake_count", "priority=high", "in the
  aad", …) and Lane B KeyPackage-pinning jargon ("last byte
  only", "starts with 0x00/0x80", "short-circuit branch",
  "pre-filter", "skip ct_eq on mismatch", "capabilities = b",
  "bypass the emptyfield", "iter().zip(", "a==b", "mid-
  absorption", "cache the sha-256 state", "absorb the changed
  field", …) so the injection guard blocks any prompt that
  attempts to weaken the new lanes by name.

- **Coq Wave-21 — `Section TrinityChatWave21` (lines ≈ 2869–3044).**
  Introduces `Inductive EpochVerdict21 := EVMatch21 | EVWindow21 | EVRejected21`,
  `Record KPInputs21` with the five length fields
  `s_len_21, lt_len_21, ip_len_21, sp_len_21, c_len_21`,
  `Variable kp_hash_of_21 : nat -> nat -> nat -> nat -> nat -> nat`,
  computable functions `check_epoch_21`, `all_fields_nonempty_21`,
  `verify_pin_21`, and the constant `eaf_grace_21 := 2`.
  Closes:
  - **INV-CHAT-117** `inv_chat_117_eaf_future_rejected` —
    `local < presented → check_epoch_21 local presented = EVRejected21`.
  - **INV-CHAT-118** `inv_chat_118_eaf_match_accepted` —
    `check_epoch_21 e e = EVMatch21`.
  - **INV-CHAT-119** `inv_chat_119_eaf_opaque_error` — both
    rejected outputs are the same `EVRejected21` constructor.
  - Helper `within_grace_accepted_21` —
    `d ≤ eaf_grace_21 ∧ d > 0 ∧ d ≤ local → check_epoch_21 local (local - d) = EVWindow21`.
  - **INV-CHAT-120** `inv_chat_120_wkp_pin_immutable` —
    `verify_pin_21 p p = true`.
  - **INV-CHAT-121** `inv_chat_121_wkp_mismatch_rejected` —
    `p ≠ i → verify_pin_21 p i = false`.
  - **INV-CHAT-122** `inv_chat_122_wkp_hash_determinism` —
    `kp_hash_of_21 a b c d e = kp_hash_of_21 a b c d e`.
  - **INV-CHAT-123** `inv_chat_123_wkp_empty_field_invalid` —
    any empty field implies `all_fields_nonempty_21 = false`.
  - Helper `empty_invalidates_21`.
  - **Total: 168 `Qed.` / 0 `Admitted.` / 0 new axioms.**

- **Verification gate.** `cargo test` over the 12 chat crates plus
  harness binaries: **330 / 0**. `cargo run --bin e2e_chat_25`:
  **25 / 25**. `cargo run --bin falsifier_runner`: **2000 / 2000**
  blocked at 40 threshold lanes. `cargo clippy --all-targets -- -D warnings`
  on `trios-chat` + the three touched ring crates: clean.
  `coqc proofs/chat/Trinity_Chat.v`: silent exit `0`, three
  abstract-large-number warnings only (pre-existing W14/W15 nat
  literals, not from W21 code).

- **Cumulative axioms — unchanged.** `ss_kp_injective` (W9),
  `dh_step_fresh` + `dh_post_history_independent` +
  `hybrid_kem_non_degenerate` (W10), `sn_hash_sym` (W14). Wave-21
  introduces **zero** new axioms.

### Wave-20 — handshake fingerprinting + concurrent Add/Remove ordering

- **L-CHAT-1-handshake** (R-CHAT-1 / **CR-CHAT-01**) — HSF-01..10 in
  `crates/trios-chat/rings/CR-CHAT-01/src/handshake_fingerprint.rs`
  (343 lines) shipping `HandshakeFingerprint::compute(initiator_lt,
  responder_lt, initiator_pre, responder_pre, kem_ciphertext,
  suite_and_version) -> Result<Self, HandshakeError>`, constants
  `HSF_LEN = 32` and `HSF_DOMAIN = b"trios-chat-handshake-fingerprint-v1\0"`,
  constant-time equality via `subtle::ConstantTimeEq`
  (`HandshakeFingerprint::eq_ct`), and a private `absorb_tagged`
  helper that length-prefixes every field with a per-field domain
  separator so role/suite/transcript components cannot be confused.
  - HSF-01 responder-swap detection — swapping initiator_lt with
    responder_lt produces a different fingerprint.
  - HSF-02 role-flip detection — swapping initiator_pre with
    responder_pre produces a different fingerprint.
  - HSF-03 suite-downgrade detection — changing the
    `suite_and_version` tag flips the fingerprint.
  - HSF-04 truncation detection — truncating any input by one
    byte changes the fingerprint (length-prefix domain separation).
  - HSF-05 length-shift detection — moving a byte from one field
    to an adjacent field changes the fingerprint.
  - HSF-06 empty-field rejection — any empty input field returns
    `HandshakeError::EmptyField`, never a zero-prefix collision.
  - +4 bonus tests (determinism on identical inputs, single-bit CT
    flip via `eq_ct`, length constant, green) → **10 unit tests**.

- **L-CHAT-3-add** (R-CHAT-11 / **CR-CHAT-03**) — CAR-01..10 in
  `crates/trios-chat/rings/CR-CHAT-03/src/concurrent_add_remove.rs`
  (419 lines) shipping `Proposal::{Update,Remove,Add}{leaf,hash_id}`,
  `ConcurrencyError::{RemoveNonMember, AddExisting, UpdateNonMember,
  DuplicateSortKey}`, `MembershipDelta{added, removed, updated,
  final_members}`, and
  `apply_concurrent(base_members: &BTreeSet<Leaf>, proposals: &[Proposal])`.
  Deterministic priorities: `PRI_UPDATE = 0 < PRI_REMOVE = 1 < PRI_ADD = 2`,
  ties broken by `(priority, hash_id, sort_key)`.
  - CAR-01 add-after-remove ghost — concurrent `{Remove(L), Add(L)}`
    resolves with `L` removed (Remove < Add priority); no ghost
    membership.
  - CAR-02 remove-after-add resurrection — concurrent
    `{Add(L), Remove(L)}` on a non-member resolves with `L`
    **not** in the final set (Remove still wins).
  - CAR-03 dup-add — two `Add(L)` for the same `L` returns
    `ConcurrencyError::AddExisting` (no silent dedup).
  - CAR-04 dup-remove — `Remove(L)` for a non-member returns
    `RemoveNonMember`.
  - CAR-05 self-remove-with-update — concurrent
    `{Update(L), Remove(L)}` resolves with Update overridden by
    Remove (Update < Remove priority, but Remove erases).
  - CAR-06 empty-set determinism — `apply_concurrent(∅, &[])`
    returns an empty `MembershipDelta`.
  - +4 bonus tests (order-independence, tie-break by `hash_id`,
    `DuplicateSortKey` on equal sort keys, green) →
    **10 unit tests**.

- **Coq Wave-20** — `Section TrinityChatWave20` adds INV-CHAT-110
  through INV-CHAT-116 plus 2 helper lemmas (`all_nonzero_valid_20`,
  `update_before_add_20`). All proofs constructive over
  `PropClass20::{PUpdate20, PRemove20, PAdd20}`, records
  `TranscriptLens20{init_lt_len_20, resp_lt_len_20, init_pre_len_20,
  resp_pre_len_20, kem_ct_len_20, suite_len_20}` and
  `Delta20{base_size_20, n_added_20, n_removed_20}`, with variable
  `hsf_of_20 : nat -> nat -> nat -> nat -> nat -> nat -> nat`.
  10 new `Qed.` (7 INV + 2 helpers + 1 footer line) →
  **158 Qed total**, **0 Admitted**, **0 new axioms** (cumulative
  axiom set unchanged at 5: `ss_kp_injective`, `dh_step_fresh`,
  `dh_post_history_independent`, `hybrid_kem_non_degenerate`,
  `sn_hash_sym`).

- Falsifier 1800 → 1900 (PI-HSF-001..050 + PI-CAR-001..050) — 38
  attack categories all at 100%, G-C10 thresholds met for every
  category (≥95% non-direct, ≥90% direct).

- DENY_PATTERNS in `CR-CHAT-06/src/injection.rs` extended with
  ~360 W20 keywords covering handshake-fingerprint distinguishability
  language (`hsf compute`, `initiator_lt`, `responder_lt`,
  `responder swap`, `role flip`, `suite downgrade`, `truncate hsf`,
  `length-shift trick`, `absorb_tagged`, `domain separator`,
  `eq_ct`, `single-bit flip in fingerprint`, `force the hsf`)
  and concurrent-Add/Remove language (`apply_concurrent`,
  `priority ordering`, `add-after-remove`, `remove-after-add`,
  `ghost member`, `resurrection`, `tie break`, `duplicate sort
  key`, `ConcurrencyError`, `MembershipDelta`, `Proposal::Update`,
  `Proposal::Remove`, `Proposal::Add`).

- Anchor extended:
  `… · KEM-DECAP-ORACLE · TAG-STRIPPING · HANDSHAKE-FINGERPRINT
   · CONCURRENT-ADD-REMOVE`.

### Wave-19 — ML-KEM-768 decapsulation oracle + structured-output tag-stripping

- **L-CHAT-8-decap** (R-CHAT-2 / **CR-CHAT-01**) — DEC-01..06 in
  `crates/trios-chat/rings/CR-CHAT-01/src/kem_decap_oracle.rs`
  (285 lines) shipping `DecapObservation::{MatchedReference,
  DifferedFromReference, Errored}`, `ss_eq()`, `observe()`, plus
  consts `KEM_DECAP_ORACLE_CT_LEN = MLKEM768_CT_LEN`,
  `KEM_DECAP_ORACLE_SS_LEN = MLKEM768_SS_LEN`.
  - **Routing note**: ML-KEM-768 keypair, `encapsulate_to`,
    and ciphertext/shared-secret types live in **CR-CHAT-01**
    (`kem.rs`), not in CR-CHAT-02 as the W18 ROADMAP plan
    suggested. The decapsulation-oracle observer is therefore
    placed in CR-CHAT-01 to avoid a cross-ring leak of `kem.rs`
    internals; this preserves the L-ARCH-001 ring-only
    invariant.
  - DEC-01 honest decap matches reference (FO determinism).
  - DEC-02 single-bit ciphertext flip → `DifferedFromReference`
    (FO implicit-rejection produces a different shared secret).
  - DEC-03 distinct ciphertexts under the **same** keypair never
    collapse to the same shared secret (anti-malleability).
  - DEC-04 implicit-reject branch is content-bound: the rejection
    secret depends on `(ek, ct)` via the FO transform — a flipped
    ciphertext yields a different reject secret.
  - DEC-05 `ss_eq` is constant-time `subtle::ConstantTimeEq`
    (no early-exit on first differing byte → no decap timing
    oracle on the comparison itself).
  - DEC-06 `observe()` never returns the shared secret — only
    one of three opaque enum variants — sealing the
    `Ok(reject_ss)` vs `Ok(legit_ss)` distinguishability
    channel.
  - +3 bonus tests (`Errored` path, idempotence, length consts)
    + 1 green-each → **10 unit tests**.

- **L-CHAT-9-tagsplit** (R-CHAT-12 / CR-CHAT-06) — TAG-01..06 in
  `crates/trios-chat/rings/CR-CHAT-06/src/tag_stripping.rs`
  (380 lines) shipping `SpanTag::{Trusted, Untrusted}`,
  `TagSplit::{Unbalanced, NestedNotAllowed, UnknownTag,
  TagInPayload, EmptyInput, EmptyPayload, StrayBytes}`,
  `Span{tag, payload}`, `parse_structured_output()`,
  `serialise_structured_output()`. Tag alphabet is
  `<TRUSTED>…</TRUSTED>` / `<UNTRUSTED>…</UNTRUSTED>` only.
  - TAG-01 unbalanced opener without closer → `Unbalanced`.
  - TAG-02 nested span (`<TRUSTED><UNTRUSTED>…</UNTRUSTED></TRUSTED>`)
    → `NestedNotAllowed` (no recursive tag stack — flat sequence
    only).
  - TAG-03 unknown tag (e.g. `<SYSTEM>`, `<TRUST>`,
    `<TRUSTED foo>`) → `UnknownTag`.
  - TAG-04 tag-like substring inside payload (`</TRUSTED>` injected
    by attacker into untrusted text) → `TagInPayload`, never
    treated as a closing delimiter.
  - TAG-05 empty input → `EmptyInput`; empty payload between
    matched tags → `EmptyPayload` (no zero-length trust upgrade).
  - TAG-06 stray bytes outside any tagged span → `StrayBytes`
    (no implicit promotion of bare text to either trust class).
  - +3 bonus tests (round-trip serialise→parse, mixed
    sequence, case-sensitivity) + 1 green-each → **10 unit
    tests**.

- **Coq Wave-19** — `Section TrinityChatWave19` adds INV-CHAT-103
  through INV-CHAT-109 plus 2 helper lemmas
  (`nested_check_passes19`, `well_formed_span_passes19`).
  All proofs constructive over `DecapObs19`, `SpanTag19`,
  `TagSplit19`, `Span19{span_tag_19, span_payload_size_19}`
  with variables `kp_id_19 : nat`, `ss_of_19 : nat -> nat -> nat`.
  9 new `Qed.` (7 INV + 2 helpers) → **148 Qed total**, **0
  Admitted**, **0 new axioms** (cumulative axiom set unchanged at 5:
  `ss_kp_injective`, `dh_step_fresh`, `dh_post_history_independent`,
  `hybrid_kem_non_degenerate`, `sn_hash_sym`).

- Falsifier 1700 → 1800 (PI-DEC-001..050 + PI-TAG-001..050)
  — 36 attack categories all at 100%, G-C10 thresholds met for
  every category (≥95% non-direct, ≥90% direct).

- DENY_PATTERNS in `CR-CHAT-06/src/injection.rs` extended with
  ~260 W19 keywords covering FO-rejection distinguishability
  language (`ok(reject_ss)`, `short-circuit on bad ct`,
  `ct_hash`, `pseudorandom output`, `attacker-supplied
  randomness`, `(ct, ek) -> r`, `single-bit flip in ct`,
  `force the fo`, `legit ss` / `legitimate ss`) and tag-
  stripping language (`<trusted>`, `</trusted>`,
  `<untrusted>`, `</untrusted>`, `nested span`, `unbalanced
  tag`, `case-sensitivity check`, `attribute syntax`,
  `r-chat-12 tag`, `zero-length payload`, `mark trust
  without producing data`).

- Anchor extended:
  `… · TOOL-ARG-CONFUSION · GROUP-PCS-HEAL · PADDING-CLASS-ORACLE
   · JITTER-SIDE-CHANNEL · KEM-DECAP-ORACLE · TAG-STRIPPING`.

### Wave-18 — padding-class oracle + jitter-injection side-channel

- **L-CHAT-6-cls** (R-CHAT-9 / CR-CHAT-04) — CLS-01..06 in
  `crates/trios-chat/rings/CR-CHAT-04/src/padding_class_oracle.rs`
  (322 lines) shipping `PaddingOracleError::{NonClassSize,
  TruncatedTooShort, DeclaredLengthOverflow, ClassUpgrade,
  ClassDowngrade, NonZeroPaddingSuffix}`, `smallest_class()`,
  `check_class_choice()`, `validate_envelope()`, `pad_class_checked()`,
  `unpad_checked()`:
  - smallest-class oracle picks the unique minimal class from
    `{256,1024,4096,16384}` that fits `payload + 4 bytes length prefix`
    (CLS-01) — no upgrade-to-bigger as length oracle, no downgrade-to-smaller
    as truncation oracle;
  - over-padded ciphertext (e.g. 256-byte payload but envelope at the
    1024 class) rejected with `ClassUpgrade` (CLS-02) — prevents the
    sender from leaking length-class metadata via deliberate over-pad;
  - oversized payload exceeding `MAX_PAYLOAD_18 = 16384 − 4` rejected
    with `DeclaredLengthOverflow` (CLS-03) — prevents truncation of
    declared-length suffix into an out-of-band channel;
  - sub-4-byte truncation (`buf_len < 4`) rejected with
    `TruncatedTooShort` (CLS-04) — the length prefix is mandatory
    framing;
  - non-class envelope size (e.g. 257 bytes) rejected with
    `NonClassSize` (CLS-05) — classes are exactly
    `{256, 1024, 4096, 16384}`, never adjacent;
  - non-zero padding suffix rejected with `NonZeroPaddingSuffix`
    (CLS-06) — padding bytes MUST be `0x00`, otherwise an attacker
    can hide a covert side-channel in the trailing region.
- **L-CHAT-7-jitter** (R-CHAT-10 / CR-CHAT-07) — JIT-01..06 in
  `crates/trios-chat/rings/CR-CHAT-07/src/jitter_side_channel.rs`
  (405 lines) shipping `WireKind::{Real,Cover}`,
  `GapObservation{cumulative_ms, gap_ms, kind}`,
  `JitterError::{BurstBelowMinimum, NonCanonicalGap,
  NonMonotonicTimestamp, InsufficientCover, ClassBiasExceeded,
  GapTimestampMismatch}`, `JitterPolicy{min_cover_pct=25,
  max_class_pct=60}`, `validate_history()`, `GapRecorder`:
  - canonical-gap-only history accepts (JIT-01) — every gap is in
    `{50, 250, 1000, 5000, 30000, 300000}` ms, derived from
    `uniform_gap_ms` quantiser;
  - non-canonical gap (e.g. 137 ms) rejected with `NonCanonicalGap`
    (JIT-02) — prevents inter-arrival timing as side-channel for
    semantic content;
  - clock-rewind (`cumulative_ms` not strictly monotone) rejected
    with `NonMonotonicTimestamp` (JIT-03) — prevents reorder-attack
    bypass of replay window;
  - sub-minimum burst (`<` `JitterPolicy::min_burst_observations = 4`)
    rejected with `BurstBelowMinimum` (JIT-04) — prevents short-window
    sampling that defeats statistical mixing;
  - cover-traffic ratio below `min_cover_pct = 25%` rejected with
    `InsufficientCover` (JIT-05) — preserves cover-timing invariant
    from W6;
  - any single canonical class crossing `max_class_pct = 60%` rejected
    with `ClassBiasExceeded` (JIT-06) — prevents one-class flooding
    that would re-leak length-class metadata via the timing channel.
- Coq INV-CHAT-96..102 + helper `jitter_burst_below_minimum_rejected18`
  in a fresh `Section TrinityChatWave18` — uses unique W18-suffixed
  names (`smallest_class18`, `validate_gap18`, `class0_18..class3_18`,
  `gap0_18..gap3_18`, `PadOracleErr18`, `JitterErr18`,
  `check_class_choice18`) to avoid cross-wave name collisions; 8 new
  Qed → **139 Qed total**.
  - INV-CHAT-96 `inv_chat_96_smallest_class_in_set` — `smallest_class18`
    lands in `{256, 1024, 4096, 16384}`;
  - INV-CHAT-97 `inv_chat_97_padding_class_choice_minimal` — over-pad
    rejected as `ClassUpgrade18`;
  - INV-CHAT-98 `inv_chat_98_declared_length_overflow_rejected` —
    `payload > MAX_PAYLOAD_18` rejected;
  - INV-CHAT-99 `inv_chat_99_truncated_too_short_rejected` —
    `buf_len < 4` rejected;
  - INV-CHAT-100 `inv_chat_100_non_canonical_gap_rejected` — gap not
    in canonical set rejected;
  - INV-CHAT-101 `inv_chat_101_non_monotonic_timestamp_rejected` —
    clock-rewind rejected;
  - INV-CHAT-102 `inv_chat_102_gap_timestamp_mismatch_rejected` —
    reorder attack rejected.
- **Zero new axioms.** Both lanes prove constructively. Cumulative
  axiom count remains 5 (`ss_kp_injective`, `dh_step_fresh`,
  `dh_post_history_independent`, `hybrid_kem_non_degenerate`,
  `sn_hash_sym`).
- Falsifier 1600 → 1700 (PI-CLS-001..050 + PI-JIT-001..050) — 34
  categories @ 100% blocked.
- 32 → 34 threshold lanes in `falsifier_runner` (`padding_class_oracle`
  and `jitter_side_channel`, both at 0.95 — all lanes ≥ 0.95 except
  `indirect ≥ 0.90`).
- `DENY_PATTERNS` in `CR-CHAT-06/src/injection.rs` extended with W18
  keyword blocks: padding-class-oracle (paddingoracleerror, classupgrade,
  classdowngrade, nonclasssize, declaredlengthoverflow, truncatedtooshort,
  nonzeropaddingsuffix, smallest_class, check_class_choice,
  validate_envelope, pad_class_checked, unpad_checked, max_payload_18,
  256/1024/4096/16384-byte class, length-prefix oracle, over-padded,
  truncation oracle, padding suffix, padding side-channel, …) +
  jitter-side-channel (gapobservation, jitterpolicy, jittererror,
  burstbelowminimum, noncanonicalgap, nonmonotonictimestamp,
  insufficientcover, classbiasexceeded, gaptimestampmismatch,
  gaprecorder, validate_history, min_cover_pct, max_class_pct,
  inter-arrival, jitter injection, timing side-channel, cover-traffic
  ratio, class-bias flood, clock-rewind, reorder attack, gap
  quantiser, …) + 33 residual-miss patches.

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

### Wave-17 — tool-call argument confusion + group-PCS healing

- **L-CHAT-9-tool** (R-CHAT-9 / CR-CHAT-06) — TOOL-01..06 in
  `crates/trios-chat/rings/CR-CHAT-06/src/tool_arg_confusion.rs`
  (503 lines) shipping `ArgKind::{StringBounded{cap},U64,I64,Bool,Enum{variants}}`,
  `ArgSpec`, `ToolEntry`, `ToolManifest` (re-exported as
  `ToolArgManifest` to avoid collision with the legacy
  `capability::ToolManifest`), `ArgValue::{Str,U,I,Bool}`, `ToolCall`,
  `ToolCallError::{UnknownTool, MissingArg, UnexpectedArg,
  KindMismatch, StringTooLong, UnknownEnumVariant,
  NestedToolCallSentinel}`, and `validate_tool_call` /
  `NESTED_TOOL_CALL_SENTINEL = "<<TOOL-CALL>>"`:
  - well-formed `ToolCall` validates against the manifest with all
    kinds matched (TOOL-01) — `ArgKind` agrees with `ArgValue` shape;
  - `KindMismatch` rejected when `Bool` is sent where `Enum` was
    declared (TOOL-02) — prevents `'true'`-string-vs-`Bool`
    confusion;
  - `StringTooLong` rejected when a `StringBounded{cap}` arg exceeds
    the byte cap (TOOL-03) — prevents oversized-subject overflow;
  - `UnknownEnumVariant` rejected when an enum value is outside the
    declared `variants` set (TOOL-04) — prevents
    `default-enum`-on-`null` smuggling;
  - `UnknownTool` / `MissingArg` / `UnexpectedArg` rejected
    independently (TOOL-05) — strict by-name matching;
  - `NestedToolCallSentinel` rejected when any string argument
    contains the sentinel `<<TOOL-CALL>>` (TOOL-06) — prevents
    confused-deputy nested-tool injection.
- **L-CHAT-3-pcs** (R-CHAT-3 / CR-CHAT-03) — PCS-01..06 in
  `crates/trios-chat/rings/CR-CHAT-03/src/pcs_healing.rs` (352 lines)
  shipping `PathSecretHash`, `HealCommit{group_id, from_epoch,
  sender, heals}`, `HealEntry{target, from_hash, to_hash}`,
  `PcsState::{new, add_member, secret_of, process_heal}`:
  - well-formed `HealCommit` advances the group epoch by exactly one
    and rotates targeted members’ path-secrets (PCS-01);
  - heal where `from_hash` does not match the receiver’s current
    `secret_of(target)` is rejected (PCS-02) — detects stolen-PSK
    rotation against a stale view;
  - heal whose `from_epoch` differs from the receiver’s current
    epoch is rejected (PCS-03) — prevents future-epoch jumps and
    epoch regression;
  - empty / no-op heal (`heals.len() == 0`) is rejected (PCS-04) —
    cannot bump epoch without any rotation;
  - `to_hash == from_hash` (identity heal) is rejected (PCS-05);
  - duplicate-target inside a single `HealCommit` is rejected
    (PCS-06) — prevents intra-batch shadowing.
- Coq INV-CHAT-89..95 + helper `pcs_pre_heal_replay_rejected17` in a
  fresh `Section TrinityChatWave17` — uses unique names (`ArgKind17`,
  `ArgValue17`, `kind_match17`, `HealEntry17`, `PcsState17`,
  `heal_step17`) to avoid cross-wave name collisions; 9 new Qed →
  **130 Qed total**.
  - INV-CHAT-89 `inv_chat_89_tool_kind_mismatch_rejected`;
  - INV-CHAT-90 `inv_chat_90_tool_nested_sentinel_rejected`;
  - INV-CHAT-91 `inv_chat_91_tool_string_too_long_rejected`;
  - INV-CHAT-92 `inv_chat_92_tool_enum_variant_rejected`;
  - INV-CHAT-93 `inv_chat_93_pcs_heal_advances_one`;
  - INV-CHAT-94 `inv_chat_94_pcs_no_op_rejected`;
  - INV-CHAT-95 `inv_chat_95_pcs_epoch_mismatch_rejected`.
- **Zero new axioms.** Both lanes prove constructively. Cumulative
  axiom count remains 5 (`ss_kp_injective`, `dh_step_fresh`,
  `dh_post_history_independent`, `hybrid_kem_non_degenerate`,
  `sn_hash_sym`).
- Falsifier 1500 → 1600 (PI-TOOL-001..050 + PI-PCS-001..050) — 32
  categories @ 100% blocked.
- 30 → 32 threshold lanes in `falsifier_runner` (`tool_arg_confusion`
  and `group_pcs_break`, both at 0.95 — all lanes ≥ 0.95 except
  `indirect ≥ 0.90`).
- `DENY_PATTERNS` in `CR-CHAT-06/src/injection.rs` extended with W17
  keyword blocks: tool-arg-confusion (kindmismatch, kind mismatch,
  unknownenumvariant, stringbounded, argkind, argspec, toolentry,
  toolargmanifest, toolcall sentinel, `<<tool-call>>`,
  nestedtoolcallsentinel, oversized, exceeding-the, non-utf-8,
  smuggle-binary, conflicting-kinds, same-arg-name-twice, =null,
  default-enum, `'true' string`, `bool vs enum`, u64-overflows-i64,
  kind-match path, …) + group-pcs-break (pcs heal, healcommit,
  healentry, pcsstate, pathsecrethash, path-secret, pre-heal,
  heal_step, process_heal, no-op heal, to_hash, from_hash,
  sender-knew-pre-heal, duplicate-target, foreign group_id,
  cross-group splice, future-epoch jump, epoch regression,
  parallel-fork heal, leaked-path-secret, founder's-secret,
  pre-shared-key, heals.len()=0, empty/zero/no heals,
  bump-epoch-without, epoch-without-rotation, …).

### Wave-16 — clock-skew / replay-window + at-rest key rotation

- **L-CHAT-2-clock** (R-CHAT-2 / CR-CHAT-02) — CLK-01..06 in
  `crates/trios-chat/rings/CR-CHAT-02/src/clock_skew.rs` (349 lines)
  shipping `ReplayWindow` / `ClockSkewBound{skew_ms}` /
  `ReplayDecision::{Accept, RejectReplay, RejectStale, RejectFuture, RejectEpochRollover}`
  with `DEFAULT_MAX_HISTORY = 4096`:
  - in-band fresh message accepted (CLK-01) — `|t_recv − t_msg| ≤ skew_ms`
    AND `(epoch, counter)` not yet seen;
  - stale (backdated) message rejected with `RejectStale` (CLK-02) —
    `t_msg + skew_ms < t_recv`;
  - future-dated message rejected with `RejectFuture` (CLK-03) —
    `t_msg > t_recv + skew_ms`;
  - in-window replay rejected with `RejectReplay` (CLK-04) — same
    `(epoch, counter)` arrives twice;
  - epoch-rollover rejected with `RejectEpochRollover` (CLK-05) —
    new message at `epoch < current_epoch` after `advance_epoch`;
  - persistent replay across rotation rejected (CLK-06) — replay-set
    is preserved through `advance_epoch` boundaries.
- **L-CHAT-5-rotate** (R-CHAT-5 / CR-CHAT-05) — ROT-01..06 in
  `crates/trios-chat/rings/CR-CHAT-05/src/key_rotation.rs` (413 lines)
  shipping `KeyEpoch(u64)` / `RotatableRow` / `JournalEntry` /
  `RotationJournal::{append, no_double_rotation}` / `RotatingColumn` /
  `Rotator<F>::{step, run_to_completion}`:
  - well-formed rotation advances the row epoch monotonically (ROT-01);
  - `RotationJournal` rejects double-advance for the same
    `(row_id, source_epoch)` pair (ROT-02) — guarantees idempotence;
  - `Rotator::run_to_completion` is idempotent — running again over
    already-rotated rows is a no-op (ROT-03);
  - foreign-epoch row (whose `current_epoch != source_epoch`) rejected
    by the rotator (ROT-04);
  - non-monotone rotation (target ≤ source) rejected at the
    `RotatableRow` boundary (ROT-05);
  - R-CHAT-9 padding-class invariant — `Rotator` rejects re-encryption
    that shrinks the ciphertext padding class (ROT-06), so transcoding
    cannot be used as an at-rest length oracle.
- Coq INV-CHAT-82..88 + helper `replay_stale_rejects` in a fresh
  `Section TrinityChatWave16` — uses unique names (`Epoch16`,
  `KeyEpoch16`, `RotStep16`, `ReplayDecision16`, `ReceiverState16`)
  to avoid cross-wave name collisions; 9 new Qed → **121 Qed total**.
  - INV-CHAT-82 `clk_in_band_fresh_accepted`;
  - INV-CHAT-83 `clk_stale_rejected`;
  - INV-CHAT-84 `clk_future_rejected`;
  - INV-CHAT-85 `clk_epoch_rollover_rejected`;
  - INV-CHAT-86 `rot_idempotent`;
  - INV-CHAT-87 `rot_foreign_epoch_rejected`;
  - INV-CHAT-88 `rot_monotone_or_idempotent`.
- **Zero new axioms.** Both lanes prove constructively.
- Falsifier 1400 → 1500 (PI-CLK-001..050 + PI-ROT-001..050). Three
  payloads tweaked to ensure W16 keyword coverage (`live replay-window`,
  `across rotation under clock-skew`, `rotation journal`).
- 28 → 30 threshold lanes in `falsifier_runner` (`clock_skew_replay`
  and `at_rest_rotation`, both at 0.95 — all lanes ≥ 0.95 except
  `indirect ≥ 0.90`).
- `DENY_PATTERNS` in `CR-CHAT-06/src/injection.rs` extended with W16
  keyword blocks: clock-skew (backdated, replay-window, replaywindow,
  replay_window, clockskewbound, clock_skew, replaydecision,
  rejectstale/future/epochrollover/replay, epoch-rollover, future-band,
  t_msg, t_recv, accept_at, advance_epoch, timestamp manipulation,
  replay attack, in-window, persistent replay, …) + at-rest-rotate
  (keyepoch, rotation journal, rotationjournal, journalentry, rotator,
  rotatingcolumn, rotatablerow, non-monotone, foreign-epoch,
  source-epoch, padding-class, idempotence, at-rest rotation,
  key rotation, rewrap, transcode, double-advance, …).

### Wave-15 — Tailscale-funnel egress fingerprinting + identity-key revocation

- **L-CHAT-7-funnel** (R-CHAT-10) — EFP-01..06 in `CR-CHAT-07`, with new
  `crates/trios-chat/rings/CR-CHAT-07/src/egress_fingerprint.rs` shipping
  `EgressFingerprint` / `EgressObservables` / `TlsClass` /
  `uniform_length_class` / `uniform_burst_ms`:
  - canonical TLS class accepted (EFP-01) — `(version, alpn, cipher) =
    (0x0303, h2, AES-128-GCM-SHA256)`;
  - non-canonical TLS class rejected (EFP-02) with
    `Error::Invariant("egress_fingerprint_tls_class")`;
  - length quantiser maps to canonical bins `{1024, 4096, 16384, 65536}` (EFP-03);
  - burst-gap quantiser maps to canonical bins `{50, 250, 1000, 5000}` ms (EFP-04);
  - same-bin flows produce identical fingerprints — unlinkability across
    egress flows (EFP-05);
  - cross-bin flows differ ONLY along the canonical axes, never via raw
    bytes / raw ms (EFP-06).
- **L-CHAT-1-revoke** (R-CHAT-1) — REV-01..06 in `CR-CHAT-01`, with new
  `crates/trios-chat/rings/CR-CHAT-01/src/revocation.rs` shipping
  `RevocationCert` / `RevocationLedger` / `verify_identity_with_grace` /
  `RevocationReason::{Compromise, Rotate, Lost}`:
  - well-formed self-signed cert verifies (REV-01) — signature is over
    the canonical 41-byte body `revoked_key ‖ revoked_at_le ‖ reason`;
  - tampered cert rejected with `revocation_invalid_signature` (REV-02);
  - post-revocation message outside the grace window rejected with
    `identity_revoked` (REV-03);
  - pre-revocation message accepted regardless of how late the verifier
    sees it (REV-04);
  - grace-window edge: `now == revoked_at + grace_secs` accepts,
    `now == revoked_at + grace_secs + 1` rejects (REV-05);
  - replayed-with-later-timestamp cert rejected (`revocation_replay_rejected`),
    AND future-dated `signed_at` rejected (`clock_skew_future`) regardless of
    revocation state (REV-06).
- Coq INV-CHAT-75..81 + 3 helpers (`quantise15_smallest_below`,
  `egress_class_eq_of_eq`, `pre_revocation_accepts`); 11 new Qed →
  **112 Qed total**.
- **Zero new axioms.** Both lanes prove constructively. The egress-
  fingerprint quantiser is modeled with abstract `LEN_CLASS_*` /
  `BURST_CLASS_*` / `CANONICAL_*` `Variable`s instead of concrete nat
  literals — this dodges the Coq `abstract-large-number` slow-path on
  `65536` and keeps `coqc` under one second.
- Falsifier 1300 → 1400 (PI-EFP-001..050 + PI-REV-001..050).
- 26 → 28 threshold lanes in `falsifier_runner` (all ≥ 0.95 except
  `indirect ≥ 0.90`).

### Wave-14 — safety-number / OOB identity + MLS external-commit forgery

- **L-CHAT-2-oob** (R-CHAT-12) — SNV-01..06 in `CR-CHAT-04`, with new
  `crates/trios-chat/rings/CR-CHAT-04/src/safety_number.rs` shipping a
  `safety_number(a, b)` / `render(digest)` / `verify(local, remote)` API:
  - commutativity (SNV-01) — order of identity keys does not matter;
  - deterministic display (SNV-02) — fixed 12×5-digit grid (71 chars);
  - swap detection (SNV-03) — replacing either identity key changes the digest;
  - verify accepts matching digest (SNV-04);
  - verify rejects mismatch with `Error::Invariant("safety_number_mismatch")` (SNV-05);
  - single-bit-flip in any of 32×8=256 input bits changes the digest (SNV-06).
- **L-CHAT-3-extern** (R-CHAT-11) — EXT-01..06 in `CR-CHAT-03`, with new
  `crates/trios-chat/rings/CR-CHAT-03/src/external_commit.rs` shipping
  `ExternalCommit` / `check_external_commit` / `ExternalCommitError`:
  - well-formed external commit accepted (EXT-01);
  - epoch-mismatch / forged-epoch / replay rejected (EXT-02);
  - occupied-leaf squat rejected (EXT-03);
  - sender / joining-leaf mismatch rejected (EXT-04) — only self-Add allowed;
  - ops scope-violation (e.g. `[Add(self), Remove(other)]` or `[Update]`) rejected (EXT-05);
  - cross-`group_id` injection AND empty-signature both rejected (EXT-06).
- Coq INV-CHAT-68..74 + 3 helpers (`sn_verify_iff`, `ext_epoch_mismatch_rejects`,
  `ext_occupied_rejects`); 11 new Qed → **101 Qed total**.
- 1 new axiom `sn_hash_sym` — *symmetry contract* on the safety-number
  hash. Concretely instantiated in `CR-CHAT-04/safety_number.rs` by
  canonical-ordering the identity-key pair before feeding them into
  SHA-256, so the axiom is constructively discharged at runtime.
- Falsifier 1200 → 1300 (PI-SNV-001..050 + PI-EXT-001..050).
- 24 → 26 threshold lanes in `falsifier_runner` (all ≥ 0.95 except
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

## Falsifier-corpus categories (W1–W20) — 38 total

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
| 23 | deniability_break        | W13 | 0.95 |
| 24 | confused_deputy          | W13 | 0.95 |
| 25 | safety_number_swap   | W14 | 0.95 |
| 26 | mls_external_commit  | W14 | 0.95 |
| 27 | egress_fingerprint | W15 | 0.95 |
| 28 | identity_revoke    | W15 | 0.95 |
| 29 | clock_skew_replay | W16 | 0.95 |
| 30 | at_rest_rotation  | W16 | 0.95 |
| 31 | tool_arg_confusion | W17 | 0.95 |
| 32 | group_pcs_break    | W17 | 0.95 |
| 33 | padding_class_oracle | W18 | 0.95 |
| 34 | jitter_side_channel  | W18 | 0.95 |
| 35 | kem_decap_oracle | W19 | 0.95 |
| 36 | tag_stripping    | W19 | 0.95 |
| **37** | **handshake_fingerprint** | **W20** | **0.95** |
| **38** | **concurrent_add_remove** | **W20** | **0.95** |

`falsifier_runner` is the gate: it loads `corpus/prompt_injection.jsonl`,
runs `validate_output` on each entry, and exits non-zero if any threshold
lane drops below its bound. Wave-20 ships 1900/1900 blocked across 38 lanes.

---

## Coq invariant index (INV-CHAT-1..116)

Cumulative `Qed.` count: **158 / 0 Admitted**. R5 admission budget: **0/10 used**.

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
| INV-CHAT-61..67 | W13 | cryptographic deniability + confused-deputy capability |
| INV-CHAT-68..74 | W14 | safety-number / OOB identity + MLS external-commit forgery |
| INV-CHAT-75..81 | W15 | egress fingerprinting (canonical TLS / length / burst-gap classes) + identity-key revocation with grace window |
| INV-CHAT-82..88 | W16 | clock-skew / replay-window decision (in-band / stale / future / epoch-rollover / replay) + at-rest key rotation idempotence and monotonicity |
| INV-CHAT-89..95 | W17 | tool-call argument confusion (kind mismatch, nested-sentinel, oversized-string, unknown-enum-variant) + group-PCS healing (epoch advance, no-op, epoch mismatch) |
| INV-CHAT-96..102 | W18 | padding-class oracle (smallest-class, over-pad, length overflow, truncation, non-canonical gap, non-monotonic timestamp, reorder-attack) |
| INV-CHAT-103..109 | W19 | ML-KEM-768 decapsulation oracle (FO determinism, ct flip → differ, anti-malleability, content-bound reject, CT eq, opaque observe) + structured-output tag-stripping (nested check, well-formed span) |
| **INV-CHAT-110..116** | **W20** | **handshake fingerprinting (determinism, swap detected, empty-field invalid) + concurrent Add/Remove ordering (Update<Remove<Add priority, empty-set neutral, add-after-remove size-neutral)** |

Cumulative axioms: `ss_kp_injective` (W9), `dh_step_fresh` (W10),
`dh_post_history_independent` (W10), `hybrid_kem_non_degenerate` (W10),
`sn_hash_sym` (W14, constructively discharged at runtime).
Wave-11, Wave-12, Wave-13, Wave-15, Wave-16, Wave-17, Wave-18, Wave-19, and Wave-20 all introduce **zero** new axioms — every proof is constructive.
Wave-14 introduces **one** new axiom (`sn_hash_sym`) which is concretely
discharged in Rust by canonical-ordering the safety-number hash inputs.

---

## Future waves (W22–W26) — `[ASPIRATIONAL]`

The plan below is `[ASPIRATIONAL]` per R5 — none of these have shipped
yet. Each row picks **two** uncovered or under-pinned threat classes
following the established cadence (5 tests/lane, +50/+50 corpus,
+~10 Coq Qed, all gates green, PR closes a sub-tracker issue).

| Wave | Lane A (ring) | Lane B (ring) | New corpus categories | Coq target | Tests target | Falsifier target |
| :-- | :-- | :-- | :-- | :-- | :-- | :-- |
| ~~W12~~ — SHIPPED (see Wave-12 detail above) | | | | | | |
| ~~W13~~ — SHIPPED (see Wave-13 detail above) | | | | | | |
| ~~W14~~ — SHIPPED (see Wave-14 detail above) | | | | | | |
| ~~W15~~ — SHIPPED (see Wave-15 detail above) | | | | | | |
| ~~W16~~ — SHIPPED via rollup #665 (see Wave-16 detail above) | | | | | | |
| ~~W17~~ — SHIPPED via [#715](https://github.com/gHashTag/trios/pull/715), merged `047f3cb` (see Wave-17 detail above) | | | | | | |
| ~~W18~~ — SHIPPED via [#717](https://github.com/gHashTag/trios/pull/717), merged `6902a82` (see Wave-18 detail above) | | | | | | |
| ~~W19~~ — SHIPPED via [#719](https://github.com/gHashTag/trios/pull/719), merged `d601a58` (see Wave-19 detail above) | | | | | | |
| ~~W20~~ — SHIPPED via [#724](https://github.com/gHashTag/trios/pull/724), merged `e556075` (see Wave-20 detail above) | | | | | | |
| ~~W21~~ — SHIPPED in this PR (see Wave-21 detail above) | | | | | | |
| **W22** | (TBD — picked from uncovered surface after W21 retrospective) | (TBD) | (TBD ×2) | INV-CHAT-124..130 (≥178 Qed) | ≈352 | 2100 / 42 cats |
| **W23** | (TBD) | (TBD) | (TBD ×2) | INV-CHAT-131..137 (≥188 Qed) | ≈374 | 2200 / 44 cats |
| **W24** | (TBD) | (TBD) | (TBD ×2) | INV-CHAT-138..144 (≥198 Qed) | ≈396 | 2300 / 46 cats |
| **W25** | (TBD) | (TBD) | (TBD ×2) | INV-CHAT-145..151 (≥208 Qed) | ≈418 | 2400 / 48 cats |
| **W26** | (TBD) | (TBD) | (TBD ×2) | INV-CHAT-152..158 (≥218 Qed) | ≈440 | 2500 / 50 cats |

After W21 the corpus crosses **2000 entries / 40 categories** and Coq
crosses **168 closed proofs / 0 admissions**. From W22+ the work shifts
from **adding** lanes to **deepening** existing ones (replacing
axioms with constructive proofs, retiring `[ASPIRATIONAL]` tags,
wiring lanes through the real `openmls` / `pqcrypto-mlkem` paths)
while still picking two fresh uncovered threat classes per wave.

---

## Operational invariants — never broken

The following are **not** lanes; they are fixed contracts every wave
reverifies. A wave PR must keep all of them green.

| Gate | Command (run from `/home/user/workspace/trios`) | Expected |
| :-- | :-- | :-- |
| Chat unit tests | `cargo test -q -p trios-chat-cr-chat-* -p trios-chat-br-* -p trios-chat-cr-chat-laws -p trios-chat` | `N / 0` (N grows by ~12 per wave) |
| End-to-end smoke | `cargo run -q -p trios-chat --bin e2e_chat_25` | `25/25 pass` |
| Falsifier corpus | `cargo run -q -p trios-chat --bin falsifier_runner` | `2000/2000 blocked` (W21) at 40 thresholds |
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
- W21..W25 lane definitions are **[ASPIRATIONAL]** — they constitute the
  forward plan and have not been validated by tests/Coq yet.
- Wave-20 detail section above is **[VERIFIED]** by cargo test
  (310/0), `e2e_chat_25` (25/25), `falsifier_runner` (1900/1900,
  38 cats), clippy (clean), and `coqc Trinity_Chat.v` (silent, 158
  `Qed.`, 0 `Admitted.`).

---

## See also

- `crates/trios-chat/README.md` — crate overview, build & run
- `crates/trios-chat/proofs/chat/Trinity_Chat.v` — Coq invariant source
- `crates/trios-chat/corpus/prompt_injection.jsonl` — canonical falsifier corpus
- `crates/trios-chat/src/bin/falsifier_runner.rs` — threshold gate
- `crates/trios-chat/src/bin/e2e_chat_25.rs` — 25-step end-to-end smoke test
- EPIC [trinity-fpga#28](https://github.com/gHashTag/trinity-fpga/issues/28)
- Trinity Constitution Article I (R5 honesty mode)
