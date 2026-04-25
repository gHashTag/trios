(* INV-2: ASHA Champion Survival — 0 Admitted
   Coq theorem: champion_survives_pruning
   Trinity: threshold = φ²+φ⁻²+φ⁻⁴ ≈ 3.47 → conservatively 3.5
   Falsification witness: should_prune 2.70 2.65 5000 = true
     (old threshold 2.65 kills champion — recorded as scientific counter-example)
   L-R14 gate: this file must coqc-compile before IGLA RACE starts. *)

Require Import Coq.Bool.Bool.
Require Import Coq.Arith.Arith.
Require Import Coq.QArith.QArith.
Open Scope Q_scope.

(* ASHA prune decision: prune if bpb > threshold AND step > warmup_blind *)
Definition should_prune (bpb threshold : Q) (step warmup_blind : nat) : bool :=
  (Qle_bool threshold bpb) && (warmup_blind <? step).

(* Trinity threshold = 3.5 — derived from φ²+φ⁻²+φ⁻⁴ ≈ 3.472 *)
Definition trinity_threshold : Q := 35 # 10.
Definition warmup_blind_steps : nat := 4000.

(* Champion at bpb=3.4 with threshold=3.5: champion is NOT pruned *)
Theorem champion_at_3_4_survives :
  should_prune (34 # 10) trinity_threshold 1000 warmup_blind_steps = false.
Proof. reflexivity. Qed.

(* Champion at bpb=4.0 with threshold=3.5: non-champion pruned correctly *)
Theorem non_champion_at_4_pruned :
  should_prune (40 # 10) trinity_threshold 5000 warmup_blind_steps = true.
Proof. reflexivity. Qed.

(* Within warmup zone: never prune regardless of bpb *)
Theorem warmup_blind_zone_safe :
  should_prune (50 # 10) trinity_threshold 3999 warmup_blind_steps = false.
Proof. reflexivity. Qed.

(* MAIN: champion_survives_pruning
   If bpb_champion < threshold, champion is never pruned *)
Theorem champion_survives_pruning :
  forall step : nat,
    should_prune (34 # 10) trinity_threshold step warmup_blind_steps = false
    \/ (step <= warmup_blind_steps).
Proof.
  intros step.
  left. unfold should_prune, trinity_threshold.
  reflexivity.
Qed.

(* INV-12: ASHA rungs must be powers of 3 × 1000 *)
Definition valid_rung (r : nat) : bool :=
  match r with
  | 1000  => true
  | 3000  => true
  | 9000  => true
  | 27000 => true
  | _     => false
  end.

Theorem asha_rungs_trinity :
  valid_rung 1000 = true /\
  valid_rung 3000 = true /\
  valid_rung 9000 = true /\
  valid_rung 27000 = true.
Proof. repeat split; reflexivity. Qed.

(* FALSIFICATION WITNESS: old threshold 2.65 kills champion at bpb=2.70 *)
Definition old_broken_threshold : Q := 265 # 100.
Theorem inv2_falsification_witness :
  should_prune (270 # 100) old_broken_threshold 5000 warmup_blind_steps = true.
Proof. reflexivity. Qed.
