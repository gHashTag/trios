# trios-dwagent

> DWService Agent installer for Railway deployment

A lightweight Rust CLI utility for deploying DWService monitoring agent to Railway containers. DWService provides secure remote access and monitoring capabilities for your infrastructure.

## Features

- **Zero-dependency** single binary deployment
- **Automatic installer download** from DWService official source
- **Railway-ready** Dockerfile with multi-stage build
- **GitHub Actions** workflow for automatic deployments
- **Clippy-clean**: Zero warnings, production-ready code

## Installation

### Local Build

```bash
# Build from trios repository
cd /Users/playra/trios
cargo build -p trios-dwagent --release

# The binary will be at target/release/trios-dwagent
```

### Cross-compile for Linux

```bash
# Add Linux target
rustup target add x86_64-unknown-linux-gnu

# Build for Linux deployment
cargo build -p trios-dwagent --release --target x86_64-unknown-linux-gnu
```

## Usage

```bash
# Download installer and display installation instructions
trios-dwagent install-all

# Download installer only
trios-dwagent download

# Clean up downloaded files
trios-dwagent cleanup

# Display help
trios-dwagent --help
```

## Deployment

### Method 1: Railway Shell (for existing IGLA project)

```bash
# Link to existing project
railway link -p e4fe33bb-3b09-4842-9782-7d2dea1abc9b

# Open shell
railway shell

# Run installer
./trios-dwagent install-all

# Follow instructions to complete installation
sudo /tmp/dwagent.sh
```

### Method 2: Manual Installation (no trios-dwagent)

```bash
railway shell

# Direct DWAgent installation
curl -L https://www.dwservice.net/download/dwagent_x86_64.sh -o /tmp/dwagent.sh
chmod +x /tmp/dwagent.sh
sudo /tmp/dwagent.sh
```

## Configuration

### Railway Config

Railway auto-detects `railway.toml` in the crate root:
- Uses `rust:slim` (latest) for optimal build
- Deploys to project IGLA
- Memory: 256MB, CPU: 0.5 vCPU

## After Deployment

1. Visit [DWService](https://www.dwservice.net)
2. Login to see your connected machines
3. Your Railway container will appear in machine list
4. Use DWService web interface for remote terminal and monitoring

## Development

### Build and Test

```bash
# Debug build
cargo build -p trios-dwagent

# Release build
cargo build -p trios-dwagent --release

# Run tests
cargo test -p trios-dwagent

# Lint (must pass before merge)
cargo clippy -p trios-dwagent -- -D warnings
```

## Links

- [Trios Repository](https://github.com/gHashTag/trios)
- [DWService](https://www.dwservice.net)
- [Railway](https://railway.app)
