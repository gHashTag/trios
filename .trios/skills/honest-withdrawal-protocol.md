# Skill: Honest Withdrawal Protocol

## Description
When false, unverified, or fabricated claims are discovered in the codebase (especially physics predictions or scientific references), follow this protocol to withdraw them honestly without destroying credibility.

## When to Use
- A `.v` proof file contains Admitted lemmas masking false physical bounds
- A scientific reference cannot be verified (arXiv ID nonexistent, DOI broken)
- A numeric prediction is found to be incorrect upon independent verification
- A competitor's paper is withdrawn by the author

## Protocol Steps

1. **STOP** — Do not silently delete or modify the false claim
2. **ARCHIVE** — Move the file to `archive/` or `withdrawn/` with a timestamp suffix (e.g., `Predictions_withdrawn_2026_06_16.v`)
3. **DOCUMENT** — In the archived file, add a header comment explaining:
   - What was withdrawn
   - Why (specific errors found)
   - When (date)
   - Who (author or agent responsible)
4. **DISCLOSE** — In the active replacement file (or README), add a note:
   - "Previous predictions withdrawn on [date] due to [reason]. See archive/[filename] for details."
5. **VERIFY** — Run the audit suite to ensure no references to the withdrawn claim remain in active files
6. **COMMIT** — Use explicit commit message: "honest withdrawal: [description]" with references to the withdrawn content

## What NOT To Do
- ❌ Silently delete the file (breaks git history and L1 TRACEABILITY)
- ❌ Replace false values with "corrected" values without disclosure (destroys credibility)
- ❌ Claim the withdrawal was planned all along (dishonest)
- ❌ Leave Admitted lemmas in active files as " placeholders"

## Example
```coq
(* WITHDRAWN 2026-06-16 — see archive/Predictions_withdrawn_2026_06_16.v
   Reason: Python spot-check revealed 15 Admitted lemmas masked false
   physical bounds (δ_CP, m_DM, Σ m_ν, sin²θ₁₃, m_νe).
   No corrected values are provided to prevent further fabrication. *)
```

## Related Skills
- [[trinity-coq-admitted-recovery]] — Handling discovered Admitted lemmas
- [[trinity-scientific-reference-audit]] — Verifying references
- [[trinity-l1-l7-compliance]] — L1 TRACEABILITY law
