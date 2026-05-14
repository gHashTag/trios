## Corrected claims table

| Prior claim | Audit result | Article action |
|---|---|---|
| 75+ sacred constants in `constants.zig` | `constants.zig` has 15 curated entries; `sacred_constants_data.zig` has 154 numeric constants | Replace with precise two-file statement |
| 21 brain modules and about 22k LOC | Two taxonomies exist; LOC counts are stale; one listed file missing | Reframe as developing cognitive layer |
| VSA TF3-9 length 729 | Sacred VSA dimension 729 is supported; another VSA dimension 10000 also exists | Cite "Sacred VSA 729", not generic VSA |
| TRI-27 has 27 registers | Supported | Keep |
| TRI-27 has 3 register banks | Not supported in t27 specs | Remove or mark as future design target |
| TRI-27 has 36 opcodes | Not supported as canonical count | Replace with 6-bit opcode field and 7 operation classes |
| Coptic register naming is clean | Supported in spirit, but code-point collisions and non-Coptic code points exist | Add caveat and fix task |
| Sacred ALU: 352 LUT on XC7A100T | Supported by synthesis report | Keep with P&R caveat |
| Sacred ALU Fmax/latency/throughput measured | Not measured; estimated | Mark as estimate |
| 71-chapter PhD is fully built | 71 files exist; build includes 0–69 only; `flos_70` skeleton | Say "71 files, TRI-1 skeleton not yet wired" |
| v21 phrase is repo-native | String not found in t27/trios; external framing | Use as integration label, not repo artifact |
| Catalog42 is a fully verified formula-by-formula layer | 19 closed-with-Qed, 23 UnderRevision, 32 quarantined `Admitted.` outside flagship chain | Use the proof-status lock above; do not claim full-catalogue formal verification |
| L01 verified, $<1\%$ error | Measured relative error ≈ 99% | Downgrade to UnderRevision / Reformulate |
| L02 verified, $<1\%$ error | Measured relative error ≈ 6% | Downgrade to UnderRevision / Widen-or-reformulate |
| L03 verified, $<1\%$ error | Measured relative error ≈ 99% | Downgrade to UnderRevision / Reformulate |
| Q03 verified, $<1\%$ error | Measured relative error ≈ 98% | Downgrade to UnderRevision / Reformulate |
| Q05 verified, $<1\%$ error | Measured relative error ≈ 1.06% | Downgrade to UnderRevision / Widen-or-chain |
| Uncapped Bonferroni product $n \cdot p \approx 15$ reported as if it were a p-value | $p_{\text{Bonf}} = \min(1, n \cdot p) = \min(1, 15) = 1$ by definition | Use the capped value, see §"Statistical multiplicity" |
