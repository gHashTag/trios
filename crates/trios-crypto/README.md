# trios-crypto

## Status: STUB MODE (TECH_DEBT)

zig-crypto-mining submodule not connected. FFI feature disabled by default.

**Tech debt tracked**: BLOCK_A — crypto vendor pending

## Features

- **stub** (default): Stub implementations return `FfiNotAvailable` error
- **ffi** (disabled): Real Zig bindings — requires `git submodule add` zig-crypto-mining

## Usage

```rust
use trios_crypto::{sha256, Sha256Hash};

// Returns Err(FfiNotAvailable) in stub mode
let hash: Result<Sha256Hash, String> = sha256(b"hello world");
```

## Resolution

To enable real FFI:
1. Add zig-crypto-mining as git submodule at `vendor/zig-crypto-mining`
2. Run `zig build -Doptimize=ReleaseFast` in vendor dir
3. Enable with `--features ffi`
