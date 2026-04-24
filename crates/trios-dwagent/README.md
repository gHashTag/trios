# trios-dwagent

DWService Agent installer for Railway deployment.

## Overview

This utility downloads and installs the DWService monitoring agent on Railway containers. DWService provides remote access and monitoring capabilities.

## Commands

```bash
# Download installer and show instructions
trios-dwagent install-all

# Download installer only
trios-dwagent download

# Clean up downloaded files
trios-dwagent cleanup
```

## Deploy on Railway

### Option 1: Direct Docker Deploy

1. Fork this repository
2. Create new project on Railway
3. Connect GitHub repo
4. Deploy!

### Option 2: CLI Deploy

```bash
railway login
railway init
railway up
```

### Option 3: Manual Binary Upload

```bash
# Build for Linux
cargo build -p trios-dwagent --release --target x86_64-unknown-linux-gnu

# In Railway shell
railway shell
./trios-dwagent install-all
sudo /tmp/dwagent.sh
```

## After Installation

1. Visit https://www.dwservice.net
2. Login to see your connected machines
3. Access the Railway container remotely

## Development

```bash
# Build
cargo build -p trios-dwagent

# Test
cargo test -p trios-dwagent

# Clippy (must pass)
cargo clippy -p trios-dwagent -- -D warnings
```

## License

MIT
