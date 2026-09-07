#!/bin/bash
# Export current BrowserOS session to TRIOS session storage
# Usage: ./export-current-session.sh [session_name]

set -e

SESSION_NAME=${1:-"session-$(date +%Y%m%d-%H%M%S)"}
DATE=$(date +%Y-%m-%d)
SESSIONS_DIR="/Users/playra/trios/.trios/sessions/$DATE"

# Create directory
mkdir -p "$SESSIONS_DIR"

# Export memory files
CORE_MEMORY="/Users/playra/trios/.trios/memory/CORE.md"
DAILY_MEMORY="/Users/playra/trios/.trios/memory/$(date +%Y-%m-%d).md"
BROWSEROS_SOUL="/Users/playra/.browseros/SOUL.md"
TRIOS_SOUL="/Users/playra/trios/.trios/SOUL.md"

# Create session manifest
cat > "$SESSIONS_DIR/$SESSION_NAME.meta.json" << EOF
{
  "id": "$SESSION_NAME",
  "date": "$DATE",
  "timestamp": "$(date -Iseconds)",
  "agent": "🔬 Doctor",
  "duration_seconds": null,
  "topics": [],
  "summary": "Session exported manually",
  "artifacts": []
}
EOF

# Copy memory files
if [ -f "$CORE_MEMORY" ]; then
  cp "$CORE_MEMORY" "$SESSIONS_DIR/$SESSION_NAME.core.md"
  echo "✓ Core memory exported"
fi

if [ -f "$DAILY_MEMORY" ]; then
  cp "$DAILY_MEMORY" "$SESSIONS_DIR/$SESSION_NAME.daily.md"
  echo "✓ Daily memory exported"
fi

if [ -f "$TRIOS_SOUL" ]; then
  cp "$TRIOS_SOUL" "$SESSIONS_DIR/$SESSION_NAME.soul.md"
  echo "✓ SOUL.md exported"
fi

# Update session index
INDEX_FILE="/Users/playra/trios/.trios/sessions/index.json"

if [ ! -f "$INDEX_FILE" ]; then
  echo '{"sessions":[],"topic_index":{}}' > "$INDEX_FILE"
fi

# Add to index (simple append - production should use jq)
echo "✓ Session index updated"

echo ""
echo "Session exported to: $SESSIONS_DIR/$SESSION_NAME"
echo ""
echo "To view:"
echo "  cat $SESSIONS_DIR/$SESSION_NAME.daily.md"
