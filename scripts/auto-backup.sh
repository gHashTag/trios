#!/bin/bash
# TRIOS Auto Backup Script
# Creates timestamped backups of critical config files

set -e

TRIOS_ROOT="/Users/playra/trios"
BACKUP_DIR="$TRIOS_ROOT/.backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_NAME="trios_backup_$TIMESTAMP"

echo "🔄 TRIOS Config Auto Backup"
echo "===================="

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Create backup archive
echo "📦 Creating backup: $BACKUP_NAME.tar.gz"
cd "$TRIOS_ROOT"
tar -czf "$BACKUP_DIR/$BACKUP_NAME.tar.gz" \
    .trios/SOUL.md \
    .trios/memory/CORE.md \
    .trios/skills/ \
    .trinity/queen/todos.json \
    README.md \
    TRIOS_CONFIG.md

# Keep only last 10 backups
echo "🧹 Cleaning old backups (keeping last 10)"
cd "$BACKUP_DIR"
ls -t trios_backup_*.tar.gz | tail -n +11 | xargs -r rm

# Show backup info
BACKUP_SIZE=$(du -h "$BACKUP_DIR/$BACKUP_NAME.tar.gz" | cut -f1)
echo "✅ Backup complete: $BACKUP_SIZE"
echo "📍 Location: $BACKUP_DIR/$BACKUP_NAME.tar.gz"
echo ""
echo "Scope: the config tree only. The trios PROJECT at"
echo "/Users/playra/BrowserOS/trios is NOT in this archive and never has been."
echo "Its safety net is git push, not this job."

# Optional: sync to cloud storage (uncomment if needed)
# rsync -av "$BACKUP_DIR/" ~/Dropbox/TRIOS-Backups/
# aws s3 cp "$BACKUP_DIR/$BACKUP_NAME.tar.gz" s3://your-bucket/trios-backups/

echo ""
echo "💡 Tip: Restore with:"
echo "   tar -xzf $BACKUP_DIR/$BACKUP_NAME.tar.gz -C $TRIOS_ROOT"
