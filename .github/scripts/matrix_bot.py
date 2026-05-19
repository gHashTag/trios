#!/usr/bin/env python3
"""matrix-bot — regenerate the 312-cell Format×Algorithm matrix in #446.

Phase C lane L-C6 of gHashTag/trios#536. Reads ``ssot.bpb_samples`` on the
Phase C SSOT (Railway phd-postgres-ssot per trios-railway#62 workaround),
groups by ``(format, algo)`` taking the minimum bpb, and rewrites the body
of `gHashTag/trios#446` with a fresh Markdown table plus coverage progress.

Inputs (env):
  * ``MATRIX_DATABASE_URL``  — Postgres DSN. Required.
  * ``GITHUB_TOKEN``         — repo:write scope on ``gHashTag/trios``. Required.
  * ``MATRIX_FORMATS``       — optional comma-separated explicit format axis.
  * ``MATRIX_ALGOS``         — optional comma-separated explicit algo axis.
  * ``MATRIX_DRY_RUN``       — if set to "1", print the body and exit
                               without touching the issue.

R5 honest: zero coverage is rendered as "0/312 (0%)" without panicking.
R7 witness: row count, latest sha and run_id are printed to stderr for the
CI log.
Anchor: phi^2 + phi^-2 = 3.
"""

from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request
from collections import defaultdict
from datetime import datetime, timezone
from typing import Dict, Iterable, List, Tuple

try:
    import psycopg2  # type: ignore
except ImportError:
    sys.stderr.write(
        "matrix_bot: psycopg2 missing; install via 'pip install psycopg2-binary'\n"
    )
    sys.exit(78)


# ----------------------------------------------------------------------------- #
# Axis definition — must match `gHashTag/trios#446` `comment-id 4370442020`
# (39 formats x 9 algos = 351 cells; soap added per gHashTag/trios#596).
# ----------------------------------------------------------------------------- #

FORMATS_ORDERED: List[str] = [
    "f32",
    "f64",
    "fp16",
    "bf16",
    "tf32",
    "fp8_e4m3",
    "fp8_e5m2",
    "fp6_e2m3",
    "fp6_e3m2",
    "fp4_e2m1",
    "gf4",
    "gf8",
    "gf12",
    "gf16",
    "gf20",
    "gf24",
    "gf32",
    "gf64",
    "int4",
    "int8",
    "int16",
    "int32",
    "uint8",
    "nf4",
    "nf8",
    "posit8",
    "posit16",
    "posit32",
    "posit64",
    "lns8",
    "mxfp4",
    "mxfp6",
    "mxfp8",
    "decimal32",
    "decimal64",
    "decimal128",
    "binary128",
    "binary256",
    "fp80",
]

ALGOS_ORDERED: List[str] = [
    "adamw",
    "muon",
    "sgdm",
    "lion",
    "adafactor",
    "lamb",
    "schedulefree",
    "rmsprop",
    "soap",
]

GH_API = "https://api.github.com"
ISSUE_OWNER = "gHashTag"
ISSUE_REPO = "trios"
ISSUE_NUMBER = 446
MARK_BEGIN = "<!-- matrix-bot:begin -->"
MARK_END = "<!-- matrix-bot:end -->"


# ----------------------------------------------------------------------------- #
# Helpers
# ----------------------------------------------------------------------------- #


def env(name: str, *, required: bool = False, default: str | None = None) -> str | None:
    val = os.environ.get(name, default)
    if required and not val:
        sys.stderr.write(f"matrix_bot: ${name} required\n")
        sys.exit(2)
    return val


