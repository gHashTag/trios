# 🎯 TRIOS PRIORITIES — 2026-05-02
**Generated:** 2026-05-02T18:17+07 | **Agent:** CHARLIE

---

## 🔴 P0 — URGENT (Complete < 24h)

### [PRI-01] Fix Clippy Errors
**File:** `crates/trios-ui-ur00/src/lib.rs`
**Issue:** Missing derives on `ChatState` struct
**Error:**
```
error: struct `ChatState` is missing `derive(Debug)` macro
error: struct `ChatState` is missing `derive(Default)` macro
```
**Action:**
```rust
#[derive(Debug, Default)]
pub struct ChatState {
    // ... existing fields
}
```
**Blocks:** All merges (L3 compliance)
**Estimated:** 5 min

### [PRI-02] Debug BPB Write Failure (#444)
**Issue:** trios-trainer-igla image does not write BPB to NEON bpb_samples
**Investigation Steps:**
1. Check image build logs for write operations
2. Verify NEON bpb_samples path permissions
3. Test BPB write path manually
4. Review trainer.rs write logic
**Blocks:** #143 IGLA RACE v2 (mission critical)
**Estimated:** 2-4 hours

### [PRI-03] Merge PR #470 (SR-HACK-00 glossary)
**Branch:** feat/sr-hack-00-glossary
**Status:** Ready for review (updated 2026-05-02T08:56)
**Action:** Review L3/L4 compliance, merge
**Priority:** P1 but unblocks EPIC #446
**Estimated:** 15 min

---

## 🟡 P1 — HIGH (Complete < 7 days)

