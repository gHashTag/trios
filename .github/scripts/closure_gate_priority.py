#!/usr/bin/env python3
"""closure_gate_priority — variant of closure_gate.py for L-MR-MATRIX-PRIORITY.

Verifies that every cell in `assertions/matrix_priority_50.csv` (the 50-cell
priority subset selected for the 2026-06-15 defense / Rehearsal #2 timeline)
has R7-honest coverage in `assertions/matrix_samples.jsonl`.

R7-honest thresholds (identical to the production closure_gate.py):
  * MIN_ROWS_PER_CELL          ≥ 3   (env: CLOSURE_MIN_ROWS)
  * MIN_DISTINCT_SEEDS_PER_CELL ≥ 2   (env: CLOSURE_MIN_SEEDS)
  * MIN_MAX_STEP_PER_CELL      ≥ 3000 (env: CLOSURE_MIN_STEP)

Differences from production closure_gate.py:
  * Reads from the local JSONL artefact, not Postgres SSOT.
  * Scope is the 50-cell priority subset, not the 312-cell full grid.
  * Pure verdict tool: PASS / FAIL with cell-by-cell breakdown on stdout;
    NEVER posts comments, NEVER closes issues, NEVER touches GitHub state.
  * Exit code: 0 = PASS, 1 = FAIL, 2 = usage error.

Usage:
    python3 .github/scripts/closure_gate_priority.py
    python3 .github/scripts/closure_gate_priority.py --json   # machine-readable

Anchor: phi^2 + phi^-2 = 3 · DOI 10.5281/zenodo.19227877.
"""
from __future__ import annotations

import argparse
import csv
import json
import os
import sys
from collections import defaultdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PRIORITY_CSV_DEFAULT = REPO_ROOT / "assertions/matrix_priority_50.csv"
SAMPLES_JSONL_DEFAULT = REPO_ROOT / "assertions/matrix_samples.jsonl"

MIN_ROWS_PER_CELL = int(os.environ.get("CLOSURE_MIN_ROWS", "3"))
MIN_DISTINCT_SEEDS_PER_CELL = int(os.environ.get("CLOSURE_MIN_SEEDS", "2"))
MIN_MAX_STEP_PER_CELL = int(os.environ.get("CLOSURE_MIN_STEP", "3000"))


