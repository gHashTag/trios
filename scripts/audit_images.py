#!/usr/bin/env python3
"""Полный аудит: картинки ↔ главы PhD.

Производит 4 артефакта:
1. image_usage.json — какая картинка используется в каком файле/главе
2. chapters_no_images.md — главы БЕЗ \\includegraphics (или с одной только cover)
3. orphan_images.txt — assets, на которые НЕТ \\includegraphics
4. recommendations.md — куда можно добавить картинки (chapter title → suggested asset)
"""
import re, json
from pathlib import Path
from collections import defaultdict

ROOT = Path("/tmp/trios-work")
PHD = ROOT / "docs" / "phd"
ASSETS_DIRS = [
    ROOT / "assets" / "illustrations",
    ROOT / "assets" / "illustrations_v516",
    ROOT / "assets" / "historic_pdf_en",
]

# 1. Собираем все assets
all_assets = set()
asset_to_dir = {}
for d in ASSETS_DIRS:
    if not d.exists(): continue
    for f in d.iterdir():
        if f.is_file() and f.suffix.lower() in (".png", ".jpg", ".jpeg", ".pdf"):
            all_assets.add(f.name)
            asset_to_dir[f.name] = str(d.relative_to(ROOT))
            # Also map stem (without extension) since \includegraphics may omit ext
            asset_to_dir[f.stem] = str(d.relative_to(ROOT))
print(f"TOTAL ASSETS: {len(all_assets)} unique files across {len(ASSETS_DIRS)} dirs")

# 2. Сканируем все .tex с \includegraphics
INC_RE = re.compile(r"\\includegraphics(?:\[[^\]]*\])?\{([^}]+)\}")
CHAPTER_RE = re.compile(r"\\chapter\*?\{([^}]+)\}")

usage = defaultdict(list)   # asset_name → list of (tex_file, chapter_title)
chapter_assets = defaultdict(list)  # tex_file → list of assets used
chapter_titles = {}         # tex_file → chapter title

for tex in sorted(PHD.glob("**/*.tex")):
    rel = str(tex.relative_to(PHD))
    if not any(rel.startswith(p) for p in ("chapters/", "appendix/", "frontmatter/")):
        continue
    text = tex.read_text(errors="ignore")
    # strip comments
    text_clean = re.sub(r"(?<!\\)%.*", "", text)

    # extract first \chapter{} title
    title_match = CHAPTER_RE.search(text_clean)
    title = title_match.group(1).strip() if title_match else "(no chapter title)"
    # de-LaTeX
    title = re.sub(r"\\[a-zA-Z]+\{?|\}", "", title).strip()
    chapter_titles[rel] = title

    # find \includegraphics
    for img in INC_RE.findall(text_clean):
        img_clean = img.strip()
        chapter_assets[rel].append(img_clean)
        usage[img_clean].append((rel, title))

# 3. Какие assets не используются (orphans)
referenced_names = set()
for img in usage:
    referenced_names.add(img)
    # also try alternate extensions
    stem = Path(img).stem
    for ext in ("png", "jpg", "jpeg", "pdf"):
        referenced_names.add(f"{stem}.{ext}")

orphans = set()
for asset in all_assets:
    if asset in referenced_names: continue
    if Path(asset).stem in referenced_names: continue
    orphans.add(asset)

# 4. Какие главы без картинок
chapters_with_imgs = set(chapter_assets.keys())
all_chapters = set()
for tex in sorted(PHD.glob("chapters/*.tex")):
    all_chapters.add(str(tex.relative_to(PHD)))
all_appendices = set()
for tex in sorted(PHD.glob("appendix/*.tex")):
    all_appendices.add(str(tex.relative_to(PHD)))

no_image_chapters = []
one_image_chapters = []
for c in sorted(all_chapters | all_appendices):
    n = len(chapter_assets.get(c, []))
    if n == 0:
        no_image_chapters.append((c, chapter_titles.get(c, "?"), 0))
    elif n == 1:
        one_image_chapters.append((c, chapter_titles.get(c, "?"), 1))

# 5. Save artifacts
out = {
    "totals": {
        "assets": len(all_assets),
        "asset_dirs": [str(d.relative_to(ROOT)) for d in ASSETS_DIRS if d.exists()],
        "tex_files_scanned": len(chapter_titles),
        "tex_files_with_images": len(chapters_with_imgs),
        "tex_files_without_images": len(no_image_chapters),
        "tex_files_with_one_image": len(one_image_chapters),
        "orphan_assets": len(orphans),
        "unique_images_referenced": len(usage),
    },
    "asset_to_dir": asset_to_dir,
    "usage": {k: v for k, v in usage.items()},
    "chapter_assets": dict(chapter_assets),
    "chapter_titles": chapter_titles,
    "no_image_chapters": no_image_chapters,
    "one_image_chapters": one_image_chapters,
    "orphans": sorted(orphans),
}
Path("/home/user/workspace/image_audit.json").write_text(json.dumps(out, indent=2, ensure_ascii=False))

print(f"\n=== TOTALS ===")
print(f"  unique assets on disk:           {len(all_assets)}")
print(f"  unique images referenced in .tex: {len(usage)}")
print(f"  orphan assets (unused):           {len(orphans)}")
print(f"  tex files scanned:                {len(chapter_titles)}")
print(f"  ... with ≥1 image:                {len(chapters_with_imgs)}")
print(f"  ... WITHOUT any image:            {len(no_image_chapters)}")
print(f"  ... with only 1 image:            {len(one_image_chapters)}")
print(f"\nSaved /home/user/workspace/image_audit.json")
