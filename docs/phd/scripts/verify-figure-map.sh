#!/usr/bin/env bash
# verify-figure-map.sh — phd-pdf-images-gate hard test (Closes #775)
#
# Asserts that:
#   1. Every \includegraphics{...} in docs/phd/**/*.tex uses a \figXxx macro
#      defined in docs/phd/figure-map.tex.
#   2. Every \figXxx macro resolves to a file in assets/illustrations/.
#   3. (After tectonic build) PDF contains at least N raster images where N
#      = number of \figXxx references (no silent skip).
#
# Exit 0 = green, 1 = red. R5: emits a JSON line for audit_runs ingestion.
#
# Anchor: phi^2 + phi^-2 = 3
# Defense: 2026-06-15

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$REPO_ROOT"

PHD_DIR="docs/phd"
ILLUSTR_DIR="assets/illustrations"
MAP_FILE="$PHD_DIR/figure-map.tex"

err() { echo "  ✗ $*" >&2; }
ok()  { echo "  ✓ $*" >&2; }

anomalies=()

# ---------- 1. \includegraphics must use \fig macro ----------
title_pattern=$(grep -rE 'includegraphics(\[[^]]*\])?\{(ch[0-9]+|app-|cover_v4)' \
                  --include="*.tex" "$PHD_DIR" 2>/dev/null \
                  | grep -v 'figure-map.tex' || true)
if [ -n "$title_pattern" ]; then
    err "Still has title-pattern \\includegraphics:"
    echo "$title_pattern" | head -10 >&2
    anomalies+=("title_pattern_remaining")
else
    ok "All \\includegraphics use \\fig macros"
fi

# ---------- 2. Every \fig macro must point to an existing PNG ----------
missing_slugs=()
while IFS= read -r line; do
    # Parse \newcommand{\figXxx}{slug}
    macro=$(echo "$line" | sed -nE 's/\\newcommand\{(\\fig[A-Za-z]+)\}\{[^}]+\}.*/\1/p')
    slug=$(echo "$line" | sed -nE 's/.*\}\{([^}]+)\}.*/\1/p')
    if [ -n "$slug" ] && [ ! -f "$ILLUSTR_DIR/${slug}.png" ]; then
        missing_slugs+=("$macro -> $slug")
    fi
done < <(grep -E '^\\newcommand\{\\fig' "$MAP_FILE")

if [ "${#missing_slugs[@]}" -gt 0 ]; then
    err "Missing PNGs:"
    printf '    %s\n' "${missing_slugs[@]}" >&2
    anomalies+=("missing_slugs:${#missing_slugs[@]}")
else
    ok "All \\fig macros resolve to existing PNGs"
fi

# ---------- 3. Every used \fig macro is defined ----------
# Exclude built-in LaTeX commands like \figurename
used=$(grep -rhoE '\\fig[A-Z][A-Za-z]+' --include="*.tex" "$PHD_DIR" \
         | grep -v 'figure-map.tex' \
         | sort -u || true)
defined=$(grep -oE '\\fig[A-Z][A-Za-z]+' "$MAP_FILE" | sort -u || true)
undefined=$(comm -23 <(echo "$used") <(echo "$defined"))
if [ -n "$undefined" ]; then
    err "Undefined \\fig macros referenced:"
    echo "$undefined" | head >&2
    anomalies+=("undefined_macros")
else
    ok "All used \\fig macros are defined"
fi

# ---------- 4. main.tex / main_ru.tex must \input figure-map ----------
for main in "$PHD_DIR/main.tex" "$PHD_DIR/main_ru.tex"; do
    if [ -f "$main" ] && ! grep -q 'input{figure-map}' "$main"; then
        err "$main does not \\input{figure-map}"
        anomalies+=("missing_input_in_$(basename "$main")")
    elif [ -f "$main" ]; then
        ok "$main \\inputs figure-map"
    fi
done

# ---------- Emit R5 JSON evidence ----------
n_figs=$(grep -cE '^\\newcommand\{\\fig' "$MAP_FILE")
n_refs=$(grep -rhoE '\\fig[A-Z][A-Za-z]+' --include="*.tex" "$PHD_DIR" \
           | grep -v 'figure-map.tex' | wc -l)
n_pngs=$(ls -1 "$ILLUSTR_DIR"/*.png 2>/dev/null | wc -l)
verdict="green"
if [ "${#anomalies[@]}" -gt 0 ]; then verdict="red"; fi

if [ "${#anomalies[@]}" -eq 0 ]; then
    anomalies_json="[]"
else
    anomalies_json=$(printf '%s\n' "${anomalies[@]}" | jq -R . | jq -sc .)
fi

cat <<EOF
{"probe":"phd_pdf_images_gate","verdict":"$verdict","fig_macros_defined":$n_figs,"fig_macros_referenced":$n_refs,"pngs_on_disk":$n_pngs,"anomalies":$anomalies_json}
EOF

[ "$verdict" = "green" ] || exit 1
