# RING — CR-CHAT-03 (trios-chat)

## Identity

| Field   | Value |
|---------|-------|
| Tier    | 🥈 Silver (Core ring) |
| Package | `trios-chat-cr-chat-03` |
| Path    | `crates/trios-chat/rings/CR-CHAT-03/` |
| Sealed  | No |

## Purpose

MLS group skeleton — implements **R-CHAT-11** (strict epoch
monotonicity, replay-resistance for group-level commits) on a tiny
deterministic state machine. Concrete RFC 9420 implementation
(`openmls`) is `[ASPIRATIONAL]` and lands behind a feature flag in a
follow-up PR.

## Public API

| Item | Role |
|---|---|
| `GroupId([u8;32])`  | random group identifier |
| `Epoch(u64)`        | strictly-monotone epoch counter |
| `LeafIndex(u32)`    | ratchet-tree leaf |
| `Op { Add, Remove, Update }` | one MLS proposal kind |
| `Welcome` / `Commit` | wire messages |
| `Group::create` | new group with one founding member |
| `Group::process_commit` | apply commit; enforces epoch + group + sender invariants |
| `Group::welcome_for` | mint a `Welcome` packet |

## Dependencies

| Dep | Why |
|---|---|
| `trios-chat-cr-chat-00` | `Error`, `Result` |
| `serde`          | Wire format for Welcome/Commit |

No async, no crypto, no I/O.

## Invariants (R-CHAT-11)

- **Group match**: `commit.group_id == self.group_id` or reject.
- **Epoch match**: `commit.from_epoch == self.epoch` or reject.
- **Sender membership**: `commit.sender ∈ self.members` or reject.
- **Strict monotonicity**: `process_commit` advances `epoch` by exactly one.

## Tests

7 unit tests — happy path, two replay/fork falsifiers, non-member
falsifier, wrong-group falsifier, remove flow, welcome carries
current epoch.

## Sibling Bronze

None — group state is in-memory by design at this stage. Persistence
is a Wave-5 question once `openmls` lands.

## Anchor

`φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
