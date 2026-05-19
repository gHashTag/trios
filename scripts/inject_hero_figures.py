#!/usr/bin/env python3
"""Идемпотентно вставляет hero-figure block после \\chapter{...} в 8 пустых разделов.

Hero-block pattern (по образцу phd-pdf-images-gate skill):
  \\begin{figure}[h]
    \\centering
    \\includegraphics[width=0.85\\textwidth]{ASSET}
    \\caption{Hero figure for chapter}
    \\label{fig:HERO-SLUG}
  \\end{figure}
"""
import re
from pathlib import Path

PHD = Path("/tmp/trios-work/docs/phd")

PLAN = [
    ("appendix/E-lexicon.tex",              "E-lexicon.jpg",          "Master glossary cover: 360 terms mapped to chapters and Coq files.", "fig:hero-app-e"),
    ("appendix/H-acm-ae-checklist.tex",     "H-acm-ae-checklist.jpg", "ACM Artifact Evaluation packet: reviewer-facing checklist of badges, datasets, and reproduction scripts.", "fig:hero-app-h"),
    ("appendix/K-agent-memory.tex",         "K-agent-memory.jpg",     "Trinity hive state architecture: agents, lanes, and the shared memory of the swarm.", "fig:hero-app-k"),
    ("appendix/L-pollen-channel.tex",       "L-pollen-channel.jpg",   "Pollen channel: how agents broadcast partial results across the apiary.", "fig:hero-app-l"),
    ("appendix/N-zenodo-doi.tex",           "H-zenodo-doi.jpg",       "Zenodo DOI registry: permanent identifiers for every monograph version (DOI 10.5281/zenodo.19227877).", "fig:hero-app-n"),
    ("chapters/flos_69.tex",                "34-depin-mesh.jpg",      "Trinity GF16 ASIC as a self-sovereign dePIN mesh node: zero-multiplier silicon, $\\varphi^2+\\varphi^{-2}=3$.", "fig:hero-flos-69"),
    ("chapters/flos_70_silicon_proofs.tex", "ch_34.jpg",              "Silicon-strand proof bridge: 24 Coq theorems certifying bit-exact RTL behaviour across four bridges (anchor, GF16 safe domain, ASHA pruning, unitarity + Lucas closure).", "fig:hero-flos-70"),
]

def already_has_image(text):
    return bool(re.search(r"\\includegraphics(?:\[[^\]]*\])?\{[^}]+\}", re.sub(r"(?<!\\)%.*", "", text)))

def inject(tex_path, asset, caption, label):
    p = PHD / tex_path
    text = p.read_text()
    if already_has_image(text):
        return f"SKIP (already has image): {tex_path}"
    # find first \chapter{...} (possibly with * and/or balanced braces)
    m = re.search(r"\\chapter\*?\{[^}]+\}\s*(?:\\label\{[^}]+\})?", text)
    if not m:
        return f"FAIL (no \\chapter found): {tex_path}"
    insert_pos = m.end()
    hero_block = (
        f"\n\n\\begin{{figure}}[h]\n"
        f"  \\centering\n"
        f"  \\includegraphics[width=0.85\\textwidth]{{{asset}}}\n"
        f"  \\caption{{{caption}}}\n"
        f"  \\label{{{label}}}\n"
        f"\\end{{figure}}\n"
    )
    new_text = text[:insert_pos] + hero_block + text[insert_pos:]
    p.write_text(new_text)
    return f"OK: {tex_path} ← {asset}"

for tex, asset, caption, label in PLAN:
    print(inject(tex, asset, caption, label))
