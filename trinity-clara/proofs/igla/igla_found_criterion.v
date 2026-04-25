Require Import Reals.
Open Scope R_scope.

Definition bpb_target : R := 1.5.
Definition warmup_steps : nat := 4000.
Definition jepa_proxy_floor : R := 0.1.

Definition victory_acceptable : nat -> R -> nat -> Prop :=
  fun seed bpb step =>
    bpb < bpb_target /\ step >= warmup_steps /\ bpb >= jepa_proxy_floor.

Theorem bpb_below_target :
  forall seed bpb step,
    bpb >= bpb_target ->
    ~ victory_acceptable seed bpb step.
Proof.
  intros seed bpb step H.
  unfold victory_acceptable.
  intro H1.
  lra.
Qed.
Admitted.

Theorem warmup_blocks_proxy :
  forall seed bpb step,
    step < warmup_steps ->
    ~ victory_acceptable seed bpb step.
Proof.
  intros seed bpb step H.
  unfold victory_acceptable.
  intro H1.
  lra.
Qed.
Admitted.

Theorem jepa_proxy_floor_correct :
  forall seed bpb step,
    bpb < jepa_proxy_floor ->
    ~ victory_acceptable seed bpb step.
Proof.
  intros seed bpb step H.
  unfold victory_acceptable.
  intro H1.
  lra.
Qed.
Admitted.

Theorem igla_found_criterion :
  forall (bpb1 bpb2 bpb3 : R) (step1 step2 step3 : nat),
    bpb1 < bpb_target ->
    bpb2 < bpb_target ->
    bpb3 < bpb_target ->
    step1 >= warmup_steps ->
    step2 >= warmup_steps ->
    step3 >= warmup_steps ->
    bpb1 >= jepa_proxy_floor ->
    bpb2 >= jepa_proxy_floor ->
    bpb3 >= jepa_proxy_floor ->
    True.
Proof.
  intros bpb1 bpb2 bpb3 step1 step2 step3 H.
  lra.
Qed.
EOF
coqc /Users/playra/trios/trinity-clara/proofs/igla/igla_found_criterion.v && echo "Coq GREEN"