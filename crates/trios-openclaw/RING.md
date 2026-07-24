# trios-openclaw — Ring Isolation

OpenClaw gateway contracts. Ported from the TS backend
`lib/agents/openclaw/*` and `lib/agents/hermes/*` (Wave 3).

The heavy VM / container execution (lima-cli, container-cli, managed-container)
drives host processes and stays in a host-runtime layer next to the machine.
These rings port the pure logic most prone to silent breakage.

```
trios-openclaw/
├── src/lib.rs          ← re-export facade only (L-ARCH-001)
└── rings/
    ├── OC-00/          ← gateway: GatewayConfig + resolve_acp_command (argv)
    └── OC-01/          ← hermes: provider mapping (pure lookup)
```

Metal: 🥉 Silver (core/domain).

Dep flow (leaf rings, no cross-imports):
```
  OC-00 → (self-contained)
  OC-01 → (self-contained)
  facade → OC-00, OC-01  (re-export only)
```

Tests: OC-00 ×3, OC-01 ×5 = 8.
