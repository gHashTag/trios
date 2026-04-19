# trios-zig-agents

Rust bindings for [zig-agents](https://github.com/gHashTag/zig-agents).

## Features

- **ffi** (default: disabled): Links against zig-agents vendor/ library
- **stub** (default: enabled): Provides stub implementations that return errors

## Usage

```rust
use trios_zig_agents::{version, health_check, deploy_to_fly, FlyRegion};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let version = version()?;
    println!("zig-agents version: {}", version);

    let health = health_check()?;
    println!("Health: {}", health);

    deploy_to_fly(FlyRegion::Singapore, Some("gHashTag"))?;

    Ok(())
}
```

## Features

- `version()` — Get Trinity version string
- `health_check()` — Get MCP server health status
- `deploy_to_fly()` — Deploy MCP server to Fly.io region
- `instance_status()` — Get current instance status
- `restart_instance()` — Restart an instance
- `stop_instance()` — Stop an instance
- Agent registration and management

## Building with FFI

```bash
cargo build --features ffi
```

This requires zig-agents to be available in `vendor/zig-agents/`.
