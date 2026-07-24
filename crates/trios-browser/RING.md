# trios-browser — Ring Isolation

Browser control contracts. Ported from the TS backend
`apps/server/src/browser/*` (Wave 2). The live CDP driver stays next to the
Chrome process; these rings are the data + action contract that trios-server
proxies over A2A.

```
trios-browser/
├── src/lib.rs          ← re-export facade only (L-ARCH-001)
└── rings/
    ├── BW-00/          ← data types: PageInfo, WindowInfo, WindowType/State
    └── BW-01/          ← action protocol: BrowserCommand / BrowserResponse
```

Metal: 🥉 Silver (core/domain).

Dep flow:
```
  BW-01 → BW-00
  facade → BW-00, BW-01  (re-export only)
```

Tests: BW-00 ×3, BW-01 ×4 = 7.
