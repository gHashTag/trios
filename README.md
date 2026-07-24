# TRIOS Hub

**Central configuration and code repository for TRIOS ecosystem**

## 🎯 Single Source of Truth

All TRIOS configuration, state, and code now lives in **one place**:

```
/Users/playra/trios/
```

No more scattered configs in `~/.trios/` or `~/.trinity/`.

## 📁 Structure

```
trios/
├── .trios/                  # Agent configuration
│   ├── SOUL.md             # Identity, rules, capabilities
│   ├── memory/             # Long-term memory
│   │   ├── CORE.md         # Permanent facts
│   │   └── YYYY-MM-DD.md   # Daily notes
│   ├── skills/             # Agent skills
│   ├── run/                # Runtime state
│   └── sessions/           # Session history
│
├── .trinity/                # Trinity state
│   ├── queen/
│   │   └── todos.json      # To-Do list
│   └── experience/         # Learned patterns
│
├── apps/
│   └── queen/              # Queen UI (Swift/macOS)
│
├── crates/                  # Rust crates
├── scripts/                 # Utility scripts
└── docs/                    # Documentation
```

## 🚀 Quick Start

### Setup (first time)

```bash
cd /Users/playra/trios
./scripts/setup.sh
```

This migrates old configs from `~/.trios/` and `~/.trinity/`.

### Health Check

```bash
./scripts/health-check.sh
```

### Wrap-up Session

```bash
trinity wrapup
```

Or manually:

```bash
./scripts/wrap-up.sh
```

## 📝 Configuration Files

### SOUL.md (`./.trios/SOUL.md`)

Agent identity and behavior rules. Edit to change:
- Name and role
- Capabilities
- Behavioral rules
- Project context

### Memory (`.trios/memory/`)

- **CORE.md** — Permanent facts (user info, projects, architecture)
- **YYYY-MM-DD.md** — Daily session notes (auto-expire after 30 days)

### Trinity State (`.trinity/`)

- **queen/todos.json** — To-Do list (edited by Queen daemon)
- **experience/** — Learned patterns from sessions

## 🛠️ Scripts

| Script | Purpose |
|--------|---------|
| `scripts/setup.sh` | Initialize and migrate configs |
| `scripts/health-check.sh` | Check service health |
| `scripts/wrap-up.sh` | Save session to memory |
| `scripts/clean-old-configs.sh` | Remove old `~/.trios/` after migration |

## 🔧 Editing Config

### SOUL.md

```bash
# Edit directly
edit .trios/SOUL.md

# Or via agent
# "Update my SOUL.md to add..."
```

### Memory

Use memory tools:

```bash
# Read core memory
memory_read_core

# Add to core memory
memory_update_core --additions "New fact here"

# Write daily note
memory_write "Session summary..."
```

### To-Do List

```bash
# View
cat .trinity/queen/todos.json

# Edit (Queen daemon auto-updates)
# Or manually via Queen UI in chat
```

## 📚 Documentation

- **TRIOS_CONFIG.md** — Full configuration reference
- **docs/ARCHITECTURE.md** — System architecture
- **docs/SETUP.md** — Detailed setup guide
- **docs/SKILLS.md** — Agent skills reference

## 🎯 Migration Status

- [x] SOUL.md migrated to `/Users/playra/trios/.trios/`
- [x] Memory migrated to `/Users/playra/trios/.trios/memory/`
- [x] Trinity todos copied to `/Users/playra/trios/.trinity/`
- [x] Setup script created
- [ ] Old configs cleaned up (`~/.trios/`, `~/.trinity/`)
- [ ] All scripts updated to use new paths

## 🌟 Benefits

1. **Single location** — No more hunting for configs
2. **Version controlled** — Track changes in git
3. **Easy backup** — One directory to backup
4. **Clear structure** — Logical organization
5. **Easy migration** — Setup script handles it

---

**Location:** `/Users/playra/trios/`  
**Status:** ✅ Active  
**Last Updated:** 2026-07-24
