# IGLA RACE — ONE SHOT INSTRUCTION

## Overview
This is a complete ONE SHOT instruction for launching `trios-igla-race` on different machines to hunt for IGLA (BPB < 1.50).

---

## Prerequisites

### Required Tools
- `cargo build --release -p trios-igla-race` — builds the race binary
- Git repository access
- Neon PostgreSQL connection

### Environment Variables
```bash
# Neon connection URL (required)
export NEON_URL="postgresql://neondb_owner:PASSWORD@ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb?sslmode=require"

# Unique machine ID (required for each machine)
export MACHINE_ID="mac-studio-1"  # Examples: mac-studio-2, macbook-pro-1, partner-server-1

# Number of parallel workers (default: 4)
export WORKERS="4"
```

---

## TASK-1: Clone Repository (5 minutes)

```bash
git clone https://github.com/gHashTag/trios.git
cd trios

# Build the race binary
cargo build --release -p trios-igla-race
```

Verify build:
```bash
./target/release/trios-igla-race --help
```

---

## TASK-2: Configure Environment (2 minutes)

Create `.env` file in trios root:

```bash
cat > .env << 'EOF'
NEON_URL="postgresql://neondb_owner:YOUR_PASSWORD@ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb?sslmode=require"
MACHINE_ID="mac-studio-1"
WORKERS="4"
EOF
```

**Important**: Replace `YOUR_PASSWORD` with actual Neon database password and set unique `MACHINE_ID` for each machine.

---

## TASK-3: Launch Race Worker (1 minute)

Start the race in tmux (to prevent disconnection):

```bash
source .env

tmux new-session -d -s igla-race

# Start race
./target/release/trios-igla-race start --workers $WORKERS

# Detach from tmux (Ctrl+B, D)
```

Monitor the race in another terminal:
```bash
tmux attach -t igla-race
```

---

## TASK-4: Monitor Live Leaderboard (ongoing)

View leaderboard:
```bash
./target/release/trios-igla-race status
```

View best trial:
```bash
./target/release/trios-igla-race best
```

---

## CLI Commands Reference

| Command | Description |
|----------|-------------|
| `start --workers N` | Launch N parallel ASHA workers |
| `status` | Show leaderboard from Neon |
| `best` | Show best trial with config |

---

## Troubleshooting

### Binary not found
```bash
cargo build --release -p trios-igla-race
./target/release/trios-igla-race --help
```

### Connection failed
```bash
# Test Neon connection
psql "$NEON_URL" -c "SELECT 1"
```

### Worker stuck
```bash
# Kill tmux session
tmux kill-session -t igla-race

# Restart
./target/release/trios-igla-race start --workers $WORKERS
```

---

## Expected Output

When running successfully:
```
IGLA RACE START | machine=mac-studio-1 | workers=4
Target BPB: 1.50
```

Leaderboard format:
```
IGLA RACE LEADERBOARD
Rank | BPB    | Steps | Machine
-----|--------|-------|--------
  #1  | 2.5329 | 12000 | mac-studio-1
```

---

## Notes

- All output is in **English** as per issue #143 requirements
- Use `tmux` for persistent sessions
- Worker continues indefinitely until IGLA found (BPB < 1.50)
- Each worker registers trials in Neon automatically
- ASHA pruning: trials pruned at rungs 1000, 3000, 9000, 27000 if not in top 33%
