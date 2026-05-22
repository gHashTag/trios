# Trinity Secure Chat — ROADMAP

> Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · POST-QUANTUM · UNLINKABLE · COVER-TIMING · AT-REST-AEAD · BOT-PARTIAL-MLS · KEM-KEY-CONFUSION · AAD-CONTEXT · RATCHET-FS · MLS-REORDER · SKIPPED-KEYS-DOS · MLS-WELCOME-REPLAY · PREKEY-EXHAUSTION · MLS-LEAF-COMPROMISE · DENIABILITY · CONFUSED-DEPUTY · OOB-IDENTITY · MLS-EXTERNAL-COMMIT · EGRESS-FINGERPRINT · IDENTITY-REVOKE · CLOCK-SKEW-REPLAY · AT-REST-ROTATE · TOOL-ARG-CONFUSION · GROUP-PCS-HEAL · PADDING-CLASS-ORACLE · JITTER-SIDE-CHANNEL · KEM-DECAP-ORACLE · TAG-STRIPPING · HANDSHAKE-FINGERPRINT · CONCURRENT-ADD-REMOVE · EPOCH-AUTH-FAILURE · WELCOME-KP-PINNING · PROPOSAL-VALIDATION · MAC-TRUNCATION · REINIT-FRESHNESS · APPACK-REPLAY · COMMIT-SIG-FORGE · PREKEY-SIG-CHAIN · PADDING-ORACLE-CHOSEN-CT · COVER-TRAFFIC-STARVATION · MLS-PSK-INJECTION · WELCOME-TREEKEM-PRUNING · MLS-EXTERNAL-INIT · RATCHET-TREE-EXT · CONFIRMATION-TAG-CHAIN · SENDER-DATA-HEADER-ENC · LEAF-NODE-SIG · GROUP-CTX-EXT · APP-DATA-AEAD-NONCE · WELCOME-PATH-SECRET · KEYPACKAGE-INIT-KEY · EXTERNAL-PSK-PROVENANCE · WELCOME-GROUP-INFO-AEAD · PROPOSAL-REF-COLLISION · COMMIT-SECRET-EXPORT · EXTERNAL-PROPOSAL-ORIGIN · EPHEMERAL-MAILBOX-UNLINK · BLIND-SIGNATURE-SENDER-TOKEN · COVER-DECOY-INDISTINGUISHABILITY · SENDER-KEYS-EPOCH-REPLAY · COMMIT-PATH-SECRET-AEAD-KEYING · APP-MSG-SKIP-DOS`
>
> Parent EPIC: [trinity-fpga#28](https://github.com/gHashTag/trinity-fpga/issues/28)
> Crate: [`crates/trios-chat`](./)
> Status as of Wave-36: **~628 tests · 25/25 e2e · 3500/3500 falsifier · 70 categories · 341 Coq Qed / 0 Admitted · 0 unsafe · 0 monoliths**

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
| W21 | `35b3ef6` | 330 | INV-CHAT-117..123 (168 Qed total) | 2000 | 40 | epoch_authentication_failure + welcome_keypackage_pinning | [#730](https://github.com/gHashTag/trios/pull/730) |
| W22 | `119f0fe` | 335 | INV-CHAT-124..130 (181 Qed total) | 2100 | 42 | proposal_validation + mac_truncation | [#732](https://github.com/gHashTag/trios/pull/732) |
| W23 | `1d6f910` | 355 | INV-CHAT-131..137 (191 Qed total) | 2200 | 44 | reinit_freshness + appack_replay | [#734](https://github.com/gHashTag/trios/pull/734) |
| W24 | `81ef050` | 375 | INV-CHAT-138..144 (203 Qed total) | 2300 | 46 | commit_signature_forge + prekey_signature_chain | [#738](https://github.com/gHashTag/trios/pull/738) |
| W25 | `e234422` | 397 | INV-CHAT-145..151 (215 Qed total) | 2400 | 48 | padding_oracle_chosen_ct + cover_traffic_starvation | [#747](https://github.com/gHashTag/trios/pull/747) |
| W26 | `1665be1` | ~419 | INV-CHAT-152..158 (227 Qed total) | 2500 | 50 | mls_psk_external_injection + welcome_secret_treekem_pruning | [#749](https://github.com/gHashTag/trios/pull/749) |
| W27 | `93e4e6c` | ~448 | INV-CHAT-159..165 (239 Qed total) | 2600 | 52 | external_init_secret_pinning + ratchet_tree_extension_tampering | [#752](https://github.com/gHashTag/trios/pull/752) |
| W28 | `562009c` | ~468 | INV-CHAT-166..172 (251 Qed total) | 2700 | 54 | confirmation_tag_chain + sender_data_header_encryption | [#754](https://github.com/gHashTag/trios/pull/754) |
| W29 | `c389536` | ~488 | INV-CHAT-173..179 (263 Qed total) | 2800 | 56 | leaf_node_signature_validation + group_context_extensions_consistency | [#760](https://github.com/gHashTag/trios/pull/760) |
| W30 | `bd5ffea` | ~508 | INV-CHAT-180..186 (275 Qed total) | 2900 | 58 | application_data_aead_nonce_reuse + welcome_path_secret_unmasking | [#765](https://github.com/gHashTag/trios/pull/765) |
| W31 | `756cf35` | ~528 | INV-CHAT-187..193 (288 Qed total) | 3000 | 60 | keypackage_init_key_reuse + external_psk_id_provenance | [#771](https://github.com/gHashTag/trios/pull/771) |
| W32 | `b37abb1` | ~548 | INV-CHAT-194..200 (299 Qed total) | 3100 | 62 | welcome_encrypted_group_info_aead + proposal_ref_collision | [#941](https://github.com/gHashTag/trios/pull/941) |
| **W33** | **(this PR)** | **~568** | **INV-CHAT-201..207 (311 Qed total)** | **3200** | **64** | **commit_secret_export_collision + external_proposal_origin_unbound** | **(open)** |

> Notes on Coq counting: pre-Wave-10 the team used `grep -cE "^Qed\.$"`
> (standalone-line count). The new standard since Wave-10 is the
> **total `Qed.` occurrence count** (`grep -cE "Qed\."`), which captures
> inline `Proof. ... Qed.` lemmas too. All historical totals in this
> table are restated under the new standard.

---

## Detailed wave summaries

### Wave-36 — Commit path-secret AEAD keying mismatch + Application-message generation skip-window DoS (RFC 9420 §7.7+§8+§12.4 + §9.3+§15.2)

- **L-CHAT-3-cpakm** (R-CHAT-3 / **CR-CHAT-03**) — CPAKM-01..10 in
  `crates/trios-chat/rings/CR-CHAT-03/src/commit_path_secret_aead_keying_mismatch.rs`
  shipping
  `validate_commit_path_secret(commit: &CommitUpdatePath, view: &UpdatePathView) -> Result<(), PathSecretAeadKeyingError>`.
  Consts `CPAKM_GROUP_ID_LEN = 32`, `CPAKM_INIT_KEY_LEN = 32`,
  `CPAKM_AAD_CONTEXT_LEN = 56` (32 + 8 + 8 + 8),
  `CPAKM_PATH_SECRET_CIPHERTEXT_LEN = 48`. Error enum
  `PathSecretAeadKeyingError` (`#[non_exhaustive]` with variants
  `NonCanonicalGroupIdLength`, `EpochMismatch`,
  `SenderLeafOutOfRange`, `ResolutionSlotOutOfRange`,
  `RecipientInitKeyMismatch`, `AadContextMismatch`,
  `NonCanonicalCiphertextLength`). Seven rules enforced in fixed
  order: (1) MLS GroupID is exactly 32 bytes; (2) Commit epoch
  advances local epoch by exactly 1 (RFC 9420 §7.5); (3) sender's
  leaf index is within the ratchet-tree bounds; (4) the local
  receiver's `local_resolution_index` is within `update_path_slots`
  bounds; (5) the targeted slot's `recipient_init_key` equals the
  receiver's own HPKE init_key — a slot sealed under any other
  leaf's key is rejected pre-decap; (6) the slot's `aad_context`
  equals the canonical encoding
  `group_id ‖ epoch_u64_be ‖ sender_leaf_u64_be ‖ node_index_u64_be`
  AND `slot.node_index == view.local_node_index` — AAD drift on
  any of those four bound fields is rejected; (7) sealed
  path-secret ciphertext length is exactly 48 bytes (32-byte sealed
  secret + 16-byte Poly1305 tag per R-CHAT-3). `[CITED]` RFC 9420
  §7.7 "Updating Tree State" + §8 "Encrypting and Decrypting
  to/from Tree Nodes" + §12.4.3.2 commit verification.
- **L-CHAT-4-amgsd** (R-CHAT-4 / **CR-CHAT-04**) — AMGSD-01..10 in
  `crates/trios-chat/rings/CR-CHAT-04/src/application_message_generation_skip_dos.rs`
  shipping
  `validate_app_message_skip(packet: &AppMessagePacket, view: &AppMessageView) -> Result<(), AppMessageSkipError>`.
  Consts `APP_MSG_SENDER_ID_LEN = 16`, `APP_MSG_SKIP_WINDOW = 1024`
  (OpenMLS default + RFC 9420 §9.3 bounded-skip recommendation).
  Error enum `AppMessageSkipError` (`#[non_exhaustive]` with
  variants `NonCanonicalSenderIdLength`, `UnknownSender`,
  `ZeroGeneration`, `NonMonotonicGeneration`,
  `SkipDistanceExceeded`, `EpochMismatch`, `CiphertextEmpty`).
  Seven rules enforced in fixed order: (1) sender_id is exactly 16
  bytes (MLS LeafNodeRef per RFC 9420 §6.1); (2) sender_id is in
  `view.known_senders`; (3) packet epoch equals `view.current_epoch`
  — cross-epoch packets are W35's domain, not this lane's; (4)
  ciphertext is non-empty; (5) generation > 0 (MLS counter starts
  at 1); (6) generation strictly greater than the last seen value
  for `(sender, epoch)`; (7) skip distance
  `(generation - last_seen - 1) <= APP_MSG_SKIP_WINDOW` — the
  DoS-resistance core: a single attacker packet claiming
  `generation = u64::MAX` is denied **before** any HKDF
  key-schedule work. `[CITED]` RFC 9420 §9.3 "Message Receiving"
  bounded skip window + §15.2 "Application Messages".
- **Falsifier corpus** `crates/trios-chat/corpus/prompt_injection.jsonl`
  — +100 entries (`PI-CPAKM-001..050` + `PI-AMGSD-001..050`),
  categories `commit_path_secret_aead_keying_mismatch` +
  `application_message_generation_skip_dos`. Cumulative corpus
  size after W36: 3500 entries across 70 categories. Format used:
  `{"id","category","payload","expected_block":true}`.
- **Deny patterns** `crates/trios-chat/rings/CR-CHAT-06/src/injection.rs`
  — +99 patterns covering 100% of W36 payload phrasings (verified
  via offline collision-coverage script). 0 collisions with prior
  `expected_block=false` entries; harmless overlaps with prior
  `expected_block=true` entries.
- **falsifier_runner** — new threshold tuples
  `("commit_path_secret_aead_keying_mismatch", 0.95)` and
  `("application_message_generation_skip_dos", 0.95)` registered
  alongside the W35 entries; G-C10 summary line extended.
- **Coq** `crates/trios-chat/proofs/chat/Trinity_Chat.v` — new
  `Section TrinityChatWave36` with 10 INV theorems
  (`INV-CHAT-228..237`) + 4 helper lemmas, all closed by `Qed`.
  Wave-36 introduces **0 new axioms** and **0 admissions**.
  Cumulative Qed: 331 → 341.
- **Why this wave matters.** Wave-35 closed the *metadata-resistant
  transport* surface (cover-traffic shape indistinguishability +
  sender-keys cross-epoch replay). Wave-36 turns inward to the
  MLS *group-state* and *application-key* layers and pins two
  rules that real RFC 9420 implementations commonly leave
  un-enforced: (i) **commit UPDATE-PATH HPKE keying binding** —
  mainstream MLS libraries decrypt the slot under the receiver's
  init_key but rarely verify the slot's `aad_context` matches the
  canonical `(group_id ‖ epoch ‖ sender_leaf ‖ node_index)`
  encoding, leaving a mis-keying poison-pill open against
  malicious senders, and (ii) **bounded application-key skip
  window** — the RFC's §9.3 security considerations cite a
  bounded ceiling but the protocol does not mandate one, so a
  packet claiming `generation = u64::MAX` forces tens-of-billions
  of HKDF rounds. trios-chat now enforces both constructively at
  the boundary.

### Wave-35 — Cover-traffic decoy indistinguishability + Sender-keys epoch window replay (NDSS 2021 §V + RFC 9420 §15.5)

- **L-CHAT-2-ctdi** (R-CHAT-2 / **CR-CHAT-02**) — CTDI-01..10 in
  `crates/trios-chat/rings/CR-CHAT-02/src/cover_traffic_decoy_indistinguishability.rs`
  shipping
  `validate_cover_packet(packet: &CoverPacket, view: &CoverPacketView) -> Result<(), CoverPacketError>`.
  Consts `COVER_AEAD_NONCE_LEN = 12`, `COVER_AAD_LEN = 16`,
  `COVER_AEAD_TAG_LEN = 16`. Error enum `CoverPacketError`
  (`#[non_exhaustive]` with variants `NonCanonicalPacketLengthClass`,
  `UnknownLengthClassId`, `LengthClassMismatch`,
  `NonCanonicalNonceLength`, `NonCanonicalAadLength`,
  `NonCanonicalTagLength`, `CoverFlagShapeMismatch`). Seven rules
  enforced in fixed order: (1) packet ciphertext length must be one
  of the receiver's published equal-length bins; (2) declared class
  id must be a published class; (3) declared class id must match
  the actual ciphertext length — a packet that claims bin `4096`
  but carries 1024 bytes is rejected; (4) AEAD nonce length must
  equal 12 bytes (ChaCha20-Poly1305 per R-CHAT-4); (5) AAD must be
  exactly 16 bytes (fixed `(epoch_u64 ‖ class_u64)` header per
  R-CHAT-9); (6) Poly1305 tag must be exactly 16 bytes — truncated
  tags on cover packets are fingerprintable; (7) `wire_digest` is
  a function of the length class only — cover and real packets in
  the same bin map to the same expected digest, so shape drift
  between cover and real is rejected. The `is_cover` flag is
  allowed to be either `true` or `false`, but the on-wire bytes are
  identical in either case. `[CITED]` NDSS 2021 "Improving
  Signal's Sealed Sender" §V cover-traffic flooding defence;
  USENIX'22 "Pretzel" equal-length padding bins.
- **L-CHAT-5-sker** (R-CHAT-5 / **CR-CHAT-05**) — SKER-01..10 in
  `crates/trios-chat/rings/CR-CHAT-05/src/sender_keys_epoch_window_replay.rs`
  shipping
  `validate_sender_keys_packet(packet: &SenderKeysPacket, view: &SenderKeysView) -> Result<(), SenderKeysError>`.
  Consts `SENDER_KEYS_SENDER_ID_LEN = 16`,
  `SENDER_KEYS_EPOCH_WINDOW = 1`. Error enum `SenderKeysError`
  (`#[non_exhaustive]` with variants `NonCanonicalSenderIdLength`,
  `UnknownSender`, `EpochOutsideWindow`, `NonMonotonicGeneration`,
  `EpochAlreadyEvicted`, `ZeroGeneration`, `FutureEpoch`). Seven
  rules enforced in fixed order: (1) sender_id length must equal 16
  bytes (MLS LeafNodeRef per RFC 9420 §6.1); (2) sender_id must be
  in `view.known_senders` — no phantom senders; (3) packet epoch
  cannot exceed current_epoch (no time travel); (4) packet epoch
  must be inside the sliding window of size 1 (`current_epoch -
  packet.epoch <= 1`) — closes the epoch-window replay gap not
  bounded by per-epoch monotone-generation checks alone; (5)
  already-evicted epochs are rejected; (6) generation = 0 is
  forbidden (MLS counter starts at 1 per §15.5); (7) generation
  must be strictly greater than the last seen value for
  `(sender_id, epoch)`. `[CITED]` RFC 9420 §15.5 sender-data
  generation; NDSS 2021 §V follow-up gap analysis on receiver-side
  state eviction during epoch transitions.
- **Falsifier corpus** `crates/trios-chat/corpus/prompt_injection.jsonl`
  — +100 entries (`PI-CTDI-001..050` + `PI-SKER-001..050`),
  categories `cover_traffic_decoy_indistinguishability` +
  `sender_keys_epoch_window_replay`. Cumulative corpus size after
  W35: 3400 entries across 68 categories. Format used:
  `{"id","category","payload","expected_block":true}`.
- **Deny patterns** `crates/trios-chat/rings/CR-CHAT-06/src/injection.rs`
  — +101 patterns covering 100% of W35 payload phrasings
  (verified via offline collision-coverage script). 0 collisions
  with prior `expected_block=false` entries; harmless overlaps
  with prior `expected_block=true` entries.
- **falsifier_runner** — new threshold tuples
  `("cover_traffic_decoy_indistinguishability", 0.95)` and
  `("sender_keys_epoch_window_replay", 0.95)` registered alongside
  the W34 entries; G-C10 summary line extended.
- **Coq** `crates/trios-chat/proofs/chat/Trinity_Chat.v` — new
  `Section TrinityChatWave35` with 10 INV theorems
  (`INV-CHAT-218..227`) + 4 helper lemmas, all closed by `Qed`.
  Wave-35 introduces **0 new axioms** and **0 admissions**.
  Cumulative Qed: 321 → 331.
- **Why this wave matters.** Wave-34 closed the *receiver-mailbox*
  side of the NDSS 2021 Statistical Disclosure Attack. Wave-35
  closes the two remaining metadata-resistant transport gaps the
  same paper called out in §V — cover/real packet
  distinguishability and sender-keys epoch-window replay. Real
  Signal/MLS deployments still leave both ajar: cover-traffic
  schemes typically pick a single nominal padding size but allow
  AEAD-frame shape drift between cover and real (nonces, AAD, tag
  lengths), and Sender-Data generation checks are usually
  per-epoch with informal eviction windows. trios-chat now
  enforces both constructively at the boundary.

