# Trinity Mesh Node — CPU daemon for Railway
# φ² + φ⁻² = 3
# Build context: repo root

FROM rust:1.86-slim AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev python3 && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates/trios-mesh/      crates/trios-mesh/
COPY crates/trios-mesh-node/ crates/trios-mesh-node/

# Stub all other workspace members with UNIQUE names derived from path
RUN python3 - <<'PY'
import re, pathlib
cargo = pathlib.Path('Cargo.toml').read_text()
members = re.findall(r'"(crates/[^"]+|contrib/[^"]+|vendor/[^"]+|tools/[^"]+)"', cargo)
skip = {'crates/trios-mesh', 'crates/trios-mesh-node'}
for m in members:
    if m in skip:
        continue
    p = pathlib.Path(m)
    if p.exists():
        continue
    # unique name = path segments joined with dashes, avoids xtask/xtask collision
    unique_name = m.replace('/', '-').replace('_', '-').lower()
    (p / 'src').mkdir(parents=True, exist_ok=True)
    (p / 'src' / 'lib.rs').write_text('')
    (p / 'Cargo.toml').write_text(
        f'[package]\nname = "{unique_name}"\nversion = "0.1.0"\nedition = "2021"\n'
    )
print('OK: stubs created with unique names')
PY

RUN cargo build --release -p trios-mesh-node

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/mesh-node /usr/local/bin/mesh-node
ENV PORT=8080
EXPOSE 8080
ENTRYPOINT ["mesh-node"]
