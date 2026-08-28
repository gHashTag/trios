#!/bin/bash
# TRIOS Health Check Script
# Validates configuration and reports status

set -e

TRIOS_ROOT="${HOME}/trios"
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🏥 TRIOS Config Health Check"
echo "============================"
echo "Scope: the config tree at $TRIOS_ROOT."
echo "NOT checked here: the Swift app, the build, the server, the git tree."
echo ""

ERRORS=0
WARNINGS=0

# Check SOUL.md
echo -n "📄 SOUL.md ... "
if [ -f "$TRIOS_ROOT/.trios/SOUL.md" ]; then
    LINES=$(wc -l < "$TRIOS_ROOT/.trios/SOUL.md")
    if [ "$LINES" -le 150 ]; then
        echo -e "${GREEN}✓ OK${NC} ($LINES lines)"
    else
        echo -e "${YELLOW}⚠ WARNING${NC} ($LINES lines, max 150)"
        ((WARNINGS++))
    fi
else
    echo -e "${RED}✗ MISSING${NC}"
    ((ERRORS++))
fi

# Check CORE.md
echo -n "🧠 CORE.md ... "
if [ -f "$TRIOS_ROOT/.trios/memory/CORE.md" ]; then
    echo -e "${GREEN}✓ OK${NC}"
else
    echo -e "${RED}✗ MISSING${NC}"
    ((ERRORS++))
fi

# Check daily memory
echo -n "📅 Daily Memory ... "
TODAY=$(date +%Y-%m-%d)
if [ -f "$TRIOS_ROOT/.trios/memory/${TODAY}.md" ]; then
    echo -e "${GREEN}✓ OK${NC} (today's file exists)"
else
    echo -e "${YELLOW}⚠ MISSING${NC} (will create on first use)"
    ((WARNINGS++))
fi

# Check todos.json
echo -n "✅ todos.json ... "
if [ -f "$TRIOS_ROOT/.trinity/queen/todos.json" ]; then
    echo -e "${GREEN}✓ OK${NC}"
else
    echo -e "${YELLOW}⚠ MISSING${NC} (will create on first task)"
    ((WARNINGS++))
fi

# Check skills directory
echo -n "🛠️  Skills ... "
if [ -d "$TRIOS_ROOT/.trios/skills" ]; then
    SKILL_COUNT=$(find "$TRIOS_ROOT/.trios/skills" -name "*.md" 2>/dev/null | wc -l | tr -d ' ')
    echo -e "${GREEN}✓ OK${NC} ($SKILL_COUNT skills)"
else
    echo -e "${RED}✗ MISSING${NC}"
    ((ERRORS++))
fi

# Check permissions
echo -n "🔐 Permissions ... "
if [ -w "$TRIOS_ROOT/.trios" ]; then
    echo -e "${GREEN}✓ OK${NC} (writable)"
else
    echo -e "${RED}✗ NOT WRITABLE${NC}"
    ((ERRORS++))
fi

# Check disk space
echo -n "💾 Disk Space ... "
FREE_SPACE=$(df -k "$TRIOS_ROOT" | tail -1 | awk '{print $4}')
if [ "$FREE_SPACE" -gt 1048576 ]; then  # 1GB
    echo -e "${GREEN}✓ OK${NC} ($(echo "scale=2; $FREE_SPACE/1048576" | bc)GB free)"
else
    echo -e "${YELLOW}⚠ LOW${NC} ($(echo "scale=2; $FREE_SPACE/1048576" | bc)GB free)"
    ((WARNINGS++))
fi

echo ""
echo "===================="
if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    echo -e "${GREEN}✅ Config intact${NC} (says nothing about the app or the build)"
elif [ $ERRORS -eq 0 ]; then
    echo -e "${YELLOW}⚠️  $WARNINGS warning(s); config usable${NC} (says nothing about the app or the build)"
else
    echo -e "${RED}❌ $ERRORS error(s) found!${NC}"
    echo "   Please fix the issues above."
fi

echo ""
echo "Config root: $TRIOS_ROOT"
echo "Run 'cd $TRIOS_ROOT && ./scripts/setup.sh' to repair issues."
echo ""
echo "The trios PROJECT is a different tree: /Users/playra/BrowserOS/trios (8.2G)."
echo "This script has never read it. Its health is reported by 'make doctor',"
echo "which the system crontab runs at :11 and :41 into doctor_reports/cron.log."

exit $ERRORS