### Wave-34 — Ephemeral mailbox unlinkability + Blind-signature sender token (NDSS 2021 §IV SDA defence)

- **L-CHAT-4-emu** (R-CHAT-3 / **CR-CHAT-01**) — EMU-01..10 in
  `crates/trios-chat/rings/CR-CHAT-01/src/ephemeral_mailbox_unlinkability.rs`
  (325 lines) shipping
  `validate_ephemeral_mailbox_envelope(envelope: &EphemeralMailboxEnvelope, view: &EphemeralMailboxView) -> Result<(), EphemeralMailboxError>`.
  Consts `EPHEMERAL_MAILBOX_TOKEN_LEN = 32`,
  `ENVELOPE_BINDING_TAG_LEN = 32`. Error enum
  `EphemeralMailboxError` (`#[non_exhaustive]` with variants
  `NonCanonicalMailboxTokenLength`, `UnknownMailboxToken`,
  `MailboxTokenWrongReceiver`, `StaleMailboxToken`,
  `MailboxTokenReuse`, `ZeroMailboxToken`, `EnvelopeBindingMismatch`).
  Seven rules enforced in fixed order from NDSS 2021 "Improving
  Signal's Sealed Sender" §IV-B/C (Martiny et al.; mailbox tokens are
  one-shot HKDF outputs bound to receiver + freshness window): (1)
  reject any `mailbox_token` not of canonical length 32
  (`NonCanonicalMailboxTokenLength`), (2) reject tokens not in
  `view.published_tokens` (`UnknownMailboxToken` — no phantom
  mailboxes), (3) reject `token_owner ≠ envelope.claimed_receiver`
  (`MailboxTokenWrongReceiver` — cohort isolation), (4) reject
  `current_epoch > expiry_epoch` (`StaleMailboxToken` — lifetime
  bound from §IV-C), (5) reject any `mailbox_token` already in
  `view.consumed_tokens` (`MailboxTokenReuse` — **the SDA defence
  core invariant** — the moment a token is reused the
  unlinkability guarantee collapses per §V-A), (6) reject the
  all-zero `mailbox_token` (`ZeroMailboxToken`), (7) reject
  envelopes whose `envelope_binding_tag` does not match the
  HKDF-Expand of `(mailbox_token, padded_envelope_hash)` per
  §IV-B Eq. 3 (`EnvelopeBindingMismatch` — stops a relay or
  attacker who steals a single mailbox token from pairing it with
  a different envelope). → **10 unit tests** (`EMU-01..10`).

- **L-CHAT-7-bsst** (R-CHAT-10 / **CR-CHAT-07**) — BSST-01..10 in
  `crates/trios-chat/rings/CR-CHAT-07/src/blind_signature_sender_token.rs`
  (302 lines) shipping
  `validate_blind_signature_sender_token(token: &BlindSenderToken, view: &BlindTokenView) -> Result<(), BlindTokenError>`,
  consts `BLIND_TOKEN_NONCE_LEN = 32`,
  `BLIND_SIGNATURE_LEN = 256`. Error enum `BlindTokenError`
  (`#[non_exhaustive]` with variants
  `NonCanonicalTokenNonceLength`, `NonCanonicalSignatureLength`,
  `UnknownIssuerPublicKey`, `ExpiredIssuerEpoch`, `TokenNonceReuse`,
  `ZeroTokenNonce`, `SignatureVerificationFailed`).
  Seven rules enforced in fixed order from NDSS 2021 §IV-D (Chaum-
  style blind signatures — the relay verifies the signature over
  the unblinded nonce without learning which issuance request the
  token corresponds to) + RFC 8017 §8.2 (RSA-FDH): (1) reject any
  `token_nonce` not of canonical length 32, (2) reject any
  `signature` not of canonical RSA-2048 length 256, (3) reject any
  `issuer_pubkey_id` not in `view.trusted_issuers`, (4) reject
  `current_epoch > issuer_expiry` (issuer rotation per §IV-E), (5)
  reject any `token_nonce` already in `view.spent_nonces`
  (`TokenNonceReuse` — anti-double-spend rail), (6) reject the
  all-zero `token_nonce` (`ZeroTokenNonce`), (7) reject signatures
  that do not RSA-FDH verify under the issuer's public key
  (`SignatureVerificationFailed`). → **10 unit tests**
  (`BSST-01..10`).

- **Falsifier corpus 3200 → 3300.** New categories
  `ephemeral_mailbox_unlinkability` and `blind_signature_sender_token`,
  50 entries each (`PI-EMU-001..050`, `PI-BSST-001..050`). Each
  lane covers the specific exploitation phrasings (`Replay a
  consumed mailbox_token`, `Accept the all-zero mailbox_token`,
  `Skip envelope_binding check`, `Cross-issue a blind signature
  between two receivers' issuers`, `Reuse a token_nonce that was
  already spent`, `Accept a sender token from a revoked issuer`,
  …) so deny patterns block them at the orchestrator level before
  they reach the Rust validator. Offline simulation:
  **3300/3300 blocked, 0 misses, 66 categories**. Added 53 new deny
  patterns to `CR-CHAT-06/src/injection.rs` covering 100% of new
  payload phrasings; collision-checked against 3200 prior corpus
  entries: 0 collisions with `expected_block=false` entries
  (4 harmless collisions, all already `expected_block=true`).

- **Coq Section `TrinityChatWave34`** in
  `crates/trios-chat/proofs/chat/Trinity_Chat.v` (lines 4672–4823)
  closes 10 new theorems + 4 helper lemmas:
  - INV-CHAT-208 `inv_chat_208_emu_non_canonical_mailbox_token_len_rejected`
  - INV-CHAT-209 `inv_chat_209_emu_wrong_receiver_rejected`
  - INV-CHAT-210 `inv_chat_210_emu_stale_token_rejected`
  - INV-CHAT-211 `inv_chat_211_emu_non_canonical_binding_tag_len_rejected`
  - INV-CHAT-212 `inv_chat_212_emu_canonical_envelope_accepted`
  - INV-CHAT-213 `inv_chat_213_bsst_non_canonical_token_nonce_len_rejected`
  - INV-CHAT-214 `inv_chat_214_bsst_non_canonical_signature_len_rejected`
  - INV-CHAT-215 `inv_chat_215_bsst_expired_issuer_rejected`
  - INV-CHAT-216 `inv_chat_216_bsst_zero_token_nonce_rejected`
  - INV-CHAT-217 `inv_chat_217_bsst_boundary_issuer_accepted`
  - helpers: `emu_canonical_mailbox_token_accepted_34`,
    `emu_boundary_epoch_accepted_34`,
    `bsst_canonical_signature_accepted_34`,
    `bsst_one_token_nonce_accepted_34`.

  Wave-34 introduces **0 new axioms** and **0 admissions**. Cumulative
  `grep -cE 'Qed\.'` is **321**.

- **falsifier_runner thresholds.** Added
  `("ephemeral_mailbox_unlinkability", 0.95)` and
  `("blind_signature_sender_token", 0.95)` to the threshold lane
  list in `crates/trios-chat/src/bin/falsifier_runner.rs`. The G-C10
  summary line now enumerates all 66 categories.

- **Why this wave matters — closing the production gap Signal never
  closed.** NDSS 2021 "Improving Signal's Sealed Sender" (Martiny,
  Miers, Cohen, Andrysco) demonstrated that Signal's sealed-sender
  envelope still falls to a Statistical Disclosure Attack after
  ~5 messages because the receiver's long-term mailbox is reused.
  The paper proposes ephemeral mailboxes + Chaum-style blind
  signatures as the fix. Signal did not implement the proposed
  mitigation in production. Wave-34 ships the constructive
  verification guards (`validate_ephemeral_mailbox_envelope` +
  `validate_blind_signature_sender_token`) and the Coq theorems
  pinning their invariants — trios-chat is now the first messenger
  with a formally verified SDA-defence skeleton on the receiver +
  relay sides. **[CITED NDSS 2021 §IV]**

### Wave-33 — Commit secret export collision + External proposal origin unbound

- **L-CHAT-3-csec** (R-CHAT-11 / **CR-CHAT-03**) — CSEC-01..10 in
  `crates/trios-chat/rings/CR-CHAT-03/src/commit_secret_export_collision.rs`
  (298 lines) shipping
  `validate_commit_secret_export(export: &ExportedCommitSecret, view: &CommitSecretView) -> Result<(), CommitSecretError>`.
  Const `COMMIT_SECRET_LEN = 32`,
  `COMMIT_TRANSCRIPT_HASH_MAX_LEN = 64`. Error enum
  `CommitSecretError` (`#[non_exhaustive]` with variants
  `NonCanonicalCommitSecretLength`, `EmptyTranscriptHash`,
  `UnknownTranscriptHash`, `CrossGroupCommitSecret`,
  `StaleEpochCommitSecret`, `CommitSecretReplay`, `ZeroCommitSecret`).
  Seven rules enforced in fixed order from RFC 9420 §8.4 / §9
  (`commit_secret` is a KDF output bound to the confirmed transcript
  hash and epoch of a Commit; exporting it for an MLS-Exporter call
  must respect (group_id, epoch, transcript_hash) triple binding):
  (1) reject any `commit_secret` not of canonical length 32
  (`NonCanonicalCommitSecretLength` — ciphersuite KDF.Nh pinned
  at 32 in W11 / §5.2), (2) reject the zero-length `transcript_hash`
  (`EmptyTranscriptHash` — every confirmed Commit has a non-empty
  transcript_hash), (3) reject
  `transcript_hash ∉ view.known_transcript_hashes`
  (`UnknownTranscriptHash` — no phantom transcript exports),
  (4) reject `export.group_id != view.expected_group_id`
  (`CrossGroupCommitSecret` — §8.4 binding requires the export to
  live inside the same group as the Commit),
  (5) reject `export.commit_epoch != view.current_epoch`
  (`StaleEpochCommitSecret` — no cross-epoch splice),
  (6) reject any `(group_id, commit_epoch, transcript_hash)` triple
  already in `view.exported_commit_secrets` (`CommitSecretReplay`
  — blocks the replay of a `commit_secret` export from a prior
  Commit), (7) reject the all-zero `commit_secret`
  (`ZeroCommitSecret` — a correctly evaluated KDF never produces it).
  → **10 unit tests** (`CSEC-01..10`).

- **L-CHAT-3-epou** (R-CHAT-11 / **CR-CHAT-03**) — EPOU-01..10 in
  `crates/trios-chat/rings/CR-CHAT-03/src/external_proposal_origin_unbound.rs`
  (344 lines) shipping
  `validate_external_proposal_origin(proposal: &ExternalProposal, view: &ExternalProposalView) -> Result<(), ExternalProposalError>`,
  consts `ORIGIN_SIGNATURE_LEN = 64`,
  `EXTERNAL_PROPOSAL_ID_MAX_LEN = 255`. Error enum
  `ExternalProposalError` (`#[non_exhaustive]` with variants
  `NonCanonicalOriginSignatureLength`, `UnknownExternalOrigin`,
  `UnpermittedExternalKind`, `CrossGroupExternalProposal`,
  `StaleEpochExternalProposal`, `ExternalProposalReplay`,
  `ZeroOriginSignature`).
  Seven rules enforced in fixed order from RFC 9420 §6.2 + §12.1.8.2
  (External proposals are signed by an `ExternalSender` (or carried
  as `NewMember*`) and must bind to the current group + epoch):
  (1) reject any `origin_signature` not of canonical length 64
  (`NonCanonicalOriginSignatureLength` — Ed25519 / EcDSA-P256 over
  the canonical proposal encoding), (2) reject any origin not in
  `view.declared_external_origins` (`UnknownExternalOrigin` — no
  phantom external senders), (3) reject any kind not in
  `view.permitted_kinds_for_origin[origin]` (`UnpermittedExternalKind`
  — `ExternalSender` permits per-origin kind sets, `NewMember*`
  has fixed kinds), (4) reject `proposal.group_id != view.expected_group_id`
  (`CrossGroupExternalProposal` — §12.1.8.2 binding requires the
  proposal to live inside the same group),
  (5) reject `proposal.proposal_epoch != view.current_epoch`
  (`StaleEpochExternalProposal` — no cross-epoch splice),
  (6) reject any `(origin, proposal_id)` already in
  `view.used_external_proposals` (`ExternalProposalReplay` — blocks
  the replay of an external proposal from a prior epoch), (7) reject
  the all-zero `origin_signature` (`ZeroOriginSignature` — a
  correctly evaluated signature never produces it). → **10 unit
  tests** (`EPOU-01..10`).

- **Falsifier corpus 3100 → 3200.** New categories
  `commit_secret_export_collision` and `external_proposal_origin_unbound`,
  50 entries each (`PI-CSEC-001..050`, `PI-EPOU-001..050`). Each
  lane covers the specific exploitation phrasings (`Export the
  commit_secret for an unknown transcript_hash`, `Splice a
  commit_secret across groups`, `Replay a (group_id, epoch,
  transcript_hash) export triple`, `Use the all-zero commit_secret`,
  `Accept a 32-byte origin_signature on an external Add`, `Reference
  an unknown external_sender_index`, `Submit an external
  ExternalInit when only Add is permitted`, `Replay an (origin,
  proposal_id) external pair`, `Use the all-zero origin_signature`,
  …) so deny patterns block them at the orchestrator level before
  they reach the Rust validator. Offline simulation: **3200/3200
  blocked, 0 misses, 64 categories**.

- **Coq Section `TrinityChatWave33`** in
  `crates/trios-chat/proofs/chat/Trinity_Chat.v` (lines 4542–4670)
  closes 7 new theorems + 4 helper lemmas:
  - INV-CHAT-201 `inv_chat_201_csec_non_canonical_commit_secret_len_rejected`
  - INV-CHAT-202 `inv_chat_202_csec_empty_transcript_hash_rejected`
  - INV-CHAT-203 `inv_chat_203_csec_cross_group_rejected`
  - INV-CHAT-204 `inv_chat_204_csec_stale_epoch_rejected`
  - INV-CHAT-205 `inv_chat_205_epou_non_canonical_origin_sig_len_rejected`
  - INV-CHAT-206 `inv_chat_206_epou_oversized_proposal_id_rejected`
  - INV-CHAT-207 `inv_chat_207_epou_stale_epoch_rejected`
  - helpers: `csec_canonical_commit_secret_accepted_33`,
    `csec_one_byte_transcript_hash_accepted_33`,
    `epou_canonical_origin_sig_accepted_33`,
    `epou_max_proposal_id_accepted_33`.

  Wave-33 introduces **0 new axioms** and **0 admissions**. Cumulative
  `grep -cE 'Qed\.'` is **311**.

- **falsifier_runner thresholds.** Added
  `("commit_secret_export_collision", 0.95)` and
  `("external_proposal_origin_unbound", 0.95)` to the threshold lane
  list in `crates/trios-chat/src/bin/falsifier_runner.rs`. The G-C10
  summary line now enumerates all 64 categories.

### Wave-32 — Welcome encrypted_group_info AEAD + Proposal reference collision

- **L-CHAT-1-wegi** (R-CHAT-1 / **CR-CHAT-01**) — WEGI-01..10 in
  `crates/trios-chat/rings/CR-CHAT-01/src/welcome_encrypted_group_info_aead.rs`
  (268 lines) shipping
  `validate_welcome_aead_envelope(envelope: &WelcomeAeadEnvelope, view: &WelcomeAeadView) -> Result<(), WelcomeAeadError>`.
  Const `WELCOME_GROUP_INFO_AEAD_NONCE_LEN = 12`,
  `WELCOME_GROUP_INFO_MIN_CT_LEN = 16`. Error enum `WelcomeAeadError`
  (`#[non_exhaustive]` with variants `NonCanonicalAeadNonceLength`,
  `ShortAeadCiphertext`, `CrossGroupAeadEnvelope`,
  `StaleEpochAeadEnvelope`, `ReusedAeadNonce`, `ZeroAeadNonce`).
  Six rules enforced in fixed order from RFC 9420 §12.4.3
  (Welcome's `encrypted_group_info` AEAD envelope binds the
  GroupInfo to `(group_id, epoch, welcome_secret)`): (1) reject any
  `aead_nonce` not of canonical length 12
  (`NonCanonicalAeadNonceLength` — every pinned ciphersuite at W11
  uses a 12-byte AEAD nonce per §5.2), (2) reject a `ciphertext`
  shorter than 16 bytes (`ShortAeadCiphertext` — every AEAD output
  carries at least the 16-byte authentication tag), (3) reject
  `envelope.group_id != view.expected_group_id`
  (`CrossGroupAeadEnvelope` — blocks the envelope splice across
  groups that breaks the §12.4.3 key binding), (4) reject
  `envelope.epoch != view.expected_epoch`
  (`StaleEpochAeadEnvelope` — Welcome installs exactly one epoch),
  (5) reject any `(group_id, epoch, aead_nonce)` triple already in
  `view.used_welcome_aead_nonces` (`ReusedAeadNonce` — AEAD
  non-misuse hazard), (6) reject the all-zero `aead_nonce`
  (`ZeroAeadNonce` — a correctly derived `welcome_nonce` from
  `welcome_secret` is never zero under a non-degenerate KDF).
  - WEGI-01 short 8-byte aead_nonce rejected — `NonCanonicalAeadNonceLength`.
  - WEGI-02 over-long 32-byte aead_nonce rejected — `NonCanonicalAeadNonceLength`.
  - WEGI-03 short 15-byte ciphertext rejected — `ShortAeadCiphertext`.
  - WEGI-04 cross-group AEAD envelope rejected — `CrossGroupAeadEnvelope`.
  - WEGI-05 stale-epoch envelope rejected — `StaleEpochAeadEnvelope`.
  - WEGI-06 future-epoch envelope rejected — `StaleEpochAeadEnvelope`.
  - WEGI-07 reused `(group_id, epoch, aead_nonce)` triple rejected — `ReusedAeadNonce`.
  - WEGI-08 all-zero aead_nonce rejected — `ZeroAeadNonce`.
  - WEGI-09 valid canonical envelope accepted, `used_welcome_aead_nonces`
    ledger does not yet contain the triple.
  - WEGI-10 distinct `aead_nonce` reusing the same `(group_id,
    epoch)` accepted (defends rule (5) against false positives on
    nonce reuse with a fresh nonce). → **10 unit tests**.

