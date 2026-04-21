#!/bin/env bash
# IGLA NEEDLE HUNT — Launch IN FLIGHT experiments
# FOXTROT (BigramHash) + ALFA (Muon WD sweep)

set -euo pipefail

cd "$(dirname "$0")/../.parameter-golf/parameter-golf"

echo "=== IGLA NEEDLE HUNT — Launching IN FLIGHT experiments ==="
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# FOXTROT: BigramHash(729)
echo "[FOXTROT] IGLA-BGH-301: BigramHash(729) starting..."
BIGRAM_VOCAB_SIZE=729 \
BIGRAM_DIM=96 \
TIED_EMBED_LR=0.1 \
NUM_LAYERS=10 \
WEIGHT_DECAY=0.04 \
SWA_ENABLED=1 \
EVAL_STRIDE=64 \
RUN_ID=igla_bgh301 \
torchrun --standalone --nproc_per_node=8 train_gpt.py 2>&1 | tee logs/igla_bgh301.log

echo ""
echo "[FOXTROT] IGLA-BGH-302: BigramHash(10240)+SmearGate starting..."
BIGRAM_VOCAB_SIZE=10240 \
BIGRAM_DIM=128 \
TIED_EMBED_LR=0.1 \
NUM_LAYERS=10 \
WEIGHT_DECAY=0.04 \
SWA_ENABLED=1 \
EVAL_STRIDE=64 \
RUN_ID=igla_bgh302 \
torchrun --standalone --nproc_per_node=8 train_gpt.py 2>&1 | tee logs/igla_bgh302.log

echo ""
echo "[ALFA] IGLA-MUON-105: Muon WD sweep starting..."
# Sweep over weight decay values
for WD in 0.02 0.03 0.04 0.05 0.06; do
  echo "  [ALFA] Testing WD=$WD..."
  BIGRAM_VOCAB_SIZE=0 \
  TIED_EMBED_LR=0.1 \
  NUM_LAYERS=10 \
  WEIGHT_DECAY=$WD \
  SWA_ENABLED=1 \
  EVAL_STRIDE=64 \
  RUN_ID=igla_muon105_wd${WD} \
  torchrun --standalone --nproc_per_node=8 train_gpt.py 2>&1 | tee logs/igla_muon105_wd${WD}.log
done

echo ""
echo "=== IGLA IN FLIGHT experiments complete ==="
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
