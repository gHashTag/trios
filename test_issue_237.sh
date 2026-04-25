#!/bin/bash
# Test script for Issue #237: CPU N-Gram Training

set -e

echo "=== Test 1: Check binaries exist ==="
test -f target/release/tri || exit 1
test -f target/release/ngram_train || exit 1
echo "✓ Binaries built"

echo ""
echo "=== Test 2: Quick training run (500 steps) ==="
timeout 60 ./target/release/tri train --seeds 42 --steps 500 --hidden 128 --lr 0.004 > test_output.txt 2>&1 || true

if grep -q "bpb=" test_output.txt; then
    echo "✓ Training produces BPB output"
    grep "bpb=" test_output.txt | head -1
else
    echo "✗ No BPB output found"
    exit 1
fi

echo ""
echo "=== Test 3: Check help commands ==="
./target/release/tri train --help > /dev/null 2>&1 || exit 1
echo "✓ tri train --help works"

echo ""
echo "=== Issue #237 Tests: PASSED ==="
