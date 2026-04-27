# trios MCP Server

MCP (Model Context Protocol) server that wraps the `tri`, `trios-igla`, and `trios-igla-race` CLI tools for AI agent integration.

## Installation

```bash
cd mcp
npm install
npm run build
```

## Building the Rust Binaries

The MCP server requires the following Rust binaries to be built:

```bash
# Build tri CLI
cargo build --release -p trios-cli --bin tri

# Build trios-igla (from trios-trainer-igla subfolder)
cd trios-trainer-igla
cargo build --release --bin trios-igla
cd ..

# Build trios-igla-race
cargo build --release -p trios-igla-race --bin trios-igla-race
```

The binaries will be located in `target/release/`.

## Configuration

The server reads binary paths from environment variables:

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `TRIOS_REPO_ROOT` | `cwd` | Path to trios repository root |
| `TRIOS_TRI_BIN` | `./target/release/tri` | Path to tri binary |
| `TRIOS_IGLA_BIN` | `./target/release/trios-igla` | Path to trios-igla binary |
| `TRIOS_IGLA_RACE_BIN` | `./target/release/trios-igla-race` | Path to trios-igla-race binary |
| `NEON_URL` | (required for race commands) | Neon PostgreSQL connection URL |

## Tools

### tri CLI (5 tools)

| Tool | Description |
|------|-------------|
| `tri_railway_deploy` | Deploy N Railway instances for IGLA training |
| `tri_railway_status` | Show Railway deployment status |
| `tri_train` | Train CPU n-gram model locally |
| `tri_race_init` | Initialize IGLA RACE with Optuna study |
| `tri_race_status` | Show live race leaderboard |

### trios-igla CLI (5 tools)

| Tool | Description | Exit Codes |
|------|-------------|------------|
| `igla_search` | Search ledger with filters | 0=hits, 2=no-match |
| `igla_list` | List last N rows as R7 triplets | 0 |
| `igla_gate` | Gate-2 quorum check | 0=PASS, 2=NOT_YET |
| `igla_check` | Embargo refusal (R9) | 0=clean, 1=embargoed |
| `igla_triplet` | Get R7 triplet by row index | 0 |

### trios-igla-race CLI (3 tools)

| Tool | Description |
|------|-------------|
| `igla_race_start` | Start ASHA worker for hyperparameter optimization |
| `igla_race_status` | Show race status from Neon PostgreSQL |
| `igla_race_best` | Show best trial from Neon PostgreSQL |

## Exit Codes

The server honestly forwards CLI exit codes (R5 - honest passthrough):

- `0` - Success
- `1` - Embargo refused (igla_check) or general error
- `2` - No match (igla_search) or NOT YET (igla_gate)

## R7 Triplet Format

Tools that emit R7 triplets return them verbatim in the response:

```
BPB=<v> @ step=<N> seed=<S> sha=<7c> jsonl_row=<L> gate_status=<g>
```

The triplet lines are extracted and included in the `triplets` array of the response.

## Claude Desktop Configuration

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "trios": {
      "command": "node",
      "args": [
        "/Users/playra/trios/mcp/dist/index.js"
      ],
      "env": {
        "TRIOS_REPO_ROOT": "/Users/playra/trios",
        "NEON_URL": "postgresql://user:pass@ep-xxx.us-east-2.aws.neon.tech/neondb"
      }
    }
  }
}
```

## Running Directly

```bash
# Build first
npm run build

# Run the server
node dist/index.js

# Test with tools/list
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | node dist/index.js
```

## Constitutional Rules

- **R1**: Server is TypeScript on stdio; no Python
- **R5**: Honest passthrough of CLI exit codes (0, 1 embargo, 2 no-match)
- **R7**: R7 triplet is property of wrapped binaries; MCP layer never invents one
- **R9**: igla_check exposes the embargo predicate

## Example Usage

### Search the ledger for Gate-2 candidates

```json
{
  "name": "igla_search",
  "arguments": {
    "bpb_max": 1.85,
    "step_min": 4000
  }
}
```

### Check Gate-2 quorum

```json
{
  "name": "igla_gate",
  "arguments": {
    "target": 1.85
  }
}
```

### Train a model locally

```json
{
  "name": "tri_train",
  "arguments": {
    "seed": 43,
    "steps": 27000,
    "hidden": 384,
    "lr": 0.004
  }
}
```

## Development

```bash
# Watch mode for development
npm run watch

# Type checking
npm run check

# Format code
npm run format
```
