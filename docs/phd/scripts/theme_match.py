#!/usr/bin/env python3
"""Theme-matched figure distribution for Flos Aureus monograph.

Each .tex file gets a manually-curated set of figures based on its REAL title,
not its filename slug. Pool of 154 figures used at most once globally.

Strategy:
  - Build TITLE-DRIVEN mapping table (slug → primary figure + secondary themes)
  - For RU chapters (NN-name.tex): use exact slug-match v516-NN-name.jpg as primary
  - For EN mirror chapters (ch_NN.tex): map by the actual chapter title to v516
    that has the closest theme — they often align (ch_06 GoldenFloat = 06-mantissa)
  - Secondary figures: thematically related from v520/v521 universal pool
  - Each figure used max 1 time globally (RU+EN+appendix combined)
"""
import re
from pathlib import Path
from collections import Counter

PHD = Path("/tmp/trios/docs/phd")
ASSETS = "assets/illustrations/v516"
POOL_DIR = PHD / ASSETS

# ============================================================================
# THEME MAPPING — primary figure + thematic secondaries for each .tex file
# ============================================================================

# Primary mapping: tex stem → v516 slug-matched primary figure
# For ch_NN files, mapped by ACTUAL title (not slug)
PRIMARY = {
    # Russian chapters (00-33) — exact slug match
    "00-monad": "v516-00-monad",
    "01-golden-egg": "v516-01-golden-egg",
    "02-golden-cut": "v516-02-golden-cut",
    "03-golden-harvest": "v516-03-golden-harvest",
    "04-golden-scales": "v516-04-golden-scales",
    "05-golden-bridge": "v516-05-golden-bridge",
    "06-golden-mantissa": "v516-06-golden-mantissa",
    "07-golden-sprout": "v516-07-golden-sprout",
    "08-golden-crystal": "v516-08-golden-crystal",
    "09-golden-seal": "v516-09-golden-seal",
    "10-golden-bloom": "v516-10-golden-bloom",
    "11-vesica-piscis": "v516-11-vesica-piscis",
    "12-flower-of-life": "v516-12-flower-of-life",
    "13-metatron-cube": "v516-13-metatron-cube",
    "14-platonic-solids": "v516-14-platonic-solids",
    "15-kepler-solids": "v516-15-kepler-solids",
    "16-sacred-ratios": "v516-16-sacred-ratios",
    "17-golden-spiral": "v516-17-golden-spiral",
    "18-torus-geometry": "v516-18-torus-geometry",
    "19-fibonacci-tesselation": "v516-19-fibonacci-tesselation",
    "20-standard-model": "v516-20-standard-model",
    "21-quantum-field": "v516-21-quantum-field",
    "22-e8-symmetry": "v516-22-e8-symmetry",
    "23-gf16-algebra": "v516-23-gf16-algebra",
    "24-igla-architecture": "v516-24-igla-architecture",
    "25-benchmarks": "v516-25-benchmarks",
    "26-data-analysis": "v516-26-data-analysis",
    "27-trinity-identity": "v516-27-trinity-identity",
    "28-momentum-algebra": "v516-28-momentum-algebra",
    "29-lucas-closure": "v516-29-lucas-closure",
    "30-golden-imagery": "v516-30-golden-imagery",
    "31-philosophy": "v516-31-philosophy",
    "32-conclusion": "v516-32-conclusion",
    "33-epilogue": "v516-33-epilogue",
    # English mirror chapters (ch_NN) — mapped by REAL title
    # ch_00 = Standard-Model phi → philosophy/standard-model
    "ch_00": "v520-31-codex-binding",  # standard model framing
    # ch_01 = Introduction TRINITY S3AI Vision → trinity-axis intro
    "ch_01": "v520-02-trinity-axis",
    # ch_02 = Background Neuro-Symbolic AI → matches RU 02-golden-cut theme
    "ch_02": "v520-03-codex-quill",  # symbolic/neural codex
    # ch_03 = Trinity Identity phi^2+phi^-2=3 → matches RU 03-golden-harvest
    "ch_03": "v520-19-magic-square",  # identity/lattice
    # ch_04 = Sacred Formula α_φ Derivation → matches RU 04-golden-scales
    "ch_04": "v520-22-quill-formula",
    # ch_05 = phi-distance Fibonacci-Lucas seeds → RU 05-golden-bridge
    "ch_05": "v520-23-stone-arch",  # bridge/arch
    # ch_06 = GoldenFloat Family GF4..GF64 → RU 06-golden-mantissa
    "ch_06": "v520-25-globe-meridian",  # measurement family
    # ch_07 = Vogel Phyllotaxis 137.5 → RU 07-golden-sprout
    "ch_07": "v520-15-tree-branch",  # botanical phyllotaxis
    # ch_08 = TF3/TF9 Sparse Ternary MatMul → RU 08-golden-crystal
    "ch_08": "v520-30-mosaic-tile",  # tessellated matrix
    # ch_09 = GF vs MXFP4 Ablation → RU 09-golden-seal
    "ch_09": "v520-38-coin-press",  # seal/press comparison
    # ch_10 = Coq L1 Range×Precision Pareto → RU 10-golden-bloom
    "ch_10": "v520-09-bell-curve",  # Pareto curve
    # ch_11 = Pre-registration H1 ≥3 seeds → RU 11-vesica-piscis
    "ch_11": "v520-12-key-lock",  # sealed protocol
    # ch_12 = Hardware Bridge (deferred) → RU 12-flower-of-life
    "ch_12": "v520-32-aqueduct",  # bridge/hardware
    # ch_13 = STROBE Sealed Seeds → distinct from 13-metatron
    "ch_13": "v520-27-cipher-disc",  # encoded/sealed
    # ch_14 = Eval Semantics BPB Metric → RU 14-platonic
    "ch_14": "v520-08-pyramid-section",  # platonic
    # ch_15 = BPB Benchmark Railway PostgreSQL → RU 15-kepler
    "ch_15": "v520-37-mill-stone",  # benchmark/grinding
    # ch_16 = 360-Lane Phi-Distance Grid → RU 16-sacred-ratios
    "ch_16": "v520-07-compass-rose",  # 360-degree dial
    # ch_17 = Ablation matrix → RU 17-golden-spiral
    "ch_17": "v520-21-loom-warp",  # matrix loom
    # ch_18 = Limitations → RU 18-torus
    "ch_18": "v520-29-scaffold",  # limitation/boundary
    # ch_19 = Statistical Analysis Welch-t → RU 19-fibonacci-tess
    "ch_19": "v520-09-bell-curve",  # CONFLICT — assigned to ch_10; reassign
    # Use bell-curve once for ch_10, give ch_19 something else
    # ch_20 = Reproducibility → RU 20-standard-model
    "ch_20": "v520-31-codex-binding",  # CONFLICT — assigned to ch_00
    # ch_21 = IGLA RACE Multi-Agent Fleet → RU 21-quantum-field
    "ch_21": "v520-24-water-mirror",  # multi-agent reflection
    # ch_22 = Railway/Trios Orchestration → RU 22-e8
    "ch_22": "v520-39-distillation",  # orchestration pipeline
    # ch_23 = MCP integration → RU 23-gf16
    "ch_23": "v520-13-spectacle-lens",  # integration/lens
    # ch_24 = Period-Locked Runtime Monitor → RU 24-igla
    "ch_24": "v520-11-hourglass",  # period-lock timer
    # ch_25 = phi-Period Cycles → RU 25-benchmarks
    "ch_25": "v520-28-water-clock",  # cycle/clock
    # ch_26 = KOSCHEI φ-Numeric Coprocessor ISA → RU 26-data-analysis
    "ch_26": "v520-12-key-lock",  # CONFLICT (ch_11)
    # ch_27 = TRI27 DSL → RU 27-trinity-identity
    "ch_27": "v520-18-tetractys",  # tri DSL
    # ch_28 = QMTech XC7A100T FPGA → RU 28-momentum-algebra
    "ch_28": "v520-20-orrery",  # FPGA mechanism
    # ch_29 = Sacred Formula V CKM/leptons → RU 29-lucas-closure
    "ch_29": "v520-10-prism-spectrum",  # particle spectrum
    # ch_30 = Trinity SAI VSA Associative Recall → RU 30-golden-imagery
    "ch_30": "v520-16-honey-comb",  # associative memory
    # ch_31 = Hardware Empirical 1003 toks HSLM → RU 31-philosophy
    "ch_31": "v520-40-mathematician-desk",
    # ch_32 = UART v6 Protocol → RU 32-conclusion
    "ch_32": "v520-14-musical-staff",  # protocol/signal
    # ch_33 = JTAG macOS BLK-001 Resolved → RU 33-epilogue
    "ch_33": "v520-36-ship-rigging",  # debugging/resolution
    # ch_34 = Energy 3000× DARPA → no RU sibling
    "ch_34": "v520-06-anvil-hammer",  # energy/forge
    # ch_35_mesh_node = Trinity GF16 ASIC dePIN Mesh
    "ch_35_mesh_node": "v516-ch_35_mesh_node",  # already exists with this exact slug
    # Appendices
    "A-catalogue": "v516-A-catalogue",
    "B-falsification": "v516-B-falsification",
    "C-golden-benchmark": "v516-C-golden-benchmark",
    "D-golden-mirror": "v516-D-golden-mirror",
    "E-lexicon": "v516-E-lexicon",
    "F-coq-citation-map": "v516-F-coq-citation-map",
    "F-fpga-bitstream": "v516-F-fpga-bitstream",
    "G-data-availability": "v516-G-data-availability",
    "H-acm-ae-checklist": "v516-H-acm-ae-checklist",
    "H-zenodo-doi": "v516-H-zenodo-doi",
    "I-xdc-pin-map": "v516-I-xdc-pin-map",
    "J-troubleshooting": "v516-J-troubleshooting",
    "K-agent-memory": "v516-K-agent-memory",
    "L-pollen-channel": "v516-L-pollen-channel",
}

