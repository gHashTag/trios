(* PowerStateSafety.v
   Apache-2.0 · TRI-1 v2 L-S43 · PhD Ch.16/S43

   Anchor: phi^2 + phi^-2 = 3
   DOI: 10.5281/zenodo.19227877

   Theorem: Wave-11b power-gate FSM never enters illegal "off-active overlap"
   state: pwr_en = 0 AND clk_en = 1 simultaneously would crash silicon.

   Invariant: forall s, pwr_en s = false -> clk_en s = false

   Issue: https://github.com/gHashTag/trios/issues/799 (L-S43)
   Author: Dmitrii Vasilev <admin@t27.ai> | Date: 2026-05-14 *)

Require Import List.
Import ListNotations.

(* ------------------------------------------------------------------ *)
(* 1.  Power-gate FSM states                                           *)
(* ------------------------------------------------------------------ *)

(** Four-state FSM for the Wave-11b power domain:
    OFF        – power off, clock off
    WAKE       – power ramping up, clock still gated
    ACTIVE     – power on, clock running
    SLEEP_REQ  – clock gating requested, power still on
*)
Inductive pstate : Type :=
  | OFF
  | WAKE
  | ACTIVE
  | SLEEP_REQ.

(* ------------------------------------------------------------------ *)
(* 2.  Signal definitions                                              *)
(* ------------------------------------------------------------------ *)

(** pwr_en: power enable signal.
    True (power on) in WAKE, ACTIVE, SLEEP_REQ; false in OFF. *)
Definition pwr_en (s : pstate) : bool :=
  match s with
  | OFF       => false
  | WAKE      => true
  | ACTIVE    => true
  | SLEEP_REQ => true
  end.

(** clk_en: clock enable signal.
    True (clock on) only in ACTIVE; gated in all other states. *)
Definition clk_en (s : pstate) : bool :=
  match s with
  | OFF       => false
  | WAKE      => false
  | ACTIVE    => true
  | SLEEP_REQ => false
  end.

(* ------------------------------------------------------------------ *)
(* 3.  FSM transition function                                         *)
(* ------------------------------------------------------------------ *)

(** Input events that drive the FSM. *)
Inductive input : Type :=
  | EVT_POWER_ON    (* request power-on *)
  | EVT_POWER_READY (* power rail stable *)
  | EVT_SLEEP       (* request clock gate *)
  | EVT_POWER_OFF   (* request power-off *)
  | EVT_NOP.        (* no event *)

(** Single-step deterministic transition. *)
Definition step (s : pstate) (i : input) : pstate :=
  match s, i with
  | OFF,       EVT_POWER_ON    => WAKE
  | WAKE,      EVT_POWER_READY => ACTIVE
  | ACTIVE,    EVT_SLEEP       => SLEEP_REQ
  | SLEEP_REQ, EVT_POWER_OFF   => OFF
  (* self-loops for all other combinations *)
  | s',        _               => s'
  end.

(** Run the FSM from an initial state over a list of inputs.
    Returns the list of all visited states (including the initial one). *)
Fixpoint run (s : pstate) (inputs : list input) : list pstate :=
  match inputs with
  | []      => [s]
  | i :: rest => s :: run (step s i) rest
  end.

(* ------------------------------------------------------------------ *)
(* 4.  Core safety theorem                                             *)
(* ------------------------------------------------------------------ *)

(** [no_off_active_overlap]
    In every reachable state, if power is off then the clock is also off.
    Proved by exhaustive case analysis on the 4-element inductive type. *)
Theorem no_off_active_overlap :
  forall s : pstate,
    pwr_en s = false -> clk_en s = false.
Proof.
  intros s H.
  destruct s; simpl in *; try reflexivity; discriminate.
Qed.

(* ------------------------------------------------------------------ *)
(* 5.  Reachability lemma                                              *)
(* ------------------------------------------------------------------ *)

(** Auxiliary: every state in [run s inputs] satisfies the invariant.
    We induct on [inputs], keeping [s] general so the IH applies to
    [step s i] in the recursive call. *)
Lemma run_invariant :
  forall (inputs : list input) (s : pstate) (t : pstate),
    In t (run s inputs) ->
    pwr_en t = false -> clk_en t = false.
Proof.
  induction inputs as [| i rest IH]; intros s t Hin Hpwr.
  - (* base: run s [] = [s], so t = s *)
    simpl in Hin.
    destruct Hin as [Heq | Hnil].
    + subst t. apply no_off_active_overlap. exact Hpwr.
    + contradiction.
  - (* step: run s (i::rest) = s :: run (step s i) rest *)
    simpl in Hin.
    destruct Hin as [Heq | Hin'].
    + subst t. apply no_off_active_overlap. exact Hpwr.
    + (* t is in run (step s i) rest — use IH with s0 := step s i *)
      apply (IH (step s i) t Hin' Hpwr).
Qed.

(** [reachable_states_safe]
    For any initial state and any input sequence, every state visited
    during the run satisfies the off-active overlap invariant. *)
Lemma reachable_states_safe :
  forall (init : pstate) (seq : list input) (s : pstate),
    In s (run init seq) ->
    pwr_en s = false -> clk_en s = false.
Proof.
  intros init seq s Hin Hpwr.
  apply (run_invariant seq init s Hin Hpwr).
Qed.

(* ------------------------------------------------------------------ *)
(* 6.  Sanity checks                                                   *)
(* ------------------------------------------------------------------ *)

(** OFF state always has clock disabled. *)
Lemma off_clk_disabled : clk_en OFF = false.
Proof. reflexivity. Qed.

(** A domain in OFF cannot have its clock enabled. *)
Lemma no_off_with_clk :
  forall s : pstate,
    s = OFF -> clk_en s = false.
Proof.
  intros s Hs. subst s. reflexivity.
Qed.
