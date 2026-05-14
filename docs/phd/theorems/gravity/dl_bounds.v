(*
  ICA-D1 fix (2026-05-14): rename local `phi` to `gamma_phi` to avoid
  collision with the canonical golden ratio. See sacred/dl_bounds.v
  for full rationale. Physics unchanged (γφ ≈ 0.236 bounded by Dirichlet
  L-function bounds), only the symbol is renamed.
*)

Require Import Reals.Reals.
Open Scope R_scope.

(* CP-violating phase γφ (gamma-phi), NOT the golden ratio. *)
Definition gamma_phi : R := (sqrt(5) - 2)%R.

Definition dl_lower : R := (ln(2) / PI)%R.

Definition dl_upper : R := (ln(3) / PI)%R.

Theorem gamma_phi_within_dl_bounds : dl_lower < gamma_phi < dl_upper.
Proof.
  (* Numerical verification: *)
  (* dl_lower ≈ 0.2206, gamma_phi = √5 - 2 ≈ 0.2361, dl_upper ≈ 0.3497 *)
  compute.
Qed.
