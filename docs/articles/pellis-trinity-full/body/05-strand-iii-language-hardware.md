## Strand III — Language, ISA, FPGA, and Throne

The t27 audit confirms the core of TRI-27 but corrects several claims.
The ISA defines **27 registers**, a **27-trit register width**, and a
Coptic/Greek-style naming table. However, the audited specs describe **one
flat register file, not three banks.** They also do **not** define a
concrete 36-opcode count. The hardware ISA has a 6-bit opcode field,
implying an upper ceiling of 64, and seven operation classes, but no
canonical `NUM_OPCODES = 36`.

The Coptic naming claim is supported in spirit but needs a caveat. The
register-name table uses mostly Greek-block code points, has a collision
on U+03C6 between R5 and R20, and uses Cyrillic code points for R24–R26.
This is fixable, but the article does not overstate it as a clean Coptic
Unicode design yet.

### FPGA layer

The FPGA/hardware layer is concrete. `fpga/vivado/` contains GF16
primitives and testbench infrastructure, and the t27 tree includes a broad
`specs/fpga` pipeline.

> The Sacred ALU synthesis report in `gHashTag/trinity` reports
> **352 LUT, 165 FF, 1 DSP48E1, and 0.6 percent LUT utilization on
> XC7A100T**, with Fmax/latency/throughput still **estimates**.

Full place-and-route and cycle-accurate simulation were blocked, so Fmax,
latency, and throughput remain estimates in the present audit.

### Throne / orchestrator

The trios repository supports the "Throne" interpretation. PhD prose,
runtime server prompts, A2A capability declarations, and MCP endpoints
frame trios as the orchestrator/command surface. The 71-file PhD chapter
set physically exists, but the build currently includes only chapters
through `flos_69`; `flos_70` is a TRI-1 skeleton and is not yet wired into
`main.tex` or `main_ru.tex`.

### Integration label

> **Treat v21 as an integration label, not as a repository-native release
> string.**

**Article wording in force.** Strand III binds language and hardware
through t27 and trios. The t27 specs support 27 ternary registers, a
27-trit word shape, a generated-artifact discipline, and FPGA/GF16
artifacts. They do not yet support a canonical three-bank register file
or a fixed 36-opcode claim. The trios stack supplies the Throne /
orchestrator layer, while the TRI-1 PhD chapter currently remains a
skeleton outside the main monograph build.
