# BR-OUTPUT — trios-core Binary Ring

## Purpose
Final artifact ring. Re-exports all CR-* rings as the public `trios-core` crate API.

## API
Re-exports: `cr_00::*`, `cr_01::*`, `cr_02::*`

## Verification
```bash
cargo build -p trios-core
cargo test -p trios-core
```
