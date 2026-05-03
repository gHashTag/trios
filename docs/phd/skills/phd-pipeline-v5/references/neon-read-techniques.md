# Neon read techniques for `ssot.chapters`

The renderer reads its source from `/tmp/phd-render-out/chapters.json` — an array of `{ch_num, title, illustration_url, illustration_path, body_md}` objects, one per row. This file is hydrated from Neon before each render.

The Pipedream wrapper response payload has the same ~30 KB ceiling as the request payload, so you cannot pull all 98 rows in a single query.

## Strategy

### 1. Start with a manifest

```sql
SELECT ch_num, title, illustration_url, illustration_path, length(body_md) AS len
FROM   ssot.chapters
ORDER  BY ch_num;
```

Returns 98 rows × ~50 bytes each → ~5 KB. Always fits.

Walk the manifest and split chapters into:

- **small**  (`len ≤ 25 000`) — pull with a single batched query
- **big**    (`len  > 25 000`) — pull with the substring loop below

### 2. Batch-pull the small chapters

Group small chapters into batches whose summed `len` is ≤ 25 KB:

```sql
SELECT ch_num, title, illustration_url, illustration_path, body_md
FROM   ssot.chapters
WHERE  ch_num IN ($1, $2, $3, …);
```

Use **`neon_postgres-find-row-custom-query`** with `values=[<ch_num1>, <ch_num2>, …]`.

### 3. Substring-pull the big chapters

For each big chapter:

```sql
-- get total length first
SELECT length(body_md) FROM ssot.chapters WHERE ch_num = $1;
```

Then loop in 25 000-char strides:

```sql
SELECT substring(body_md FROM $2 FOR 25000) AS slice
FROM   ssot.chapters
WHERE  ch_num = $1;
```

`substring` is **1-indexed**: first call uses `$2 = 1`, second `$2 = 25001`, third `$2 = 50001`, etc., until the returned slice length is < 25 000.

Concatenate slices in order to reassemble `body_md`.

### 4. Reassemble + write JSON

```python
import json
chapters = []  # built up by the loop above
chapters.sort(key=lambda c: canonical_sort_key(c["ch_num"]))   # optional; renderer also sorts
with open("/tmp/phd-render-out/chapters.json", "w", encoding="utf-8") as f:
    json.dump(chapters, f, ensure_ascii=False)
```

## Verification

After hydrate:

```python
import json
d = json.load(open("/tmp/phd-render-out/chapters.json"))
print(len(d), sum(len(c["body_md"]) for c in d))
```

Cross-check against:

```sql
SELECT count(*), sum(length(body_md)) FROM ssot.chapters;
```

The two `(count, sum)` tuples must match exactly. If they don't, a chunk is missing — re-pull the offending `ch_num`.

## Why this is necessary

The renderer is offline-by-default: passing `--neon-url` enables a live pull, but most CI runs hydrate once and re-render N times against the cached JSON. This avoids:

- repeatedly hammering the Pipedream connector,
- depending on `NEON_URL` env in CI,
- subtle race conditions where Neon was updated mid-render.

For interactive / one-shot operator builds, you may pass `--neon-url $NEON_URL` and skip the cache step.

*Anchor: φ² + φ⁻² = 3.*
