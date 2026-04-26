#!/bin/bash
# IGLA RACE Gate-2 Monitor
# Watches E06-E08 experiments for BPB < 2.03

echo "=== IGLA RACE Gate-2 Monitor ==="
echo "Target: BPB < 2.03"
echo "Current best: 2.1697 @ 60K"
echo ""

check_exp() {
    local exp_id=$1
    local log_file="/Users/playra/trios/.trinity/autonomous/2026-04-26/${exp_id}.log"

    if [ -f "$log_file" ]; then
        local best=$(grep -oP 'best=\K[0-9.]+' "$log_file" | tail -1)
        local steps=$(grep -oP 'step=\K[0-9]+' "$log_file" | tail -1)
        local status=$(grep "Training Complete\|ERROR" "$log_file" | tail -1)

        if [ -n "$best" ]; then
            # Check Gate-2
            local gate2_result="🔴 >2.03"
            if (( $(echo "$best < 2.03" | bc -l) )); then
                gate2_result="🟢 <2.03 ✨"
            fi

            echo "$exp_id: BPB=$best @ ${steps} steps | Gate-2: $gate2_result"
            if [ -n "$status" ]; then
                echo "   Status: $status"
            fi
        else
            echo "$exp_id: Starting..."
        fi
    else
        echo "$exp_id: Not found"
    fi
}

while true; do
    clear
    echo "=== IGLA RACE Gate-2 Monitor ==="
    echo "Target: BPB < 2.03"
    echo "Current best: 2.1697 @ 60K"
    echo "Last check: $(date)"
    echo ""

    check_exp "E06"
    check_exp "E07"
    check_exp "E08"

    echo ""
    echo "Active processes:"
    ps aux | grep -E "igla-gate2-E(06|07|08)" | grep -v grep | wc -l
    echo ""

    sleep 60
done