# Resolve conflicts in ch_NN mappings (each figure used max 1 time)
# These were duplicates above, fix them:
PRIMARY["ch_19"] = "v520-19-magic-square"  # CONFLICT with ch_03 — use different
PRIMARY["ch_03"] = "v521-15-syllogism"     # syllogism = identity proof
PRIMARY["ch_20"] = "v521-26-vellum"        # reproducibility ledger
PRIMARY["ch_26"] = "v521-17-ratchet"       # ISA opcodes
PRIMARY["ch_19"] = "v521-28-quincunx"      # Welch-t = Galton quincunx (statistical!)


def slug_to_caption(slug):
    s = re.sub(r"^v5\d+-", "", slug)
    s = re.sub(r"^[A-Za-z]?-", "", s)
    s = re.sub(r"^\d+-", "", s)
    s = s.replace("-", " ").replace("_", " ")
    return s.title()


def sanitize(s):
    s = s.replace("&", "\\&").replace("%", "\\%").replace("$", "\\$").replace("#", "\\#")
    s = s.replace("_", " ")
    s = re.sub(r"[^\x00-\x7F]+", " ", s)
    s = re.sub(r"\s+", " ", s).strip()
    return s


FIGURE_TPL = r"""
\begin{{figure}}[H]
\centering
\makebox[\linewidth][c]{{\includegraphics[width=1.05\linewidth,keepaspectratio]{{{path}}}}}
\caption{{{caption}}}
\end{{figure}}
"""


