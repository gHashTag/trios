# Trinity CPU Mesh Network — Railway Deployment

> φ² + φ⁻² = 3  
> CPU simulation of Trinity GF16 mesh nodes before ASIC tapeout

## Architecture

```
  Railway Project: trinity-mesh
  ┌─────────────────────────────────────────────────────┐
  │                                                     │
  │  ┌──────────────┐   ANNOUNCE   ┌──────────────┐    │
  │  │  node-0      │◄────────────►│  node-1      │    │
  │  │  seed=0      │              │  seed=1      │    │
  │  │  :8080       │              │  :8080       │    │
  │  └──────┬───────┘              └──────┬───────┘    │
  │         │           ANNOUNCE          │            │
  │         └────────────────┬────────────┘            │
  │                          │                         │
  │                 ┌────────▼───────┐                 │
  │                 │  node-2        │                 │
  │                 │  seed=2        │                 │
  │                 │  :8080         │                 │
  │                 └────────────────┘                 │
  └─────────────────────────────────────────────────────┘

  Each node exposes:
    GET  /health       → "ok"
    GET  /info         → {name, dest_hash, routes, tick, power_mw}
    GET  /routes       → [{dest, next_hop, hops, quality}]
    POST /announce     → process RNS ANNOUNCE packet
    POST /next-hop     → lookup next hop for dest_hash
```

## Deploy to Railway

### Option A — Railway CLI (recommended)

```bash
# Install Railway CLI
npm install -g @railway/cli
railway login

# Create project
railway init --name trinity-mesh

# Deploy Node-0 (bootstrap)
railway up \
  --dockerfile crates/trios-mesh-node/Dockerfile \
  --service trinity-node-0 \
  --env MESH_SEED=0 \
  --env MESH_NODE_NAME=trinity-node-0 \
  --env RUST_LOG=info

# Deploy Node-1
railway up \
  --dockerfile crates/trios-mesh-node/Dockerfile \
  --service trinity-node-1 \
  --env MESH_SEED=1 \
  --env MESH_NODE_NAME=trinity-node-1 \
  --env RUST_LOG=info

# Deploy Node-2
railway up \
  --dockerfile crates/trios-mesh-node/Dockerfile \
  --service trinity-node-2 \
  --env MESH_SEED=2 \
  --env MESH_NODE_NAME=trinity-node-2 \
  --env RUST_LOG=info
```

### Option B — Railway Dashboard

1. New Project → "Deploy from GitHub"
2. Select `gHashTag/trios`
3. Root directory: `crates/trios-mesh-node`
4. Set env vars:
   - `MESH_SEED=0` (change per node)
   - `MESH_NODE_NAME=trinity-node-0`
   - `RUST_LOG=info`
5. Repeat for node-1 and node-2

## Test the mesh

```bash
# After deploying, get URLs from Railway dashboard
NODE0=https://trinity-node-0.up.railway.app
NODE1=https://trinity-node-1.up.railway.app
NODE2=https://trinity-node-2.up.railway.app

# Check node info
curl $NODE0/info | jq .

# Get node-0 dest_hash
NODE0_HASH=$(curl -s $NODE0/info | jq -r .dest_hash)
NODE1_HASH=$(curl -s $NODE1/info | jq -r .dest_hash)
NODE2_HASH=$(curl -s $NODE2/info | jq -r .dest_hash)

echo "Node-0: $NODE0_HASH"
echo "Node-1: $NODE1_HASH"
echo "Node-2: $NODE2_HASH"

# Send ANNOUNCE from node-1 to node-0
# (node-1 announces node-2 to node-0 via itself)
curl -X POST $NODE0/announce \
  -H 'Content-Type: application/json' \
  -d "{
    \"dest_hash\": \"$NODE2_HASH\",
    \"sender\": \"$NODE1_HASH\",
    \"hops\": 1,
    \"quality\": 2
  }"

# Now ask node-0 for next-hop to node-2
curl -X POST $NODE0/next-hop \
  -H 'Content-Type: application/json' \
  -d "{\"dest_hash\": \"$NODE2_HASH\"}"
# Expected: {"next_hop": "<node1_hash>", "local": false}

# Full mesh bootstrap (run this script after all 3 nodes are up):
# node-0 announces itself to node-1 and node-2
curl -X POST $NODE1/announce \
  -H 'Content-Type: application/json' \
  -d "{\"dest_hash\": \"$NODE0_HASH\", \"sender\": \"$NODE0_HASH\", \"hops\": 1, \"quality\": 1}"

curl -X POST $NODE2/announce \
  -H 'Content-Type: application/json' \
  -d "{\"dest_hash\": \"$NODE0_HASH\", \"sender\": \"$NODE0_HASH\", \"hops\": 1, \"quality\": 1}"

echo "✅ Trinity CPU Mesh Network is running!"
echo "φ² + φ⁻² = 3"
```

## Power comparison

| Mode | Power | tok/s | Autonomous? |
|------|-------|-------|-------------|
| CPU (Railway) | ~800 mW | unlimited | ❌ grid power |
| ESP32-S3 | ~150 mW | ~30 | ⚠️ solar borderline |
| **ASIC TTIHP27a** | **<25 mW** | **1193** | **✅ fully autonomous** |

## This is Phase A (M35-1 / M35-2)

CPU mesh on Railway validates:
- RNS ANNOUNCE/next-hop protocol correctness
- GF16 routing table behaviour under real network conditions  
- API design before embedding in ASIC firmware

Next: Phase B — cross-compile to `thumbv7em-none-eabihf` for ESP32-S3.
