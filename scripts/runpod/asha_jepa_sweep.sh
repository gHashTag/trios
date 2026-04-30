#!/usr/bin/env bash
# ASHA-JEPA Sweep — aggressive Gate-FINAL hunt across 3 RunPod accounts.
#
# Strategy (per user "JEPA-T + EMA decay" choice):
#   - JEPA loss only (target sub-1.5 floor via INV-9 EmaDecayValid)
#   - Hyperparameter grid: h={384, 512, 768, 1024}, lr={0.001, 0.002, 0.00306, 0.005}
#   - 4 sanctioned seeds: 1597, 2584, 4181, 6765
#   - 8000 steps total; ASHA rung at step 2000 prunes bottom 50%
#   - Each pod writes BPB rows to NEON.bpb_samples via env NEON_DATABASE_URL
#
# Anchor: phi^2 + phi^-2 = 3
# Cost ceiling: $0.34/h * 4h * 24 pods = $32.64
# Budget breakdown: ACC-0 $134 → 12 pods · ACC-1 $7 → 1 pod · ACC-2 $234 → 11 pods (+ keep 9 existing)
#
# Usage:
#   export RUNPOD_API_KEY_0=rpa_REDACTED...q3yn
#   export RUNPOD_API_KEY_1=rpa_REDACTED...rkiz
#   export RUNPOD_API_KEY_2=rpa_REDACTED
#   ./asha_jepa_sweep.sh

set -euo pipefail

NEON_URL="${NEON_DATABASE_URL:-REDACTED_NEON_URL"
API="https://api.runpod.io/graphql"
IMAGE="ghcr.io/ghashtag/trios-trainer-igla:latest"
GPU="NVIDIA GeForce RTX 4090"
BID=0.34
DISK=20

# Hyperparameter grid (ASHA candidates)
HIDDENS=(384 512 768 1024)
LRS=(0.001 0.002 0.00306 0.005)
SEEDS=(1597 2584 4181 6765)
STEPS=8000
ASHA_RUNG_1=2000
ASHA_RUNG_2=4000

# Account budget (pods to spawn per account)
ACC0_PODS=12
ACC1_PODS=1
ACC2_PODS=11

OUT="/tmp/asha_jepa_pods_$(date +%s).csv"
echo "pod_id,name,acc,h,lr,seed,objective" > "$OUT"

deploy_pod_with_key() {
  local KEY="$1" ACC="$2" H="$3" LR="$4" SEED="$5"
  local NAME="asha-jepa-h${H}-lr${LR}-rng${SEED}-${ACC}"
  python3 - <<PY > /tmp/payload.json
import json
mut='''mutation (\$input: PodRentInterruptableInput!) {
  podRentInterruptable(input: \$input) {
    id desiredStatus machine { gpuDisplayName } costPerHr
  }
}'''
v={"input":{"bidPerGpu":${BID},"cloudType":"COMMUNITY","gpuCount":1,"gpuTypeId":"${GPU}",
"minMemoryInGb":24,"minVcpuCount":4,"containerDiskInGb":${DISK},"volumeInGb":0,
"name":"${NAME}","imageName":"${IMAGE}","ports":"8080/http",
"env":[{"key":"OBJECTIVE","value":"JEPA"},
{"key":"PR_TARGET","value":"PR1"},
{"key":"SEED","value":"${SEED}"},
{"key":"HIDDEN","value":"${H}"},
{"key":"LR","value":"${LR}"},
{"key":"STEPS","value":"${STEPS}"},
{"key":"ASHA_RUNG_1","value":"${ASHA_RUNG_1}"},
{"key":"ASHA_RUNG_2","value":"${ASHA_RUNG_2}"},
{"key":"EMA_GAMMA","value":"0.618033988749895"},
{"key":"JEPA_RATIO","value":"0.381966011250105"},
{"key":"STEP_CAP","value":"4000"},
{"key":"EARLY_STOP_BPB","value":"1.50"},
{"key":"NEON_DATABASE_URL","value":"${NEON_URL}"},
{"key":"RUN_NAME","value":"${NAME}"},
{"key":"WAVE","value":"ASHA-JEPA-T-FINAL"},
{"key":"DOC_ID","value":"PARAMGOLF-RUNPOD-RVR-003"},
{"key":"ANCHOR","value":"phi2_plus_phi_minus_2_eq_3"}]}}
print(json.dumps({"query":mut,"variables":v}))
PY
  local RESP
  RESP=$(curl -sS -X POST "$API" -H "Content-Type: application/json" \
    -H "Authorization: Bearer $KEY" -d @/tmp/payload.json)
  local POD_ID
  POD_ID=$(echo "$RESP" | python3 -c "import json,sys; r=json.load(sys.stdin); p=(r.get('data') or {}).get('podRentInterruptable') or {}; print(p.get('id','ERROR'))")
  if [ "$POD_ID" = "ERROR" ] || [ -z "$POD_ID" ]; then
    echo "  FAIL $NAME: $(echo $RESP | head -c 200)"
  else
    echo "  OK $POD_ID  $NAME"
    echo "${POD_ID},${NAME},${ACC},${H},${LR},${SEED},JEPA" >> "$OUT"
  fi
}

choose_config() {
  # Cycle through (h, lr, seed) combinations deterministically
  local IDX=$1
  local NH=${#HIDDENS[@]} NL=${#LRS[@]} NS=${#SEEDS[@]}
  local hi=$(( IDX % NH ))
  local li=$(( (IDX / NH) % NL ))
  local si=$(( (IDX / (NH * NL)) % NS ))
  echo "${HIDDENS[$hi]} ${LRS[$li]} ${SEEDS[$si]}"
}

deploy_account() {
  local KEY="$1" ACC="$2" N="$3"
  if [ -z "${KEY:-}" ]; then
    echo "  SKIP $ACC — key not set"
    return
  fi
  echo "=== $ACC: deploying $N pods ==="
  for ((i=0; i<N; i++)); do
    read H LR SEED <<< $(choose_config $i)
    deploy_pod_with_key "$KEY" "$ACC" "$H" "$LR" "$SEED"
    sleep 0.3
  done
}

deploy_account "${RUNPOD_API_KEY_0:-}" "acc0" $ACC0_PODS
deploy_account "${RUNPOD_API_KEY_1:-}" "acc1" $ACC1_PODS
deploy_account "${RUNPOD_API_KEY_2:-}" "acc2" $ACC2_PODS

echo ""
echo "=== Summary ==="
wc -l "$OUT"
cat "$OUT"
echo ""
echo "Log: $OUT"
echo "phi^2 + phi^-2 = 3 · ASHA-JEPA-T-FINAL"