def make_block(slug, real_title=None):
    name = slug_to_caption(slug)
    # If real_title is provided, use it for thematic caption
    if real_title:
        cap = f"Triptych illustrating the chapter '{real_title}' (Trinity S\\textsuperscript{{3}}AI v5.21.2, da Vinci x Azbuka)."
    else:
        cap = f"{name} - Trinity S\\textsuperscript{{3}}AI codex plate (da Vinci x Azbuka v5.21.2)."
    cap = sanitize(cap)
    return FIGURE_TPL.format(path=f"{ASSETS}/{slug}.jpg", caption=cap)


# Strip ALL existing figure blocks
FIG_BLOCK_RE = re.compile(r"\n?\\begin\{figure\}.*?\\end\{figure\}\s*", re.DOTALL)


def get_title(text):
    m = re.search(r'\\(chapter|section)\*?(?:\[[^\]]*\])?\{([^}]+)\}', text)
    if not m:
        return None
    title = m.group(2)
    title = re.sub(r"\\textsuperscript\{[^}]*\}", "", title)
    title = re.sub(r"\\texorpdfstring\{([^}]*)\}\{[^}]*\}", r"\1", title)
    title = re.sub(r"\\\\[a-zA-Z]+\{([^}]*)\}", r"\1", title)
    title = title.replace("---", "—").replace("--", "—")
    return title.strip()


