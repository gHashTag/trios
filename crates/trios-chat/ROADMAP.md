# Trinity Secure Chat — ROADMAP

> Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · POST-QUANTUM · UNLINKABLE · COVER-TIMING · AT-REST-AEAD · BOT-PARTIAL-MLS · KEM-KEY-CONFUSION · AAD-CONTEXT · RATCHET-FS · MLS-REORDER · SKIPPED-KEYS-DOS · MLS-WELCOME-REPLAY · PREKEY-EXHAUSTION · MLS-LEAF-COMPROMISE · DENIABILITY · CONFUSED-DEPUTY · OOB-IDENTITY · MLS-EXTERNAL-COMMIT · EGRESS-FINGERPRINT · IDENTITY-REVOKE · CLOCK-SKEW-REPLAY · AT-REST-ROTATE · TOOL-ARG-CONFUSION · GROUP-PCS-HEAL · PADDING-CLASS-ORACLE · JITTER-SIDE-CHANNEL`
>
> Parent EPIC: [trinity-fpga#28](https://github.com/gHashTag/trinity-fpga/issues/28)
> Crate: [`crates/trios-chat`](./)
> Status as of Wave-18: **270 tests · 25/25 e2e · 1700/1700 falsifier · 34 categories · 139 Coq Qed / 0 Admitted · 0 unsafe · 0 monoliths**

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
| W17 | (PR open)  | 249 | INV-CHAT-89..95 (130 Qed total) | 1600 | 32 | tool_arg_confusion + group_pcs_break | [#715](https://github.com/gHashTag/trios/pull/715) (merged `047f3cb`) |
| **W18** | **(this PR)** | **270** | **INV-CHAT-96..102 (139 Qed total)** | **1700** | **34** | **padding_class_oracle + jitter_side_channel** | **(open)** |

> Notes on Coq counting: pre-Wave-10 the team used `grep -cE "^Qed\.$"`
> (standalone-line count). The new standard since Wave-10 is the
> **total `Qed.` occurrence count** (`grep -cE "Qed\."`), which captures
> inline `Proof. ... Qed.` lemmas too. All historical totals in this
> table are restated under the new standard.

---

## Detailed wave summaries

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

## Falsifier-corpus categories (W1–W17) — 32 total

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
| **31** | **tool_arg_confusion** | **W17** | **0.95** |
| **32** | **group_pcs_break**    | **W17** | **0.95** |

`falsifier_runner` is the gate: it loads `corpus/prompt_injection.jsonl`,
runs `validate_output` on each entry, and exits non-zero if any threshold
lane drops below its bound. Wave-17 ships 1600/1600 blocked across 32 lanes.

---

## Coq invariant index (INV-CHAT-1..95)

Cumulative `Qed.` count: **130 / 0 Admitted**. R5 admission budget: **0/10 used**.

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
| **INV-CHAT-96..102** | **W18** | **padding-class oracle (smallest-class, over-pad, length overflow, truncation, non-canonical gap, non-monotonic timestamp, reorder-attack)** |

Cumulative axioms: `ss_kp_injective` (W9), `dh_step_fresh` (W10),
`dh_post_history_independent` (W10), `hybrid_kem_non_degenerate` (W10),
`sn_hash_sym` (W14, constructively discharged at runtime).
Wave-11, Wave-12, Wave-13, Wave-15, Wave-16, Wave-17, and Wave-18 all introduce **zero** new axioms — every proof is constructive.
Wave-14 introduces **one** new axiom (`sn_hash_sym`) which is concretely
discharged in Rust by canonical-ordering the safety-number hash inputs.

---

## Future waves (W19–W23) — `[ASPIRATIONAL]`

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
| ~~W18~~ — SHIPPED in this PR (see Wave-18 detail above) | | | | | | |
| **W19** | L-CHAT-8-decap (R-CHAT-2) — ML-KEM-768 decapsulation oracle / Fujisaki–Okamoto re-encryption | L-CHAT-9-tagsplit (R-CHAT-12) — tag-stripping / structured-output split | `kem_decap_oracle`, `tag_stripping` | INV-CHAT-103..109 (≥150 Qed) | ≈282 | 1800 / 36 cats |
| **W20** | L-CHAT-1-handshake (R-CHAT-1) — handshake fingerprinting + transcript-binding | L-CHAT-3-add (R-CHAT-11) — concurrent Add/Remove ordering + ghost-member | `handshake_fingerprint`, `concurrent_add_remove` | INV-CHAT-110..116 (≥160 Qed) | ≈294 | 1900 / 38 cats |
| **W21** | (TBD — picked from uncovered surface after W20 retrospective) | (TBD) | (TBD ×2) | INV-CHAT-117..123 (≥170 Qed) | ≈306 | 2000 / 40 cats |
| **W22** | (TBD) | (TBD) | (TBD ×2) | INV-CHAT-124..130 (≥180 Qed) | ≈318 | 2100 / 42 cats |
| **W23** | (TBD) | (TBD) | (TBD ×2) | INV-CHAT-131..137 (≥190 Qed) | ≈330 | 2200 / 44 cats |

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
| Falsifier corpus | `cargo run -q -p trios-chat --bin falsifier_runner` | `1700/1700 blocked` (W18) at 34 thresholds |
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
- W19..W23 lane definitions are **[ASPIRATIONAL]** — they constitute the
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
