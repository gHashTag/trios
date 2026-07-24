#!/bin/bash
# TRIOS Memory Sync Script
# Syncs daily memory between old and new locations (for transition period)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TRIOS_ROOT="$(dirname "$SCRIPT_DIR")"

OLD_MEMORY_DIR="${HOME}/.trios/memory"
NEW_MEMORY_DIR="${TRIOS_ROOT}/.trios/memory"

echo "🔄 TRIOS Memory Sync"
echo "===================="
echo ""

# Check if old location exists
if [ ! -d "$OLD_MEMORY_DIR" ]; then
    echo "ℹ️  Old memory location not found (~/.trios/memory)"
    echo "   Using new location only: $NEW_MEMORY_DIR"
    exit 0
fi

# Sync from old to new
echo "📥 Syncing from old to new location..."
cp -u "$OLD_MEMORY_DIR"/*.md "$NEW_MEMORY_DIR/" 2>/dev/null || true
SYNCED=$(find "$NEW_MEMORY_DIR" -name "*.md" -newer "$OLD_MEMORY_DIR" 2>/dev/null | wc -l | tr -d ' ')
echo "  ✓ Synced $SYNCED file(s)"

# Sync from new to old (for backward compatibility)
echo "📤 Syncing from new to old location (backward compat)..."
cp -u "$NEW_MEMORY_DIR"/*.md "$OLD_MEMORY_DIR/" 2>/dev/null || true
echo "  ✓ Done"

echo ""
echo "✅ Memory sync complete!"
echo "   Both locations are now in sync."
echo ""
echo "Tip: Run this script daily or add to cron:"
echo "   0 * * * * ${SCRIPT_DIR}/sync-memory.sh"
