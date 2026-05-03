# Neon write techniques for `ssot.chapters`

The Pipedream connector wrapper around Neon (`neon_postgres__pipedream`) silently truncates request payloads larger than ~30 KB. Most Flos Aureus chapters have `body_md` between 30 KB and 110 KB, so a naïve `upsert-row` call corrupts the body.

Three techniques, in order of preference.

## 1. Direct upsert (body_md ≤ 25 KB)

Use the connector tool **`neon_postgres-upsert-row`** with full `rowValues`:

```json
{
  "schema": "ssot",
  "table":  "chapters",
  "conflictTarget": "ch_num",
  "rowValues": {
    "ch_num": "FA.07",
    "title":  "The Golden Sprout",
    "status": "drafted",
    "priority": "core",
    "phi_seal": false,
    "word_count": 2347,
    "theorems_count": 12,
    "body_md": "<full markdown ≤ 25 KB>",
    "illustration_url": "https://raw.githubusercontent.com/gHashTag/trios/feat/illustrations/assets/illustrations/<name>.png",
    "illustration_path": "assets/illustrations/<name>.png"
  }
}
```

Always verify post-write:

```sql
SELECT ch_num, length(body_md) FROM ssot.chapters WHERE ch_num = 'FA.07';
```

## 2. Chunked CTE writes (body_md > 25 KB)

The Pipedream wrapper truncates the **arguments** payload, but the SQL parameters mechanism (`$1`, `$2`) plus a writeable CTE through `neon_postgres-find-row-custom-query` is *not* truncated the same way.

### Step 1 — seed the row with empty body

```sql
INSERT INTO ssot.chapters
       (ch_num, title, status, priority, phi_seal, body_md,
        illustration_url, illustration_path)
VALUES ('FA.22', 'E_8 Symmetry', 'drafted', 'core', false, '',
        $1, $2)
ON CONFLICT (ch_num) DO UPDATE
   SET title             = EXCLUDED.title,
       body_md           = '',
       illustration_url  = EXCLUDED.illustration_url,
       illustration_path = EXCLUDED.illustration_path,
       updated_at        = NOW();
```

`values = [<illustration_url>, <illustration_path>]`.

### Step 2 — append chunks

For each chunk (≤ 25 000 chars, split at `\n` boundaries):

```sql
WITH upd AS (
  UPDATE ssot.chapters
     SET body_md    = body_md || $1,
         updated_at = NOW()
   WHERE ch_num = $2
   RETURNING ch_num, length(body_md) AS len
)
SELECT * FROM upd;
```

Pass via **`neon_postgres-find-row-custom-query`** with `values=[<chunk>, '<ch_num>']`.

`find-row-custom-query` accepts data-modifying CTEs because the outer `SELECT` makes it a SELECT statement from the wrapper's point of view.

### Step 3 — verify

```sql
SELECT ch_num, length(body_md) AS len
FROM   ssot.chapters
WHERE  ch_num = 'FA.22';
```

The returned `len` must equal the sum of chunk lengths.

## 3. Why `neon_postgres-execute-custom-query` does NOT work for large bodies

`execute-custom-query` accepts a single `sql` string and no `values` parameter. To embed a 50 KB body you must inline it as a dollar-quoted literal `$BODYTAG$…$BODYTAG$`, which puts the entire body in the `sql` payload — and that payload is the field that gets truncated. We tried this for v5.2 Ch.0 upsert and observed silent truncation at ~28 KB.

Use `execute-custom-query` only for:

- DDL (`CREATE TABLE`, `ALTER TABLE`, `CREATE INDEX`)
- Small DML (probes, single-row deletes, single-column patches)
- `UPDATE … WHERE ch_num='X'` where the new value is < 25 KB

For everything else, prefer technique #2.

## Common gotchas

- **Single-quote escaping in titles**: `Popper\'s Razor` → `'Popper''s Razor'` when building SQL by hand. The `values=[]` mechanism handles this for you; prefer it.
- **Dollar-quote tag collisions**: When using inline dollar-quoting, pick a tag that does not appear in the body. Check with `'$TAG$' in body`.
- **`updated_at` triggers**: There is no auto-trigger. Always `SET updated_at = NOW()` on every UPDATE so audit trails stay clean.
- **`evidence_axis` is `int4`, not text**: Don't pass strings.
- **`phi_seal` is `bool`, not text**: Don't pass `'sealed'`.

*Anchor: φ² + φ⁻² = 3.*