def fetch_min_bpb(dsn: str) -> Tuple[Dict[Tuple[str, str], float], int, str, str]:
    """Return (min_bpb_by_cell, total_rows, latest_sha, latest_run_id)."""
    cells: Dict[Tuple[str, str], float] = {}
    total_rows = 0
    latest_sha = ""
    latest_run_id = ""
    with psycopg2.connect(dsn) as conn:  # type: ignore[arg-type]
        with conn.cursor() as cur:
            cur.execute(
                "SELECT format, algo, MIN(bpb) "
                "  FROM ssot.bpb_samples "
                " WHERE bpb > 1.0 "
                " GROUP BY format, algo"
            )
            for fmt, algo, mb in cur.fetchall():
                cells[(str(fmt), str(algo))] = float(mb)
            cur.execute("SELECT COUNT(*) FROM ssot.bpb_samples")
            total_rows = int(cur.fetchone()[0])
            cur.execute(
                "SELECT sha, run_id FROM ssot.bpb_samples "
                " ORDER BY ts DESC LIMIT 1"
            )
            row = cur.fetchone()
            if row:
                latest_sha = row[0] or ""
                latest_run_id = row[1] or ""
    return cells, total_rows, latest_sha, latest_run_id


def render_table(
    cells: Dict[Tuple[str, str], float],
    formats: Iterable[str],
    algos: Iterable[str],
) -> str:
    formats = list(formats)
    algos = list(algos)
    head = "| Format ↓ \\ Algo → | " + " | ".join(f"**{a}**" for a in algos) + " |"
    rule = "|---" + "|---:" * len(algos) + "|"
    body_rows: List[str] = []
    # Compute per-row winners for bold marking.
    for fmt in formats:
        row_vals: List[Tuple[str, float | None]] = []
        for algo in algos:
            row_vals.append((algo, cells.get((fmt, algo))))
        finite = [v for _, v in row_vals if v is not None]
        winner = min(finite) if finite else None
        cells_md: List[str] = []
        for _, v in row_vals:
            if v is None:
                cells_md.append("🔲")
            elif winner is not None and abs(v - winner) < 1e-6:
                cells_md.append(f"**{v:.4f}**")
            else:
                cells_md.append(f"{v:.4f}")
        body_rows.append(f"| **{fmt}** | " + " | ".join(cells_md) + " |")
    return "\n".join([head, rule] + body_rows)


def render_progress(
    measured_cells: int,
    total_cells: int,
    formats_seen: int,
    formats_total: int,
    algos_seen: int,
    algos_total: int,
) -> str:
    def bar(n: int, d: int, width: int = 20) -> str:
        if d <= 0:
            return "░" * width
        filled = max(0, min(width, round(width * n / d)))
        return "█" * filled + "░" * (width - filled)

    pct_cells = (measured_cells * 100.0 / total_cells) if total_cells else 0.0
    pct_fmt = (formats_seen * 100.0 / formats_total) if formats_total else 0.0
    pct_algo = (algos_seen * 100.0 / algos_total) if algos_total else 0.0
    return (
        "```\n"
        f"Formats:  {bar(formats_seen, formats_total)}  {formats_seen}/{formats_total}  "
        f"({pct_fmt:.0f}%)\n"
        f"Algos:    {bar(algos_seen, algos_total)}   {algos_seen}/{algos_total}    "
        f"({pct_algo:.0f}%)\n"
        f"Cells:    {bar(measured_cells, total_cells)}  "
        f"{measured_cells}/{total_cells} ({pct_cells:.1f}%)\n"
        f"Target:   ████████████████████  {total_cells}/{total_cells} (100%) for PhD thesis\n"
        "```"
    )


def build_payload(
    cells: Dict[Tuple[str, str], float],
    total_rows: int,
    latest_sha: str,
    latest_run_id: str,
    formats: List[str],
    algos: List[str],
) -> str:
    measured_cells = sum(1 for f in formats for a in algos if (f, a) in cells)
    total_cells = len(formats) * len(algos)
    formats_seen = len({f for (f, _) in cells})
    algos_seen = len({a for (_, a) in cells})
    table = render_table(cells, formats, algos)
    progress = render_progress(
        measured_cells, total_cells, formats_seen, len(formats), algos_seen, len(algos)
    )
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%MZ")
    return (
        f"{MARK_BEGIN}\n"
        f"### 🔬 LIVE MATRIX (matrix-bot · L-C6 · {now})\n\n"
        f"Source: `ssot.bpb_samples` on Phase C SSOT (Railway phd-postgres-ssot, "
        f"trios-railway#62 workaround).\n"
        f"Total rows in SSOT: **{total_rows}**. Latest sha: `{latest_sha or '∅'}` "
        f"run_id `{latest_run_id or '∅'}`.\n\n"
        f"{table}\n\n"
        f"{progress}\n\n"
        f"Closure gate (L-C7): close `gHashTag/trios#446` ⇔ "
        f"`measured_cells == total_cells == {total_cells}` AND every cell has "
        f"`bpb > 1.0` AND anti-fake-pass guard PASS at steps>=3000.\n\n"
        f"Anchor: `φ² + φ⁻² = 3`.\n"
        f"{MARK_END}"
    )