# Pool of figures actually present
pool = sorted([f.stem for f in POOL_DIR.glob("*.jpg")])
pool_set = set(pool)
print(f"Pool: {len(pool)} figures")

# Verify all primary mappings exist in pool
missing = [(k, v) for k, v in PRIMARY.items() if v not in pool_set]
if missing:
    print(f"WARNING: {len(missing)} primary figures missing from pool:")
    for k, v in missing[:10]:
        print(f"  {k} -> {v}")

# Detect conflicts
primary_counts = Counter(PRIMARY.values())
conflicts = {k: v for k, v in primary_counts.items() if v > 1}
if conflicts:
    print(f"CONFLICTS — figures used as primary by multiple chapters:")
    for fig, n in conflicts.items():
        owners = [k for k, v in PRIMARY.items() if v == fig]
        print(f"  {fig} → {owners}")

# Allocate secondaries from unused pool, matched to file size
usage = Counter()
allocations = {}  # tex_stem → [list of figs]

# Build deterministic priority list of unused figures for secondary allocation
# Avoid v516-* with chapter slugs that aren't in PRIMARY (those are reserved)
# Priority: v521 (newest, varied) > v520 (universal) > leftover v516
unused_v521 = [p for p in pool if p.startswith("v521-")]
unused_v520 = [p for p in pool if p.startswith("v520-")]


# First pass: assign primaries
for stem, primary in PRIMARY.items():
    if primary in pool_set:
        allocations[stem] = [primary]
        usage[primary] += 1
    else:
        allocations[stem] = []
        print(f"  SKIP primary {stem} → {primary} (not in pool)")


# Compute target count per file based on length (read content)
def compute_targets():
    targets = {}
    for d, kind in [(PHD / "chapters", "chapter"), (PHD / "appendix", "appendix")]:
        for f in d.glob("*.tex"):
            text = FIG_BLOCK_RE.sub("", f.read_text())
            chars = len(text)
            n_sections = len(re.findall(r"\n\\section\{", text))
            if kind == "appendix":
                t = max(1, min(2, round(chars / 7000)))
            else:
                t = max(1, min(3, round(chars / 7000)))
            t = min(t, max(1, n_sections + 1))
            targets[f.stem] = t
    return targets


targets = compute_targets()


# Second pass: allocate secondaries
remaining_pool = [p for p in pool if usage[p] == 0]
# Order: v521 > v520 > rest
remaining_pool.sort(key=lambda p: (
    0 if p.startswith("v521-") else (1 if p.startswith("v520-") else 2),
    p
))

# Distribute secondaries cyclically — give each chapter one extra in rotation
# Use chapter order = sorted file stems
all_stems = sorted(allocations.keys())
# How many secondaries each chapter wants
need = {s: max(0, targets.get(s, 1) - len(allocations[s])) for s in all_stems}
total_need = sum(need.values())
print(f"Total secondary slots needed: {total_need}, available: {len(remaining_pool)}")