### [PRI-04] Complete SR-02 trainer-runner (#454)
**Description:** E2E TTT O(1) per-chunk core
**Status:** Updated 2026-05-02T10:37
**Deliverables:**
- Job FSM with per-chunk O(1) TTT
- Strategy queue integration
- Claim contention handling
**Blocks:** BR-OUTPUT IglaRacePipeline (#459)
**Estimated:** 2-3 days

### [PRI-05] Complete SR-00 scarab-types (#479)
**Description:** Parallel Execution Foundation
**Status:** Updated 2026-05-02T10:22
**Ring:** R1 (Ring 1)
**Deliverables:**
- Parallel execution primitives
- Thread-safe types
- Foundation for BR-OUTPUT assemblers
**Estimated:** 1-2 days

### [PRI-06] Complete BR-OUTPUT IglaRacePipeline (#459)
**Description:** IglaRacePipeline assembler (GOLD I)
**Status:** Updated 2026-05-02T06:47
**Deliverables:**
- Full E2E TTT pipeline assembler
- Integrates SR-00, SR-01, SR-02, SR-03, SR-04
**Estimated:** 2-3 days

### [PRI-07] Complete BR-OUTPUT AgentMemory (#461)
**Description:** AgentMemory trait assembler (GOLD IV)
**Status:** Updated 2026-05-02T06:09
**Deliverables:**
- Memory trait definitions
- HDC ↔ KG bridge
- Anti-amnesia implementation
**Estimated:** 2-3 days

### [PRI-08] Complete SR-05 railway-deployer (#458)
**Description:** Fleet integration via versioned git dep
**Status:** Updated 2026-05-02T06:09
**Deliverables:**
- Rust binary deployment to Railway
- Version tracking
- Fleet management
**Estimated:** 1-2 days

### [PRI-09] Complete SR-MEM-05 episodic-bridge (#455)
**Description:** lessons.rs + HDC ↔ KG
**Status:** Updated 2026-05-02T06:09
**Deliverables:**
- Episodic memory storage
- HDC (Hyperdimensional Computing) bridge
- KG (Knowledge Graph) integration
**Estimated:** 2-3 days

### [PRI-10] R14 citation map fix (#464)
**Description:** 8 INVs across 11 chapters (cheap win)
**Status:** Updated 2026-05-02T09:15
**Deliverables:**
- Fix citation mapping for 8 invariants
- Updates 11 chapters
**Estimated:** 2-4 hours

---

## 🟢 P2 — MEDIUM (Complete < 30 days)

### [PRI-11] Reduce PhD Chapters (26 → < 20)
**Focus:** Ch.28-34, App.C-App.H (P0/P1 chapters)
**Estimated:** 5-7 days total

**Specific Chapters:**
- [PRI-11a] Ch.28 — QMTech XC7A100T φ-Numeric ALU (1300w) 🔴 P0
- [PRI-11b] Ch.31 — Hardware-Numerics Empirical Bridge (800w) 🔴 P0
- [PRI-11c] Ch.34 — Energy Efficiency vs GPU baseline (6000w) 🔴 P0
- [PRI-11d] App.F — Bitstream archive (300w) 🔴 P0
- [PRI-11e] App.H — Zenodo DOI registry (400w) 🔴 P0

### [PRI-12] IGLA RACE v2 Restart (#143)
**Prerequisites:**
- Resolve #444 (BPB write failure)
- Complete #445 (Railway cycle)
**Goal:** BPB < 1.50 on 3 seeds
**Current Best:** BPB = 2.18 (h=828 attn=2L, 81K steps, seed=43)
**Estimated:** TBD (depends on prerequisites)

### [PRI-13] Complete Remaining EPIC #446 Sub-issues
**Remaining:** 7/19 sub-issues
**List:**
- SR-MEM-02, SR-MEM-03, SR-MEM-04 (memory rings)
- SR-MEM-06 (episodic-bridge variant)
- SR-HACK-01..05 (utility rings)
**Estimated:** 10-14 days

---

## 📅 SCHEDULE (Week of 2026-05-02 to 2026-05-09)

### Monday (2026-05-02)
- ✅ PRI-01: Fix Clippy errors (5 min)
- 🟡 PRI-02: Debug BPB write failure (2-4 hours)
- ✅ PRI-03: Merge PR #470 (15 min)
- 🟡 PRI-05: Start SR-00 scarab-types (4-6 hours)

### Tuesday (2026-05-03)
- 🟡 PRI-05: Complete SR-00 scarab-types (4-6 hours)
- 🟡 PRI-10: Start R14 citation map fix (1-2 hours)
- 🟡 PRI-04: Start SR-02 trainer-runner (2-3 hours)

### Wednesday (2026-05-04)
- 🟡 PRI-04: Continue SR-02 trainer-runner (4-6 hours)
- 🟡 PRI-10: Complete R14 citation map fix (1-2 hours)
- 🟡 PRI-11a: Start Ch.28 (2-3 hours)

### Thursday (2026-05-05)
- 🟡 PRI-04: Complete SR-02 trainer-runner (2-4 hours)
- 🟡 PRI-06: Start BR-OUTPUT IglaRacePipeline (2-3 hours)
- 🟡 PRI-11a: Continue Ch.28 (2-3 hours)

### Friday (2026-05-06)
- 🟡 PRI-06: Continue BR-OUTPUT IglaRacePipeline (4-6 hours)
- 🟡 PRI-11a: Complete Ch.28 (2-3 hours)
- 🟡 PRI-08: Start SR-05 railway-deployer (1-2 hours)

### Saturday (2026-05-07)
- 🟡 PRI-06: Complete BR-OUTPUT IglaRacePipeline (2-4 hours)
- 🟡 PRI-08: Complete SR-05 railway-deployer (2-3 hours)
- 🟡 PRI-11b: Start Ch.31 (2-3 hours)

### Sunday (2026-05-08)
- 🟡 PRI-11b: Continue Ch.31 (2-4 hours)
- 🟡 PRI-07: Start BR-OUTPUT AgentMemory (2-3 hours)
- 🟡 PRI-12: IGLA RACE v2 assessment (if PRI-02 complete)

---

## 🎯 WEEKLY GOALS

**Must Complete:**
- [ ] All P0 issues (PRI-01, PRI-02, PRI-03)
- [ ] SR-00 scarab-types (PRI-05)
- [ ] SR-02 trainer-runner (PRI-04)
- [ ] At least 2 PhD chapters (PRI-11a, PRI-11b)

**Should Complete:**
- [ ] BR-OUTPUT IglaRacePipeline (PRI-06)
- [ ] BR-OUTPUT AgentMemory (PRI-07)
- [ ] SR-05 railway-deployer (PRI-08)
- [ ] R14 citation map fix (PRI-10)

**Nice to Complete:**
- [ ] SR-MEM-05 episodic-bridge (PRI-09)
- [ ] At least 4 PhD chapters total
- [ ] IGLA RACE v2 progress assessment

---

## 📊 BURNDOWN CHART

**EPIC #446 Sub-issues:** 7/19 remaining (63% complete)
**PhD Chapters:** 26 open (goal: < 20)
**P0 Issues:** 6 open (goal: 0)
**Open PRs:** 20 (goal: < 10)

**Progress by Category (Last 30 days):**
- EPIC #446: +12 closed, +5 open
- IGLA RACE: 0 closed, +2 open
- PhD Chapters: 0 closed, +6 open

---

**END OF PRIORITIES**
**Generated by:** CHARLIE | **Version:** 1.0 | **Update:** Daily