def gh_request(
    method: str,
    path: str,
    token: str,
    payload: dict | None = None,
) -> dict:
    url = f"{GH_API}{path}"
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Authorization", f"Bearer {token}")
    req.add_header("Accept", "application/vnd.github+json")
    req.add_header("X-GitHub-Api-Version", "2022-11-28")
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read())
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", "replace")
        sys.stderr.write(f"matrix_bot: GH {method} {path} -> {e.code}: {body}\n")
        raise


def upsert_section(body: str, payload: str) -> str:
    if MARK_BEGIN in body and MARK_END in body:
        head, _, rest = body.partition(MARK_BEGIN)
        _, _, tail = rest.partition(MARK_END)
        return head + payload + tail
    sep = "\n\n---\n\n" if body.strip() else ""
    return body.rstrip() + sep + payload + "\n"


def main() -> int:
    dsn = env("MATRIX_DATABASE_URL", required=True)
    token = env("GITHUB_TOKEN", required=True)
    formats_env = env("MATRIX_FORMATS")
    algos_env = env("MATRIX_ALGOS")
    formats = [f for f in (formats_env.split(",") if formats_env else FORMATS_ORDERED) if f]
    algos = [a for a in (algos_env.split(",") if algos_env else ALGOS_ORDERED) if a]
    dry = env("MATRIX_DRY_RUN") == "1"
    fail_soft = env("MATRIX_FAIL_SOFT", default="1") == "1"

    try:
        cells, total_rows, sha, run_id = fetch_min_bpb(dsn)  # type: ignore[arg-type]
    except psycopg2.OperationalError as exc:  # type: ignore[attr-defined]
        # R5-honest fail-soft: stale DSN must NOT keep paging the cron loop
        # every hour. Log loudly, exit 0, and let the queen rotate the
        # secret out-of-band (tracked as a ONE SHOT issue).
        sys.stderr.write(
            f"matrix_bot: SSOT connection failed ({exc.__class__.__name__}): "
            f"{str(exc).strip()[:240]}\n"
        )
        sys.stderr.write(
            "matrix_bot: MATRIX_FAIL_SOFT=1 -> exiting 0 without PATCHing #446.\n"
            "matrix_bot: rotate secrets.MATRIX_DATABASE_URL on gHashTag/trios "
            "(see ONE SHOT issue) to restore live updates.\n"
        )
        return 0 if fail_soft else 3
    sys.stderr.write(
        f"matrix_bot: cells={len(cells)} total_rows={total_rows} "
        f"latest_sha={sha} latest_run_id={run_id}\n"
    )
    payload = build_payload(cells, total_rows, sha, run_id, formats, algos)
    if dry:
        print(payload)
        return 0

    issue = gh_request(
        "GET",
        f"/repos/{ISSUE_OWNER}/{ISSUE_REPO}/issues/{ISSUE_NUMBER}",
        token,
    )
    body = issue.get("body") or ""
    new_body = upsert_section(body, payload)
    if new_body == body:
        sys.stderr.write("matrix_bot: body unchanged; skipping PATCH.\n")
        return 0
    gh_request(
        "PATCH",
        f"/repos/{ISSUE_OWNER}/{ISSUE_REPO}/issues/{ISSUE_NUMBER}",
        token,
        payload={"body": new_body},
    )
    sys.stderr.write("matrix_bot: PATCH issue body OK.\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
