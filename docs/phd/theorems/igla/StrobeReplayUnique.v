(* StrobeReplayUnique.v
   Apache-2.0 · TRI-1 v2 L-S26 · PhD Ch.16/S16

   Anchor: phi^2 + phi^-2 = 3
   DOI: 10.5281/zenodo.19227877

   Theorem: inside a Lucas-period window L_n, strobe seeds are unique
   (replay attack is impossible).

   Issue: https://github.com/gHashTag/trinity-fpga/issues/54 (L-S26)
   Author: Dmitrii Vasilev <admin@t27.ai> | Date: 2025-01-01 *)

Require Import Arith.
Require Import Lia.

(* Lucas L_5 = 11, L_6 = 18, L_7 = 29 — real strobe periods *)
Definition L5 : nat := 11.
Definition L6 : nat := 18.
Definition L7 : nat := 29.

(* Strobe seed as a function of cycle in [0, L_n):
   Within one period, cycle/period = 0, so seed = cycle mod period = cycle *)
Definition strobe_seed (cycle : nat) (period : nat) : nat :=
  (cycle mod period) + ((cycle / period) * 34) mod 34.

(* Lemma: for c1 <> c2 strictly inside one period, (c1 mod period) <> (c2 mod period) *)
Lemma strobe_unique_in_period :
  forall (period c1 c2 : nat),
    period > 0 ->
    c1 < period ->
    c2 < period ->
    c1 <> c2 ->
    (c1 mod period) <> (c2 mod period).
Proof.
  intros period c1 c2 Hp Hc1 Hc2 Hne.
  rewrite (Nat.mod_small c1 period Hc1).
  rewrite (Nat.mod_small c2 period Hc2).
  exact Hne.
Qed.

(* Lemma: for cycle < period, cycle / period = 0 *)
Lemma div_small_zero :
  forall (cycle period : nat),
    period > 0 ->
    cycle < period ->
    cycle / period = 0.
Proof.
  intros cycle period Hp Hc.
  apply Nat.div_small.
  exact Hc.
Qed.

(* Lemma: strobe_seed within one period reduces to cycle mod period *)
Lemma strobe_seed_in_period :
  forall (cycle period : nat),
    period > 0 ->
    cycle < period ->
    strobe_seed cycle period = cycle.
Proof.
  intros cycle period Hp Hc.
  unfold strobe_seed.
  rewrite (div_small_zero cycle period Hp Hc).
  rewrite Nat.mod_small; [ | exact Hc ].
  simpl.
  lia.
Qed.

(* Main theorem: within any Lucas period (L5, L6, L7), strobe seeds are unique *)
Theorem strobe_replay_unique :
  forall (period c1 c2 : nat),
    period = L5 \/ period = L6 \/ period = L7 ->
    c1 < period ->
    c2 < period ->
    c1 <> c2 ->
    strobe_seed c1 period <> strobe_seed c2 period.
Proof.
  intros period c1 c2 Hper Hc1 Hc2 Hne.
  destruct Hper as [-> | [-> | ->]].
  - (* period = L5 = 11 *)
    unfold L5 in *.
    rewrite (strobe_seed_in_period c1 11 ltac:(lia) Hc1).
    rewrite (strobe_seed_in_period c2 11 ltac:(lia) Hc2).
    exact Hne.
  - (* period = L6 = 18 *)
    unfold L6 in *.
    rewrite (strobe_seed_in_period c1 18 ltac:(lia) Hc1).
    rewrite (strobe_seed_in_period c2 18 ltac:(lia) Hc2).
    exact Hne.
  - (* period = L7 = 29 *)
    unfold L7 in *.
    rewrite (strobe_seed_in_period c1 29 ltac:(lia) Hc1).
    rewrite (strobe_seed_in_period c2 29 ltac:(lia) Hc2).
    exact Hne.
Qed.