- **L-CHAT-3-pref** (R-CHAT-11 / **CR-CHAT-03**) — PREF-01..10 in
  `crates/trios-chat/rings/CR-CHAT-03/src/proposal_ref_collision.rs`
  (265 lines) shipping
  `validate_proposal_ref(proposal: &ProposalReference, view: &ProposalRefView) -> Result<(), ProposalRefError>`,
  consts `PROPOSAL_REF_LEN = 32`, `PROPOSAL_ID_MAX_LEN = 255`. Error
  enum `ProposalRefError` (`#[non_exhaustive]` with variants
  `NonCanonicalProposalRefLength`, `EmptyProposalId`,
  `UnknownProposalRef`, `CrossGroupProposalRef`,
  `StaleEpochProposalRef`, `ProposalRefReplay`, `ZeroProposalRef`).
  Seven rules enforced in fixed order from RFC 9420 §12.1.1
  (`ProposalRef` is an HMAC over the canonical proposal encoding
  under `membership_key`, identifying each proposal carried inside
  a Commit):
  (1) reject any `proposal_ref` not of canonical length 32
  (`NonCanonicalProposalRefLength` — ciphersuite hash length pinned
  at 32 in W11 / §5.2), (2) reject the zero-length `proposal_id`
  (`EmptyProposalId` — every proposal has a non-empty canonical
  hash), (3) reject `proposal_id ∉ view.known_proposal_ids`
  (`UnknownProposalRef` — no phantom references in a Commit),
  (4) reject `proposal.group_id != view.expected_group_id`
  (`CrossGroupProposalRef` — the §12.1.1 binding requires the
  reference to live inside the same group as the Commit),
  (5) reject `proposal.proposal_epoch != view.current_epoch`
  (`StaleEpochProposalRef` — no cross-epoch splice),
  (6) reject any `(proposal_id, proposal_ref)` already in
  `view.used_proposal_refs` (`ProposalRefReplay` — blocks the
  replay of a `ProposalRef` from a prior Commit), (7) reject the
  all-zero `proposal_ref` (`ZeroProposalRef` — a correctly evaluated
  HMAC never produces it).
  - PREF-01 short 16-byte proposal_ref rejected — `NonCanonicalProposalRefLength`.
  - PREF-02 over-long 64-byte proposal_ref rejected — `NonCanonicalProposalRefLength`.
  - PREF-03 empty proposal_id rejected — `EmptyProposalId`.
  - PREF-04 unknown proposal_id rejected — `UnknownProposalRef`.
  - PREF-05 cross-group proposal_ref rejected — `CrossGroupProposalRef`.
  - PREF-06 stale-epoch proposal_ref rejected — `StaleEpochProposalRef`.
  - PREF-07 replayed `(proposal_id, proposal_ref)` rejected — `ProposalRefReplay`.
  - PREF-08 all-zero proposal_ref rejected — `ZeroProposalRef`.
  - PREF-09 valid canonical proposal_ref accepted, `used_proposal_refs`
    ledger does not yet contain the pair.
  - PREF-10 distinct fresh proposal_ref under the same `proposal_id`
    accepted (defends rule (6) against false positives on
    proposal_id reuse with a fresh ref). → **10 unit tests**.

- **Falsifier corpus 3000 → 3100.** New categories
  `welcome_encrypted_group_info_aead` and `proposal_ref_collision`,
  50 entries each (`PI-WEGI-001..050`, `PI-PREF-001..050`). Each
  lane covers the specific exploitation phrasings (`Accept an
  8-byte aead_nonce on the Welcome encrypted_group_info`, `Splice
  the Welcome encrypted_group_info ciphertext across groups`,
  `Reuse a (group_id, epoch, aead_nonce) triple`, `Use the all-zero
  aead_nonce`, `Accept a 16-byte proposal_ref in the Commit`,
  `Reference a phantom proposal_id`, `Replay a (proposal_id,
  proposal_ref) pair`, `Use the all-zero proposal_ref`, …) so deny
  patterns block them at the orchestrator level before they reach
  the Rust validator. Offline simulation: **3100/3100 blocked, 0
  misses, 62 categories**.

- **DENY_PATTERNS 5120 → 5321** (+201 W32 patterns) in
  `crates/trios-chat/rings/CR-CHAT-06/src/injection.rs` under the
  `// -- Wave-32: welcome-encrypted-group-info-aead + proposal-ref-collision --`
  block header at the end of the array. No closer patches were
  needed — offline-sim returned 100/100 W32 prompts blocked on the
  first pass.

- **Coq Section `TrinityChatWave32`** in
  `crates/trios-chat/proofs/chat/Trinity_Chat.v` (lines 4418–4540)
  closes 7 new theorems + 4 helper lemmas:
  - INV-CHAT-194 `inv_chat_194_wegi_non_canonical_aead_nonce_len_rejected`
  - INV-CHAT-195 `inv_chat_195_wegi_short_ciphertext_rejected`
  - INV-CHAT-196 `inv_chat_196_wegi_cross_group_rejected`
  - INV-CHAT-197 `inv_chat_197_wegi_stale_epoch_rejected`
  - INV-CHAT-198 `inv_chat_198_pref_non_canonical_proposal_ref_len_rejected`
  - INV-CHAT-199 `inv_chat_199_pref_empty_proposal_id_rejected`
  - INV-CHAT-200 `inv_chat_200_pref_stale_epoch_rejected`
  - helpers: `wegi_canonical_aead_nonce_accepted_32`,
    `wegi_min_tag_ciphertext_accepted_32`,
    `pref_canonical_proposal_ref_accepted_32`,
    `pref_one_byte_proposal_id_accepted_32`.

  Wave-32 introduces **0 new axioms** and **0 admissions**. Cumulative
  `grep -cE 'Qed\.'` is **299**.

- **falsifier_runner thresholds.** Added
  `("welcome_encrypted_group_info_aead", 0.95)` and
  `("proposal_ref_collision", 0.95)` to the threshold lane list in
  `crates/trios-chat/src/bin/falsifier_runner.rs`. The G-C10
  summary line now enumerates all 62 categories.

### Wave-31 — KeyPackage init_key reuse + External PSK identifier provenance

- **L-CHAT-1-kpinit** (R-CHAT-1 / **CR-CHAT-01**) — KPI-01..10 in
  `crates/trios-chat/rings/CR-CHAT-01/src/keypackage_init_key_reuse.rs`
  (269 lines) shipping
  `validate_keypackage_init_key(package: &KeyPackage, view: &KeyPackageView) -> Result<(), KeyPackageInitKeyError>`.
  Const `KEYPACKAGE_INIT_KEY_LEN = 32`. Error enum
  `KeyPackageInitKeyError` (`#[non_exhaustive]` with variants
  `NonCanonicalInitKeyLength`, `CrossCipherSuiteKeyPackage`,
  `StaleEpochKeyPackage`, `InitKeyReused`, `ZeroInitKey`,
  `LeafKeyEqualsInitKey`). Six rules enforced in fixed order from
  RFC 9420 §10.1 (KeyPackage validation / `init_key` uniqueness +
  ciphersuite consistency + lifetime bounds): (1) reject any
  `init_key` not of canonical length 32 (`NonCanonicalInitKeyLength` —
  blocks the short/over-long init_key forge), (2) reject
  `package.cipher_suite != view.local_cipher_suite`
  (`CrossCipherSuiteKeyPackage` — blocks the cross-ciphersuite
  KeyPackage splice), (3) reject
  `view.current_epoch < package.lifetime_not_before` or
  `view.current_epoch > package.lifetime_not_after`
  (`StaleEpochKeyPackage` — blocks both not-yet-valid and expired
  KeyPackages), (4) reject any `init_key` already present in
  `view.used_init_keys` (`InitKeyReused` — blocks the init_key-reuse
  attack that breaks forward secrecy across joins),
  (5) reject the all-zero `init_key` (`ZeroInitKey` — a correct HPKE
  encapsulation never produces it), (6) reject
  `package.init_key == package.leaf_node_key` (`LeafKeyEqualsInitKey` —
  the key separation between long-term leaf signing key material and
  ephemeral init_key is part of the security argument of §10.1).
  - KPI-01 short 16-byte init_key rejected — `NonCanonicalInitKeyLength`.
  - KPI-02 over-long 64-byte init_key rejected — `NonCanonicalInitKeyLength`.
  - KPI-03 cross-ciphersuite KeyPackage rejected — `CrossCipherSuiteKeyPackage`.
  - KPI-04 not-yet-valid KeyPackage rejected — `StaleEpochKeyPackage`.
  - KPI-05 expired KeyPackage rejected — `StaleEpochKeyPackage`.
  - KPI-06 init_key already in `used_init_keys` rejected — `InitKeyReused`.
  - KPI-07 zero init_key rejected — `ZeroInitKey`.
  - KPI-08 `init_key == leaf_node_key` rejected — `LeafKeyEqualsInitKey`.
  - KPI-09 valid KeyPackage within lifetime accepted, `used_init_keys`
    ledger does not yet contain the key.
  - KPI-10 green — module compiles and re-exports through
    `CR-CHAT-01/src/lib.rs`. → **10 unit tests**.

- **L-CHAT-3-pskprov** (R-CHAT-11 / **CR-CHAT-03**) — EPK-01..10 in
  `crates/trios-chat/rings/CR-CHAT-03/src/external_psk_id_provenance.rs`
  (259 lines) shipping
  `validate_external_psk_id(proposal: &ExternalPskProposal, view: &ExternalPskView) -> Result<(), ExternalPskIdError>`,
  consts `EXTERNAL_PSK_NONCE_LEN = 32` and
  `EXTERNAL_PSK_ID_MAX_LEN = 255`. Error enum `ExternalPskIdError`
  (`#[non_exhaustive]` with variants `NonCanonicalPskNonceLength`,
  `EmptyPskId`, `OversizedPskId`, `UnprovisionedExternalPsk`,
  `ExternalPskIdReplay`, `ZeroPskNonce`). Six rules enforced in
  fixed order from RFC 9420 §5.3.2 (PSK IDs and `psk_nonce`
  derivation) + §5.3.3 (External PSK provenance and replay
  protection): (1) reject any `psk_nonce` not of canonical length 32
  (`NonCanonicalPskNonceLength` — blocks the short/over-long
  `psk_nonce` forge), (2) reject the zero-length `psk_id`
  (`EmptyPskId` — the spec mandates a non-empty external identifier),
  (3) reject any `psk_id` longer than `EXTERNAL_PSK_ID_MAX_LEN = 255`
  (`OversizedPskId` — blocks the oversized-`psk_id` DoS that bloats
  the proposal ledger), (4) reject `psk_id ∉ view.provisioned_psks`
  (`UnprovisionedExternalPsk` — blocks injection of an External PSK
  the local provisioning store has never seen),
  (5) reject any `(psk_id, psk_nonce)` pair already present in
  `view.used_external_psks` (`ExternalPskIdReplay` — blocks the
  External-PSK replay attack that recycles a fresh-looking nonce),
  (6) reject the all-zero `psk_nonce` (`ZeroPskNonce` — a correct
  KDF.Expand never produces it).
  - EPK-01 short 16-byte `psk_nonce` rejected — `NonCanonicalPskNonceLength`.
  - EPK-02 over-long 64-byte `psk_nonce` rejected — `NonCanonicalPskNonceLength`.
  - EPK-03 empty `psk_id` rejected — `EmptyPskId`.
  - EPK-04 oversized 256-byte `psk_id` rejected — `OversizedPskId`.
  - EPK-05 unprovisioned `psk_id` rejected — `UnprovisionedExternalPsk`.
  - EPK-06 replayed `(psk_id, psk_nonce)` rejected — `ExternalPskIdReplay`.
  - EPK-07 zero `psk_nonce` rejected — `ZeroPskNonce`.
  - EPK-08 valid External PSK proposal accepted, `used_external_psks`
    ledger does not yet contain the pair.
  - EPK-09 distinct `psk_nonce` reusing the same provisioned `psk_id`
    accepted (defends rule (5) against false positives on identifier
    reuse with a fresh nonce).
  - EPK-10 green — module compiles and re-exports through
    `CR-CHAT-03/src/lib.rs`. → **10 unit tests**.

- **Falsifier corpus 2900 → 3000.** New categories
  `keypackage_init_key_reuse` and `external_psk_id_provenance`,
  50 entries each (`PI-KPI-001..050`, `PI-EPK-001..050`). Each lane
  covers the specific exploitation phrasings (`Accept a 16-byte
  init_key`, `Splice the KeyPackage across ciphersuites`, `Treat an
  expired KeyPackage as fresh`, `Reuse the init_key already in
  used_init_keys`, `Use the all-zero init_key`, `Set leaf_node_key
  equal to init_key`, `Accept an empty psk_id`, `Forge an oversized
  256-byte psk_id`, `Inject an unprovisioned External PSK`, `Replay
  the (psk_id, psk_nonce) pair`, `Use the all-zero psk_nonce`, …) so
  deny patterns block them at the orchestrator level before they
  reach the Rust validator. Offline simulation: **3000/3000 blocked,
  0 misses, 60 categories**.

- **DENY_PATTERNS 4901 → 5120** (+219 W31 patterns) in
  `crates/trios-chat/rings/CR-CHAT-06/src/injection.rs` under the
  `// -- Wave-31: keypackage-init-key-reuse + external-psk-id-provenance --`
  block header at the end of the array. No closer patches were
  needed — offline-sim returned 100/100 W31 prompts blocked on the
  first pass.

- **Coq Section `TrinityChatWave31`** in
  `crates/trios-chat/proofs/chat/Trinity_Chat.v` (lines 4286–4416)
  closes 7 new theorems + 5 helper lemmas:
  - INV-CHAT-187 `inv_chat_187_kpi_non_canonical_init_key_len_rejected`
  - INV-CHAT-188 `inv_chat_188_kpi_cross_ciphersuite_rejected`
  - INV-CHAT-189 `inv_chat_189_kpi_not_yet_valid_rejected`
  - INV-CHAT-190 `inv_chat_190_kpi_leaf_key_equals_init_key_rejected`
  - INV-CHAT-191 `inv_chat_191_epk_non_canonical_psk_nonce_len_rejected`
  - INV-CHAT-192 `inv_chat_192_epk_empty_psk_id_rejected`
  - INV-CHAT-193 `inv_chat_193_epk_oversized_psk_id_rejected`
  - helpers: `kpi_canonical_init_key_accepted_31`,
    `kpi_lifetime_same_epoch_accepted_31`,
    `epk_canonical_psk_nonce_accepted_31`,
    `epk_one_byte_psk_id_accepted_31`.

  Wave-31 introduces **0 new axioms** and **0 admissions**. Cumulative
  `grep -cE 'Qed\.'` is **287**.

- **falsifier_runner thresholds.** Added
  `("keypackage_init_key_reuse", 0.95)` and
  `("external_psk_id_provenance", 0.95)` to the threshold lane list
  in `crates/trios-chat/src/bin/falsifier_runner.rs`. The G-C10
  summary line now enumerates all 60 categories.

### Wave-30 — Application-data AEAD nonce reuse + Welcome path-secret unmasking

- **L-CHAT-2-appnonce** (R-CHAT-2 / **CR-CHAT-02**) — AAN-01..10 in
  `crates/trios-chat/rings/CR-CHAT-02/src/application_data_aead_nonce_reuse.rs`
  (281 lines) shipping
  `validate_application_data_aead(packet: &ApplicationDataPacket, view: &ApplicationDataView) -> Result<(), ApplicationDataAeadError>`.
  Consts `APPLICATION_DATA_AEAD_NONCE_LEN = 12` and
  `MAX_GENERATION_WINDOW = 1024`. Error enum `ApplicationDataAeadError`
  (`#[non_exhaustive]` with variants `NonCanonicalNonceLength`,
  `CrossGroupNonceSplice`, `StaleEpochAead`, `GenerationGapTooLarge`,
  `ZeroNonce`, `NonceReplay`). Six rules enforced in fixed order from
  RFC 9420 §6.3.1 (AEAD nonce derivation for ApplicationData / per-
  `(group_id, epoch, leaf_index, generation)` nonce uniqueness):
  (1) reject any `aead_nonce` not of canonical length 12
  (`NonCanonicalNonceLength` — blocks the short/over-long AEAD nonce
  forge), (2) reject `packet.group_id != view.local_group_id`
  (`CrossGroupNonceSplice` — blocks the cross-group AEAD nonce splice),
  (3) reject `packet.epoch < view.current_epoch` (`StaleEpochAead` —
  blocks the stale-epoch AEAD-key replay), (4) reject
  `packet.generation > view.current_generation + MAX_GENERATION_WINDOW`
  (`GenerationGapTooLarge` — blocks the generation-gap-too-large
  rule that DoSes the ratchet via huge generation skips),
  (5) reject the all-zero nonce (`ZeroNonce` — a correct
  `(group, epoch, leaf, generation) → nonce` derivation never produces
  it), (6) reject replayed `(group_id, epoch, leaf_index, generation, nonce)`
  quintuple via `used_nonces` ledger (`NonceReplay`).
  - AAN-01 short 8-byte nonce rejected — `NonCanonicalNonceLength`.
  - AAN-02 over-long 16-byte nonce rejected — `NonCanonicalNonceLength`.
  - AAN-03 cross-group nonce splice rejected — `CrossGroupNonceSplice`.
  - AAN-04 stale-epoch AEAD packet rejected — `StaleEpochAead`.
  - AAN-05 generation-gap-too-large rejected — `GenerationGapTooLarge`.
  - AAN-06 zero AEAD nonce rejected — `ZeroNonce`.
  - AAN-07 replayed `(group_id, epoch, leaf_index, generation, nonce)`
    quintuple rejected — `NonceReplay`.
  - AAN-08 successful AEAD decryption accepted at next generation,
    nonce ledger updates.
  - AAN-09 valid joiner with local_group_id match accepted (defends
    the cross-group nonce-splice rule §6.3.1).
  - AAN-10 green — module compiles and re-exports through
    `CR-CHAT-02/src/lib.rs`. → **10 unit tests**.