def load_priority(path: Path) -> list[tuple[str, str, int, str]]:
    if not path.exists():
        sys.stderr.write(f"closure_gate_priority: {path} not found\n")
        sys.exit(2)
    out: list[tuple[str, str, int, str]] = []
    with path.open(newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            out.append(
                (
                    row["format"],
                    row["algo"],
                    int(row["priority_rank"]),
                    row["priority_reason"],
                )
            )
    return out


def load_samples(path: Path) -> list[dict]:
    rows: list[dict] = []
    if not path.exists():
        # An absent JSONL is a legitimate state before the runner has been
        # executed — treat it as an empty sample set and let the gate fail
        # cell-by-cell rather than crashing.
        return rows
    with path.open() as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('{"_schema"'):
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            # Only consider rows with the canonical fields.
            if all(k in obj for k in ("format", "algo", "seed_phi", "step", "bpb")):
                rows.append(obj)
    return rows


def evaluate(
    priority: list[tuple[str, str, int, str]],
    samples: list[dict],
) -> tuple[bool, list[dict]]:
    by_cell: dict[tuple[str, str], list[dict]] = defaultdict(list)
    for s in samples:
        # closure_gate.py also requires bpb > 1.0 to filter out degenerate
        # zero-rows; mirror that filter here.
        try:
            if float(s["bpb"]) <= 1.0:
                continue
        except (TypeError, ValueError):
            continue
        by_cell[(s["format"], s["algo"])].append(s)

    breakdown: list[dict] = []
    all_pass = True
    for fmt, algo, rank, reason in priority:
        cell_rows = by_cell.get((fmt, algo), [])
        n_rows = len(cell_rows)
        try:
            distinct_seeds = len({int(r["seed_phi"]) for r in cell_rows})
            max_step = max((int(r["step"]) for r in cell_rows), default=0)
        except (TypeError, ValueError):
            distinct_seeds = 0
            max_step = 0
        passed = (
            n_rows >= MIN_ROWS_PER_CELL
            and distinct_seeds >= MIN_DISTINCT_SEEDS_PER_CELL
            and max_step >= MIN_MAX_STEP_PER_CELL
        )
        if not passed:
            all_pass = False
        reasons: list[str] = []
        if n_rows < MIN_ROWS_PER_CELL:
            reasons.append(f"n_rows={n_rows}<{MIN_ROWS_PER_CELL}")
        if distinct_seeds < MIN_DISTINCT_SEEDS_PER_CELL:
            reasons.append(
                f"distinct_seeds={distinct_seeds}<{MIN_DISTINCT_SEEDS_PER_CELL}"
            )
        if max_step < MIN_MAX_STEP_PER_CELL:
            reasons.append(f"max_step={max_step}<{MIN_MAX_STEP_PER_CELL}")
        breakdown.append(
            {
                "format": fmt,
                "algo": algo,
                "priority_rank": rank,
                "priority_reason": reason,
                "n_rows": n_rows,
                "distinct_seeds": distinct_seeds,
                "max_step": max_step,
                "passed": passed,
                "fail_reasons": reasons,
            }
        )
    return all_pass, breakdown


def render_text(breakdown: list[dict], passed: bool) -> str:
    lines: list[str] = []
    lines.append("L-MR-MATRIX-PRIORITY closure gate (50-cell subset)")
    lines.append(
        f"  thresholds: rows>={MIN_ROWS_PER_CELL}, "
        f"seeds>={MIN_DISTINCT_SEEDS_PER_CELL}, "
        f"step>={MIN_MAX_STEP_PER_CELL}"
    )
    n_total = len(breakdown)
    n_passed = sum(1 for c in breakdown if c["passed"])
    lines.append(f"  cells passed: {n_passed}/{n_total}")
    lines.append("")
    lines.append(
        f"  {'rank':>4}  {'format':<10}  {'algo':<14}  "
        f"{'rows':>4}  {'seeds':>5}  {'maxstep':>7}  verdict"
    )
    for c in breakdown:
        verdict = "PASS" if c["passed"] else f"FAIL ({'; '.join(c['fail_reasons'])})"
        lines.append(
            f"  {c['priority_rank']:>4}  {c['format']:<10}  {c['algo']:<14}  "
            f"{c['n_rows']:>4}  {c['distinct_seeds']:>5}  {c['max_step']:>7}  {verdict}"
        )
    lines.append("")
    lines.append(f"VERDICT: {'PASS' if passed else 'FAIL'}")
    lines.append("Anchor: phi^2 + phi^-2 = 3 · DOI 10.5281/zenodo.19227877")
    return "\n".join(lines) + "\n"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--priority-csv", default=str(PRIORITY_CSV_DEFAULT))
    ap.add_argument("--samples-jsonl", default=str(SAMPLES_JSONL_DEFAULT))
    ap.add_argument(
        "--json",
        action="store_true",
        help="Emit machine-readable JSON instead of the human table.",
    )
    args = ap.parse_args()

    priority = load_priority(Path(args.priority_csv))
    samples = load_samples(Path(args.samples_jsonl))
    passed, breakdown = evaluate(priority, samples)

    if args.json:
        out = {
            "verdict": "PASS" if passed else "FAIL",
            "thresholds": {
                "min_rows_per_cell": MIN_ROWS_PER_CELL,
                "min_distinct_seeds_per_cell": MIN_DISTINCT_SEEDS_PER_CELL,
                "min_max_step_per_cell": MIN_MAX_STEP_PER_CELL,
            },
            "cells_total": len(breakdown),
            "cells_passed": sum(1 for c in breakdown if c["passed"]),
            "breakdown": breakdown,
            "anchor": "phi^2 + phi^-2 = 3",
        }
        sys.stdout.write(json.dumps(out, indent=2) + "\n")
    else:
        sys.stdout.write(render_text(breakdown, passed))

    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
