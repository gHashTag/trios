# Trinity Mesh Node — CPU daemon for Railway
# φ² + φ⁻² = 3
# syntax=docker/dockerfile:1

FROM rust:1.86-slim AS builder

WORKDIR /app

# System deps
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy workspace root manifests
COPY Cargo.toml Cargo.lock ./

# Copy ONLY the two crates we need
COPY crates/trios-mesh/        crates/trios-mesh/
COPY crates/trios-mesh-node/   crates/trios-mesh-node/

# Stub out ALL other workspace members so Cargo doesn't error on missing paths
# (Cargo requires every member path to exist even if we don't build them)
RUN python3 - <<'PY'
import re, os, pathlib

cargo = pathlib.Path('Cargo.toml').read_text()
members = re.findall(r'"(crates/[^"]+|contrib/[^"]+|vendor/[^"]+|tools/[^"]+)"', cargo)
skip = {'crates/trios-mesh', 'crates/trios-mesh-node'}

for m in members:
    if m in skip:
        continue
    p = pathlib.Path(m)
    if p.exists():
        continue
    p.mkdir(parents=True, exist_ok=True)
    (p / 'src').mkdir(exist_ok=True)
    (p / 'src' / 'lib.rs').write_text('')
    name = p.name
    (p / 'Cargo.toml').write_text(f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2021"\n')
print('Stubs created')
PY

# Build only mesh-node
RUN cargo build --release -p trios-mesh-node

# ── Runtime image ────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mesh-node /usr/local/bin/mesh-node

# Railway injects PORT automatically; default to 8080 for local runs
ENV PORT=8080
EXPOSE 8080

ENTRYPOINT ["mesh-node"]
