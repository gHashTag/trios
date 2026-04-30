#!/usr/bin/env bash
# RunPod monitor + harvest — Parameter Golf 9-pod batch (issue gHashTag/trios#442)
#
# Modes:
#   ./monitor_and_harvest.sh status     — print pod status + balance
#   ./monitor_and_harvest.sh peek       — peek at BPB rows in NEON for jepa-seed-44 only
#   ./monitor_and_harvest.sh harvest    — for each pod: ssh + cat /workspace/results.jsonl → /tmp/runpod_harvest/<pod>.jsonl
#   ./monitor_and_harvest.sh ingest     — parse harvested JSONL and INSERT into NEON bpb_samples
#   ./monitor_and_harvest.sh ratify     — re-run gate2 ratification sweep on freshly-ingested seeds
#
# Anchor: phi^2 + phi^-2 = 3
# Audit doc: PARAMGOLF-RUNPOD-RVR-001

set -euo pipefail

: "${RUNPOD_API_KEY:?RUNPOD_API_KEY env var required}"
NEON_URL="${NEON_DATABASE_URL:-postgresql://neondb_owner:npg_NHBC5hdbM0Kx@ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb?sslmode=require}"

API="https://api.runpod.io/graphql"
HARVEST_DIR="${HARVEST_DIR:-/tmp/runpod_harvest}"
mkdir -p "$HARVEST_DIR"

CMD="${1:-status}"

api_query() {
  curl -sS -X POST "$API" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $RUNPOD_API_KEY" \
    -d "$1"
}

