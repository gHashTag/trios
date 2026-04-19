# TECH_DEBT.md — TRIOS Workspace Technical Debt

**Last Updated**: 2026-04-19  
**Milestone**: П1 → П2 transition

## Critical

### TD-001: zig-sacred-geometry local vendor
- **Severity**: Medium  
- **Introduced**: П1 (2026-04-19)  
- **Decision**: A1-relaxed  
- **Description**: `zig-sacred-geometry` upstream repo returned 404. Local vendor created at `crates/trios-sacred/vendor/zig-sacred-geometry/` with hand-written C-ABI exports.  
- **Resolution**: Replace with upstream git submodule when available, or publish as upstream.  
- **Files**: `crates/trios-sacred/vendor/zig-sacred-geometry/*`

### TD-002: External process file corruption
- **Severity**: High  
- **Introduced**: Ongoing  
- **Description**: An external concurrent process repeatedly corrupts source files:
  - `trios-crypto/src/lib.rs`: Duplicate content appended (happened 3+ times)
  - `trios-hdc/build.rs`: Rewritten with broken syntax
  - `trios-hdc/src/phi_quantization.rs`: Externally added with 14 compilation errors
  - `Cargo.toml`: Rewritten with garbage content
  - `trios-server/src/tools.rs`: Created causing module ambiguity
  - `trinity-brain/`: Externally added to workspace
  - `trios-kg/tests/`: Externally added with wrong field names
- **Resolution**: Identify and stop the external process. Add file integrity checks.  
- **Impact**: Makes stable verification extremely difficult.

## Medium

### TD-003: trios-hdc phi_quantization module disabled
- **Severity**: Medium  
- **Description**: `crates/trios-hdc/src/phi_quantization.rs` was externally added with 14 compilation errors. Module disabled with `// pub mod phi_quantization;`.  
- **Resolution**: Fix compilation errors or remove file entirely.

### TD-004: Workspace resolver warning
- **Severity**: Low  
- **Description**: Cargo warns about `resolver = "1"` vs `resolver = "2"`.  
- **Resolution**: Add `resolver = "2"` to `[workspace]` in root `Cargo.toml`.

### TD-005: Unused imports in trios-server tools
- **Severity**: Low  
- **Description**: Several unused imports in `tools/fs.rs`, `tools/git.rs`, `tools/mod.rs` flagged by compiler warnings.  
- **Resolution**: Run `cargo fix --bin trios-server -p trios-server --tests`.

### TD-006: trios-server tools.rs deleted
- **Severity**: Low  
- **Description**: `crates/trios-server/src/tools.rs` was deleted to resolve module ambiguity with `tools/mod.rs`. The `trios-crypto` tool module was also removed (no crate dependency).  
- **Resolution**: Add `trios-crypto` as dependency to trios-server if crypto MCP tools are needed.

## Low

### TD-007: Ignored FFI tests
- **Severity**: Info  
- **Description**: 6 tests are `#[ignore]` because they require Zig vendor submodules to be built and linked. They pass when `--features ffi` is enabled and vendors are compiled.  
- **Resolution**: These are by design — they validate FFI integration when Zig libs are available.

### TD-008: trios-claraParameter directory remains
- **Severity**: Info  
- **Description**: The `crates/trios-claraParameter/` directory still exists on disk (untracked) after removal from workspace. The tracked files were removed via `git rm`.  
- **Resolution**: Clean up untracked directory when convenient.
