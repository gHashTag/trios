#!/usr/bin/env bash
# RunPod batch deploy — Parameter Golf 3-PR submission (issue gHashTag/trios#442)
#
# Spawns 9 RTX 4090 spot pods on RunPod ACC-2 (...1lan, $240.40):
#   PR1 JEPA-T:               seeds 43, 44, 42
#   PR2 Universal Ring:       seeds 1597, 2584, 4181
#   PR3 NCA E2E TTT:          seeds 43, 44, 42
#
# Anchor: phi^2 + phi^-2 = 3
# Gate-2 horizon: STEPS=4236 (rung 3 from theorem-derived ASHA schedule)
# Hyperparameters from t27/proofs/derived/TrainerConfig.v (PR trios#440):
#   HIDDEN=384  (= d_model_min, INV-3)
#   LR=0.00306  (= lr_alpha_phi, INV-1)
#
# Usage:  RUNPOD_API_KEY=<acc2-key> ./deploy_pr_batch.sh
#
# Cost estimate (90 min wall clock, 9 pods @ $0.34/h spot):
#   9 * 1.5h * $0.34 = $4.59 worst case
#   ACC-2 balance: $240.40  →  ample headroom (>50 such batches)

set -euo pipefail

: "${RUNPOD_API_KEY:?RUNPOD_API_KEY env var required (use ACC-2 ...1lan)}"

NEON_DATABASE_URL="${NEON_DATABASE_URL:-postgresql://neondb_owner:npg_NHBC5hdbM0Kx@ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb?sslmode=require}"

API="https://api.runpod.io/graphql"
IMAGE="ghcr.io/ghashtag/trios-trainer-igla:latest"
GPU_TYPE_ID="NVIDIA GeForce RTX 4090"
BID_USD_PER_HR=0.34
CONTAINER_DISK_GB=20
VOLUME_GB=0
HIDDEN=384
LR=0.00306
STEPS=4236

# Output collector
OUT="/tmp/runpod_deploy_$(date +%s).jsonl"
echo "Deploy log → $OUT"

# JSON-quote helper
quote() { python3 -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$1"; }

deploy_pod() {
  local PR="$1"          # PR1 / PR2 / PR3
  local OBJECTIVE="$2"   # JEPA / RING / NCA
  local SEED="$3"
  local POD_NAME="paramgolf-${PR,,}-${OBJECTIVE,,}-seed${SEED}"

  local ENV_JSON
  ENV_JSON=$(cat <<JSON
[
  {"key":"OBJECTIVE","value":"${OBJECTIVE}"},
  {"key":"PR_TARGET","value":"${PR}"},
  {"key":"SEED","value":"${SEED}"},
  {"key":"HIDDEN","value":"${HIDDEN}"},
  {"key":"LR","value":"${LR}"},
  {"key":"STEPS","value":"${STEPS}"},
  {"key":"STEP_CAP","value":"4000"},
  {"key":"EARLY_STOP_BPB","value":"1.85"},
  {"key":"NEON_DATABASE_URL","value":"${NEON_DATABASE_URL}"},
  {"key":"RUN_NAME","value":"${POD_NAME}"},
  {"key":"DOC_ID","value":"PARAMGOLF-RVR-001"},
  {"key":"WAVE","value":"PARAMGOLF-${PR}"},
  {"key":"ANCHOR","value":"phi2_plus_phi_minus_2_eq_3"}
]
JSON
)

  # GraphQL mutation: podRentInterruptable (spot bid, no template)
  read -r -d '' QUERY <<'GQL' || true
mutation ($input: PodRentInterruptableInput!) {
  podRentInterruptable(input: $input) {
    id
    desiredStatus
    imageName
    machine { gpuDisplayName }
    costPerHr
  }
}
GQL

  # Build variables JSON
  local VARS
  VARS=$(python3 - <<PY
import json, os
print(json.dumps({
  "input": {
    "bidPerGpu": ${BID_USD_PER_HR},
    "cloudType": "COMMUNITY",
    "gpuCount": 1,
    "gpuTypeId": "${GPU_TYPE_ID}",
    "minMemoryInGb": 24,
    "minVcpuCount": 4,
    "containerDiskInGb": ${CONTAINER_DISK_GB},
    "volumeInGb": ${VOLUME_GB},
    "name": "${POD_NAME}",
    "imageName": "${IMAGE}",
    "ports": "8080/http",
    "env": ${ENV_JSON}
  }
}))
PY
)

  # Send
  local PAYLOAD
  PAYLOAD=$(python3 -c "import json,sys; q=open('/dev/stdin').read(); v=json.loads(sys.argv[1]); print(json.dumps({'query': q, 'variables': v}))" "$VARS" <<< "$QUERY")

  local RESP
  RESP=$(curl -sS -X POST "$API" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ${RUNPOD_API_KEY}" \
    -d "$PAYLOAD")

  echo "$RESP" | python3 -c "
import json, sys, os
r = json.load(sys.stdin)
data = r.get('data', {}) or {}
pod = (data.get('podRentInterruptable') or {})
errs = r.get('errors')
log = {
    'pr': '${PR}', 'objective': '${OBJECTIVE}', 'seed': ${SEED},
    'pod_name': '${POD_NAME}',
    'pod_id': pod.get('id'),
    'cost_per_hr': pod.get('costPerHr'),
    'gpu': (pod.get('machine') or {}).get('gpuDisplayName'),
    'errors': errs,
    'raw_keys': list(r.keys()),
}
print(json.dumps(log))
" | tee -a "$OUT"
}

echo "=== PR1 JEPA-T (3 seeds) ==="
for SEED in 43 44 42; do
  deploy_pod "PR1" "JEPA" "$SEED"
done

echo "=== PR2 Universal Ring (3 seeds) ==="
for SEED in 1597 2584 4181; do
  deploy_pod "PR2" "RING" "$SEED"
done

echo "=== PR3 NCA E2E TTT (3 seeds) ==="
for SEED in 43 44 42; do
  deploy_pod "PR3" "NCA" "$SEED"
done

echo ""
echo "=== Summary ==="
wc -l "$OUT"
echo "Log: $OUT"
echo "phi^2 + phi^-2 = 3 · TRINITY · 9 pods deployed"
