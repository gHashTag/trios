(* SPDX-License-Identifier: Apache-2.0                                        *)
(* Author: Dmitrii Vasilev <admin@t27.ai>                                      *)
(* PhiPriorQuantCorrect.v — Formal correctness proof for L-S37                 *)
(*   φ-prior on-chip weight quantizer (FP Q1.15 → ternary)                    *)
(*                                                                              *)
(* Closes #793                                                                  *)
(*                                                                              *)
(* Quantizer rule (φ-prior):                                                   *)
(*   |w| >= phi_inv_sq  →  sign(w)  in {+1, -1}                               *)
(*   |w| <  phi_inv_sq  →  0                                                   *)
(*                                                                              *)
(* phi_inv_sq = 12533 in Q1.15 (project φ⁻² threshold).                       *)
(* φ² + φ⁻² = 3  (Trinity algebraic anchor).                                  *)
(*                                                                              *)
(* DOI: 10.5281/zenodo.19227877                                                 *)

Require Import Coq.ZArith.ZArith.
Require Import Coq.micromega.Lia.

Open Scope Z_scope.

(* ======================================================================== *)
(* Section 1: Parameters and ternary type                                   *)
(* ======================================================================== *)

(** φ⁻² threshold in Q1.15 fixed-point. *)
Definition phi_inv_sq : Z := 12533.

(** Ternary weight alphabet {-1, 0, +1}. *)
Inductive Ternary : Set :=
  | TPos  : Ternary    (* +1 *)
  | TZero : Ternary    (*  0 *)
  | TNeg  : Ternary.   (* -1 *)

(* ======================================================================== *)
(* Section 2: Quantizer function                                             *)
(* ======================================================================== *)

(** Quantize a signed Q1.15 integer weight w.
    Returns TPos (+1), TZero (0), or TNeg (-1). *)
Definition quantize (w : Z) : Ternary :=
  if phi_inv_sq <=? w then TPos
  else if w <=? - phi_inv_sq then TNeg
  else TZero.

(* ======================================================================== *)
(* Section 3: Main correctness theorems                                      *)
(* ======================================================================== *)

(** Theorem 1 (Ternary Completeness):
    The quantizer always returns a value in {+1, 0, -1}. *)
Theorem quantize_ternary :
  forall w : Z,
    quantize w = TPos \/ quantize w = TZero \/ quantize w = TNeg.
Proof.
  intro w.
  unfold quantize.
  destruct (phi_inv_sq <=? w).
  - left. reflexivity.
  - destruct (w <=? - phi_inv_sq).
    + right. right. reflexivity.
    + right. left. reflexivity.
Qed.

(** Theorem 2 (Zero Region):
    If |w| < phi_inv_sq then quantize w = TZero. *)
Theorem quantize_zero_region :
  forall w : Z,
    Z.abs w < phi_inv_sq ->
    quantize w = TZero.
Proof.
  intros w Habs.
  unfold quantize.
  apply Z.abs_lt in Habs.
  destruct Habs as [Hlo Hhi].
  assert (H1: (phi_inv_sq <=? w) = false).
  { apply Z.leb_gt. exact Hhi. }
  assert (H2: (w <=? - phi_inv_sq) = false).
  { apply Z.leb_gt. lia. }
  rewrite H1. rewrite H2. reflexivity.
Qed.

(** Theorem 3 (Positive Region):
    If w >= phi_inv_sq then quantize w = TPos. *)
Theorem quantize_positive_region :
  forall w : Z,
    phi_inv_sq <= w ->
    quantize w = TPos.
Proof.
  intros w Hge.
  unfold quantize.
  assert (H: (phi_inv_sq <=? w) = true) by (apply Z.leb_le; exact Hge).
  rewrite H. reflexivity.
Qed.

(** Theorem 4 (Negative Region):
    If w <= -phi_inv_sq then quantize w = TNeg. *)
Theorem quantize_negative_region :
  forall w : Z,
    w <= - phi_inv_sq ->
    quantize w = TNeg.
Proof.
  intros w Hle.
  unfold quantize.
  assert (H1: (phi_inv_sq <=? w) = false).
  { apply Z.leb_gt. unfold phi_inv_sq in *. lia. }
  assert (H2: (w <=? - phi_inv_sq) = true).
  { apply Z.leb_le. exact Hle. }
  rewrite H1. rewrite H2. reflexivity.
Qed.

(* ======================================================================== *)
(* Section 4: Boundary correctness                                           *)
(* ======================================================================== *)

(** Boundary: w = phi_inv_sq → TPos (+1). *)
Theorem quantize_at_pos_boundary :
  quantize phi_inv_sq = TPos.
Proof.
  apply quantize_positive_region. lia.
Qed.

(** Boundary: w = phi_inv_sq - 1 → TZero (0). *)
Theorem quantize_just_below_pos_boundary :
  quantize (phi_inv_sq - 1) = TZero.
Proof.
  apply quantize_zero_region. unfold phi_inv_sq. lia.
Qed.

(** Boundary: w = -phi_inv_sq → TNeg (-1). *)
Theorem quantize_at_neg_boundary :
  quantize (- phi_inv_sq) = TNeg.
Proof.
  apply quantize_negative_region. lia.
Qed.

(** Boundary: w = -(phi_inv_sq - 1) → TZero (0). *)
Theorem quantize_just_below_neg_boundary :
  quantize (- (phi_inv_sq - 1)) = TZero.
Proof.
  apply quantize_zero_region. unfold phi_inv_sq. lia.
Qed.

(** Boundary: w = 0 → TZero (0). *)
Theorem quantize_zero :
  quantize 0 = TZero.
Proof.
  apply quantize_zero_region.
  rewrite Z.abs_0. unfold phi_inv_sq. lia.
Qed.

(* ======================================================================== *)
(* Section 5: Q1.15 range completeness                                       *)
(* ======================================================================== *)

(** For any Q1.15 signed value w ∈ [-32768, 32767],
    quantize w ∈ {+1, 0, -1}. *)
Theorem quantize_q115_complete :
  forall w : Z,
    -32768 <= w <= 32767 ->
    quantize w = TPos \/ quantize w = TZero \/ quantize w = TNeg.
Proof.
  intros w _.
  exact (quantize_ternary w).
Qed.

(* ======================================================================== *)
(* Section 6: Threshold properties                                           *)
(* ======================================================================== *)

(** phi_inv_sq is positive. *)
Theorem phi_inv_sq_positive : 0 < phi_inv_sq.
Proof. unfold phi_inv_sq. lia. Qed.

(** phi_inv_sq is within Q1.15 positive range. *)
Theorem phi_inv_sq_in_range : 0 < phi_inv_sq /\ phi_inv_sq <= 32767.
Proof. split; unfold phi_inv_sq; lia. Qed.

(** The ternary encoding uses 2 bits vs 16 bits for Q1.15 FP.
    Formal statement: 2 < 16. *)
Theorem ternary_encoding_smaller : (2 : Z) < 16.
Proof. lia. Qed.

(* ======================================================================== *)
(* Section 7: Symmetry                                                       *)
(* ======================================================================== *)

(** quantize (-w) = TNeg when quantize w = TPos. *)
Theorem quantize_symmetric_pos :
  forall w : Z,
    quantize w = TPos -> quantize (- w) = TNeg.
Proof.
  intros w H.
  assert (Hge: phi_inv_sq <= w).
  { unfold quantize in H.
    destruct (phi_inv_sq <=? w) eqn:Hw.
    - apply Z.leb_le. exact Hw.
    - destruct (w <=? - phi_inv_sq); discriminate H. }
  unfold quantize.
  assert (H1: phi_inv_sq <=? - w = false).
  { apply Z.leb_gt. unfold phi_inv_sq in *. lia. }
  assert (H2: - w <=? - phi_inv_sq = true).
  { apply Z.leb_le. unfold phi_inv_sq in *. lia. }
  rewrite H1, H2. reflexivity.
Qed.

(** quantize (-w) = TPos when quantize w = TNeg. *)
Theorem quantize_symmetric_neg :
  forall w : Z,
    quantize w = TNeg -> quantize (- w) = TPos.
Proof.
  intros w H.
  assert (Hle: w <= - phi_inv_sq).
  { unfold quantize in H.
    destruct (phi_inv_sq <=? w) eqn:Hw; [ discriminate H | ].
    destruct (w <=? - phi_inv_sq) eqn:Hw2;
      [ apply Z.leb_le; exact Hw2 | discriminate H ]. }
  unfold quantize.
  assert (H1: phi_inv_sq <=? - w = true).
  { apply Z.leb_le. unfold phi_inv_sq in *. lia. }
  rewrite H1. reflexivity.
Qed.

(** quantize (-w) = TZero when quantize w = TZero. *)
Theorem quantize_symmetric_zero :
  forall w : Z,
    quantize w = TZero -> quantize (- w) = TZero.
Proof.
  intros w H.
  assert (Hrange: - phi_inv_sq < w < phi_inv_sq).
  { unfold quantize in H.
    destruct (phi_inv_sq <=? w) eqn:Hw; [ discriminate H | ].
    destruct (w <=? - phi_inv_sq) eqn:Hw2; [ discriminate H | ].
    apply Z.leb_gt in Hw. apply Z.leb_gt in Hw2.
    unfold phi_inv_sq in *. lia. }
  unfold quantize.
  assert (H1: phi_inv_sq <=? - w = false).
  { apply Z.leb_gt. unfold phi_inv_sq in *. lia. }
  assert (H2: - w <=? - phi_inv_sq = false).
  { apply Z.leb_gt. unfold phi_inv_sq in *. lia. }
  rewrite H1, H2. reflexivity.
Qed.

(* ======================================================================== *)
(* End of PhiPriorQuantCorrect.v                                             *)
(* All theorems proved with Qed (no Admitted).                               *)
(* ======================================================================== *)
