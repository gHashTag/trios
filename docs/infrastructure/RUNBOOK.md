# IGLA RACE Distributed Runbook

## Machines

| Machine | IP | Role | Status |
|---------|-----|------|--------|
| MacBook Pro | 100.66.38.103 | Primary | Active |
| RunPod GPU | - | Accelerator | Grant pending |

## Machine 1 — MacBook Pro (100.66.38.103)

### Quick Start
```bash
# Navigate to trios repo
cd /path/to/trios

# Export environment
export NEON_URL="postgresql://user:pass@host/neondb?sslmode=require"
export MACHINE_ID="mac-pro-1"

# Start IGLA race
./target/release/trios-igla-race start --workers 4
```

### Endpoints
- HTTP: `http://100.66.38.103:8080` (health check)
- IGLA RACE CLI: `./target/release/trios-igla-race`

### Tailscale Setup
```bash
# Install via App Store (recommended)
# Or via brew:
brew install tailscale
tailscale up

# Funnel setup
tailscale funnel 8080
```

### Health Check
```bash
curl http://100.66.38.103:8080/health
# Expected: {"status":"ok"}

# Check IGLA race status
./target/release/trios-igla-race status
```

### Stop Procedure
```bash
# Stop IGLA race workers
pkill -f "trios-igla-race"

# Stop any trainer processes
pkill -f "trios-igla-trainer"

# Verify cleanup
ps aux | grep trios
```

## Machine 2 — RunPod GPU

### SSH Access
```bash
ssh root@runpod-pod-id
```

### Clone and Build
```bash
# Clone repository
git clone https://github.com/gHashTag/trios.git
cd trios

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Build
cargo build --release --bin igla_train
cargo build --release --bin tri
```

### Launch IGLA Training
```bash
# Set environment
export NEON_URL="postgresql://user:pass@host/neondb?sslmode=require"
export MACHINE_ID="runpod-gpu-1"

# Start training
./target/release/igla_train --steps 12000 --lr 0.004 --hidden 128
```

### Target Metrics
- BPB < 1.5 (IGLA target)
- MSE < 0.5 (for JEPA)
- Training speed: > 100 samples/sec

## CI/CD

### GitHub Actions Workflows

| Workflow | Trigger | Purpose | Status |
|----------|---------|---------|--------|
| build-and-test | push | Build & test | ✅ Active |
| igla-race | manual | Distributed race | ✅ Ready |
| jepa-train | push | JEPA training | 🚧 WIP |
| gf16-training | push | GF16 experiments | ✅ Active |

### Local Pre-Push Checklist (LAWS L-R4/L-R5)
- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace -- -D warnings`
- [ ] `cargo build --release --all`
- [ ] Manual smoke test of key binaries

## Key Files

| File | Purpose | Location |
|------|---------|----------|
| LAWS.md | Law reference | `docs/laws/` |
| NOW.json | Current state | `docs/` |
| tjepa.rs | Temporal JEPA | `crates/trios-train-cpu/src/` |
| gf16.rs | GF16 arithmetic | `crates/trios-train-cpu/src/` |

## Trinity References

| Topic | Document | Status |
|-------|----------|--------|
| JEPA-T | `docs/trinity/JEPA_T.md` | ✅ Complete |
| NCA | `docs/trinity/NCA.md` | ✅ Complete |
| Hybrid (JEPA+NCA) | `docs/trinity/HYBRID.md` | ✅ Complete |
| VSA | `docs/trinity/VSA.md` | ✅ Complete |
| Ternary | `docs/trinity/TERNARY.md` | ✅ Complete |

## Troubleshooting

### Clippy Warnings
```bash
# Fix clippy warnings
cargo clippy --workspace --fix

# Check for remaining issues
cargo clippy --workspace -- -D warnings
```

### Tailscale Funnel Not Working
```bash
# Check tailscale status
tailscale status

# Restart funnel
tailscale funnel reset
tailscale funnel 8080

# Check firewall
sudo ufw status
```

### Port 9005 Already in Use
```bash
# Find process using port 9005
lsof -i :9005

# Kill the process
kill -9 <PID>
```

### Neon Connection Issues
```bash
# Test connection
psql $NEON_URL -c "SELECT 1"

# Check SSL
psql "postgresql://user:pass@host:5432/neondb?sslmode=require"
```
