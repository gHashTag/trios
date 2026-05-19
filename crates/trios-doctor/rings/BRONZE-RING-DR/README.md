# RING — BRONZE-RING-DR

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Crate | trios-doctor |
| Package | trios-doctor-bronze |
| Sealed | No |

## Purpose

Binary output ring. The ONLY ring that produces executables.
Orchestrates Silver rings: DR-01 (check) → DR-02 (heal) → DR-03 (report).

## Output

- `trios-doctor` — main CLI binary
- `validate_bpb` — BPB validation binary

## Dependencies

- SILVER-RING-DR-00 (types)
- SILVER-RING-DR-01 (check runner)
- SILVER-RING-DR-02 (heal)
- SILVER-RING-DR-03 (report)

## Laws

- R4: Bronze files stay in Bronze — never move to Silver rings
- No business logic here — only orchestration and CLI argument parsing
- All logic lives in Silver rings
