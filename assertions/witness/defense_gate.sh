#!/usr/bin/env bash
# LD lane witness — thin call-through to the canonical Rust binary.
# Per R1 (Rust/Zig only), all business logic lives in tools-defense-gate.
# This file exists only to satisfy the ONE SHOT v2.0 spec letter (#265:4321142675).
set -euo pipefail
exec cargo run --quiet -p tools-defense-gate "$@"
