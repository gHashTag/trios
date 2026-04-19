# TECH_DEBT.md — TRIOS Workspace Technical Debt

**Last Updated**: 2026-04-19  
**Milestone**: Issue #56 Trinity Agent Bridge complete

## Critical

### TD-002: External process file corruption
- **Severity**: High  
- **Introduced**: Ongoing  
- **Description**: An external concurrent process repeatedly modifies source files and overwrites commits:
  - `Cargo.toml`: Strips `resolver = "2"`, adds `workspace = true` inside `[workspace.dependencies]` (circular), adds/removes crates
  - `trios-core/src/types.rs`: Removes `GbBranch` struct, adds 800+ lines of new types
  - `trios-ternary/`: Rewrites with broken `repr(transparent)` enums, `FibonacciMask` struct conflicts
  - `trios-bridge/`: Entire directory deleted during active development
  - `TECH_DEBT.md` / `BUILD_STATUS.md`: Reverted to outdated versions, losing accurate updates
  - Git history: External process commits overwrite agent commits
- **Resolution**: Identify and stop the external process. Add file integrity checks.  
- **Impact**: Makes stable verification extremely difficult. Every build requires re-fixing Cargo.toml.

## Medium

### TD-001: zig-sacred-geometry local vendor
- **Severity**: Medium  
- **Introduced**: П1 (2026-04-19)  
- **Decision**: A1-relaxed  
- **Description**: `zig-sacred-geometry` upstream repo returned 404. Local vendor created at `crates/trios-sacred/vendor/zig-sacred-geometry/` with hand-written C-ABI exports.  
- **Resolution**: Replace with upstream git submodule when available, or publish as upstream.  
- **Files**: `crates/trios-sacred/vendor/zig-sacred-geometry/*`

### TD-003: trios-hdc phi_quantization module disabled
- **Severity**: Medium  
- **Description**: `crates/trios-hdc/src/phi_quantization.rs` was externally added with compilation errors. Module disabled with `// pub mod phi_quantization;`.  
- **Resolution**: Fix compilation errors or remove file entirely.

### TD-009: trios-training stub
- **Severity**: Medium
- **Description**: `trios-training` was rewritten as a minimal stub after external process added 7 sub-modules that didn't compile. Full implementation deferred to Φ6.
- **Resolution**: Implement full JEPA training loop in Φ6.

### TD-010: trinity-gf16 crate deleted
- **Severity**: Medium
- **Description**: `crates/trinity-gf16/` (IGLA-GF16 static quantization router) was deleted by external process. The router functionality exists in `trios-golden-float/src/router.rs` as a replacement.
- **Resolution**: Verify router.rs covers all trinity-gf16 functionality. Recreate if needed.

### TD-011: trios-bridge agents.rs orphaned file
- **Severity**: Low
- **Description**: `crates/trios-bridge/src/agents.rs` (old `AgentRegistry`) exists but is not referenced by `lib.rs`. The new `router.rs` (`AgentRouter`) replaced it.
- **Resolution**: Delete `agents.rs` or keep as reference.

### TD-012: Chrome Extension not built
- **Severity**: Medium
- **Description**: `extension/` has full Manifest V3 TypeScript source (service-worker, content scripts, React popup) but `npm install` and `npm run build` have not been run. No `dist/` directory.
- **Resolution**: Run `cd extension && npm install && npm run build` to verify TypeScript compilation.

### TD-013: New crates compilation status unknown
- **Severity**: Medium
- **Description**: External process added several new crates to workspace: `trios-llm`, `trios-sdk`, `trios-ca-mask`, `trios-phi-schedule`, `trios-trinity-init`, `trios-data`, `trios-vm`, `trios-vsa`, `trinity-brain`, `trios-tri`. Individual compilation status not verified.
- **Resolution**: Run `cargo check --workspace` to identify any broken crates.

### TD-014: trios-tri bin/tri.rs missing deps
- **Severity**: Medium
- **Description**: `crates/trios-tri/src/bin/tri.rs` uses `clap`, `tokio`, `tracing`, `trios_bridge`, `tokio-tungstenite`, `serde_json` but `trios-tri/Cargo.toml` doesn't declare these dependencies.
- **Resolution**: Add missing deps to `trios-tri/Cargo.toml` or remove the binary.

## Low

### TD-004: Workspace resolver warning
- **Severity**: Low  
- **Description**: Cargo warns about `resolver = "1"` vs `resolver = "2"`. External process removes `resolver = "2"` from Cargo.toml. Fixed repeatedly but keeps getting reverted.
- **Resolution**: Ensure `resolver = "2"` stays in `[workspace]`.

### TD-005: Unused imports in trios-server tools
- **Severity**: Low  
- **Description**: Several unused imports in tools modules flagged by compiler warnings.  
- **Resolution**: Run `cargo fix --bin trios-server -p trios-server --tests`.

### TD-006: trios-server trios-crypto tool module
- **Severity**: Low  
- **Description**: `crates/trios-server/src/tools/trios-crypto.rs` exists but is not declared in `tools/mod.rs`. Dead code.  
- **Resolution**: Add `trios-crypto` as dependency to trios-server if crypto MCP tools are needed, or remove the file.

### TD-007: Ignored FFI tests
- **Severity**: Info  
- **Description**: Tests are `#[ignore]` because they require Zig vendor submodules to be built and linked. They pass when `--features ffi` is enabled and vendors are compiled.  
- **Resolution**: These are by design — they validate FFI integration when Zig libs are available.

### TD-008: trios-claraParameter directory remains
- **Severity**: Info  
- **Description**: The `trinity-claraParameter/` directory still exists on disk (untracked) after removal from workspace.  
- **Resolution**: Clean up untracked directory when convenient.
