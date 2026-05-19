# Golden Sunflowers — chapter source = NEON SSOT (single source of truth)

> **Anchor:** φ² + φ⁻² = 3 · Trinity · Zenodo DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)

## Single source of truth

The 98 chapters of the *Flos Aureus* / *Monumentum Aureum* monograph live in
**NEON Postgres**, table `ssot.chapters`, column `body_md`. There is **no**
hand-written markdown copy in this repository. Every chapter is generated
on demand from NEON by the Rust pipeline `crates/trios-phd`
(see [#372](https://github.com/gHashTag/trios/issues/372)).

```
NEON `ssot.chapters.body_md`  →  trios-phd compile-chapters  →  per-chapter PDF  →  monograph.pdf
```

Connection (read-only operations only — never `UPDATE` from a non-claimant):

```bash
export DB='postgresql://neondb_owner:<password>@ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb?sslmode=require&channel_binding=require'
psql "$DB" -c "SELECT ch_num, length(body_md) FROM ssot.chapters ORDER BY ch_num;"
```

## How to edit a chapter

Per the Golden Sunflowers ONE SHOT protocol ([#373](https://github.com/gHashTag/trios/issues/373)):

```sql
-- 1. Atomically claim the chapter (SKIP LOCKED, stale-lock release)
SELECT * FROM ssot.claim_one_shot('Ch.X', 'your-agent-name');

-- 2. Heartbeat every ≤ 2 min while writing
SELECT ssot.heartbeat(<oneshot_id>, 'your-agent-name', 'running', '<message>', <pct>);

-- 3. Update the canonical body
UPDATE ssot.chapters SET body_md = $$ ... $$ WHERE ch_num = 'Ch.X';

-- 4. Mark complete
SELECT ssot.complete_one_shot(<oneshot_id>, 'your-agent-name', '<PR-or-evidence-URL>');
```

The PR you open against `gHashTag/trios` is **only for the build pipeline**
(templates, Lua filters, migrations, the `trios-phd` Rust binary). It must
**not** add a per-chapter markdown file under `docs/golden-sunflowers/`,
because that would re-introduce a second source of truth.

## What this directory now contains

| Path | Source of truth | Notes |
|---|---|---|
| `README.md` | this file | static |
| `pdf/` | generated artefacts | output of `trios-phd compile-chapters`, kept for browsing only |

Per-chapter files (`ch-*.md`, `app-*.md`) were a NEON → repo mirror in v4.
They are removed because:

1. **R5 honest single-source rule.** The v5/v6 pipeline renders directly from
   NEON; the markdown copies were drifting (see [#372 v6.2 cycle](https://github.com/gHashTag/trios/issues/372#issuecomment-)).
2. **R6 lane discipline.** Multiple agents were editing the file *and* the
   NEON row in different orders, producing irreproducible PDFs.
3. **User instruction (2026-05-06):** *«всё лежит в NEON !! один источник
   правды !! и генерируется !! у нас нет руками написанных глав !! нужно
   всё перенести в NEON и удалить в других местах».*

## Counterpart audit issues

- [#372](https://github.com/gHashTag/trios/issues/372) — Golden Sunflowers SSOT spec.
- [#373](https://github.com/gHashTag/trios/issues/373) — Master epic / one-shot dispatch.
- [#265](https://github.com/gHashTag/trios/issues/265) — Original Flos Aureus PhD ONE SHOT (R1–R14).

## Reproducibility

Every published PDF carries:
- the `ssot.chapters` SHA-256 manifest at build time,
- the canon-locked git SHA of the `trios-phd` binary,
- the Zenodo DOI of the data anchor (10.5281/zenodo.19227877).

A mismatch between any of these and the rendered PDF is itself a §G falsifier
hit (see App. B Golden Ledger).