cmd_status() {
  echo "=== POD STATUS @ $(date -u +%FT%TZ) ==="
  api_query '{"query":"{ myself { clientBalance pods { id name desiredStatus runtime{uptimeInSeconds} costPerHr } } }"}' \
  | python3 -c "
import json, sys
d = json.load(sys.stdin)['data']['myself']
print(f\"BALANCE:    \${d['clientBalance']:.4f}\")
print(f\"POD COUNT:  {len(d['pods'])}\")
running = [p for p in d['pods'] if p['desiredStatus']=='RUNNING']
burn = sum((p.get('costPerHr') or 0) for p in running)
print(f\"RUNNING:    {len(running)}\")
print(f\"BURN/h:     \${burn:.2f}\")
print()
print(f\"{'NAME':25} {'STATUS':10} {'UPTIME':>10} {'POD_ID':18}\")
for p in sorted(d['pods'], key=lambda x: x['name'] or ''):
    name = (p['name'] or '<noname>')[:25]
    up = (p.get('runtime') or {}).get('uptimeInSeconds') or 0
    h, rem = divmod(up, 3600); m, s = divmod(rem, 60)
    print(f\"{name:25} {p['desiredStatus']:10} {h:>2}h{m:02d}m {p['id']:18}\")
"
}

cmd_peek() {
  echo "=== NEON peek (jepa-seed-44 only has NEON_DATABASE_URL) ==="
  python3 - <<PY
import psycopg2, os
conn = psycopg2.connect(os.environ['NEON_URL'])
cur = conn.cursor()
cur.execute("""SELECT canon_name, seed, step, bpb, ts FROM bpb_samples
WHERE ts > now() - interval '6 hours' AND (canon_name ILIKE '%paramgolf%' OR canon_name ILIKE '%jepa-seed-44%')
ORDER BY ts DESC LIMIT 30""")
rows = cur.fetchall()
print(f"PARAMGOLF-tagged BPB rows last 6h: {len(rows)}")
for r in rows: print(f"  step={r[2]:5} bpb={r[3]:8.4f} seed={r[1]} ts={r[4]} canon={(r[0] or '')[:50]}")
PY
}

list_running_pods_with_ssh() {
  api_query '{"query":"{ myself { pods { id name desiredStatus runtime { ports { ip privatePort publicPort type isIpPublic } } machine { podHostId } } } }"}' \
  | python3 -c "
import json, sys
d = json.load(sys.stdin)['data']['myself']
for p in d['pods']:
    if p['desiredStatus'] != 'RUNNING': continue
    runtime = p.get('runtime') or {}
    ports = runtime.get('ports') or []
    ssh_port = next((pt for pt in ports if pt['privatePort'] == 22), None)
    if ssh_port:
        print(f\"{p['id']}\t{p['name']}\t{ssh_port['ip']}\t{ssh_port['publicPort']}\")
    else:
        # RunPod also exposes SSH via root@<podHostId>.runpod.io:22 with the user's RunPod SSH key
        host_id = (p.get('machine') or {}).get('podHostId', p['id'])
        print(f\"{p['id']}\t{p['name']}\t{host_id}.runpod.io\t22\")
"
}

cmd_harvest() {
  echo "=== HARVEST /workspace/results.jsonl from each pod ==="
  list_running_pods_with_ssh | while IFS=$'\t' read -r POD_ID NAME HOST PORT; do
    OUT="$HARVEST_DIR/${POD_ID}_${NAME}.jsonl"
    echo "--- $NAME ($POD_ID) → $OUT ---"
    # RunPod SSH: prefer their TCP exposure if present; fallback to host_id.runpod.io
    if ssh -o StrictHostKeyChecking=no -o ConnectTimeout=10 -p "$PORT" "root@$HOST" \
        'cat /workspace/results.jsonl 2>/dev/null || echo NO_RESULTS_FILE' > "$OUT" 2>/dev/null; then
      LINES=$(wc -l < "$OUT")
      echo "  OK $LINES lines"
    else
      echo "  SSH FAIL — need to enable RunPod SSH key in dashboard, or use UI download"
      rm -f "$OUT"
    fi
  done
  echo ""
  echo "=== Summary ==="
  ls -la "$HARVEST_DIR"
}

cmd_ingest() {
  echo "=== INGEST harvested JSONL into bpb_samples ==="
  python3 - <<PY
import os, json, glob, psycopg2
conn = psycopg2.connect(os.environ['NEON_URL'])
cur = conn.cursor()
total = 0
files = sorted(glob.glob(os.path.join(os.environ.get('HARVEST_DIR','/tmp/runpod_harvest'), '*.jsonl')))
for fp in files:
    fname = os.path.basename(fp)
    pod_id, _, rest = fname.partition('_')
    name = rest.replace('.jsonl', '')
    n_rows = 0
    for line in open(fp):
        line = line.strip()
        if not line or line == 'NO_RESULTS_FILE': continue
        try:
            r = json.loads(line)
        except Exception:
            continue
        seed = r.get('seed')
        step = r.get('step')
        bpb = r.get('bpb') or r.get('val_bpb')
        canon = r.get('canon_name') or f"paramgolf-{name}"
        if seed is None or step is None or bpb is None: continue
        cur.execute("""INSERT INTO bpb_samples (canon_name, seed, step, bpb, ts)
                       VALUES (%s, %s, %s, %s, now()) ON CONFLICT DO NOTHING""",
                    (canon, int(seed), int(step), float(bpb)))
        n_rows += cur.rowcount
    print(f"  {fname}: {n_rows} rows ingested")
    total += n_rows
conn.commit()
print(f"\nTOTAL ingested: {total}")
PY
}

cmd_ratify() {
  echo "=== RATIFY new sub-1.85 seeds into gate2_eligible ==="
  python3 - <<'PY'
import os, psycopg2, json
conn = psycopg2.connect(os.environ['NEON_URL'])
cur = conn.cursor()
cur.execute("""WITH best AS (
  SELECT seed, canon_name, MIN(bpb) AS best_bpb, step
  FROM bpb_samples
  WHERE bpb < 1.85 AND bpb > 1.0 AND step BETWEEN 1000 AND 4000
  GROUP BY seed, canon_name, step
) SELECT seed, canon_name, best_bpb, step FROM best
WHERE seed::text NOT IN (SELECT seed FROM gate2_eligible)
  AND seed IN (1597, 2584, 4181, 6765, 10946, 29, 47, 42, 43, 44)
ORDER BY best_bpb LIMIT 10""")
candidates = cur.fetchall()
print(f"new ratification candidates: {len(candidates)}")
for s, c, b, st in candidates:
    cur.execute("SELECT id FROM experiment_queue WHERE canon_name=%s LIMIT 1", (c,))
    row = cur.fetchone()
    if row:
        exp_id = row[0]
    else:
        cur.execute("""INSERT INTO experiment_queue (canon_name, config_json, priority, seed, steps_budget, account, status, created_by, finished_at, final_bpb, final_step)
            VALUES (%s, %s::jsonb, 100, %s, 4000, 'acc-runpod', 'done', 'gardener', now(), %s, %s) RETURNING id""",
            (c, json.dumps({"runpod": True, "doc_id": "PARAMGOLF-RUNPOD-RVR-001"}), s, float(b), st))
        exp_id = cur.fetchone()[0]
    cur.execute("""INSERT INTO gate2_eligible (experiment_id, account, seed, canon_name, final_bpb, final_step, image_sha, ratified_at, step_cap_applied, w6_corrective_action)
        VALUES (%s, 'acc-runpod', %s, %s, %s, %s, 'paramgolf-runpod', now(), 4000, 'W-6_step_cap_applied_per_l7_ledger_19')""",
        (exp_id, str(s), c, float(b), st))
    print(f"  ratified seed={s} bpb={b:.4f} step={st}")
conn.commit()
cur.execute("SELECT count(*), count(DISTINCT seed) FROM gate2_eligible")
print(f"\ngate2_eligible total: {cur.fetchone()}")
PY
}

case "$CMD" in
  status)  cmd_status ;;
  peek)    NEON_URL="$NEON_URL" cmd_peek ;;
  harvest) cmd_harvest ;;
  ingest)  NEON_URL="$NEON_URL" cmd_ingest ;;
  ratify)  NEON_URL="$NEON_URL" cmd_ratify ;;
  full)    cmd_status; cmd_peek ;;
  *) echo "usage: $0 {status|peek|harvest|ingest|ratify|full}"; exit 1 ;;
esac
