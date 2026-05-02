#!/usr/bin/env bash
# render.sh — render all chapters under src/*.md to build/<ch>.pdf using
# the v5 pipeline (pandoc + force-fullwidth-hero.lua + tectonic).
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
TECTONIC="${TECTONIC:-/home/user/workspace/tectonic}"

mkdir -p "$ROOT/build"
ln -sfn "$ROOT/assets" "$ROOT/build/assets"

ok=0; fail=0
for md in "$ROOT"/src/*.md; do
  base="$(basename "${md%.md}")"
  tex="$ROOT/build/${base}.tex"
  pdf="$ROOT/build/${base}.pdf"
  log="$ROOT/build/${base}.log"

  # Pre-process: pandoc's $...$ math heuristic occasionally mis-pairs when
  # two short fragments sit close to each other (e.g. `$10^{13}$ ...; at $\sim$0.1`).
  # Convert any `$\sim$` to the unambiguous `\(\sim\)` form. Same for a
  # handful of other tiny math fragments that are common in this corpus.
  md_safe="$ROOT/build/${base}.preproc.md"
  # Replace tiny `$\macro$` math fragments (which pandoc misparses when
  # adjacent to other $-math) with their Unicode equivalents — fontspec
  # in the template renders these correctly.
  sed -E \
    -e 's/\$\\sim\$/∼/g' \
    -e 's/\$\\approx\$/≈/g' \
    -e 's/\$\\propto\$/∝/g' \
    -e 's/\$\\pm\$/±/g' \
    -e 's/\$\\times\$/×/g' \
    -e 's/\$\\to\$/→/g' \
    "$md" > "$md_safe"

  pandoc "$md_safe" \
    --from markdown+tex_math_dollars+tex_math_single_backslash+raw_tex \
    --template="$ROOT/templates/chapter.template.tex" \
    --lua-filter="$ROOT/filters/force-fullwidth-hero.lua" \
    --standalone \
    -o "$tex" 2>"$log"
  pstatus=$?
  if [ $pstatus -ne 0 ]; then
    fail=$((fail+1))
    echo "❌ pandoc failed for $base — see $log"
    continue
  fi

  # rewrite absolute illustration URLs to local paths under build/assets/
  sed -i \
    -e 's#https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/##g' \
    "$tex"

  if "$TECTONIC" --keep-logs --outdir "$ROOT/build" "$tex" >>"$log" 2>&1; then
    if [ -s "$pdf" ]; then
      ok=$((ok+1))
      printf "✅ %-7s %5d KB\n" "$base" "$(($(stat -c%s "$pdf")/1024))"
    else
      fail=$((fail+1))
      echo "❌ tectonic produced no PDF for $base — see $log"
    fi
  else
    fail=$((fail+1))
    echo "❌ tectonic failed for $base — see $log"
  fi
done

echo
echo "Rendered: ${ok} ok, ${fail} failed"
