## Strand II — Cognitive substrate

The v21 "Consciousness" strand is real as a repository theme, but the
exact module count needs careful wording. Two module systems coexist.
`docs/S3AI_BRAIN_MODULES.md` describes ten modules rooted at `src/tri/`:
*Insula, Amygdala, Basal Ganglia, Hippocampus, Hypothalamus, Thalamus,
Queen ACC, Queen DLPFC, Queen PCC, Queen OFC*. `docs/ARCHITECTURE.md`
describes a different 21-region list rooted at `src/brain/`.

The article must not claim a single clean "21 modules, 22k LOC" layer
without caveats. The audit found stale LOC counts, some files
substantially larger than documented, and one listed file
(`src/brain/cerebellum.zig`) absent. The scientific framing here is
*"two active cognitive taxonomies"*, not *"one finalized brain map."*

**Article wording in force.** Strand II implements the S3AI cognitive
layer as a developing brain-module architecture. The repository currently
contains two overlapping taxonomies: a ten-module `src/tri/` S3AI module
list and a broader `src/brain/` region list. This supports the
cognitive-layer interpretation, but the article treats module counts and
LOC totals as implementation-state evidence rather than finalized anatomy.