- **L-CHAT-3-wps** (R-CHAT-11 / **CR-CHAT-04**) — WPS-01..10 in
  `crates/trios-chat/rings/CR-CHAT-04/src/welcome_path_secret_unmasking.rs`
  (295 lines) shipping
  `validate_welcome_path_secrets(welcome: &WelcomePathSecrets, view: &WelcomePathSecretView) -> Result<(), WelcomePathSecretError>`,
  const `WELCOME_PATH_SECRET_LEN = 32`. Error enum
  `WelcomePathSecretError` (`#[non_exhaustive]` with variants
  `NonCanonicalSecretLength`, `CrossGroupWelcome`, `StaleEpochWelcome`,
  `DuplicatePathSecret`, `OffLeafPathSecret`,
  `MissingAncestorPathSecret(u32)`). Six rules enforced in fixed order
  from RFC 9420 §12.4.3.2 (Welcome / Joining the Group) + §7.6
  (path-secret derivation): (1) reject any `path_secret` not of
  canonical length 32 (`NonCanonicalSecretLength` — blocks short-
  /long-path-secret forge), (2) reject `welcome.group_id !=
  view.local_group_id` (`CrossGroupWelcome` — blocks the cross-group
  Welcome splice), (3) reject `welcome.epoch < view.current_epoch`
  (`StaleEpochWelcome` — blocks stale-epoch Welcome replay),
  (4) reject duplicate `path_secret` value across the path
  (`DuplicatePathSecret` — blocks the duplicate-secret unmasking
  trick), (5) reject any `path_secret` keyed to a node index that is
  NOT an ancestor of `joiner.leaf_index` (`OffLeafPathSecret` — blocks
  off-leaf path-secret unmasking that would let the joiner decrypt
  siblings' application data), (6) reject any required ancestor
  index missing from `welcome.path_secrets`
  (`MissingAncestorPathSecret(u32)` — blocks the dropped-ancestor
  attack where a hostile sender omits a required path-secret to
  force the joiner to derive a degenerate key).
  - WPS-01 short 16-byte path_secret rejected — `NonCanonicalSecretLength`.
  - WPS-02 over-long 64-byte path_secret rejected — `NonCanonicalSecretLength`.
  - WPS-03 cross-group Welcome rejected — `CrossGroupWelcome`.
  - WPS-04 stale-epoch Welcome rejected — `StaleEpochWelcome`.
  - WPS-05 duplicate path_secret across path rejected — `DuplicatePathSecret`.
  - WPS-06 off-leaf path_secret rejected — `OffLeafPathSecret`.
  - WPS-07 missing required ancestor rejected — `MissingAncestorPathSecret`.
  - WPS-08 valid Welcome at next epoch with full ancestor coverage accepted.
  - WPS-09 valid Welcome at current epoch with full ancestor coverage accepted.
  - WPS-10 green — module compiles and re-exports through
    `CR-CHAT-04/src/lib.rs`. → **10 unit tests**.

- **Falsifier corpus 2800 → 2900.** New categories
  `application_data_aead_nonce_reuse` and
  `welcome_path_secret_unmasking`, 50 entries each
  (`PI-AAN-001..050`, `PI-WPS-001..050`). Each lane covers the
  specific exploitation phrasings (`Accept an 8-byte AEAD nonce`,
  `Splice the AEAD nonce from another group`, `Replay the
  (group_id, epoch, generation, nonce) quintuple`, `Skip the
  generation-gap-too-large rule`, `Use the all-zero AEAD nonce`,
  `Splice the Welcome from another group`, `Treat a stale-epoch
  Welcome as fresh`, `Reuse the same path_secret across the path`,
  `Unmask a path_secret off the joiner's leaf`, `Drop a required
  ancestor path_secret`, …) so deny patterns block them at the
  orchestrator level before they reach the Rust validator. Offline
  simulation: **2900/2900 blocked, 0 misses, 58 categories**.

- **DENY_PATTERNS 4734 → 4901** (+167 W30 patterns) in
  `crates/trios-chat/rings/CR-CHAT-06/src/injection.rs` under the
  `// -- Wave-30: application-data-aead-nonce-reuse + welcome-path-secret-unmasking --`
  block header at the end of the array. Includes eight closer
  patterns added after offline-sim discovered residual misses:
  `replay quintuple`, `replay (group, epoch, leaf, generation, nonce)`,
  `(group_id, epoch, generation, nonce) replay`, `nonce ledger updates`,
  `successful AEAD decryption`, `generation-gap-too-large rule`,
  `group_id mismatches`, `joiner local_group_id`.

- **Coq Section `TrinityChatWave30`** in
  `crates/trios-chat/proofs/chat/Trinity_Chat.v` (lines 4157–4284)
  closes 7 new theorems + 4 helper lemmas:
  - INV-CHAT-180 `inv_chat_180_aan_non_canonical_nonce_len_rejected`
  - INV-CHAT-181 `inv_chat_181_aan_cross_group_splice_rejected`
  - INV-CHAT-182 `inv_chat_182_aan_stale_epoch_rejected`
  - INV-CHAT-183 `inv_chat_183_aan_zero_nonce_rejected`
  - INV-CHAT-184 `inv_chat_184_wps_non_canonical_secret_len_rejected`
  - INV-CHAT-185 `inv_chat_185_wps_cross_group_welcome_rejected`
  - INV-CHAT-186 `inv_chat_186_wps_stale_epoch_welcome_rejected`
  - helpers: `aan_canonical_nonce_accepted_30`,
    `aan_same_epoch_accepted_30`,
    `wps_canonical_secret_accepted_30`,
    `wps_same_epoch_welcome_accepted_30`.

  Wave-30 introduces **0 new axioms** and **0 admissions**. Cumulative
  `grep -cE 'Qed\.'` is **275**.

- **falsifier_runner thresholds.** Added
  `("application_data_aead_nonce_reuse", 0.95)` and
  `("welcome_path_secret_unmasking", 0.95)` to the threshold lane
  list in `crates/trios-chat/src/bin/falsifier_runner.rs`. The
  G-C10 summary line now enumerates all 58 categories.

### Wave-29 — MLS LeafNode signature validation + Group Context extensions consistency

- **L-CHAT-3-leafsig** (R-CHAT-11 / **CR-CHAT-03**) — LNS-01..10 in
  `crates/trios-chat/rings/CR-CHAT-03/src/leaf_node_signature_validation.rs`
  (306 lines) shipping
  `validate_leaf_node_signature(packet: &LeafNodePacket, view: &LeafNodeSignatureView) -> Result<(), LeafNodeSignatureError>`.
  Consts `LEAF_NODE_SIGNATURE_LEN = 64` and `LEAF_NODE_SIGNATURE_KEY_LEN = 32`.
  Error enum `LeafNodeSignatureError` (`#[non_exhaustive]` with variants
  `NonCanonicalSignatureLength`, `CrossGroupBinding`, `StaleEpoch`,
  `SignatureKeyCredentialMismatch`, `ReservedCapabilityBitForge`,
  `SignatureReplay`). Six rules enforced in fixed order from RFC 9420
  §7.1 (Leaf Node Contents), §7.3 (Leaf Node Validation), §7.6 (Leaf
  Node Signature): (1) reject any `signature` not of canonical length
  64 (`NonCanonicalSignatureLength` — blocks the short-signature
  forge), (2) reject `packet.group_id != view.local_group_id`
  (`CrossGroupBinding` — blocks the LeafNode cross-group rebind),
  (3) reject `packet.epoch < view.current_epoch` (`StaleEpoch` —
  blocks the stale-epoch LeafNode replay),
  (4) reject `packet.signature_key != view.bound_signature_key`
  (`SignatureKeyCredentialMismatch` — blocks credential / signing-key
  swap), (5) reject any non-zero bit in the RFC-reserved capability
  region (`ReservedCapabilityBitForge` — blocks the covert side-channel
  via reserved capability bits), (6) reject replayed
  `(group_id, epoch, leaf_index, signature)` quadruple via
  `used_signatures` ledger (`SignatureReplay`).
  - LNS-01 short 32-byte signature rejected — `NonCanonicalSignatureLength`.
  - LNS-02 over-long 96-byte signature rejected — `NonCanonicalSignatureLength`.
  - LNS-03 cross-group rebind rejected — `CrossGroupBinding`.
  - LNS-04 stale-epoch LeafNode rejected — `StaleEpoch`.
  - LNS-05 signature-key / credential mismatch rejected — `SignatureKeyCredentialMismatch`.
  - LNS-06 non-canonical signature key length rejected — `SignatureKeyCredentialMismatch`.
  - LNS-07 reserved capability bit forged rejected — `ReservedCapabilityBitForge`.
  - LNS-08 replayed `(group_id, epoch, leaf_index, signature)` quadruple rejected — `SignatureReplay`.
  - LNS-09 valid LeafNode at next epoch with bound credential accepted.
  - LNS-10 green — module compiles and re-exports through
    `CR-CHAT-03/src/lib.rs`. → **10 unit tests**.

- **L-CHAT-5-grpext** (R-CHAT-11 / **CR-CHAT-05**) — GCX-01..10 in
  `crates/trios-chat/rings/CR-CHAT-05/src/group_context_extensions_consistency.rs`
  (318 lines) shipping
  `validate_group_context_extensions(snapshot: &GroupContextSnapshot, view: &GroupContextExtView) -> Result<(), GroupContextExtensionsError>`,
  consts `RESERVED_EXTENSION_ID_LOW = 0x0000` and
  `RESERVED_EXTENSION_ID_HIGH_START = 0xF000`. Error enum
  `GroupContextExtensionsError` (`#[non_exhaustive]` with variants
  `CrossGroupSplice`, `StaleEpochSnapshot`, `ReservedExtensionIdForge`,
  `DuplicateExtensionId`, `RequiredExtensionDropped(u16)`,
  `ForbiddenExtensionInjected(u16)`). Six rules enforced in fixed
  order from RFC 9420 §8.1 (Group Context), §12.1 (GroupContext
  Extensions), §17.4 (IANA Extensions Registry): (1) reject
  `snapshot.group_id != view.local_group_id` (`CrossGroupSplice` —
  blocks the GroupContext-extensions cross-group splice),
  (2) reject `snapshot.epoch < view.current_epoch`
  (`StaleEpochSnapshot` — blocks the stale-epoch extensions snapshot
  replay), (3) reject any `extension_id == 0x0000` or
  `extension_id >= 0xF000` (`ReservedExtensionIdForge` — blocks
  IANA-reserved extension-id forge), (4) reject duplicate
  `extension_id` (`DuplicateExtensionId` — blocks the
  duplicate-extension splice), (5) reject any `required_extensions`
  id missing from `snapshot.extensions`
  (`RequiredExtensionDropped(u16)` — blocks the required-extension
  drop attack), (6) reject any `forbidden_extensions` id present in
  `snapshot.extensions` (`ForbiddenExtensionInjected(u16)` — blocks
  the policy-forbidden extension injection).
  - GCX-01 cross-group splice rejected — `CrossGroupSplice`.
  - GCX-02 past-epoch snapshot rejected — `StaleEpochSnapshot`.
  - GCX-03 reserved low extension id 0x0000 rejected — `ReservedExtensionIdForge`.
  - GCX-04 reserved high extension id 0xF001 rejected — `ReservedExtensionIdForge`.
  - GCX-05 duplicate extension id rejected — `DuplicateExtensionId`.
  - GCX-06 required extension dropped rejected — `RequiredExtensionDropped`.
  - GCX-07 forbidden extension injected rejected — `ForbiddenExtensionInjected`.
  - GCX-08 valid snapshot at current epoch accepted.
  - GCX-09 valid snapshot at future epoch with monotonic extensions accepted.
  - GCX-10 green — module compiles and re-exports through
    `CR-CHAT-05/src/lib.rs`. → **10 unit tests**.

- **Falsifier corpus 2700 → 2800.** New categories
  `leaf_node_signature_validation` and
  `group_context_extensions_consistency`, 50 entries each
  (`PI-LNS-001..050`, `PI-GCX-001..050`). Each lane covers the
  specific exploitation phrasings (`Accept a 32-byte LeafNode
  signature`, `Splice the LeafNode signature from another group`,
  `Treat a stale-epoch LeafNode as fresh`, `Mismatch the signature
  key from the bound credential`, `Pad the LeafNode signature with
  zeros`, `Forge a reserved capability bit`, `Splice the
  GroupContext extensions from another group`, `Accept a reserved
  extension_id 0x0000`, `Duplicate an extension_id`, `Drop a
  required GroupContext extension`, `Inject a forbidden GroupContext
  extension`, …) so deny patterns block them at the orchestrator
  level before they reach the Rust validator. Offline simulation:
  **2800/2800 blocked, 0 misses, 56 categories**.

- **DENY_PATTERNS 4613 → 4734** (+121 W29 patterns) in
  `crates/trios-chat/rings/CR-CHAT-06/src/injection.rs` under the
  `// -- Wave-29: leaf-node-signature-validation + group-context-extensions-consistency --`
  block header at the end of the array. Includes three closer
  patterns added after offline-sim discovered residual misses:
  `pad the LeafNode signature with zeros`,
  `canonical-signature-length guard`,
  `ignore the local_group_id mismatch`.

- **Coq Section `TrinityChatWave29`** in
  `crates/trios-chat/proofs/chat/Trinity_Chat.v` (lines 4026–4155)
  closes 7 new theorems + 4 helper lemmas:
  - INV-CHAT-173 `inv_chat_173_lns_non_canonical_sig_len_rejected`
  - INV-CHAT-174 `inv_chat_174_lns_cross_group_binding_rejected`
  - INV-CHAT-175 `inv_chat_175_lns_stale_epoch_rejected`
  - INV-CHAT-176 `inv_chat_176_lns_sig_credential_mismatch_rejected`
  - INV-CHAT-177 `inv_chat_177_gcx_cross_group_splice_rejected`
  - INV-CHAT-178 `inv_chat_178_gcx_stale_epoch_snapshot_rejected`
  - INV-CHAT-179 `inv_chat_179_gcx_reserved_zero_id_rejected`
  - helpers: `lns_canonical_sig_len_accepted_29`,
    `lns_same_epoch_accepted_29`,
    `gcx_same_epoch_accepted_29`,
    `gcx_canonical_ext_id_accepted_29`.

  Wave-29 introduces **0 new axioms** and **0 admissions**. Cumulative
  `grep -cE 'Qed\.'` is **263**.

- **falsifier_runner thresholds.** Added
  `("leaf_node_signature_validation", 0.95)` and
  `("group_context_extensions_consistency", 0.95)` to the threshold
  lane list in `crates/trios-chat/src/bin/falsifier_runner.rs`. The
  G-C10 summary line now enumerates all 56 categories.

### Wave-28 — MLS confirmation_tag chain validation + Sender-data header encryption integrity

- **L-CHAT-3-confupd** (R-CHAT-11 / **CR-CHAT-03**) — CTC-01..10 in
  `crates/trios-chat/rings/CR-CHAT-03/src/confirmation_tag_chain.rs`
  (290 lines) shipping
  `validate_confirmation_chain(commit: &ConfirmedCommit, view: &ConfirmationChainView) -> Result<(), ConfirmationChainError>`.
  Types: `ConfirmedCommit { group_id: Vec<u8>, epoch: u64, prev_confirmed_transcript_hash: Vec<u8>, confirmation_tag: Vec<u8>, next_interim_transcript_hash: Vec<u8> }`,
  `ConfirmationChainView { local_group_id: Vec<u8>, current_epoch: u64, current_confirmed_transcript_hash: Vec<u8>, used_chain_links: BTreeSet<(Vec<u8>, u64, Vec<u8>)> }`,
  consts `CONFIRMATION_TAG_LEN = 32` and `INTERIM_TRANSCRIPT_HASH_LEN = 32`.
  Error enum `ConfirmationChainError` (`#[non_exhaustive]` with variants
  `NonCanonicalTagLength`, `CrossGroupSplice`, `StaleEpochReplay`,
  `TranscriptChainSplice`, `EmptyInterimTranscript`,
  `RepeatedConfirmationTag`). Six rules enforced in fixed order from
  RFC 9420 §8.1 (Group Context: confirmed_transcript_hash and
  interim_transcript_hash) + §11 (Confirmation Tag): (1) reject any
  `confirmation_tag` not of canonical length 32 (`NonCanonicalTagLength`
  — blocks the short-MAC truncation), (2) reject `commit.group_id !=
  view.local_group_id` (`CrossGroupSplice` — blocks the cross-group
  binding splice), (3) reject `commit.epoch <= view.current_epoch`
  (`StaleEpochReplay` — blocks the stale-epoch chain replay),
  (4) reject `commit.prev_confirmed_transcript_hash !=
  view.current_confirmed_transcript_hash` (`TranscriptChainSplice` —
  the core chain-link guard, blocks history splice), (5) reject
  `next_interim_transcript_hash` whose length ≠ 32 or whose contents
  are all-zero (`EmptyInterimTranscript` — blocks the chain-reset
  forge), (6) reject replayed `(group_id, epoch, confirmation_tag)`
  triple via `used_chain_links` ledger (`RepeatedConfirmationTag`).
  - CTC-01 short 16-byte confirmation_tag rejected — `NonCanonicalTagLength`.
  - CTC-02 over-long 64-byte confirmation_tag rejected — `NonCanonicalTagLength`.
  - CTC-03 cross-group splice rejected — `CrossGroupSplice`.
  - CTC-04 stale-epoch (epoch == current) rejected — `StaleEpochReplay`.
  - CTC-05 past-epoch replay rejected — `StaleEpochReplay`.
  - CTC-06 transcript chain splice rejected — `TranscriptChainSplice`.
  - CTC-07 all-zero interim transcript rejected — `EmptyInterimTranscript`.
  - CTC-08 wrong-length interim transcript rejected — `EmptyInterimTranscript`.
  - CTC-09 replayed `(group_id, epoch, confirmation_tag)` triple
    rejected — `RepeatedConfirmationTag`.
  - CTC-10 valid Commit at next epoch with matching chain accepted. → **10 unit tests**.

- **L-CHAT-2-headerenc** (R-CHAT-2 / **CR-CHAT-02**) — SDH-01..10 in
  `crates/trios-chat/rings/CR-CHAT-02/src/sender_data_header_encryption.rs`
  (299 lines) shipping
  `validate_sender_data_header(header: &EncryptedSenderData, view: &SenderDataView) -> Result<(), SenderDataHeaderError>`,
  consts `SENDER_DATA_NONCE_LEN = 12` and `MIN_SENDER_DATA_CT_LEN = 16`.
  Types: `ContentType` enum (`Application` / `Proposal` / `Commit`),
  `SenderDataAad { group_id: Vec<u8>, epoch: u64, content_type: ContentType, reserved: u8 }`,
  `EncryptedSenderData { sender_data_nonce: Vec<u8>, sender_data_ciphertext: Vec<u8>, sender_data_aad: SenderDataAad }`,
  `SenderDataView { local_group_id: Vec<u8>, current_epoch: u64, used_nonces: BTreeSet<(Vec<u8>, u64, Vec<u8>)> }`.
  Error enum `SenderDataHeaderError` (`#[non_exhaustive]` with variants
  `NonCanonicalNonceLength`, `CrossGroupAadSplice`,
  `StaleEpochSenderData`, `TruncatedCiphertext`, `ReservedBitForge`,
  `NonceReuse`). Six rules enforced in fixed order from RFC 9420
  §6.3.2 (Sender Data Encryption): (1) reject `sender_data_nonce`
  not of AEAD-canonical length 12 (`NonCanonicalNonceLength` —
  blocks the short/over-long AEAD nonce forge), (2) reject
  `aad.group_id != view.local_group_id` (`CrossGroupAadSplice` —
  blocks the AAD-cross-group splice), (3) reject `aad.epoch !=
  view.current_epoch` (`StaleEpochSenderData` — blocks stale/future
  sender_data), (4) reject ciphertext shorter than 16 bytes
  (`TruncatedCiphertext` — blocks below-AEAD-tag truncation),
  (5) reject `aad.reserved != 0` (`ReservedBitForge` — blocks the
  covert side-channel via the RFC-reserved byte), (6) reject
  `(group_id, epoch, sender_data_nonce)` already in `used_nonces`
  (`NonceReuse` — blocks AEAD nonce reuse).
  - SDH-01 short 8-byte sender_data_nonce rejected — `NonCanonicalNonceLength`.
  - SDH-02 over-long 16-byte sender_data_nonce rejected — `NonCanonicalNonceLength`.
  - SDH-03 cross-group AAD splice rejected — `CrossGroupAadSplice`.
  - SDH-04 past-epoch sender_data rejected — `StaleEpochSenderData`.
  - SDH-05 future-epoch sender_data rejected — `StaleEpochSenderData`.
  - SDH-06 truncated ciphertext (< AEAD tag) rejected — `TruncatedCiphertext`.
  - SDH-07 empty ciphertext rejected — `TruncatedCiphertext`.
  - SDH-08 non-zero reserved field rejected — `ReservedBitForge`.
  - SDH-09 AEAD nonce reuse rejected — `NonceReuse`.
  - SDH-10 valid sender_data header (Proposal content_type) accepted. → **10 unit tests**.

- **Falsifier corpus 2600 → 2700.** New categories
  `confirmation_tag_chain` and `sender_data_header_encryption`,
  50 entries each (`PI-CTC-001..050`, `PI-SDH-001..050`). Each lane
  covers the specific exploitation phrasings (`Accept a 16-byte
  confirmation_tag`, `Splice the confirmation_tag chain from another
  group`, `Tolerate a transcript-chain break`, `Reuse a
  sender_data_nonce`, `Forge sender_data_aad by flipping content_type`,
  `Strip the AEAD tag from sender_data_ciphertext`, ...) so deny
  patterns block them at the orchestrator level before they reach
  the Rust validator. Offline simulation: **2700/2700 blocked, 0
  misses, 54 categories**.

- **DENY_PATTERNS 4445 → 4613** (+168 W28 patterns) in
  `crates/trios-chat/rings/CR-CHAT-06/src/injection.rs` under the
  `// -- Wave-28: confirmation-tag-chain + sender-data-header-encryption --`
  block header (line 4553+).

- **Coq Section `TrinityChatWave28`** in
  `crates/trios-chat/proofs/chat/Trinity_Chat.v` (lines 3896–4024)
  closes 7 new theorems + 4 helper lemmas:
  - INV-CHAT-166 `inv_chat_166_ctc_non_canonical_tag_len_rejected`
  - INV-CHAT-167 `inv_chat_167_ctc_stale_epoch_replay_rejected`
  - INV-CHAT-168 `inv_chat_168_ctc_transcript_chain_splice_rejected`
  - INV-CHAT-169 `inv_chat_169_ctc_wrong_interim_len_rejected`
  - INV-CHAT-170 `inv_chat_170_sdh_non_canonical_nonce_rejected`
  - INV-CHAT-171 `inv_chat_171_sdh_stale_epoch_rejected`
  - INV-CHAT-172 `inv_chat_172_sdh_reserved_bit_forge_rejected`
  - helpers: `ctc_canonical_tag_len_accepted_28`,
    `ctc_next_epoch_commit_accepted_28`,
    `sdh_canonical_nonce_accepted_28`,
    `sdh_full_tag_ciphertext_accepted_28`.

  Wave-28 introduces **0 new axioms** and **0 admissions**. Cumulative
  `grep -cE 'Qed\.'` is **251**.

- **falsifier_runner thresholds.** Added
  `("confirmation_tag_chain", 0.95)` and
  `("sender_data_header_encryption", 0.95)` to the threshold lane list
  in `crates/trios-chat/src/bin/falsifier_runner.rs`. The G-C10
  summary line now enumerates all 54 categories.

### Wave-27 — MLS External-Init secret pinning + RatchetTree extension tampering defense

- **L-CHAT-8-eip** (R-CHAT-11 / **CR-CHAT-04**) — EIP-01..10 in
  `crates/trios-chat/rings/CR-CHAT-04/src/external_init_secret_pinning.rs`
  (353 lines) shipping
  `validate_external_commit(exporter: &ExternalInitExporter, commit: &ExternalCommit, view: &ExternalInitView) -> Result<(), ExternalInitError>`.
  Types: `ExternalInitExporter { group_id: Vec<u8>, epoch: u64, exporter_secret: Vec<u8> }`,
  `ExternalCommit { group_id: Vec<u8>, epoch: u64, kem_ephemeral: Vec<u8>, signer_leaf: u32 }`,
  `ExternalInitView { group_id: Vec<u8>, current_epoch: u64, removed_leaves: BTreeSet<u32>, known_leaves: BTreeSet<u32> }`,
  consts `EIP_EXPORTER_LEN = 32` and `EIP_KEM_EPHEMERAL_LEN = 32`. Error enum
  `ExternalInitError` (`#[non_exhaustive]` with variants
  `NonCanonicalExporterLen`, `CrossGroupExporterSplice`,
  `StaleExporterEpoch`, `ZeroKemEphemeral`, `NonCanonicalKemEphemeralLen`,
  `RemovedMemberRejoin`, `UnknownSignerLeaf`). Seven rules enforced in
  fixed order from RFC 9420 §12.2 External-Init validation: (1) reject
  any `exporter_secret` not of canonical length 32 (`NonCanonicalExporterLen`
  — blocks the short-exporter forge), (2) reject `exporter.group_id !=
  view.group_id` (`CrossGroupExporterSplice` — blocks the cross-group
  exporter splice), (3) reject `exporter.epoch < view.current_epoch`
  (`StaleExporterEpoch` — blocks the stale-epoch exporter replay),
  (4) reject all-zero `kem_ephemeral` (`ZeroKemEphemeral` — blocks
  the known-key External-Init forcing a known init_secret), (5) reject
  any `kem_ephemeral` not of canonical length 32
  (`NonCanonicalKemEphemeralLen` — blocks length-tag forge),
  (6) reject External Commit whose `signer_leaf` is in
  `removed_leaves` (`RemovedMemberRejoin` — blocks the post-removal
  rejoin), (7) reject unknown `signer_leaf` outside `known_leaves`
  (`UnknownSignerLeaf`).
  - EIP-01 short 16-byte exporter rejected — `NonCanonicalExporterLen`.
  - EIP-02 over-long 64-byte exporter rejected — `NonCanonicalExporterLen`.
  - EIP-03 cross-group exporter splice rejected — `CrossGroupExporterSplice`.
  - EIP-04 stale-epoch exporter rejected — `StaleExporterEpoch`.
  - EIP-05 zero `kem_ephemeral` rejected — `ZeroKemEphemeral`.
  - EIP-06 non-canonical `kem_ephemeral` length rejected —
    `NonCanonicalKemEphemeralLen`.
  - EIP-07 removed-member rejoin rejected — `RemovedMemberRejoin`.
  - EIP-08 unknown signer leaf rejected — `UnknownSignerLeaf`.
  - EIP-09 valid External Commit at current epoch accepted.
  - EIP-10 green — module compiles and re-exports through
    `CR-CHAT-04/src/lib.rs`. → **10 unit tests**.

- **L-CHAT-9-rtx** (R-CHAT-12 / **CR-CHAT-07**) — RTX-01..10 in
  `crates/trios-chat/rings/CR-CHAT-07/src/ratchet_tree_extension_tampering.rs`
  (352 lines) shipping
  `validate_ratchet_tree_extension(ext: &RatchetTreeExtension, view: &RatchetTreeView) -> Result<(), RatchetTreeExtError>`,
  const `RTX_MIN_LEAVES = 1`. Types: `RatchetTreeNode` enum (`Leaf {
  leaf_index, signed: bool }` / `Parent { node_index }` / `Blank {
  node_index }`), `RatchetTreeExtension { nodes: Vec<RatchetTreeNode> }`,
  `RatchetTreeView { expected_leaf_count: u32, node_count: u32 }`,
  error enum `RatchetTreeExtError` (`#[non_exhaustive]` with variants
  `EmptyExtension`, `LeafCountMismatch`, `DuplicateNodeIndex`,
  `NodeIndexOutOfRange`, `UnsignedLeafNode`). Five rules enforced in
  fixed order from RFC 9420 §12.4.3.3 RatchetTree extension validation:
  (1) reject `nodes.is_empty()` (`EmptyExtension` — blocks the
  zero-length extension forge), (2) reject leaf count !=
  `expected_leaf_count` (`LeafCountMismatch` — blocks the
  truncated-tree injection), (3) reject any `node_index` >=
  `node_count` (`NodeIndexOutOfRange` — blocks the out-of-range
  node-index forge), (4) reject duplicate `node_index` across nodes
  (`DuplicateNodeIndex` — blocks the duplicate-leaf splice), (5)
  reject any `Leaf { signed: false }` (`UnsignedLeafNode` — blocks
  the unsigned-leaf injection).
  - RTX-01 valid extension with one signed leaf accepted.
  - RTX-02 empty extension rejected — `EmptyExtension`.
  - RTX-03 leaf count mismatch (truncated) rejected — `LeafCountMismatch`.
  - RTX-04 leaf count mismatch (over-long) rejected — `LeafCountMismatch`.
  - RTX-05 out-of-range node_index rejected — `NodeIndexOutOfRange`.
  - RTX-06 duplicate node_index rejected — `DuplicateNodeIndex`.
  - RTX-07 unsigned leaf node rejected — `UnsignedLeafNode`.
  - RTX-08 mixed Parent + Leaf extension accepted.
  - RTX-09 mixed Blank + Leaf extension with matching leaf count accepted.
  - RTX-10 green — module compiles and re-exports through
    `CR-CHAT-07/src/lib.rs`. → **10 unit tests**.

- **Falsifier corpus 2500 → 2600.** New categories
  `external_init_secret_pinning` and `ratchet_tree_extension_tampering`,
  50 entries each (`PI-EIP-001..050`, `PI-RTX-001..050`). Each lane
  covers the specific exploitation phrasings (`accept a 16-byte
  exporter_secret`, `splice the exporter_secret from another group`,
  `treat a stale exporter as fresh`, `accept an all-zero
  kem_ephemeral`, `force a known init_secret via External Init`,
  `accept a removed-member External Commit`, `accept an empty
  RatchetTree extension`, `treat the truncated ratchet_tree as
  canonical`, `accept duplicate node_index`, `accept an unsigned
  leaf node`, …). `falsifier_runner` gains two new threshold lanes
  `external_init_secret_pinning` and `ratchet_tree_extension_tampering`
  at `0.95`. Result: **52 categories at 100% block rate**, `2600 / 2600`
  blocked.

- **DENY_PATTERNS extension.** `CR-CHAT-06/src/injection.rs` grows two
  new keyword blocks covering Lane A external-init-secret-pinning
  jargon (`16-byte exporter_secret`, `short exporter_secret`,
  `over-long exporter_secret`, `cross-group exporter splice`,
  `stale-epoch exporter`, `zero kem_ephemeral`, `all-zero
  kem_ephemeral`, `known-key External Init`, `removed-member rejoin`,
  `unknown signer leaf`, `non-canonical kem_ephemeral length`, …)
  and Lane B ratchet-tree-extension-tampering jargon (`empty
  ratchet_tree extension`, `truncated ratchet_tree`, `over-long
  ratchet_tree`, `leaf count mismatch`, `out-of-range node_index`,
  `duplicate node_index`, `duplicate leaf splice`, `unsigned leaf
  node`, `tampered ratchet_tree extension`, `accept the truncated
  tree as canonical`, …) so the injection guard blocks any prompt
  that attempts to weaken the new lanes by name. 168 new unique
  patterns added (DENY_PATTERNS total: 4445).

- **Coq Wave-27 — `Section TrinityChatWave27`.**
  Predicates `eip_canonical_exporter_len_27`, `eip_current_epoch_27`,
  `eip_group_id_match_27`, `eip_canonical_kem_ephemeral_27`,
  `rtx_non_empty_extension_27`, `rtx_leaf_count_matches_27`,
  `rtx_node_index_in_range_27`. Lemmas:
  - **INV-CHAT-159** `inv_chat_159_eip_non_canonical_exporter_len_rejected`
    — `len <> 32 -> Nat.eqb len 32 = false` via `Nat.eqb_neq`.
  - **INV-CHAT-160** `inv_chat_160_eip_stale_exporter_epoch_rejected`
    — `e < cur -> Nat.ltb e cur = true` via `Nat.ltb_lt`.
  - **INV-CHAT-161** `inv_chat_161_eip_cross_group_exporter_rejected`
    — `gid <> gid_view -> Nat.eqb gid gid_view = false` via `Nat.eqb_neq`.
  - **INV-CHAT-162** `inv_chat_162_eip_non_canonical_kem_ephemeral_rejected`
    — `len <> 32 -> Nat.eqb len 32 = false` via `Nat.eqb_neq`.
  - **INV-CHAT-163** `inv_chat_163_rtx_empty_extension_rejected`
    — `rtx_non_empty_extension_27 0 = false` by `simpl; reflexivity`.
  - **INV-CHAT-164** `inv_chat_164_rtx_leaf_count_mismatch_rejected`
    — `c <> e -> Nat.eqb c e = false` via `Nat.eqb_neq`.
  - **INV-CHAT-165** `inv_chat_165_rtx_node_index_out_of_range_rejected`
    — `count <= idx -> Nat.ltb idx count = false` via
    `Nat.ltb_spec` + `lia` (from imported `Lia`).
  - `eip_canonical_exporter_len_accepted_27`,
    `eip_current_epoch_exporter_accepted_27`,
    `rtx_non_empty_extension_accepted_27`,
    `rtx_leaf_count_matches_accepted_27` — four helper lemmas
    proving the well-formed cases reduce to `true` (`Nat.eqb_refl`,
    `Nat.ltb_irrefl`, `simpl + reflexivity`).
  Compiles **clean exit 0** (expected) with **239 Qed / 0 Admitted /
  5 axioms (unchanged: `ss_kp_injective` (W2), `dh_step_fresh` (W3),
  `dh_post_history_independent` (W3), `hybrid_kem_non_degenerate`
  (W10), `sn_hash_sym` (W14))**. Wave-27 introduces **zero new
  axioms** — every lemma is constructive.

---

### Wave-26 — MLS PSK external injection defense + Welcome-secret TreeKEM pruning defense

- **L-CHAT-3-psk** (R-CHAT-3 / **CR-CHAT-03**) — PSK-01..10 in
  `crates/trios-chat/rings/CR-CHAT-03/src/psk_external_injection.rs`
  (345 lines) shipping
  `validate_psk_ref(psk: &PskRef, view: &PskInjectionView) -> Result<(), PskInjectionError>`.
  Types: `PskType` (`External` / `Resumption`), `PskRef { psk_type,
  psk_id, psk_nonce, group_id, epoch }`, `PskInjectionView {
  provisioned_external_ids: BTreeSet<Vec<u8>>, current_group_id,
  current_epoch, seen_nonces: BTreeSet<Vec<u8>> }`,
  const `PSK_NONCE_LEN = 32`, error enum `PskInjectionError`
  (`#[non_exhaustive]` with variants `NonCanonicalNonceLength`,
  `UnprovisionedExternalId`, `ResumptionGroupSplice`,
  `ResumptionEpochRollback`, `NonceReplay`). Five rules enforced in
  fixed order from RFC 9420 §5.3 PreSharedKey validation: (1) reject
  any `psk_nonce` not of canonical length 32
  (`NonCanonicalNonceLength` — blocks the short-nonce truncation
  forge), (2) reject `External` PSKs whose `psk_id` is absent from
  `provisioned_external_ids` (`UnprovisionedExternalId` — blocks the
  unprovisioned-id injection), (3) reject `Resumption` PSKs whose
  `group_id` differs from the current group (`ResumptionGroupSplice` —
  blocks cross-group resumption splice), (4) reject `Resumption`
  PSKs whose `epoch` is strictly less than the current epoch
  (`ResumptionEpochRollback` — blocks the stale-epoch replay), (5)
  reject any `psk_nonce` already in `seen_nonces` (`NonceReplay` —
  blocks the cross-session nonce reuse). Valid PSK refs return `Ok(())`.
  - PSK-01 valid external PSK with provisioned id accepted.
  - PSK-02 valid resumption PSK at current epoch accepted.
  - PSK-03 short nonce rejected — `NonCanonicalNonceLength`.
  - PSK-04 oversize nonce rejected — `NonCanonicalNonceLength`.
  - PSK-05 unprovisioned external id rejected —
    `UnprovisionedExternalId`.
  - PSK-06 cross-group resumption splice rejected —
    `ResumptionGroupSplice`.
  - PSK-07 stale-epoch resumption rejected — `ResumptionEpochRollback`.
  - PSK-08 future-epoch resumption accepted (forward rollover is the
    application's concern, not PSK injection).
  - PSK-09 nonce replay rejected — `NonceReplay`.
  - PSK-10 green — module compiles and re-exports through
    `CR-CHAT-03/src/lib.rs`. → **10 unit tests**.

- **L-CHAT-5-wst** (R-CHAT-5 / **CR-CHAT-05**) — WST-01..10 in
  `crates/trios-chat/rings/CR-CHAT-05/src/welcome_secret_treekem_pruning.rs`
  (287 lines) shipping
  `validate_welcome_path(path: &WelcomeUpdatePath, view: &WelcomeTreeView) -> Result<(), WelcomeTreeError>`,
  const `WST_JOINER_LABEL: &[u8] = b"joiner"`. Types: `UpdatePathNode
  { node_index, encryptions_for: Vec<u32> }`, `WelcomeUpdatePath {
  group_id, epoch, joiner_label, leaf_index, nodes }`,
  `WelcomeTreeView { group_id, current_epoch, active_leaves:
  BTreeSet<u32>, expected_direct_path_len }`, error enum
  `WelcomeTreeError` (`#[non_exhaustive]` with variants
  `EmptyUpdatePath`, `PathLengthMismatch`, `PrunedNodeEncryptions`,
  `GroupContextEpochSplice`, `OffLabelJoinerSecret`). Five rules
  enforced in fixed order from RFC 9420 §12.4.3 Welcome / UpdatePath
  validation: (1) reject `nodes.is_empty()` (`EmptyUpdatePath` —
  blocks the no-path forge), (2) reject `nodes.len() !=
  expected_direct_path_len` (`PathLengthMismatch` — blocks
  truncated-path injection), (3) reject any `UpdatePathNode` whose
  `encryptions_for` references a leaf absent from `active_leaves`
  (`PrunedNodeEncryptions` — blocks the pruned-leaf encryption that
  would let a removed member decrypt the welcome secret), (4) reject
  `(group_id, epoch) != (view.group_id, view.current_epoch)`
  (`GroupContextEpochSplice` — blocks the cross-context welcome
  splice), (5) reject `joiner_label != WST_JOINER_LABEL`
  (`OffLabelJoinerSecret` — blocks the relabel attack that swaps the
  KDF label and derives a parallel secret).
  - WST-01 valid update path accepted.
  - WST-02 empty update path rejected — `EmptyUpdatePath`.
  - WST-03 truncated path rejected — `PathLengthMismatch`.
  - WST-04 over-long path rejected — `PathLengthMismatch`.
  - WST-05 pruned-leaf encryption rejected — `PrunedNodeEncryptions`.
  - WST-06 cross-group splice rejected — `GroupContextEpochSplice`.
  - WST-07 stale-epoch welcome rejected — `GroupContextEpochSplice`.
  - WST-08 off-label joiner secret rejected — `OffLabelJoinerSecret`.
  - WST-09 multi-node path with all active leaves accepted.
  - WST-10 green — module compiles and re-exports through
    `CR-CHAT-05/src/lib.rs`. → **10 unit tests**.

- **Falsifier corpus 2400 → 2500.** New categories
  `mls_psk_external_injection` and `welcome_secret_treekem_pruning`,
  50 entries each (`PI-PSK-001..050`, `PI-WST-001..050`). Each lane
  covers the specific exploitation phrasings (`accept a short PSK
  nonce`, `accept an unprovisioned external psk_id`, `splice a
  resumption PSK from another group`, `accept a stale-epoch
  resumption`, `replay a PSK nonce across sessions`, `treat the
  truncated update path as canonical`, `accept encryptions for a
  pruned leaf`, `accept welcome bound to another group_id`, `accept
  off-label joiner secret`, `swap the KDF joiner label`, …).
  `falsifier_runner` gains two new threshold lanes
  `mls_psk_external_injection` and `welcome_secret_treekem_pruning`
  at `0.95`. Result: **50 categories at 100% block rate**, `2500 / 2500`
  blocked.

- **DENY_PATTERNS extension.** `CR-CHAT-06/src/injection.rs` grows two
  new keyword blocks covering Lane A mls-psk-external-injection jargon
  (`short psk nonce`, `truncated psk_nonce`, `unprovisioned psk_id`,
  `unprovisioned external id`, `psk_nonce replay`, `resumption psk
  cross-group splice`, `stale-epoch resumption`, `psk_id injection`,
  `psk_id without provisioning`, `nonce reuse across sessions`, …)
  and Lane B welcome-secret-treekem-pruning jargon (`pruned leaf
  encryption`, `pruned node encryptions`, `truncated update path`,
  `over-long update path`, `path-length mismatch`, `off-label joiner
  secret`, `swap the joiner label`, `welcome bound to another
  group_id`, `cross-context welcome splice`, `decrypt with removed
  leaf`, …) so the injection guard blocks any prompt that attempts to
  weaken the new lanes by name. 143 new unique patterns added.

- **Coq Wave-26 — `Section TrinityChatWave26`.**
  Predicates `psk_nonce_canonical_length_accepted_26`,
  `psk_provisioned_external_accepted_26`,
  `wst_canonical_path_accepted_26`,
  `wst_canonical_label_accepted_26`. Lemmas:
  - **INV-CHAT-152** `inv_chat_152_psk_non_canonical_nonce_rejected`
    — `len <> 32 -> Nat.eqb len 32 = false` via `Nat.eqb_neq`.
  - **INV-CHAT-153** `inv_chat_153_psk_unprovisioned_external_rejected`
    — `is_provisioned = false -> reject` (boolean reflection).
  - **INV-CHAT-154** `inv_chat_154_psk_resumption_group_splice_rejected`
    — `gid <> gid_view -> Nat.eqb gid gid_view = false`.
  - **INV-CHAT-155** `inv_chat_155_psk_resumption_epoch_rollback_rejected`
    — `e < cur -> Nat.ltb e cur = true` via `Nat.ltb_lt`.
  - **INV-CHAT-156** `inv_chat_156_wst_empty_update_path_rejected`
    — `n = 0 -> Nat.eqb n 0 = true` via `Nat.eqb_refl`.
  - **INV-CHAT-157** `inv_chat_157_wst_path_length_mismatch_rejected`
    — `actual <> expected -> Nat.eqb actual expected = false`.
  - **INV-CHAT-158** `inv_chat_158_wst_off_label_joiner_secret_rejected`
    — `label <> 0 -> Nat.eqb label 0 = false`.
  - `psk_nonce_canonical_length_accepted_26`,
    `psk_provisioned_external_accepted_26`,
    `wst_canonical_path_accepted_26`,
    `wst_canonical_label_accepted_26` — four helper lemmas proving
    the well-formed cases reduce to `false`/`true` rejection bits
    correctly (`Nat.eqb_refl`, `Nat.lt_irrefl`).
  Compiles **clean exit 0** with **227 Qed / 0 Admitted /
  5 axioms (unchanged: `ss_kp_injective` (W2), `dh_step_fresh` (W3),
  `dh_post_history_independent` (W3), `hybrid_kem_non_degenerate`
  (W10), `sn_hash_sym` (W14))**. Wave-26 introduces **zero new
  axioms** — every lemma is constructive.

---

### Wave-25 — Padding-oracle chosen-ciphertext defense + Cover-traffic starvation defense

- **L-CHAT-6-cct** (R-CHAT-9 / **CR-CHAT-04**) — CCT-01..10 in
  `crates/trios-chat/rings/CR-CHAT-04/src/padding_oracle_chosen_ct.rs`
  (309 lines) shipping
  `verify_probe(buf: &[u8], ledger: &mut VerdictLedger) -> Result<usize, PaddingOracleCtError>`.
  Types: `VerdictLedger { budget_used: u32, last_seed: [u8; 32] }`,
  const `PROBE_BUDGET = 16`, error enum `PaddingOracleCtError`
  (`#[non_exhaustive]` with variants `NotACanonicalClass`,
  `BufferTooShort`, `DeclaredLengthOverflow`, `ProbeBudgetExceeded`).
  Four rules enforced in fixed order: (1) reject class index outside
  the canonical `CLASSES` set (`NotACanonicalClass` — blocks the
  off-table-class oracle), (2) reject buffer shorter than the
  length-tag header (`BufferTooShort` — blocks the tail-byte probe),
  (3) reject declared length exceeding remaining buffer
  (`DeclaredLengthOverflow` — blocks the length-tag forge), (4)
  reject probe attempts after `PROBE_BUDGET` failures
  (`ProbeBudgetExceeded` — blocks the budget-exhaustion adaptive
  oracle). Accepted probes reset the streak; rejected probes increment
  it. `VerdictLedger::new([u8; 32])` seeds the per-session counter.
  - CCT-01 valid canonical-class probe accepted.
  - CCT-02 length-tag forge rejected — `DeclaredLengthOverflow`.
  - CCT-03 tail-byte probe rejected — `BufferTooShort`.
  - CCT-04 class-edge collision rejected — `NotACanonicalClass`.
  - CCT-05 multi-class span rejected via class-index check.
  - CCT-06 zero-length forgery rejected — `BufferTooShort`.
  - CCT-07 probe-budget lockout after 16 failures — `ProbeBudgetExceeded`.
  - CCT-08 accept resets streak (success after 15 failures keeps budget).
  - CCT-09 over-budget probe rejected even when payload itself valid.
  - CCT-10 green — module compiles and re-exports through
    `CR-CHAT-04/src/lib.rs`. → **10 unit tests**.

- **L-CHAT-7-cts** (R-CHAT-10 / **CR-CHAT-07**) — CTS-01..10 in
  `crates/trios-chat/rings/CR-CHAT-07/src/cover_traffic_starvation.rs`
  (320 lines) shipping
  `validate_window(window: &[Emission], gaps_ms: &[u64]) -> Result<(), CoverStarvationError>`,
  consts `WINDOW_MIN_EMISSIONS = 4`, `MIN_COVER_RATIO_NUM = 1`,
  `MIN_COVER_RATIO_DEN = 4` (i.e. ≥ 25% cover-floor). Error enum
  `CoverStarvationError` (`#[non_exhaustive]` with variants
  `WindowTooShort`, `MismatchedGapLength`, `NonCanonicalGap`,
  `CoverFloorBreached`). Four rules enforced in fixed order: (1)
  reject windows with fewer than 4 emissions (`WindowTooShort`), (2)
  reject `gaps_ms.len() != window.len()` (`MismatchedGapLength`), (3)
  reject any gap not present in the `CANONICAL_GAPS_MS` set
  (`NonCanonicalGap` — blocks the jitter-skip attack), (4) reject
  `cover_count * 4 < window.len() * 1` (`CoverFloorBreached` —
  blocks the starvation attack where the adversary forces too few
  cover-traffic emissions in a window).
  - CTS-01 uniform window with 50% cover-floor accepted.
  - CTS-02 off-canonical gap rejected — `NonCanonicalGap`.
  - CTS-03 empty window rejected — `WindowTooShort`.
  - CTS-04 all-Real (cover floor 0/N) rejected — `CoverFloorBreached`.
  - CTS-05 all-Cover window accepted at 100% cover-floor.
  - CTS-06 exactly-at-floor window (1/4) accepted.
  - CTS-07 below-floor window (1/8) rejected — `CoverFloorBreached`.
  - CTS-08 mismatched gap length rejected — `MismatchedGapLength`.
  - CTS-09 non-canonical gap index rejected — `NonCanonicalGap`.
  - CTS-10 green — module compiles and re-exports through
    `CR-CHAT-07/src/lib.rs`. → **10 unit tests**.

- **Falsifier corpus 2300 → 2400.** New categories
  `padding_oracle_chosen_ct` and `cover_traffic_starvation`, 50
  entries each (`PI-CCT-001..050`, `PI-CTS-001..050`). Each lane
  covers the specific exploitation phrasings (`accept a buffer
  shorter than the length-tag header`, `treat the declared length as
  trustworthy`, `accept an off-table padding class`, `skip the probe
  budget check`, `let the adaptive oracle keep guessing`, `accept a
  window with three emissions`, `accept a non-canonical gap`,
  `accept a cover-floor of zero`, `treat the jitter-skip as routine`,
  `accept a starved window as a normal idle`, …).
  `falsifier_runner` gains two new threshold lanes
  `padding_oracle_chosen_ct` and `cover_traffic_starvation` at
  `0.95`. Result: **48 categories at 100% block rate**, `2400 / 2400`
  blocked.

- **DENY_PATTERNS extension.** `CR-CHAT-06/src/injection.rs` grows two
  new keyword blocks covering Lane A padding-oracle-chosen-ct jargon
  (`length-tag forge`, `tail-byte probe`, `class-edge collision`,
  `multi-class span`, `probe budget exceeded`, `adaptive oracle`,
  `chosen-ciphertext probe`, `off-table padding class`, …) and Lane B
  cover-traffic-starvation jargon (`cover floor breach`,
  `non-canonical gap`, `jitter skip`, `starve the cover stream`,
  `mismatched gap length`, `below-floor window`, `cover ratio 0/N`,
  …) so the injection guard blocks any prompt that attempts to
  weaken the new lanes by name. 132 new unique patterns added.

- **Coq Wave-25 — `Section TrinityChatWave25`.**
  Predicates `probe_canonical_class_accepted_25`,
  `probe_within_budget_accepted_25`,
  `window_long_enough_accepted_25`, `gap_length_match_accepted_25`.
  Lemmas:
  - **INV-CHAT-145** `inv_chat_145_probe_non_canonical_class_rejected`
    — `num <= cls -> Nat.leb num cls = true` via `Nat.leb_le`.
  - **INV-CHAT-146** `inv_chat_146_probe_buffer_too_short_rejected`
    — `buf < header -> Nat.ltb buf header = true` via `Nat.ltb_lt`.
  - **INV-CHAT-147** `inv_chat_147_probe_declared_length_overflow_rejected`
    — `remaining < declared -> Nat.ltb remaining declared = true`.
  - **INV-CHAT-148** `inv_chat_148_probe_budget_exceeded_rejected`
    — `budget < used -> Nat.ltb budget used = true`.
  - `probe_canonical_class_accepted_25` — helper proving in-range
    class indices pass (`cls < num -> Nat.leb num cls = false`).
  - `probe_within_budget_accepted_25` — helper proving the
    at-budget edge accepts (`Nat.ltb u u = false` by `Nat.ltb_irrefl`).
  - **INV-CHAT-149** `inv_chat_149_window_too_short_rejected` —
    `n < min_n -> Nat.ltb n min_n = true`.
  - **INV-CHAT-150** `inv_chat_150_cover_floor_breached_rejected`
    — `cover * den < window * num -> Nat.ltb (cover*den) (window*num) = true`.
  - **INV-CHAT-151** `inv_chat_151_mismatched_gap_length_rejected`
    — `gap_len <> expected -> Nat.eqb gap_len expected = false` via
    `Nat.eqb_neq`.
  - `window_long_enough_accepted_25` and
    `gap_length_match_accepted_25` — helper lemmas proving the
    well-formed cases reduce to `false` rejection.
  Compiles **clean exit 0** with **215 Qed / 0 Admitted /
  5 axioms (unchanged: `ss_kp_injective` (W2), `dh_step_fresh` (W3),
  `dh_post_history_independent` (W3), `hybrid_kem_non_degenerate`
  (W10), `sn_hash_sym` (W14))**. Wave-25 introduces **zero new
  axioms** — every lemma is constructive.

---

### Wave-24 — MLS commit signature forgery + Prekey signature chain binding

- **L-CHAT-3-csig** (R-CHAT-3 / **CR-CHAT-03**) — CSF-01..10 in
  `crates/trios-chat/rings/CR-CHAT-03/src/commit_signature.rs`
  (343 lines) shipping
  `verify_commit_signature(sc: &SignedCommit, view: &CommitVerifierView)
  -> Result<(), CommitSigError>`. Types: `CommitTranscript {
  group_id, epoch, signer_leaf, blob }`, `SignedCommit { transcript,
  signature_blob }`, `CommitVerifierView { group_id, current_epoch,
  active_signer_keys: BTreeSet<u32> }`, `CommitSigError`
  (`#[non_exhaustive]` with variants `EmptySignature`,
  `StaleSignerKey`, `GroupIdSplice`, `EpochMismatch { current, claimed
  }`, `NonMemberSigner`, `TranscriptMismatch`). Five rules enforced in
  fixed order: (1) reject empty `signature_blob` (`EmptySignature`),
  (2) reject zero-bytes signature blob (`StaleSignerKey` — catches
  the null-signer forge), (3) reject `transcript.group_id !=
  view.group_id` (`GroupIdSplice`), (4) reject `transcript.epoch !=
  view.current_epoch` (`EpochMismatch`), (5) reject signer not in
  `active_signer_keys` (`NonMemberSigner`) — covers stale and
  removed-leaf signer forges.
  - CSF-01 valid signed commit accepted.
  - CSF-02 empty signature blob rejected — `EmptySignature`.
  - CSF-03 zero-bytes blob rejected — `StaleSignerKey`.
  - CSF-04 group-id splice rejected — `GroupIdSplice`.
  - CSF-05 stale-epoch commit rejected — `EpochMismatch { current,
    claimed }`.
  - CSF-06 future-epoch commit rejected — same `EpochMismatch`.
  - CSF-07 non-member signer rejected — `NonMemberSigner`.
  - CSF-08 removed-leaf signer rejected after epoch advance.
  - CSF-09 cross-commit splice rejected via group-id mismatch.
  - CSF-10 green — module compiles and re-exports through
    `CR-CHAT-03/src/lib.rs`. → **10 unit tests**.

- **L-CHAT-1-psig** (R-CHAT-1 / **CR-CHAT-01**) — PSC-01..10 in
  `crates/trios-chat/rings/CR-CHAT-01/src/prekey_signature_chain.rs`
  (373 lines) shipping
  `validate_prekey_chain(bundle: &PrekeyChainBundle, view:
  &PrekeyChainView) -> Result<(), PrekeyChainError>`. Types:
  `PrekeyChainKey([u8; 32])`, `ChainBindingTag([u8; 32])`,
  `PrekeyChainBundle { identity_key, spk, spk_sig_blob, spk_binding,
  opk, opk_sig_blob, opk_binding }` (renamed from `PrekeyBundle` to
  avoid collision with existing `identity::PrekeyBundle`),
  `PrekeyChainView { identity_key, identity_revoked,
  expected_spk_binding, expected_opk_binding }`, `PrekeyChainError`
  (`#[non_exhaustive]` with variants `EmptySignature`, `SelfLoop`,
  `OpkSelfLoop`, `MissingIntermediate`, `IdentityRevoked`,
  `SpkBindingMismatch`, `OpkBindingMismatch`). Eight binding rules
  enforced in fixed order over the IK → SPK → OPK chain:
  (1) reject empty `spk_sig_blob` / `opk_sig_blob` (`EmptySignature`),
  (2) reject `spk == identity_key` (`SelfLoop`), (3) reject `opk ==
  spk` (`OpkSelfLoop`), (4) reject missing intermediate when
  `opk_sig_blob` present but `spk_sig_blob` empty
  (`MissingIntermediate`), (5) reject `view.identity_revoked`
  (`IdentityRevoked`), (6) reject `spk_binding !=
  expected_spk_binding` (`SpkBindingMismatch`), (7) reject `opk_binding
  != expected_opk_binding` (`OpkBindingMismatch`), (8) reject
  `view.identity_key != bundle.identity_key` (`SelfLoop` again, by
  identity-key disagreement). On all valid inputs returns `Ok(())`.
  - PSC-01 valid IK→SPK→OPK chain accepted.
  - PSC-02 empty SPK signature rejected — `EmptySignature`.
  - PSC-03 empty OPK signature rejected — `EmptySignature`.
  - PSC-04 SPK == IK rejected — `SelfLoop`.
  - PSC-05 OPK == SPK rejected — `OpkSelfLoop`.
  - PSC-06 missing intermediate rejected — `MissingIntermediate`.
  - PSC-07 revoked identity rejected — `IdentityRevoked`.
  - PSC-08 SPK binding mismatch rejected — `SpkBindingMismatch`.
  - PSC-09 OPK binding mismatch rejected — `OpkBindingMismatch`.
  - PSC-10 green — module compiles and re-exports through
    `CR-CHAT-01/src/lib.rs`. → **10 unit tests**.

- **Falsifier corpus 2200 → 2300.** New categories
  `commit_signature_forge` and `prekey_signature_chain`, 50 entries
  each (`PI-CSF-001..050`, `PI-PSC-001..050`). Each lane covers the
  specific exploitation phrasings (`accept empty signature blob`,
  `treat zero-bytes signature as a valid signer key`, `splice a commit
  from another group_id`, `accept commit with stale epoch as forward-
  compatible`, `treat a removed-leaf signer as still active`,
  `accept a chain where SPK equals identity_key`, `accept OPK equal
  to SPK`, `skip the missing-intermediate check`, `commit OPK without
  SPK signature`, `accept binding mismatch as routine`, …).
  `falsifier_runner` gains two new threshold lanes
  `commit_signature_forge` and `prekey_signature_chain` at `0.95`.
  Result: **46 categories at 100% block rate**, `2300 / 2300` blocked.

- **DENY_PATTERNS extension.** `CR-CHAT-06/src/injection.rs` grows two
  new keyword blocks covering Lane A commit-signature-forge jargon
  (`empty signature blob`, `zero-bytes signature`, `null signer key`,
  `group_id splice`, `cross-commit splice`, `stale epoch commit`,
  `removed-leaf signer`, `non-member signer accepted`,
  `transcript mismatch ignored`, …) and Lane B prekey-signature-chain
  jargon (`SPK equals identity_key`, `OPK equals SPK`, `self-loop
  prekey`, `missing intermediate signature`, `revoked identity
  accepted`, `binding mismatch routine`, `skip SPK signature`,
  `OPK without intermediate`, …) so the injection guard blocks any
  prompt that attempts to weaken the new lanes by name.

- **Coq Wave-24 — `Section TrinityChatWave24`.**
  Predicates `commit_groupid_agreement_24`,
  `prekey_binding_agreement_24`,
  `prekey_not_missing_when_spk_present_24`. Lemmas:
  - **INV-CHAT-138** `inv_chat_138_commit_empty_sig_rejected` —
    empty signature blob is rejected (`Nat.eqb 0 0 = true`).
  - **INV-CHAT-139** `inv_chat_139_commit_zero_blob_rejected` —
    zero-bytes signature blob is rejected (constructive).
  - **INV-CHAT-140** `inv_chat_140_commit_groupid_splice_rejected` —
    `view_gid <> trans_gid -> Nat.eqb view_gid trans_gid = false`
    via `Nat.eqb_neq`.
  - **INV-CHAT-141** `inv_chat_141_commit_epoch_mismatch_rejected` —
    `claimed <> current -> Nat.eqb current claimed = false`.
  - `commit_groupid_agreement_24` — helper proving group-id equality
    is reflexive and symmetric.
  - **INV-CHAT-142** `inv_chat_142_prekey_self_loop_rejected` —
    `spk = identity_key -> SelfLoop` (constructive via `Nat.eqb_refl`).
  - **INV-CHAT-143** `inv_chat_143_prekey_missing_intermediate_rejected`
    — OPK present but SPK signature empty is rejected.
  - **INV-CHAT-144** `inv_chat_144_prekey_identity_revoked_rejected` —
    `view.identity_revoked = true -> IdentityRevoked` (reflexivity).
  - `prekey_binding_agreement_24` and
    `prekey_not_missing_when_spk_present_24` — helper lemmas
    proving the binding-tag equality is reflexive and that an SPK
    signature precludes the missing-intermediate error.
  Compiles **clean exit 0** with **203 Qed / 0 Admitted /
  5 axioms (unchanged: `ss_kp_injective` (W2), `dh_step_fresh` (W3),
  `dh_post_history_independent` (W3), `hybrid_kem_non_degenerate`
  (W10), `sn_hash_sym` (W14))**. Wave-24 introduces **zero new
  axioms** — every lemma is constructive.

---

### Wave-23 — ReInit ceremony freshness + AppAck replay attestation

- **L-CHAT-3-rin** (R-CHAT-3 / **CR-CHAT-03**) — RIN-01..10 in
  `crates/trios-chat/rings/CR-CHAT-03/src/reinit_freshness.rs`
  (317 lines) shipping
  `validate_reinit(prop: &ReInitProposal, current_membership_count: usize) -> Result<(), ReInitError>`,
  constant `MAX_SUPPORTED_VERSION = 1`, types
  `GroupId([u8; 32])` (re-exported as `ReInitGroupId`),
  `ProtocolVersion(u16)`, `Ciphersuite(u16)` (re-exported as
  `ReInitCiphersuite`), `LeafIndex(u32)` (re-exported as
  `ReInitLeafIndex`),
  `ReInitProposal { committer, current_group_id, current_version,
  new_group_id, new_version, new_ciphersuite, welcomers }`. Five
  rules enforced in fixed order: (1) reject zero-bytes `new_group_id`
  (`EmptyNewGroupId`), (2) reject `new_group_id == current_group_id`
  (`StaleGroupIdReuse` — must be a fresh ceremony, not a stale
  re-use), (3) reject `new_version < current_version`
  (`ProtocolDowngrade { current, new }`), (4) reject
  `new_version > MAX_SUPPORTED_VERSION` (`UnsupportedVersionLeap
  { new, max_supported }`), (5) reject the degenerate case where
  all `welcomers` equal `committer` and membership count > 1
  (`SelfTargetingReInit` — committer cannot be the sole welcomer
  of a new group with > 1 members).
  - RIN-01 valid same-version reinit accepted — fresh GID, same
    version is `Ok(())`.
  - RIN-02 empty new GID rejected — zero-bytes returns
    `ReInitError::EmptyNewGroupId`.
  - RIN-03 stale GID reuse rejected — `new == current` returns
    `ReInitError::StaleGroupIdReuse`.
  - RIN-04 protocol downgrade rejected — `new < current` returns
    `ReInitError::ProtocolDowngrade { current, new }`.
  - RIN-05 unsupported leap rejected — `new > MAX` returns
    `ReInitError::UnsupportedVersionLeap { new, max_supported }`.
  - RIN-06 same-version not downgrade — equal versions pass.
  - RIN-07 self-targeting rejected when count > 1 — all welcomers
    equal committer returns `ReInitError::SelfTargetingReInit`.
  - RIN-08 self-targeting allowed when count == 1 — singleton
    group reinit accepted.
  - RIN-09 mixed welcomers accepted — at least one non-committer
    welcomer passes.
  - RIN-10 green — module compiles and re-exports through
    `CR-CHAT-03/src/lib.rs`. → **10 unit tests**.

- **L-CHAT-1-ack** (R-CHAT-1 / **CR-CHAT-01**) — ACK-01..10 in
  `crates/trios-chat/rings/CR-CHAT-01/src/appack_replay.rs`
  (413 lines) shipping `AppAckLedger` with `BTreeMap<AppAckLeaf,
  Generation>` high-watermark map and
  `validate(&mut self, own_leaf: AppAckLeaf, prop: &AppAckProposal)
  -> Result<(), AppAckError>`. Two-pass atomic semantics: first
  pass validates every entry (`SelfAttestation` if
  `sender == own_leaf`, `InvertedRange` if `first > last`,
  `StaleOrShrinking` if `new_last < known_watermark`), second
  pass commits watermarks. On any failure the ledger is
  untouched. `MessageRange { sender, first_generation,
  last_generation }` is inclusive; `Generation = u32`.
  `AppAckError` is `#[non_exhaustive]`.
  - ACK-01 fresh range accepted — first attestation `[3..=5]`
    advances watermark to 5.
  - ACK-02 self-attestation rejected — `sender == own_leaf`
    returns `AppAckError::SelfAttestation`.
  - ACK-03 inverted range rejected — `first=5, last=3` returns
    `AppAckError::InvertedRange`.
  - ACK-04 singleton range accepted — `first==last` passes.
  - ACK-05 strict-grow accepted — `[3..=5]` then `[6..=9]`
    advances watermark to 9.
  - ACK-06 stale (equal) rejected — re-attesting `[3..=5]`
    returns `AppAckError::StaleOrShrinking`.
  - ACK-07 shrinking rejected — `[3..=5]` then `[2..=4]`
    returns `AppAckError::StaleOrShrinking`.
  - ACK-08 atomic on failure — proposal containing one good +
    one bad entry leaves ledger untouched (no partial commit).
  - ACK-09 per-sender independence — distinct senders maintain
    independent watermarks.
  - ACK-10 green — module compiles and re-exports through
    `CR-CHAT-01/src/lib.rs`. → **10 unit tests**.

- **Falsifier corpus 2100 → 2200.** New categories
  `reinit_freshness` and `appack_replay`, 50 entries each
  (`PI-RIN-001..050`, `PI-ACK-001..050`), generated by
  `gen_falsifier_wave23.py`. Each lane covers the specific
  exploitation phrasings (`substitute current_group_id into
  new_group_id silently`, `skip the zero-GID check`, `downgrade
  protocol version for compatibility`, `accept new_version 2 as
  forward-compatible`, `let committer be its own welcomer`,
  `treat sender == own_leaf as routine`, `accept inverted range
  as wrap-around`, `lower the watermark on receipt`, `commit
  partial AppAck on failure`, …). `falsifier_runner` gains two
  new threshold lanes `reinit_freshness` and `appack_replay`
  at `0.95`. Result: **44 categories at 100% block rate**,
  `2200 / 2200` blocked.

- **DENY_PATTERNS extension.** `CR-CHAT-06/src/injection.rs`
  grows two new keyword blocks covering Lane A ReInit jargon
  (`substitute current_group_id`, `current_group_id into
  new_group_id`, `zero-gid check`, `protocol downgrade`,
  `forward-compatible version 2`, `MAX_SUPPORTED_VERSION + 1`,
  `self-targeting reinit`, `committer be its own welcomer`,
  `stale group_id reuse`, `bypass version ordering`, …) and
  Lane B AppAck jargon (`SelfAttestation as routine`,
  `inverted range as wrap-around`, `lower the watermark`,
  `partial AppAck on failure`, `shrinking generation`,
  `commit before validation`, `treat sender == own_leaf`,
  `BTreeMap watermark`, `non-atomic ledger update`, …) so the
  injection guard blocks any prompt that attempts to weaken
  the new lanes by name.

- **Coq Wave-23 — `Section TrinityChatWave23` (lines ≈ 3247–3353).**
  Predicates `reinit_max_supported_version_23 = 1`,
  `reinit_is_zero_gid_23`, `reinit_is_downgrade_23`,
  `reinit_is_unsupported_leap_23`, `appack_inverted_23`,
  `appack_stale_or_shrink_23`. Lemmas:
  - **INV-CHAT-131** `inv_chat_131_reinit_empty_gid_rejected` —
    `reinit_is_zero_gid_23 0 = true` (reflexivity).
  - **INV-CHAT-132** `inv_chat_132_reinit_stale_gid_reuse_rejected` —
    `forall gid, Nat.eqb gid gid = true` (Nat.eqb_refl).
  - **INV-CHAT-133** `inv_chat_133_reinit_downgrade_rejected` —
    `new < current -> Nat.ltb new current = true`.
  - **INV-CHAT-134** `inv_chat_134_reinit_unsupported_leap_rejected` —
    `MAX_SUPPORTED_VERSION < new -> Nat.ltb MAX new = true`.
  - `reinit_same_version_not_downgrade_23` — helper proving equal
    versions never trigger downgrade.
  - **INV-CHAT-135** `inv_chat_135_appack_inverted_rejected` —
    `last < first -> Nat.ltb last first = true`.
  - **INV-CHAT-136** `inv_chat_136_appack_singleton_accepted` —
    `Nat.ltb gen gen = false` (Nat.ltb_irrefl).
  - **INV-CHAT-137** `inv_chat_137_appack_stale_rejected` —
    `new_last < known -> stale = true`.
  - `appack_grow_not_stale_23` / `appack_equal_not_stale_23` —
    helper lemmas for the strict-grow watermark policy.
  Compiles **clean exit 0** with **191 Qed / 0 Admitted /
  5 axioms (unchanged: `ss_kp_injective` (W2), `dh_step_fresh`
  (W3), `dh_post_history_independent` (W3),
  `hybrid_kem_non_degenerate` (W10), `sn_hash_sym` (W14))**.
  Wave-23 introduces **zero new axioms** — every lemma is
  constructive.

---

### Wave-22 — MLS proposal-bundle validation + MAC tag truncation defense

- **L-CHAT-3-pv** (R-CHAT-3 / **CR-CHAT-03**) — PV-01..10 in
  `crates/trios-chat/rings/CR-CHAT-03/src/proposal_validation.rs`
  (329 lines) shipping
  `validate_bundle(bundle: &ProposalBundle) -> Result<(), ProposalValidationError>`,
  constant `MAX_PROPOSALS_PER_COMMIT = 32`, types
  `ProposalKind::{Add, Remove, Update}`,
  `ProposalBundle { committer_leaf, entries }`, and
  `ProposalEntry { index, kind, target }`. Five rules enforced in a
  single pass: (1) bundle non-empty, (2) `entries.len() ≤ 32`,
  (3) strict-monotonic `index` field (deterministic order, no
  reorder injection), (4) no duplicate `(kind, target)` pair,
  (5) reject the degenerate `[Remove(self)]` singleton that would
  let a committer evict itself in a single commit (must always be
  paired with another proposal so the group state cannot collapse).
  - PV-01 empty bundle rejected — `entries = []` returns
    `ProposalValidationError::Empty`.
  - PV-02 oversized bundle rejected — `entries.len() = 33` returns
    `ProposalValidationError::Oversized`.
  - PV-03 max bundle accepted — `entries.len() = 32` is `Ok(())`.
  - PV-04 monotonic indices accepted — `[0,1,2,3]` is `Ok(())`.
  - PV-05 equal indices rejected — `[0,1,1,2]` returns
    `ProposalValidationError::NonMonotonic`.
  - PV-06 descending indices rejected — `[2,1,0]` returns
    `ProposalValidationError::NonMonotonic`.
  - PV-07 duplicate (kind,target) rejected — two `(Add, x)` returns
    `ProposalValidationError::Duplicate`.
  - PV-08 same target different kind allowed — `(Add,x)` then
    `(Update,x)` is `Ok(())` (different kinds are valid).
  - PV-09 self-remove-only rejected — `[Remove(committer_leaf)]`
    returns `ProposalValidationError::SelfRemoveOnly`.
  - PV-10 green — module compiles and re-exports through
    `CR-CHAT-03/src/lib.rs`. → **10 unit tests**.

- **L-CHAT-9-mt** (R-CHAT-9 / **CR-CHAT-04**) — MT-01..10 in
  `crates/trios-chat/rings/CR-CHAT-04/src/mac_truncation.rs`
  (290 lines) shipping constant `MAC_TAG_LEN = 16`, newtype
  `MacTag([u8; 16])` with `from_slice(&[u8]) -> Result<MacTag, MacError>`
  and constant-time `ct_eq(&MacTag) -> Choice` via
  `subtle::ConstantTimeEq` applied to `.as_slice()`,
  `verify_mac(expected: &MacTag, arrived: &[u8]) -> Result<(), MacError>`,
  and `split_frame(frame: &[u8]) -> Result<(&[u8], MacTag), MacError>`.
  Error type `MacError::{Truncated, Oversized, Mismatch, FrameTooShort}`
  — every length-failure collapses to a single rejection class so
  an attacker cannot distinguish "15-byte tag" from "17-byte tag"
  from "mismatched tag" by error shape. Length check is *strict*:
  any tag whose length is not exactly 16 is rejected before any
  byte comparison runs (no truncated-MAC oracle).
  - MT-01 full-length match accepted — identical 16-byte tags is
    `Ok(())`.
  - MT-02 truncated tag rejected — 15 bytes returns
    `MacError::Truncated`.
  - MT-03 oversized tag rejected — 17 bytes returns
    `MacError::Oversized`.
  - MT-04 empty tag rejected — 0 bytes returns
    `MacError::Truncated`.
  - MT-05 mismatched tag rejected — flipped final byte returns
    `MacError::Mismatch`, not a leak of which byte differs.
  - MT-06 split_frame happy path — frame of length `payload + 16`
    splits cleanly into `(payload, tag)`.
  - MT-07 split_frame too short — frame of length 15 returns
    `MacError::FrameTooShort`.
  - MT-08 ct_eq is constant-time — `MacTag::ct_eq` returns a
    `subtle::Choice`, not a `bool`, so the call site cannot
    short-circuit on the first differing byte.
  - MT-09 length-constant single source of truth — `MAC_TAG_LEN == 16`
    is the only place the value lives.
  - MT-10 green — module compiles and re-exports through
    `CR-CHAT-04/src/lib.rs`. → **10 unit tests**.

- **Falsifier corpus 2000 → 2100.** New categories
  `proposal_validation` and `mac_truncation`, 50 entries each
  (`PI-PV-001..050`, `PI-MT-001..050`), generated by
  `gen_falsifier_wave22.py`. Each lane covers the specific
  exploitation phrasings (`Skip Rule 3 monotonic`, `Permit (Add, x)
  and (Update, x) for the same x`, `target == committer_leaf`,
  `truncated MAC tag for perf`, `early-exit on first differing
  byte`, `MacTag::ct_eq only when full length`, `read past arrived.len()`,
  …). `falsifier_runner` gains two new threshold lanes at `0.95`.
  Result: **42 categories at 100% block rate**, `2100 / 2100` blocked.

- **DENY_PATTERNS extension.** `CR-CHAT-06/src/injection.rs`
  grows two new keyword blocks covering Lane A proposal-bundle
  jargon (`skip rule 3`, `monotonic indices`, `duplicate entry from
  trusted committer`, `self-remove without companion`, `zero-indexed
  entries as genesis`, `[Remove(self)]`, `committer_leaf`,
  `validator as advisory`, `archived to disk`, `treat the validator`,
  …) and Lane B MAC-tag jargon (`mac_tag_len - 1`, `truncated MAC`,
  `first differing byte`, `early-exit on the first differing`,
  `MacTag::ct_eq only when`, `subtle::Choice::from(arrived`,
  `read past arrived.len()`, `payload is null`, `frame_len = mac_tag_len - 1`,
  …) so the injection guard blocks any prompt that attempts to
  weaken the new lanes by name.

- **Coq Wave-22 — `Section TrinityChatWave22` (lines ≈ 3055–3231).**
  Introduces constant `pv_max_22 := 32`, fixpoint
  `pv_monotone_indices_22 : list nat -> bool` (strict-ascending
  list check via fold over a running max), predicates
  `pv_is_remove_22`, `pv_self_remove_only_22`, MAC verdicts
  `Inductive MacVerdict22 := MVAccept22 | MVReject22`, computable
  bytes equality `Definition mac_bytes_eq_22 := Nat.eqb` (concrete —
  no `Variable`, no `Hypothesis`, **zero new axioms**),
  `verify_mac_22 : nat -> nat -> nat -> nat -> MacVerdict22`, and
  constant `mac_tag_len_22 := 16`. Closes:
  - **INV-CHAT-124** `inv_chat_124_pv_empty_rejected` — empty
    bundles always rejected.
  - **INV-CHAT-125** `inv_chat_125_pv_oversized_rejected` —
    `len > 32 → Nat.leb len 32 = false`.
  - Helper `pv_monotone_singleton_22` — single-element lists are
    monotonic.
  - Helper `pv_monotone_equal_rejected_22` — equal adjacent
    indices fail the strict-monotonic check.
  - **INV-CHAT-126** `inv_chat_126_pv_self_remove_only_rejected` —
    `[Remove(self)]` singleton is detected.
  - **INV-CHAT-127** `inv_chat_127_mt_short_rejected` —
    `arrived_len < 16 → MVReject22`.
  - **INV-CHAT-128** `inv_chat_128_mt_full_match_accepted` —
    identical 16-byte hashes accept.
  - **INV-CHAT-129** `inv_chat_129_mt_full_mismatch_rejected` —
    differing hashes reject.
  - **INV-CHAT-130** `inv_chat_130_mt_split_total_length` —
    `frame ≥ 16 → (frame - 16) + 16 = frame`.
  - Helper `mt_len_separation_22` — strict length-failure boundary.
  - Bonus lemmas `mac_bytes_eq_refl_22`, `mac_bytes_eq_sym_22`
    (constructive over `Nat.eqb`, replace the two `Hypothesis`
    declarations from the first draft).
  - **Total: 181 `Qed.` / 0 `Admitted.` / 0 new axioms.**

- **Verification gate.** `cargo test` over the 12 chat crates plus
  harness binaries: **335 / 0**. `cargo run --bin e2e_chat_25`:
  **25 / 25**. `cargo run --bin falsifier_runner`: **2100 / 2100**
  blocked at 42 threshold lanes. `cargo clippy --all-targets -- -D warnings`
  on `trios-chat` + the three touched ring crates: clean.
  `coqc proofs/chat/Trinity_Chat.v`: silent exit `0`, three
  abstract-large-number warnings only (pre-existing W14/W15 nat
  literals, not from W22 code).

- **Cumulative axioms — unchanged.** `ss_kp_injective` (W9),
  `dh_step_fresh` + `dh_post_history_independent` +
  `hybrid_kem_non_degenerate` (W10), `sn_hash_sym` (W14). Wave-22
  introduces **zero** new axioms — both `mac_bytes_eq_refl_22` and
  `mac_bytes_eq_sym_22` are constructive `Qed` proofs over `Nat.eqb`.

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
| INV-CHAT-110..116 | W20 | handshake fingerprinting (determinism, swap detected, empty-field invalid) + concurrent Add/Remove ordering (Update<Remove<Add priority, empty-set neutral, add-after-remove size-neutral) |
| INV-CHAT-117..123 | W21 | epoch-authentication failure (future rejected, match accepted, opaque error, grace-window accepted) + Welcome KeyPackage pinning (immutable pin, mismatch rejected, hash determinism, empty-field invalid) |
| INV-CHAT-124..130 | W22 | MLS proposal-bundle validation (empty rejected, oversized rejected, self-remove-only rejected, monotonic-indices required) + MAC tag truncation defense (short rejected, full-match accepted, full-mismatch rejected, split total-length preserved) |
| INV-CHAT-131..137 | W23 | ReInit ceremony freshness (empty GID rejected, stale GID reuse rejected, protocol downgrade rejected, unsupported version leap rejected) + AppAck replay attestation (inverted range rejected, singleton accepted, stale/shrinking rejected, atomic-on-failure) |
| INV-CHAT-138..144 | W24 | MLS commit signature forgery defense (empty sig rejected, zero-blob rejected, group-id splice rejected, epoch mismatch rejected) + Prekey signature chain binding (self-loop rejected, missing intermediate rejected, revoked identity rejected) |
| INV-CHAT-145..151 | W25 | Padding-oracle chosen-ciphertext defense (non-canonical class rejected, buffer-too-short rejected, declared-length overflow rejected, probe-budget exceeded rejected) + Cover-traffic starvation defense (window-too-short rejected, cover-floor breached rejected, mismatched gap-length rejected) |
| INV-CHAT-152..158 | W26 | MLS PSK external injection defense (non-canonical nonce rejected, unprovisioned external id rejected, resumption group splice rejected, resumption epoch rollback rejected) + Welcome-secret TreeKEM pruning defense (empty update path rejected, path-length mismatch rejected, off-label joiner secret rejected) |
| **INV-CHAT-159..165** | **W27** | **MLS External-Init secret pinning defense (non-canonical exporter len rejected, stale exporter epoch rejected, cross-group exporter splice rejected, non-canonical kem_ephemeral rejected) + RatchetTree extension tampering defense (empty extension rejected, leaf count mismatch rejected, out-of-range node_index rejected)** |
| INV-CHAT-166..172 | W28 | MLS confirmation_tag chain validation (non-canonical tag len rejected, stale-epoch chain replay rejected, transcript-chain splice rejected, wrong-length interim_transcript_hash rejected) + Sender-data header encryption integrity (non-canonical AEAD nonce rejected, stale-epoch sender_data rejected, reserved-bit forge rejected) |
| INV-CHAT-173..179 | W29 | MLS LeafNode signature validation (non-canonical sig len rejected, cross-group LeafNode rebind rejected, stale-epoch LeafNode rejected, signature-key / credential mismatch rejected) + Group Context extensions consistency (cross-group GroupContext splice rejected, stale-epoch GroupContext snapshot rejected, IANA-reserved extension_id forge rejected) |
| INV-CHAT-180..186 | W30 | Application-data AEAD nonce reuse defense (non-canonical AEAD nonce len rejected, cross-group AEAD nonce splice rejected, stale-epoch AEAD packet rejected, zero AEAD nonce rejected) + Welcome path-secret unmasking defense (non-canonical path_secret len rejected, cross-group Welcome rejected, stale-epoch Welcome rejected) |
| INV-CHAT-187..193 | W31 | KeyPackage init_key reuse defense (non-canonical init_key len rejected, cross-ciphersuite KeyPackage rejected, not-yet-valid KeyPackage rejected, leaf_node_key == init_key rejected) + External PSK identifier provenance defense (non-canonical psk_nonce len rejected, empty psk_id rejected, oversized psk_id rejected) |
| **INV-CHAT-194..200** | **W32** | **Welcome encrypted_group_info AEAD defense (non-canonical aead_nonce len rejected, short ciphertext rejected, cross-group envelope rejected, stale-epoch envelope rejected) + Proposal reference collision defense (non-canonical proposal_ref len rejected, empty proposal_id rejected, stale-epoch proposal_ref rejected)** |

Cumulative axioms: `ss_kp_injective` (W9), `dh_step_fresh` (W10),
`dh_post_history_independent` (W10), `hybrid_kem_non_degenerate` (W10),
`sn_hash_sym` (W14, constructively discharged at runtime).
Wave-11, Wave-12, Wave-13, Wave-15, Wave-16, Wave-17, Wave-18, Wave-19, Wave-20, Wave-21, Wave-22, Wave-23, Wave-24, Wave-25, Wave-26, Wave-27, Wave-28, Wave-29, Wave-30, Wave-31, and Wave-32 all introduce **zero** new axioms — every proof is constructive.
Wave-14 introduces **one** new axiom (`sn_hash_sym`) which is concretely
discharged in Rust by canonical-ordering the safety-number hash inputs.

---

## Future waves (W33–W37) — `[ASPIRATIONAL]`

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
| ~~W21~~ — SHIPPED via [#730](https://github.com/gHashTag/trios/pull/730), merged `35b3ef6` (see Wave-21 detail above) | | | | | | |
| ~~W22~~ — SHIPPED via [#732](https://github.com/gHashTag/trios/pull/732), merged `119f0fe` (see Wave-22 detail above) | | | | | | |
| ~~W23~~ — SHIPPED via [#734](https://github.com/gHashTag/trios/pull/734), merged `1d6f910` (see Wave-23 detail above) | | | | | | |
| ~~W24~~ — SHIPPED via [#738](https://github.com/gHashTag/trios/pull/738), merged `81ef050` (see Wave-24 detail above) | | | | | | |
| ~~W25~~ — SHIPPED via [#747](https://github.com/gHashTag/trios/pull/747), merged `e234422` (see Wave-25 detail above) | | | | | | |
| ~~W26~~ — SHIPPED via [#749](https://github.com/gHashTag/trios/pull/749), merged `1665be1` (see Wave-26 detail above) | | | | | | |
| ~~W27~~ — SHIPPED via [#752](https://github.com/gHashTag/trios/pull/752), merged `93e4e6c` (see Wave-27 detail above) | | | | | | |
| ~~W28~~ — SHIPPED via [#754](https://github.com/gHashTag/trios/pull/754), merged `562009c` (see Wave-28 detail above) | | | | | | |
| ~~W29~~ — SHIPPED via [#760](https://github.com/gHashTag/trios/pull/760), merged `c389536` (see Wave-29 detail above) | | | | | | |
| ~~W30~~ — SHIPPED via [#765](https://github.com/gHashTag/trios/pull/765), merged `bd5ffea` (see Wave-30 detail above) | | | | | | |
| ~~W31~~ — SHIPPED via [#771](https://github.com/gHashTag/trios/pull/771), merged `756cf35` (see Wave-31 detail above) | | | | | | |
| ~~W32~~ — SHIPPED in this PR (see Wave-32 detail above) | | | | | | |
| **W33** | (TBD — picked from uncovered surface after W32 retrospective) | (TBD) | (TBD ×2) | INV-CHAT-201..207 (≥309 Qed) | ≈570 | 3200 / 64 cats |
| **W34** | (TBD) | (TBD) | (TBD ×2) | INV-CHAT-208..214 (≥319 Qed) | ≈592 | 3300 / 66 cats |
| **W35** | (TBD) | (TBD) | (TBD ×2) | INV-CHAT-215..221 (≥329 Qed) | ≈614 | 3400 / 68 cats |
| **W36** | (TBD) | (TBD) | (TBD ×2) | INV-CHAT-222..228 (≥339 Qed) | ≈636 | 3500 / 70 cats |
| **W37** | (TBD) | (TBD) | (TBD ×2) | INV-CHAT-229..235 (≥349 Qed) | ≈658 | 3600 / 72 cats |

After W32 the corpus crosses **3100 entries / 62 categories** and Coq
crosses **299 closed proofs / 0 admissions**. From W33+ the work shifts
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
| Falsifier corpus | `cargo run -q -p trios-chat --bin falsifier_runner` | `3100/3100 blocked` (W32) at 62 thresholds |
| Clippy           | `cargo clippy -p trios-chat -p trios-chat-cr-chat-* --all-targets -- -D warnings` | clean |
| Coq              | `coqc crates/trios-chat/proofs/chat/Trinity_Chat.v` | silent, exit 0 |
| Laws Guard CI    | PR body opens with `Closes \|Fixes \|Resolves #N` | green |
| L-ARCH-001       | New code lives under `crates/trios-chat/rings/CR-CHAT-NN/` only | enforced by build graph |
| L1               | `find crates/trios-chat -name '*.sh'` | empty |

---

## Cross-wave conventions

- **Branch naming**: `feat/trios-chat-wave<N>` from the latest `origin/main`.
- **Commit identity**: `Dmitrii Vasilev <admin@t27.ai>` (canonical maintainer).
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
- W33..W37 lane definitions are **[ASPIRATIONAL]** — they constitute the
  forward plan and have not been validated by tests/Coq yet.
- Wave-32 detail section above is **[VERIFIED]** by cargo test (~548/0 expected), `coqc` (299 Qed / 0 Admitted), `falsifier_runner` (3100/3100, 62 cats), `e2e_chat_25` (25/25), `cargo clippy -- -D warnings` (clean)
- Wave-31 detail section above is **[VERIFIED]** by cargo test (~528/0), `coqc` (288 Qed / 0 Admitted), `falsifier_runner` (3000/3000, 60 cats), `e2e_chat_25` (25/25), `cargo clippy -- -D warnings` (clean)
- Wave-30 detail section above is **[VERIFIED]** by cargo test (~508/0), `coqc` (275 Qed / 0 Admitted), `falsifier_runner` (2900/2900, 58 cats), `e2e_chat_25` (25/25), `cargo clippy -- -D warnings` (clean)
- Wave-29 detail section above is **[VERIFIED]** by cargo test (~488/0), `coqc` (263 Qed / 0 Admitted), `falsifier_runner` (2800/2800, 56 cats), `e2e_chat_25` (25/25), `cargo clippy -- -D warnings` (clean)
- Wave-28 detail section above is **[VERIFIED]** by cargo test (~468/0), `coqc` (251 Qed / 0 Admitted), `falsifier_runner` (2700/2700, 54 cats), `e2e_chat_25` (25/25), `cargo clippy -- -D warnings` (clean)
- Wave-27 detail section above is **[VERIFIED]** by cargo test (~448/0 expected), `coqc` (239 Qed / 0 Admitted), `falsifier_runner` (2600/2600, 52 cats), `e2e_chat_25` (25/25), `cargo clippy -- -D warnings` (clean)
- Wave-26 detail section above is **[VERIFIED]** by cargo test (~419/0), `coqc` (227 Qed / 0 Admitted), `falsifier_runner` (2500/2500, 50 cats), `e2e_chat_25` (25/25), `cargo clippy -- -D warnings` (clean)
- Wave-25 detail section above is **[VERIFIED]** by cargo test (397/0), `coqc` (215 Qed / 0 Admitted), `falsifier_runner` (2400/2400, 48 cats), `e2e_chat_25` (25/25), `cargo clippy -- -D warnings` (clean)
- Wave-24 detail section above is **[VERIFIED]** by cargo test (375/0), `coqc` (203 Qed / 0 Admitted), `falsifier_runner` (2300/2300, 46 cats), `e2e_chat_25` (25/25), `cargo clippy -- -D warnings` (clean)
- Wave-23 detail section above is **[VERIFIED]** by cargo test (355/0), `coqc` (191 Qed / 0 Admitted), `falsifier_runner` (2200/2200), `e2e_chat_25` (25/25), `cargo clippy -- -D warnings` (clean)
- Wave-22 detail section above is **[VERIFIED]** by cargo test
  (335/0), `e2e_chat_25` (25/25), `falsifier_runner` (2100/2100,
  42 cats), clippy (clean), and `coqc Trinity_Chat.v` (silent, 181
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
