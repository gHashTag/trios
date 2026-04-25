# IGLA RACE — ONE SHOT Deployment (2026-04-24)

## Prerequisite
```bash
cd ~/trios
cargo build --release -p trios-igla-race
```

## Each Machine (4 total)

### Step 1: Set environment (unique MACHINE_ID)
```bash
export NEON_URL="postgresql://user:pass@ep-xxx-pooler.ap-southeast-1.aws.neon.tech/neondb?sslmode=require"
export MACHINE_ID="mac-studio-1"  # ← CHANGE PER MACHINE
export WORKERS=4
```

| Machine | MACHINE_ID |
|---------|-------------|
| Mac Studio 1 | `mac-studio-1` |
| Mac Studio 2 | `mac-studio-2` |
| MacBook Pro | `macbook-pro-1` |
| Partner Server | `partner-1` |

### Step 2: Run in tmux (no SSH disconnect)
```bash
tmux new-session -d -s igla_race
tmux send-keys -t igla_race "NEON_URL='$NEON_URL' MACHINE_ID='$MACHINE_ID' WORKERS=4 ./target/release/trios-igla-race start" Enter
```

### Step 3: Monitor
```bash
./target/release/trios-igla-race status
./target/release/trios-igla-race best
```

## Throughput
- 1 machine × 4 workers × ASHA Rung-0(1000 steps) = ~80 trials/hour
- 4 machines × 4 workers = ~320 trials/hour
- 4 machines × 8 workers = ~640 trials/hour

## Target
BPB < 1.50 (IGLA FOUND) at step 27000
