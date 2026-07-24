# TRIOS Configuration

## 📁 Directory Structure

```
/Users/playra/trios/
├── .trios/                  # Agent configuration
│   ├── SOUL.md             # Agent identity and behavior
│   ├── memory/             # Long-term memory
│   │   ├── CORE.md         # Permanent facts
│   │   └── YYYY-MM-DD.md   # Daily session notes
│   ├── skills/             # Agent skills
│   ├── run/                # Runtime state
│   └── sessions/           # Session history
│
├── .trinity/                # Trinity state
│   ├── queen/
│   │   └── todos.json      # To-Do list
│   ├── experience/         # Learned patterns
│   └── *.json              # State files
│
├── apps/
│   └── queen/              # Queen UI (Swift/macOS)
│       └── QueenUI/
│
├── crates/                  # Rust crates
│   ├── trios-server/
│   ├── trios-bridge/
│   ├── trios-mcp/
│   └── ...
│
├── scripts/                 # Utility scripts
│   ├── health-check.sh
│   ├── wrap-up.sh
│   └── ...
│
└── docs/                    # Documentation
    ├── ARCHITECTURE.md
    ├── SETUP.md
    └── ...
```

## 🔧 Configuration Files

### Agent Identity
- **File:** `.trios/SOUL.md`
- **Purpose:** Agent personality, rules, capabilities
- **Edit:** Direct edit or via memory tools

### Memory
- **Core:** `.trios/memory/CORE.md` — permanent facts
- **Daily:** `.trios/memory/YYYY-MM-DD.md` — session notes
- **Tools:** `memory_read_core`, `memory_update_core`, `memory_write`

### Trinity State
- **To-Do:** `.trinity/queen/todos.json`
- **Experience:** `.trinity/experience/`
- **State:** `.trinity/*.json`

## 🚀 Quick Commands

```bash
# Health check
./scripts/health-check.sh

# Wrap-up session
trinity wrapup

# Start services
pm2 start ecosystem.config.cjs

# View logs
pm2 logs trios-server
```

## 📝 Edit History

- **2026-07-24:** Consolidated all config into `/Users/playra/trios/`
  - Moved `~/.trios/SOUL.md` → `/Users/playra/trios/.trios/SOUL.md`
  - Moved `~/.trios/memory/` → `/Users/playra/trios/.trios/memory/`
  - Copied `.trinity/queen/todos.json`

## 🎯 Single Source of Truth

**This repository (`/Users/playra/trios/`) is now the ONLY location for:**
- Agent configuration (SOUL.md, memory)
- Trinity state (todos, experience)
- Application code (Queen UI, Rust crates)
- Scripts and documentation

**No more scattered configs in `~/.trios/` or `~/.trinity/`**
