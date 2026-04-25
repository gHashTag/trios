(* Trinity Identity anchor: phi^2 + phi^-2 = 3 *)
(* Zenodo DOI: 10.5281/zenodo.19227877 *)
(* Compile order: lucas_closure -> gf16 -> nca -> lr -> asha -> igla_found_criterion *)
(* Rust target: crates/trios-igla-race/src/victory.rs *)
(* INV-7: igla_found_criterion — L7 Victory Gate *)

Require Import Coq.Reals.Reals.
Require Import Coq.Lists.List.
Require Import Coq.Arith.Arith.
Open Scope R_scope.

(* ================================================================ *)
(* Core definitions                                                 *)
(* ================================================================ *)

Definition warmup_min_steps : R := 4000.
Definition jepa_proxy_floor : R := 0.1.
Definition victory_bpb_target : R := 1.5.
Definition n_required_seeds : nat := 3.

Definition is_finite (x : R) : Prop := x <> 0/0 /\ x <> Rdiv 1 0 /\ x <> Rdiv (-1) 0.
Definition valid_step (step : R) : Prop := step >= warmup_min_steps.
Definition valid_bpb (bpb : R) : Prop :=
  is_finite bpb /\ bpb >= jepa_proxy_floor /\ bpb < victory_bpb_target.
Definition victory_acceptable (seed : nat) (bpb : R) (step : R) : Prop :=
  valid_step step /\ valid_bpb bpb.

Inductive seed_result : Type :=
  | SeedResult : nat -> R -> R -> seed_result.

Definition sr_seed (sr : seed_result) : nat :=
  match sr with SeedResult s _ _ => s end.
Definition sr_bpb (sr : seed_result) : R :=
  match sr with SeedResult _ b _ => b end.
Definition sr_step (sr : seed_result) : R :=
  match sr with SeedResult _ _ t => t end.

Fixpoint all_distinct (seeds : list nat) : Prop :=
  match seeds with
  | nil => True
  | s :: rest => ~ (In s rest) /\ all_distinct rest
  end.

Fixpoint all_acceptable (results : list seed_result) : Prop :=
  match results with
  | nil => True
  | sr :: rest =>
    victory_acceptable (sr_seed sr) (sr_bpb sr) (sr_step sr) /\
    all_acceptable rest
  end.

Definition victory_three_seeds (results : list seed_result) : Prop :=
  length results >= n_required_seeds /\
  all_distinct (map sr_seed results) /\
  all_acceptable results.

(* ================================================================ *)
(* Falsification witnesses — appear FIRST per R8                    *)
(* ================================================================ *)

Example refutation_pre_warmup_admitted :
  ~ (forall step seed bpb,
       step < 4000 -> bpb < 1.5 -> victory_acceptable seed bpb step).
Proof.
  intro H. apply (H 100 0 0.5).
  left. reflexivity.
  left. reflexivity.
Qed.

Example refutation_jepa_proxy_admitted :
  ~ (forall seed bpb step,
       bpb < 0.1 -> step >= 4000 -> victory_acceptable seed bpb step).
Proof.
  intro H. apply (H 0 0.014 5000).
  (* 0.014 < 0.1 *)
  apply Rlt_0_1.
  (* but 0.014 < 0.1 so valid_bpb fails *)
  simpl. unfold valid_bpb.
  split; [| split].
  - unfold is_finite. split; [| split]; lra.
  - lra.
  - lra.
Qed.

Example refutation_duplicate_seeds_admitted :
  ~ (forall s b1 b2 b3 t,
       victory_three_seeds [SeedResult s b1 t; SeedResult s b2 t; SeedResult s b1 t]).
Proof.
  intros s b1 b2 b3 t H.
  unfold victory_three_seeds in H.
  destruct H as [Hlen [Hdist Hacc]].
  unfold all_distinct in Hdist.
  simpl in Hdist.
  destruct Hdist as [Hnin Hrest].
  apply Hnin. left. reflexivity.
Qed.

(* ================================================================ *)
(* Core theorems — INV-7                                            *)
(* ================================================================ *)

Theorem warmup_blocks_proxy : forall seed bpb step,
  step < warmup_min_steps -> ~ victory_acceptable seed bpb step.
Proof.
  intros seed bpb step Hlt Hacc.
  unfold victory_acceptable in Hacc.
  destruct Hacc as [Hstep _].
  unfold valid_step in Hstep.
  unfold warmup_min_steps in *.
  lra.
Qed.

Theorem distinct_seeds_required : forall s1 s2 b1 b2 t1 t2,
  s1 = s2 -> ~ victory_three_seeds [SeedResult s1 b1 t1; SeedResult s2 b2 t2; SeedResult s1 b1 t1].
Proof.
  intros s1 s2 b1 b2 t1 t2 Heq Hvic.
  unfold victory_three_seeds in Hvic.
  destruct Hvic as [_ [Hdist _]].
  unfold all_distinct in Hdist.
  simpl in Hdist.
  destruct Hdist as [Hnin _].
  subst s2.
  apply Hnin. left. reflexivity.
Qed.

Theorem jepa_proxy_floor_correct : forall seed bpb step,
  bpb < jepa_proxy_floor -> ~ victory_acceptable seed bpb step.
Proof.
  intros seed bpb step Hlt Hacc.
  unfold victory_acceptable in Hacc.
  destruct Hacc as [_ Hbpb].
  unfold valid_bpb in Hbpb.
  destruct Hbpb as [_ [Hfloor _]].
  unfold jepa_proxy_floor in *.
  lra.
Qed.

Theorem nan_rejected : forall seed step,
  is_finite (0/0) = False -> victory_acceptable seed (0/0) step -> False.
Proof.
  intros seed step Hnf Hacc.
  unfold victory_acceptable in Hacc.
  destruct Hacc as [_ Hbpb].
  unfold valid_bpb in Hbpb.
  destruct Hbpb as [Hfin _].
  unfold is_finite in Hfin.
  destruct Hfin as [Hneq _].
  apply Hneq. reflexivity.
Qed.
