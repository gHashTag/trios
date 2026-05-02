#!/usr/bin/env bash
# compile_all.sh — pull every chapter from ssot.chapters and render to PDF
# with hero-image @ 100% width, [H] placement.
#
# Inputs:
#   $NEON_URL    — postgres connection string
#   $REPO_ROOT   — directory containing this script (default: dirname $0)
#
# Outputs:
#   build/<ch_num>.pdf   — one PDF per chapter
#   build/monograph.pdf  — single concatenated PDF (optional, requires pdftk/qpdf)
#
# Usage:
#   NEON_URL='postgres://...' ./compile_all.sh
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")" && pwd)}"
TECTONIC="${TECTONIC:-tectonic}"
PANDOC="${PANDOC:-pandoc}"
PSQL="${PSQL:-psql}"

mkdir -p "$REPO_ROOT/src" "$REPO_ROOT/assets/illustrations" "$REPO_ROOT/build"

echo "[1/4] Pulling 44 chapters from ssot.chapters …"
"$PSQL" "$NEON_URL" -A -F $'\t' -t -c \
  "SELECT ch_num, illustration_path, illustration_url, body_md
     FROM ssot.chapters
    ORDER BY (CASE WHEN ch_num LIKE 'Ch.%' THEN 0 ELSE 1 END),
             CAST(NULLIF(regexp_replace(ch_num,'[^0-9]','','g'),'') AS int) NULLS LAST,
             ch_num" \
  > "$REPO_ROOT/build/chapters.tsv"

while IFS=$'\t' read -r ch_num illust_path illust_url body_md; do
  [ -z "$ch_num" ] && continue
  safe="${ch_num//\//_}"

  # Save markdown body
  printf '%b' "$body_md" > "$REPO_ROOT/src/${safe}.md"

  # Pre-fetch illustration locally so tectonic doesn't need network
  if [ -n "$illust_url" ] && [ -n "$illust_path" ]; then
    out_png="$REPO_ROOT/${illust_path}"
    mkdir -p "$(dirname "$out_png")"
    if [ ! -s "$out_png" ]; then
      curl -fsSL "$illust_url" -o "$out_png" || echo "WARN: failed to fetch $illust_url"
    fi
  fi
done < "$REPO_ROOT/build/chapters.tsv"

echo "[2/4] Rendering each chapter via pandoc + tectonic …"
ok=0; fail=0
for md in "$REPO_ROOT"/src/*.md; do
  base="$(basename "${md%.md}")"
  tex="$REPO_ROOT/build/${base}.tex"
  pdf="$REPO_ROOT/build/${base}.pdf"

  "$PANDOC" "$md" \
    --from markdown+tex_math_dollars+raw_tex \
    --template="$REPO_ROOT/templates/chapter.template.tex" \
    --lua-filter="$REPO_ROOT/filters/force-fullwidth-hero.lua" \
    --standalone \
    -o "$tex"

  # tectonic resolves relative paths from the .tex file's directory.
  # Patch the absolute github raw URL (which would force a network fetch)
  # into the local relative path under build/.
  ln -sf "$REPO_ROOT/assets" "$REPO_ROOT/build/assets" 2>/dev/null || true

  if "$TECTONIC" "$tex" >/dev/null 2>"$REPO_ROOT/build/${base}.log"; then
    ok=$((ok+1))
    echo "  ✅ $base"
  else
    fail=$((fail+1))
    echo "  ❌ $base — see build/${base}.log"
  fi
done

echo "[3/4] Summary: ${ok} ok, ${fail} failed"

echo "[4/4] (Optional) Concatenate all chapter PDFs into monograph.pdf"
if command -v pdftk >/dev/null 2>&1; then
  pdftk "$REPO_ROOT"/build/*.pdf cat output "$REPO_ROOT/build/monograph.pdf"
elif command -v qpdf >/dev/null 2>&1; then
  qpdf --empty --pages "$REPO_ROOT"/build/*.pdf -- "$REPO_ROOT/build/monograph.pdf"
else
  echo "  (skip: install pdftk or qpdf to concatenate)"
fi

echo "Done. Per-chapter PDFs in $REPO_ROOT/build/"
