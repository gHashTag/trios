#!/usr/bin/env bash
# =============================================================================
# compile_chapter.sh — PhD v5 per-chapter Markdown -> TeX -> PDF pipeline.
#
# Usage:
#   scripts/compile_chapter.sh <input.md> <output_basename>
#
# Behaviour:
#   1. Pandoc converts the Markdown chapter (body_md from ssot.chapters)
#      to LaTeX using templates/chapter.template.tex.
#   2. filters/force-fullwidth-hero.lua promotes the first standalone image
#      to position 1 and forces width=100%.
#   3. tectonic compiles the resulting .tex to .pdf.
#
# Environment:
#   PANDOC   — pandoc binary (default: pandoc)
#   TECTONIC — tectonic binary (default: tectonic)
# =============================================================================

set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: $0 <input.md> <output_basename>" >&2
  exit 64
fi

IN="$1"
OUT="$2"

PANDOC="${PANDOC:-pandoc}"
TECTONIC="${TECTONIC:-tectonic}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMPLATE="${ROOT}/templates/chapter.template.tex"
LUA_FILTER="${ROOT}/filters/force-fullwidth-hero.lua"

[[ -f "${TEMPLATE}"   ]] || { echo "missing template: ${TEMPLATE}"   >&2; exit 66; }
[[ -f "${LUA_FILTER}" ]] || { echo "missing filter:   ${LUA_FILTER}" >&2; exit 66; }
[[ -f "${IN}"         ]] || { echo "missing input:    ${IN}"         >&2; exit 66; }

"${PANDOC}" "${IN}" \
  --from=markdown \
  --to=latex \
  --standalone \
  --template="${TEMPLATE}" \
  --lua-filter="${LUA_FILTER}" \
  -o "${OUT}.tex"

"${TECTONIC}" --keep-logs --outdir "$(dirname "${OUT}")" "${OUT}.tex"

echo "[v5] compiled ${OUT}.pdf"
