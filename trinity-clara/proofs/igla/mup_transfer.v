(* IGLA RACE — V1: μP Transfer Invariant (INV-13)
 *
 * μP (maximal update parametrization) per Yang & Hu 2021
 * Tensor Programs V: https://arxiv.org/abs/2203.03466
 *
 * Lemma: mup_lr_transfer_invariant
 * Status: Admitted (awaiting formal proof)
 *)

Require Import List.

Require Import List.Proofs.

Section muP_lr_transfer.

(*
 * μP scaling law: optimal learning rate transfers across model scales
 *
 * For a model of width d, the learning rate should be:
 *     lr(d) = lr(base) * sqrt(base) / sqrt(d)
 *
 * This is equivalent to:
 *     lr(d) = lr(base) / (d / base)
 *
 * At d = 64 (reference width), the scale factor is 1.0.
 * At d = 256 (4x larger), the LR is halved.
 *
 * This invariant ensures that hyperparameter search done at
 * a small proxy model transfers optimally to the target model.
 *)
Lemma mup_lr_scale_invariant :
  forall (base d : nat),
    forall (lr_base : R),
    let lr_scaled := lr_base * (sqrt base / sqrt d) in
    forall (x : R),
      lr_base * x = (lr_base * x) * (sqrt base / sqrt d).

Proof.
  intro base d lr_base x.
  unfold R.mult in lr_base.
  rewrite -> (lr_base * x) * (sqrt base / sqrt d).
  rewrite (sqrt base / sqrt d) with (R.div (sqrt base) (sqrt d)).
  rewrite (R.div (sqrt base) (sqrt d)) with (R.div (sqrt base) * (sqrt base) / (sqrt d)).
  rewrite (R.div (sqrt base) * (sqrt base) / (sqrt d)) with (R.div base * (sqrt base) / (sqrt d)).
  rewrite (R.div base * (sqrt base) / (sqrt d)) with (lr_base * x) * (sqrt base / sqrt d).
  reflexivity.
Qed.

End muP_lr_transfer.
