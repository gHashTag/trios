#!/bin/bash
# TRIOS Setup Script
# Initializes the TRIOS hub at /Users/playra/trios/

set -e

TRIOS_ROOT="/Users/playra/trios"
OLD_TRIOS="$HOME/.trios"
OLD_TRINITY="$HOME/.trinity"

echo "🚀 TRIOS Setup"
echo "=============="
echo

# Check if TRIOS_ROOT exists
if [ ! -d "$TRIOS_ROOT" ]; then
    echo "❌ TRIOS root not found at $TRIOS_ROOT"
    echo "Creating directory structure..."
    mkdir -p "$TRIOS_ROOT"/{.trios/{memory,skills,run,sessions},.trinity/queen,apps,crates,scripts,docs}
fi

# Migrate old configs if they exist
if [ -d "$OLD_TRIOS" ]; then
    echo "📦 Found old config at $OLD_TRIOS"
    echo "Migrating to $TRIOS_ROOT/.trios/..."
    
    # Backup old SOUL.md
    if [ -f "$OLD_TRIOS/SOUL.md" ]; then
        cp "$OLD_TRIOS/SOUL.md" "$TRIOS_ROOT/.trios/SOUL.md"
        echo "  ✓ SOUL.md migrated"
    fi
    
    # Migrate memory
    if [ -d "$OLD_TRIOS/memory" ]; then
        cp -r "$OLD_TRIOS/memory/"* "$TRIOS_ROOT/.trios/memory/" 2>/dev/null || true
        echo "  ✓ Memory migrated"
    fi
    
    echo "✅ Old config migrated successfully"
    echo
fi

# Migrate Trinity state if exists
if [ -d "$OLD_TRINITY" ]; then
    echo "📦 Found Trinity state at $OLD_TRINITY"
    echo "Copying to $TRIOS_ROOT/.trinity/..."
    
    # Copy queen todos
    if [ -f "$OLD_TRINITY/queen/todos.json" ]; then
        mkdir -p "$TRIOS_ROOT/.trinity/queen"
        cp "$OLD_TRINITY/queen/todos.json" "$TRIOS_ROOT/.trinity/queen/"
        echo "  ✓ todos.json copied"
    fi
    
    echo "✅ Trinity state copied"
    echo
fi

# Verify structure
echo "🔍 Verifying structure..."
REQUIRED_FILES=(
    ".trios/SOUL.md"
    ".trios/memory/CORE.md"
    ".trinity/queen/todos.json"
    "TRIOS_CONFIG.md"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$TRIOS_ROOT/$file" ]; then
        echo "  ✓ $file"
    else
        echo "  ⚠ $file (missing)"
    fi
done

echo
echo "✅ TRIOS Hub ready at $TRIOS_ROOT"
echo
echo "📝 Next steps:"
echo "  1. Review TRIOS_CONFIG.md for structure"
echo "  2. Update any scripts pointing to ~/.trios/"
echo "  3. Update any scripts pointing to ~/.trinity/"
echo "  4. Optionally remove old configs after verification"
echo
echo "🎯 Single source of truth: $TRIOS_ROOT"
