# RING — trios-crypto (Gold Crate)

| Field | Value |
|-------|-------|
| Metal | 🥇 Gold |
| Type | Crate |

## Purpose

Cryptographic identity, signing, and verification primitives.
Foundation for DePIN and BTC mining workflows in TRIOS.

## Ring Structure

```
crates/trios-crypto/
├── src/lib.rs          ← preserved (FFI to zig-crypto-mining)
└── rings/
    ├── CY-00/          ← identity (KeyId, PublicKey, PrivateKey)
    ├── CY-01/          ← signing
    ├── CY-02/          ← verification
    └── BR-OUTPUT/      ← assembly
```

## Dependency Flow

```
BR-OUTPUT
    ↓
  CY-02 → CY-01 → CY-00
```

R9: rings cannot import siblings — only deeper-numbered or BR-OUTPUT can import shallower.

## Laws

- L-ARCH-001 / R1–R5 / R9 / L6
- Anchor: `phi^2 + phi^-2 = 3`
