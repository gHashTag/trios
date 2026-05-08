# Legacy Seed Service Archive — 2026-05-09

**Anchor:** `phi^2 + phi^-2 = 3` · DOI 10.5281/zenodo.19227877

## Context

Seed canon for matrix #446 is locked at `{47, 89, 144, 123}` (2 Lucas + 2 Fibonacci). Pre-canon experimental seed runners are scheduled for sunset under L-SEED-CANON-L5 to free Railway compute quota across 5 accounts.

## Inventory at archive time (TRI MCP `seed_list` 2026-05-08T19:23Z)

### Account `acc1` / project `angelic-embrace` (8ab06401-aa28-4af7-9faf-39a1548b7008) — 7 legacy + 3 GF + 1 v2
| Service ID | Name | Created |
|---|---|---|
| `78c220c9-4574-4588-96c3-ab51cd290e12` | trios-train-seed-42-e1-champion-reproduce | 2026-04-28 |
| `49c2809e-c4c8-46fe-83a5-ee4ebe98e05c` | trios-train-seed-42-e4-capacity-push-h1536 | 2026-04-28 |
| `e864e380-e63d-4513-abc9-e0538fcc1da9` | trios-train-seed-42-e5-gf16-storage-test | 2026-04-28 |
| `cb47a5fb-bbcb-4789-9881-e361b11135d4` | trios-train-seed-42-e6-hybrid-001-test | 2026-04-28 |
| `fb85dca3-2868-49d3-8b61-0bfbb0e514a1` | trios-train-seed-42-e7-lr-phi-optimal | 2026-04-28 |
| `4298bb84-12c9-4db7-87cd-e4b9d00f8995` | trios-train-seed-43-e2-quorum-seed43 | 2026-04-28 |
| `f1e2e1b8-ba39-4794-aa96-3cc84dbd9801` | trios-train-seed-44-e3-quorum-seed44 | 2026-04-28 |
| `0bce8293-afe8-47d5-94ae-20374f8d58b4` | igla-gf-seed10001 | 2026-04-28 |
| `c768f520-b67c-40d6-9c43-486a679def56` | igla-gf-seed10004 | 2026-04-28 |
| `c2ed3a9f-e06e-40f3-b5b0-4b445d9fa22a` | igla-gf-seed10008 | 2026-04-28 |
| `982361d5-ad80-4ba5-874a-06795e0cdda0` | trios-train-v2-acc3-s1597 | 2026-04-30 |

### Account `acc2` / project `reasonable-perception` (12c508c7-1196-468d-b06d-d8de8cb77e93)
| Service ID | Name | Created |
|---|---|---|
| `ed44c56a-3bac-4815-bd74-51ee49c95747` | trios-train-v2-acc2-s1597 | 2026-04-30 |

### Account `acc4` / project `believable-connection` (0247abaa-6487-4347-811c-168d7fe53078)
| Service ID | Name | Created |
|---|---|---|
| `4db62ce6-6aa3-475d-b6c9-59756ca01605` | trios-train-v2-acc4-s1597 | 2026-04-30 |

### Account `acc6` / project `robust-radiance` (475a2290-d990-426a-af57-594a934cf6f4)
| Service ID | Name | Created |
|---|---|---|
| `a140a250-8511-4410-93d3-6369af5c9b04` | trios-train-v2-acc6-s1597 | 2026-04-30 |

### Canonical (NOT to be deleted) — IGLA project `e4fe33bb-3b09-4842-9782-7d2dea1abc9b`
| Service ID | Name | Note |
|---|---|---|
| `94a833e9-5950-49fe-b227-d1d3a39d0e85` | trios-train-ONE-v2-acc1-s1597 | KEEP — bridges canon to seed1597 v2 line |
| `71f5aac2-d4d5-4640-8895-90ced5d4ea63` | trios-mr-priority-runner | tier1 |
| `e2c7447c-093b-49b3-abe4-05f994094aba` | trios-mr-tier2a-runner | tier2a |
| `683c4ece-5836-4718-961c-c6a5e0aaaf34` | trios-mr-tier2b-runner | tier2b |
| `b0bd370e-a49d-4ae6-84c4-8170edf92d2d` | trios-mr-tier3-runner | tier3a (CELL_RANGE 0:100) |
| `22d82210-7612-4bd1-9ff8-301c4e6b4938` | trios-mr-tier3b-runner | tier3b (CELL_RANGE 100:201) |
| `066163be-ef96-469b-b4ae-d064f6b20416` | trios-postrun-sidecar | bpb_samples ingest |

## Sunset plan

Total to delete: **14 services** across 4 non-canonical projects.

Trigger: TRI MCP `railway_service_delete` with `confirm: true` and explicit `project=<UUID>` per service.

Pre-sunset validation:
- Confirm `seed_results.jsonl` from each legacy run is checked into the trios repo or Zenodo deposit before delete.
- Final BPB rows (if any) for legacy seeds must already be ingested into `ssot.bpb_samples` (sidecar live since 2026-05-08T18:33Z).

## Rollback

Re-deploy via TRI MCP `railway_service_deploy` with the same image tag and env. SHA pins recoverable from `.trinity/experience/trios_railway_*.trinity` ledger.

---

Anchor: `phi^2 + phi^-2 = 3` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
