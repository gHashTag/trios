# IGLA RACE — Operator Runbook

> TASK-8: Distributed Multi-Machine Rollout
> Last Updated: 2026-04-24T16:10Z

---

## 1) BUILD (ONE-TIME per machine)

Clone and build the IGLA race binaries:

```bash
cd /path/to/trios
cargo build --release -p trios-igla-race -p trios-igla-trainer
```

Verify binaries:
```bash
./target/release/trios-igla-race --help
./target/release/trios-igla-trainer --help
```

Expected output:
- `igla-race`: Commands: start, status, best
- `igla-trainer`: Options: --arch, --hidden, --context, --lr, --steps, --seed, --exp-id

---

## 2) ENV SETUP (PER MACHINE)

Set environment variables for each machine:

```bash
# Neon database connection (replace with your credentials)
export NEON_URL="postgresql://username:password@host/neondb?sslmode=require"

# Unique machine ID per machine
export MACHINE_ID="mac-studio-1"  # Change to: mac-studio-2, macbook-pro-1, partner-1, etc.

# Optional: Debug logging
export RUST_LOG="info"
```

Recommended workers per machine:
- Mac Studio (M1/M2/M3): 8 workers
- MacBook Pro (M1/M2): 4 workers
- Linux workstation: 4-8 workers (depending on CPU)

---

## 3) START RACE UNDER TMUX

On each machine, start the race in a tmux session:

```bash
# Create tmux session
tmux new -s igla-race

# In tmux session:
cd /path/to/trios
export NEON_URL="postgresql://username:password@host/neondb?sslmode=require"
export MACHINE_ID="mac-studio-1"  # Unique per machine
export RUST_LOG="info"

./target/release/trios-igla-race start --workers 4  # or 8 on stronger machines
```

To detach from tmux: `Ctrl+B`, then `D`

To reattach later: `tmux attach -t igla-race`

**Repeat on all machines** with different `MACHINE_ID`.

---

## 4) CHECK STATUS

Check race status from any machine:

```bash
# View full leaderboard
./target/release/trios-igla-race status

# View best trial
./target/release/trios-igla-race best
```

---

## 5) VERIFY NEON

Connect to Neon and verify activity:

```sql
-- Check trials per machine
SELECT machine_id, COUNT(*) as trial_count
FROM igla_race_trials
GROUP BY machine_id
ORDER BY trial_count DESC;

-- Check top performing trials
SELECT machine_id, config::text, final_bpb, final_step
FROM igla_race_trials
WHERE final_bpb IS NOT NULL
ORDER BY final_bpb ASC
LIMIT 10;

-- Check recent activity
SELECT machine_id, status, started_at, final_bpb
FROM igla_race_trials
ORDER BY started_at DESC
LIMIT 20;
```

---

## 6) STOPPING THE RACE

To stop the race on a machine:

```bash
# Stop tmux session
tmux kill-session -t igla-race

# Verify no orphaned trainer processes
ps aux | grep trios-igla-trainer
# If any remain:
pkill -f trios-igla-trainer
```

---

## Troubleshooting

### Trainer hangs
- Check logs in tmux session
- Verify NEON_URL is correct
- Check if trainer timeout is being triggered (30s per 1000 steps)

### BPB parsing errors
- Ensure trainer stdout only contains `BPB=X.XXXX` on last line
- Check stderr logs in tmux for actual errors

### No trials appearing in Neon
- Verify NEON_URL connection string
- Check Neon table exists: `igla_race_trials`
- Ensure `machine_id` is set correctly

---

## Monitoring Commands

```bash
# Watch logs in tmux (detach first)
tmux attach -t igla-race

# Check local experience logs
tail -f .trinity/experience/trios_$(date +%Y%m%d).trinity

# Check for stray trainer processes
watch -n 5 'ps aux | grep trios-igla-trainer | grep -v grep'
```

---

## Success Indicators

- ✅ All machines have trials in Neon
- ✅ `final_bpb` values are decreasing over time
- ✅ Trials are being pruned at rung 1000 (expected ASHA behavior)
- ✅ No orphaned trainer processes
- ✅ Experience logs being written

---

## Next Steps After IGLA Found (BPB < 1.50)

1. Stop all machines
2. Extract winning config from Neon
3. Document the configuration
4. Consider JEPA integration (TASK-5A)
