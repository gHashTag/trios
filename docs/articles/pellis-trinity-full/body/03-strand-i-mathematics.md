## Strand I — Mathematical substrate

The audited Trinity repository supports the mathematical substrate, but the
article must cite the correct files. The earlier statement *"75+ sacred values
in `src/tri/math/constants.zig`"* is inaccurate as written. That file
contains **15 curated `ConstantEntry` records** across four groups. The
larger constant table exists in `src/tri/math/sacred_constants_data.zig`,
which contains **154 numeric `pub const` values**.

The CHARTER layer is strong evidence. `src/sacred/CHARTER.md` contains eight
principles: *Exact Trinitism, Validated Empiricism, Lattice Consistency,
Tautology Prevention, Gamma Non-Axiom, Cross-Domain Consistency, Occam
Precedence, and Prediction Honesty.* This is the cleanest governance
artifact in the audit.

The Sacred VSA dimension 729 is also well supported. The Sacred stack
repeatedly treats $729 = 3^{6}$, including verification, lookup, vocabulary,
hidden dimension, and metabolism references. The article must disambiguate
this from the separate Firebird-style VSA implementation with dimension
10000.

**Article wording in force.** Strand I is grounded in a curated constant
layer and a larger numeric constant table: 15 documented `ConstantEntry`
rows in `constants.zig`, 154 numeric constants in
`sacred_constants_data.zig`, eight CHARTER principles, and a Sacred VSA
convention using dimension $729 = 3^{6}$. The repository also contains a
separate 10000-dimensional VSA convention, so the article refers
specifically to the *Sacred VSA* path when citing 729.
