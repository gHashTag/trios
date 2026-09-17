#!/usr/bin/env bash
# TURBOBABY SPECS — the extracted business law, checked by generating it.
#
# `specs/turbobaby/*.t27` carries the decision law of two Telegram bots
# (mxfill77/turbobaby-manager-bot and mxfill77/turbobaby-userbot) written where
# it can be generated instead of transcribed. Nothing in the app imports these
# yet, which is exactly why they need a gate: an unreferenced spec rots in
# total silence, and the first person to notice is whoever generates from it a
# year later.
#
# WHY COUNTS AND NOT EXIT STATUS. t27c's parser discards any statement it
# cannot parse - `parse_fn_body` recovers to the next `;` at brace depth zero -
# and then exits 0 with an empty stderr. Trusting its exit code means a spec
# can lose half its branches and still look healthy. So every function,
# constant and `return` in the spec is counted against the generated Rust, per
# function, and a single missing branch fails this gate.
#
# Comment lines are stripped before counting on both sides. The word "return"
# occurs in the prose of these files often enough that not stripping it
# produces a false alarm - it already did once.
#
# Skipped, not failed, when t27c is absent: the compiler lives in a sibling
# repository, and a machine without it must not be told the specs are broken.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SPEC_DIR="$ROOT/specs/turbobaby"
T27C="${T27C:-$HOME/t27/target/release/t27c}"

if [ ! -x "$T27C" ]; then
  echo "[SKIP] turbobaby_specs: NOT MEASURED - no t27c at $T27C"
  exit 0
fi
if [ ! -d "$SPEC_DIR" ]; then
  echo "FAIL [turbobaby_specs]: $SPEC_DIR does not exist"
  exit 1
fi

shopt -s nullglob
SPECS=("$SPEC_DIR"/*.t27)
if [ "${#SPECS[@]}" -eq 0 ]; then
  # Zero specs checked cleanly is the shape of a gate that has stopped
  # measuring anything. Say so rather than print OK.
  echo "FAIL [turbobaby_specs]: no .t27 files under $SPEC_DIR"
  exit 1
fi

TMP="$(mktemp -d /tmp/tb-specs.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

fail=0
for spec in "${SPECS[@]}"; do
  name="$(basename "$spec" .t27)"
  out="$TMP/$name.rs"

  if ! "$T27C" gen-rust "$spec" > "$out" 2>"$TMP/$name.err"; then
    echo "FAIL [turbobaby_specs]: $name did not generate"
    sed 's/^/       /' "$TMP/$name.err" | head -5
    fail=1
    continue
  fi

  if ! SPEC="$spec" OUT="$out" NAME="$name" python3 - <<'PY'
import os, re, sys

def strip(text):
    return "\n".join(l for l in text.split("\n")
                     if not l.lstrip().startswith("//"))

def per_fn(text):
    counts = {}
    marks = list(re.finditer(r'pub fn (\w+)', text))
    for i, m in enumerate(marks):
        end = marks[i + 1].start() if i + 1 < len(marks) else len(text)
        counts[m.group(1)] = len(re.findall(r'\breturn\b', text[m.end():end]))
    return counts

spec = strip(open(os.environ["SPEC"]).read())
gen = strip(open(os.environ["OUT"]).read())
name = os.environ["NAME"]

problems = []

for what, pat in (("functions", r'\bpub fn '), ("constants", r'\bpub const ')):
    a, b = len(re.findall(pat, spec)), len(re.findall(pat, gen))
    if a != b:
        problems.append("%s: spec %d, generated %d" % (what, a, b))

if not re.findall(r'\bpub fn ', spec):
    problems.append("the spec declares no functions at all")

sf, gf = per_fn(spec), per_fn(gen)
for fn, n in sf.items():
    if fn not in gf:
        problems.append("%s() is missing from the generated Rust" % fn)
    elif gf[fn] != n:
        problems.append("%s(): %d returns in the spec, %d generated "
                        "- a branch was dropped" % (fn, n, gf[fn]))

if problems:
    print("FAIL [turbobaby_specs]: %s lost law in generation" % name)
    for p in problems:
        print("       %s" % p)
    sys.exit(1)

print("  %-18s %2d fns, %2d consts, %2d returns - all present"
      % (name, len(sf), len(re.findall(r'\bpub const ', spec)),
         sum(sf.values())))
PY
  then
    fail=1
    continue
  fi

  # Generating is not the same as being generable INTO SOMETHING THAT BUILDS.
  # Bare rustc, no crate, no dependencies - if the law needs a runtime to
  # compile, it is not law any more.
  if command -v rustc >/dev/null 2>&1; then
    if ! rustc --crate-type lib --edition 2021 \
         -A unused_parens -A dead_code \
         "$out" -o "$TMP/$name.rlib" 2>"$TMP/$name.rustc"; then
      echo "FAIL [turbobaby_specs]: generated Rust for $name does not compile"
      sed 's/^/       /' "$TMP/$name.rustc" | head -15
      fail=1
    fi
  else
    echo "[SKIP] turbobaby_specs: NOT MEASURED - no rustc, so $name was"
    echo "       counted but never compiled by this run."
  fi
done

if [ "$fail" != "0" ]; then
  exit 1
fi

echo "[OK] ${#SPECS[@]} turbobaby spec(s) generate whole and compile"
