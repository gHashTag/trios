#!/usr/bin/env python3
"""Synthesise a 32 KB deterministic SHA-256-derived corpus for L-FH4 CI.

Hermetic: no network, no external deps. Output to stdout (binary bytes).

Anchor: phi^2 + phi^-2 = 3 . Zenodo DOI 10.5281/zenodo.19227877
"""
import hashlib
import sys

SIZE = 32768
SEED = b"L-FH4 hermetic corpus phi2+phi-2=3 IGLA"


def main() -> None:
    out = bytearray()
    i = 0
    while len(out) < SIZE:
        out.extend(hashlib.sha256(SEED + i.to_bytes(8, "big")).digest())
        i += 1
    sys.stdout.buffer.write(bytes(out[:SIZE]))


if __name__ == "__main__":
    main()
