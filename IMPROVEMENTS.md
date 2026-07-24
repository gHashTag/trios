# 🚀 TRIOS Improvements Roadmap

**Generated:** 2025-01-XX  
**Status:** Ready for implementation  
**Priority:** 🔴 High | 🟡 Medium | 🟢 Low

---

## 📊 Current State Summary

| Component | Status | Issues |
|-----------|--------|--------|
| Config Location | ✅ Migrated | All in `/Users/playra/trios/` |
| SOUL.md | ✅ OK | 57 lines |
| CORE.md | ✅ OK | Loaded |
| Daily Memory | ⚠️ Missing | Will create on first use |
| todos.json | ✅ OK | Dynamic updates working |
| Skills | ⚠️ Empty | 0 skills in new location |
| Health Check | ✅ Working | Script functional |

---

## 🔴 HIGH PRIORITY (Do First)

### 1. Sync Skills from Old Location
**Problem:** Skills directory empty after migration  
**Impact:** Agent loses specialized capabilities  
**Effort:** 15 min

```bash
# Copy skills from old location
cp -r ~/.claude/skills/* /Users/playra/trios/.trios/skills/

# Verify
ls -la /Users/playra/trios/.trios/skills/
```

**Files to copy:**
- `phi-loop/SKILL.md`
- `tri-pipeline/SKILL.md`
- `wrap-up/SKILL.md`
- `test-driven-development/SKILL.md`
- `systematic-debugging/SKILL.md`
- (21 skills total)

**Test:** After copying, run `./scripts/health-check.sh` — should show "21 skills"

---

### 2. Create Today's Daily Memory
**Problem:** Daily memory file missing  
**Impact:** Session context not saved  
**Effort:** 2 min

```bash
# Create today's file
cat > /Users/playra/trios/.trios/memory/$(date +%Y-%m-%d).md << 'EOF'
# $(date +%Y-%m-%d)

## Session Notes
- Migrated all configs to /Users/playra/trios/
- TaskTrackerPanelView implemented with sidebar
- Health check script created

## Tasks
- [ ] Sync skills
- [ ] Test full workflow
- [ ] Update documentation
EOF
```

**Test:** Agent should auto-append to this file during session

---

### 3. Update SOUL.md Paths
**Problem:** SOUL.md may reference old paths  
**Impact:** Memory read/write may fail  
**Effort:** 10 min

**Check:**
```bash
grep -n "memory\|\.trios\|\.trinity" /Users/playra/trios/.trios/SOUL.md
```

**Expected:** All paths should be relative or point to `/Users/playra/trios/`

---

## 🟡 MEDIUM PRIORITY (Do Second)

### 4. Git Initialization
**Problem:** Configs not version controlled  
**Impact:** No backup, no history  
**Effort:** 5 min

```bash
cd /Users/playra/trios
git init
git add .trios/SOUL.md .trios/memory/CORE.md scripts/ README.md
git commit -m "Initial TRIOS configuration"
git remote add origin <your-repo-url>
git push -u origin main
```

**.gitignore to create:**
```gitignore
.trios/memory/*.md
.trios/run/
.trios/sessions/
.trinity/queen/todos.json
.DS_Store
```

**Note:** Don't commit daily memories (privacy)

---

### 5. Automate Health Check
**Problem:** Manual health check  
**Impact:** Issues may go unnoticed  
**Effort:** 20 min

**Add to launch agent:**
```bash
# Create ~/Library/LaunchAgents/com.trios.health.plist
cat > ~/Library/LaunchAgents/com.trios.health.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.trios.health</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Users/playra/trios/scripts/health-check.sh</string>
    </array>
    <key>StartInterval</key>
    <integer>3600</integer>
    <key>StandardOutPath</key>
    <string>/Users/playra/trios/.trios/run/health.log</string>
    <key>StandardErrorPath</key>
    <string>/Users/playra/trios/.trios/run/health.err</string>
</dict>
</plist>
EOF

launchctl load ~/Library/LaunchAgents/com.trios.health.plist
```

