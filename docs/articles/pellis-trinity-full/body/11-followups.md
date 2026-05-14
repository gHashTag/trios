## Required follow-up issues

1. Fix unresolved merge-conflict markers in
   `gHashTag/trinity/docs/ARCHITECTURE.md`.
2. Correct the sacred-constants citation path from `constants.zig` to
   `sacred_constants_data.zig`.
3. Decide whether Strand II canonical taxonomy is the `src/tri/` ten-module
   S3AI list or the `src/brain/` 21-region architecture.
4. Fix the TRI-27 Coptic table collision (U+03C6 between R5 and R20) and
   replace Greek/Cyrillic code points if true Coptic naming is required.
5. Add a canonical opcode enumeration if "36 opcodes" is to be claimed.
6. Add an explicit register-bank spec if "3 banks" is to be claimed.
7. Wire `flos_70.tex` into the PhD build only after it meets the R3
   quality floor.
8. Add missing TRI-1 Coq witness files or downgrade those references.
9. Run Catalog42 on a Coq-equipped machine with `coq-interval` so that a
   fresh compiler verdict can replace the current source-level audit.
10. Continue Catalog42 from 19/42 to 42/42 only after formulas, reference
    values, and tolerances are frozen, **and** after L01, L02, L03, Q03,
    Q05 are either reformulated, widened with explicit justification, or
    closed with Qed.
