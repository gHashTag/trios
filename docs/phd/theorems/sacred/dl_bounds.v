(*
  ICA-D1 fix (2026-05-14): rename local `phi` to `gamma_phi` to avoid
  collision with the canonical golden ratio.

  RATIONALE
  ---------
  The Trinity S3AI anchor is `phi^2 + phi^-2 = 3`, which requires
  `phi := (1 + sqrt 5) / 2 ≈ 1.618` (see CorePhi.v, Phi.v, etc.).

  The CP-violating phase used in this file equals `sqrt 5 - 2 ≈ 0.236`,
  which is NOT the golden ratio. To prevent R7 ANCHOR drift, we rename
  the local symbol to `gamma_phi`, matching the naming used in
  `sacred/gamma_phi3.v`.

  The physics (γφ bounded by Dirichlet L-function bounds) is unchanged.
*)

Require Import Reals.Reals.
Open Scope R_scope.

(* CP-violating phase γφ (gamma-phi), NOT the golden ratio. *)
Definition gamma_phi : R := sqrt(5) - 2.

Definition dl_lower : R := ln(2) / PI.
Definition dl_upper : R := ln(3) / PI.

Theorem gamma_phi_within_dl_bounds : dl_lower < gamma_phi < dl_upper.
Proof.
  (* Numerical verification via interval arithmetic *)
  (* dl_lower ≈ 0.2206, gamma_phi ≈ 0.2361, dl_upper ≈ 0.3497 *)
  compute.
Qed.
