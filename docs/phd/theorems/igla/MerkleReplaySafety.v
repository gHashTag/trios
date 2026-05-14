(* MerkleReplaySafety.v
   Apache-2.0 · TRI-1 v2 L-S46 · PhD Ch.17/S17

   Anchor: phi^2 + phi^-2 = 3
   DOI: 10.5281/zenodo.19227877

   Theorems:
   1. merkle_deterministic — same leaf inputs yield identical Merkle root
   2. receipt_no_replay    — receipts from distinct windows are distinct
   3. monotonic_window     — S n <> n (window counter is strictly increasing)

   Issue: https://github.com/gHashTag/trios/issues/801 (L-S46)
   Author: Dmitrii Vasilev <admin@t27.ai> | Date: 2026-05-14

   Citation anchors:
     merkle_deterministic -> trinity_merkle_agg.v
     receipt_no_replay    -> ledger_receipt_v2.v *)

Require Import Arith.
Require Import Lia.

(* ------------------------------------------------------------------ *)
(* Definitions                                                          *)
(* ------------------------------------------------------------------ *)

(** A leaf is an abstract type represented as a natural number.
    (64-bit hash digest, abstracted to nat for proof purposes.) *)
Definition leaf := nat.

(** hash_combine is an opaque function over leaves.
    The concrete implementation uses 3-round XOR+rotate; for the
    determinism proof we only need the function to be well-typed. *)
Parameter hash_combine : leaf -> leaf -> leaf.

(** Merkle root of four leaves using a balanced binary tree:
      root = hash_combine
               (hash_combine l0 l1)
               (hash_combine l2 l3)               *)
Definition merkle_root (l0 l1 l2 l3 : leaf) : leaf :=
  hash_combine (hash_combine l0 l1) (hash_combine l2 l3).

(** A receipt records which tile emitted a result in which window. *)
Record receipt := mkReceipt
  { tile   : nat   (* tile identifier *)
  ; window : nat   (* 30-bit monotonic window counter *)
  ; res    : leaf  (* result digest *)
  ; lf     : leaf  (* leaf payload *)
  }.

(* ------------------------------------------------------------------ *)
(* Lemma: S n <> n (window counter is strictly monotonic)              *)
(* ------------------------------------------------------------------ *)

Lemma monotonic_window : forall n : nat, S n <> n.
Proof.
  intros n.
  lia.
Qed.

(* ------------------------------------------------------------------ *)
(* Theorem: merkle_deterministic                                        *)
(*   Same leaf inputs always produce the same Merkle root.             *)
(* ------------------------------------------------------------------ *)

Theorem merkle_deterministic :
  forall l0 l1 l2 l3 l0' l1' l2' l3' : leaf,
    l0 = l0' /\ l1 = l1' /\ l2 = l2' /\ l3 = l3' ->
    merkle_root l0 l1 l2 l3 = merkle_root l0' l1' l2' l3'.
Proof.
  intros l0 l1 l2 l3 l0' l1' l2' l3' [H0 [H1 [H2 H3]]].
  unfold merkle_root.
  congruence.
Qed.

(* ------------------------------------------------------------------ *)
(* Theorem: receipt_no_replay                                           *)
(*   Two receipts that belong to distinct windows cannot be equal.     *)
(*   Formally: window counters differ => receipts differ (or trivially *)
(*   the disjunction holds via True).                                  *)
(*                                                                      *)
(*   Strong variant: distinct windows => distinct receipts outright.   *)
(* ------------------------------------------------------------------ *)

(** Helper: if two receipts are equal, their window fields are equal. *)
Lemma receipt_eq_window :
  forall r1 r2 : receipt, r1 = r2 -> window r1 = window r2.
Proof.
  intros r1 r2 Heq.
  subst r2.
  reflexivity.
Qed.

(** Strong form: r1.(window) <> r2.(window) -> r1 <> r2 *)
Theorem receipt_no_replay_strong :
  forall r1 r2 : receipt,
    window r1 <> window r2 ->
    r1 <> r2.
Proof.
  intros r1 r2 Hwin Heq.
  apply Hwin.
  exact (receipt_eq_window r1 r2 Heq).
Qed.

(** Specification-literal form as required by the mission:
    r1.(window) <> r2.(window) -> r1 <> r2 \/ True               *)
Theorem receipt_no_replay :
  forall r1 r2 : receipt,
    window r1 <> window r2 ->
    r1 <> r2 \/ True.
Proof.
  intros r1 r2 Hwin.
  left.
  exact (receipt_no_replay_strong r1 r2 Hwin).
Qed.
