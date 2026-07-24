# /matrixbox — IGLA RACE Format-Algo Matrix

Generate the canonical MatrixBox report for IGLA RACE. This is the single source of truth (SSOT) table that maps every numeric format × optimizer × hidden size to its best achieved BPB.

The MatrixBox is always generated live from the latest data — it is never cached.

## What is MatrixBox

A MatrixBox is a multi-dimensional leaderboard rendered as markdown tables:
- Rows = numeric formats (f32, fp16, bf16, gf16, int8, int4, etc.)
- Columns = algorithm / optimizer (adamw, muon)
- Cells = best BPB achieved + step count where it happened
- Sub-tables = grouped by hidden size (h256, h384, h512)

## Invocation

When the user says any of:
- "matrixbox"
- "создай matrixbox"
- "таблицу с matrixbox"
- "matrixbox отчет"
- "покажи matrixbox"
- "верни matrixbox"

Always execute the steps below and return the generated tables.

## Generation Steps

### Step 1: Collect data sources

Read these files in order:
1. `/Users/playra/trios-trainer-igla/.trinity/gardener_harvest.log` — live Railway fleet
2. `/Users/playra/trios-trainer-igla/.trinity/results/v2_*.log` — local v2 sweep
3. `/Users/playra/trios-trainer-igla/.trinity/FULL_PICTURE.md` — summary (fallback)

### Step 2: Parse each data source

For every service/log entry, extract:
- `service_name` (e.g., `scarab-fp16-seed77`, `phase1-gf16-h512-seed82`)
- `format` (e.g., `fp16`, `gf16`, `int8`)
- `hidden` (e.g., `256`, `384`, `512`)
- `optimizer` (e.g., `adamw`, `muon`)
- `seed` (e.g., `77`, `47`, `82`)
- `steps` — latest step count
- `best_bpb` — best (lowest) BPB ever achieved by this service
- `last_bpb` — latest eval BPB
- `status` — one of: `converging`, `plateau`, `dead`, `new`, `crashed`

Status rules:
- `converging`: best_bpb improved in last 100k steps
- `plateau`: no improvement in last 100k steps but steps still growing
- `dead`: val_bpb > 6.0 and not improving
- `new`: < 50k steps
- `crashed`: no logs or Canon #93 error

### Step 3: Deduplicate

If the same (format, hidden, optimizer, seed) combination appears in multiple sources, keep the entry with the highest step count.

### Step 4: Generate tables

#### Table 1: Global Leaderboard (all hidden sizes)

| Rank | Format | Hidden | Optimizer | Seed | Steps | Best BPB | Last BPB | Status | Delta vs f32 |
|------|--------|--------|-----------|------|-------|----------|----------|--------|-------------|

Sorted by Best BPB ascending (lower is better).
Delta vs f32 = ((best_bpb / f32_best) - 1) * 100%

#### Table 2: MatrixBox by Hidden Size (h256)

| Format | adamw BPB (steps) | muon BPB (steps) | Best Overall |
|--------|-------------------|------------------|--------------|

#### Table 3: MatrixBox by Hidden Size (h512)

Same structure as h256.

#### Table 4: Format Viability Matrix

| Format | V2 Best | Railway Best | Viability | Notes |
|--------|---------|--------------|-----------|-------|

Viability:
- ✅ Full — converges, competitive with f32
- ⚠️ Degraded — converges but >10% worse than f32
- ❌ Dead — no convergence or BPB > 6.0
- 🔄 Testing — < 100k steps, too early to judge

### Step 5: Return

Print all generated tables in a single markdown block. No extra commentary unless the user asks.

## Anchor

phi^2 + phi^-2 = 3