**Result:** Health check runs every hour

---

### 6. Backup Script
**Problem:** No automated backup  
**Impact:** Config loss on disk failure  
**Effort:** 30 min

**Create `/Users/playra/trios/scripts/backup.sh`:**
```bash
#!/bin/bash
# TRIOS Backup Script

BACKUP_DIR="$HOME/trios-backups"
DATE=$(date +%Y-%m-%d_%H-%M-%S)
BACKUP_PATH="$BACKUP_DIR/trios_$DATE"

mkdir -p "$BACKUP_DIR"

# Backup critical configs
cp -r .trios/SOUL.md .trios/memory/CORE.md "$BACKUP_PATH/"

# Optional: backup to cloud
# rsync -av "$BACKUP_PATH/" user@backup-server:/backups/trios/

echo "✅ Backup created: $BACKUP_PATH"
```

**Schedule:** Run daily via cron or launchd

---

## 🟢 LOW PRIORITY (Nice to Have)

### 7. Config UI
**Problem:** Edit configs via terminal  
**Impact:** Less user-friendly  
**Effort:** 2-3 hours

**Features:**
- SwiftUI panel to edit SOUL.md
- Visual memory browser
- Skill manager (enable/disable)
- Health status dashboard

**Location:** Add to Queen app as new panel (петал 20?)

---

### 8. Memory Search CLI
**Problem:** Manual memory search  
**Impact:** Hard to find old notes  
**Effort:** 1 hour

**Create `/Users/playra/trios/scripts/memory-search.sh`:**
```bash
#!/bin/bash
# Search memory by keyword

QUERY="$1"
if [ -z "$QUERY" ]; then
    echo "Usage: memory-search.sh <query>"
    exit 1
fi

grep -r -i "$QUERY" .trios/memory/ --include="*.md"
```

**Usage:** `./scripts/memory-search.sh "trinity"`

---

### 9. Skill Hot-Reload
**Problem:** Need restart to load new skills  
**Impact:** Development slowdown  
**Effort:** 1-2 hours

**Implementation:**
- File watcher on `.trios/skills/`
- Auto-reload on change
- Notify user via notification

---

### 10. Migration Cleanup
**Problem:** Old configs still in ~/.trios  
**Impact:** Confusion, duplicate data  
**Effort:** 10 min (after verification)

```bash
# AFTER verifying everything works:
mv ~/.trios ~/.trios.backup
mv ~/.trinity ~/.trinity.backup

# Test TRIOS for a week, then:
# rm -rf ~/.trios.backup ~/.trinity.backup
```

---

## 📈 Implementation Order

### Phase 1 (Today — 30 min)
1. ✅ Sync skills
2. ✅ Create daily memory
3. ✅ Verify SOUL.md paths
4. ✅ Run health check

### Phase 2 (This Week — 1 hour)
5. Git initialization
6. Backup script
7. Test full workflow

### Phase 3 (Next Week — 2-3 hours)
8. Automate health check
9. Config UI (optional)
10. Memory search CLI

### Phase 4 (Later)
11. Skill hot-reload
12. Migration cleanup
13. Advanced features

---

## 🎯 Quick Wins (5 min each)

- [ ] Run `./scripts/health-check.sh` now
- [ ] Create today's daily memory file
- [ ] Copy skills directory
- [ ] Test memory read/write
- [ ] Verify todos.json updates

---

## 📞 Need Help?

Run these commands to diagnose:

```bash
# Check structure
cd /Users/playra/trios && tree -L 2

# Check permissions
ls -la .trios/

# Test memory
cat .trios/memory/CORE.md

# Test health
./scripts/health-check.sh

# Check logs
cat .trios/run/health.log
```

---

**Status:** Ready for implementation  
**Next Action:** Start with Phase 1 (High Priority items)