# Round-robin distribute
ptr = 0
while ptr < len(remaining_pool):
    progress = False
    for s in all_stems:
        if need[s] > 0 and ptr < len(remaining_pool):
            fig = remaining_pool[ptr]
            allocations[s].append(fig)
            usage[fig] += 1
            need[s] -= 1
            ptr += 1
            progress = True
    if not progress:
        break


def distribute_file(filepath, kind):
    text = filepath.read_text()
    text = FIG_BLOCK_RE.sub("", text)
    title = get_title(text) or ""
    figs = allocations.get(filepath.stem, [])
    if not figs:
        filepath.write_text(text)
        return 0

    # Insert primary after \chapter{...}\label{...} + first paragraph break
    new_text = text
    if kind == "chapter":
        anchor = re.search(r"(\\chapter\{[^}]*\}(?:\s*\\label\{[^}]*\})?)", new_text)
    else:
        anchor = re.search(r"(\\chapter\{[^}]*\}(?:\s*\\label\{[^}]*\})?)", new_text)
        if not anchor:
            anchor = re.search(r"(\\section\*?\{[^}]*\}(?:\s*\\label\{[^}]*\})?)", new_text)
    if anchor:
        pos = anchor.end()
        after = new_text[pos:]
        para = re.search(r"\n\n", after)
        if para and para.start() < 4000:
            pos += para.end()
        block = make_block(figs[0], title)
        new_text = new_text[:pos] + "\n" + block + new_text[pos:]

    # Insert secondaries after \section{...}, in reverse order to keep offsets
    # Skip the FIRST \section if there was no \chapter anchor (primary already lives there)
    if len(figs) > 1:
        had_chapter = bool(re.search(r"\\chapter\{", text))
        sections = list(re.finditer(r"(\\section\{[^}]*\}(?:\s*\\label\{[^}]*\})?)", new_text))
        # If no chapter, the primary is at the first section; secondaries start at section index 1
        start_idx = 0 if had_chapter else 1
        usable = sections[start_idx:]
        secondaries = figs[1:]
        n = min(len(usable), len(secondaries))
        for i in range(n - 1, -1, -1):
            sec = usable[i]
            slug = secondaries[i]
            pos = sec.end()
            after = new_text[pos:]
            para = re.search(r"\n\n", after)
            if para and para.start() < 3000:
                pos += para.end()
            block = make_block(slug)
            new_text = new_text[:pos] + "\n" + block + new_text[pos:]

    filepath.write_text(new_text)
    return len(figs)


# Run
total = 0
for d, kind in [(PHD / "chapters", "chapter"), (PHD / "appendix", "appendix")]:
    for f in sorted(d.glob("*.tex")):
        total += distribute_file(f, kind)

print(f"\nTotal placements: {total}")
print(f"Unique figures used: {sum(1 for v in usage.values() if v > 0)}")
print(f"Max usage: {max(usage.values()) if usage else 0}")
print(f"Histogram: {Counter(usage.values())}")
unused = [p for p in pool if usage[p] == 0]
print(f"Unused: {len(unused)} -> {unused[:10]}")

# Check primary correctness
print("\n=== PRIMARY VERIFICATION ===")
correct = 0
mismatched = []
for d in [PHD / "chapters", PHD / "appendix"]:
    for f in sorted(d.glob("*.tex")):
        text = f.read_text()
        figs = re.findall(r"\\includegraphics(?:\[[^\]]*\])?\{[^}]*illustrations/[^/]+/([A-Za-z0-9_\-]+)\.jpg\}", text)
        expected = PRIMARY.get(f.stem)
        actual_first = figs[0] if figs else None
        if actual_first == expected:
            correct += 1
        else:
            mismatched.append((f.stem, expected, actual_first))
print(f"Primary match: {correct} / 84")
if mismatched:
    print(f"Mismatched ({len(mismatched)}):")
    for s, exp, act in mismatched[:5]:
        print(f"  {s}: expected {exp}, got {act}")
