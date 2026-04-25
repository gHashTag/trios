# TASK-8: IGLA RACE Distributed Runbook

## Build
```bash
cd /Users/playra/trios
cargo build --release -p trios-igla-race -p trios-igla-trainer
./target/release/trios-igla-race --help
./target/release/trios-igla-trainer --help
```

## Environment
```bash
export NEON_URL="postgresql://trios_user:secure_password@neon.aws.com/neondb?sslmode=require"
export MACHINE_ID="mac-studio-1"  # mac-studio-2, linux-server-1
```

## Start (per machine)
```bash
tmux new -d -s igla-race-1 \
  '\
   export NEON_URL="..." && \
   export MACHINE_ID="mac-studio-1" && \
   ./target/release/trios-igla-race start --workers 4'
```

## Status
```bash
./target/release/trios-igla-race status
./target/release/trios-igla-race best
```

## Neon SQL
```sql
-- Per-machine trial counts
SELECT machine_id, COUNT(*) as trial_count, 
       COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed_count,
       AVG(final_bpb) FILTER (WHERE final_bpb IS NOT NULL) as avg_bpb
FROM igla_race_trials GROUP BY machine_id ORDER BY trial_count DESC;

-- Top-10 best final_bpb
SELECT trial_id::text, machine_id, final_bpb, final_step, config::text, started_at
FROM igla_race_trials WHERE final_bpb IS NOT NULL 
ORDER BY final_bpb ASC LIMIT 10;
```

## Stop
```bash
tmux kill-session -t igla-race-1
pkill -f "trios-igla-race"
ps aux | grep trios-igla | grep -v grep
```

## Final Check
```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo build --release --all
```
