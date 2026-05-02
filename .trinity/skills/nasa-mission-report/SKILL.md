---
name: nasa-mission-report
description: "Format any verification, deploy, audit, smoke-test, or status report in the NASA mission documentation style — with Document ID, Verification Matrix table, As-Flown Configuration, Anomaly→Corrective Action (ICA) entries, Constitutional Compliance, GO/NO-GO Poll, and Active Artifacts. Load when the user asks for a 'NASA report', 'NASA format', 'NASA mission report', 'отчёт в формате NASA', 'докумен(то)оборот NASA', mission verification report, flight readiness review, R5-honest verification report, or any deploy/audit/test result framed as a mission. Default language: match the user (Russian or English)."
license: MIT
metadata:
  author: ghashtag-agent
  version: '1.0'
  anchor: "phi^2 + phi^-2 = 3"
---

# NASA Mission Report Skill

Format engineering / deploy / audit / verification results as a NASA-style
mission documentation packet. Designed for R5-honest reporting on the
trios / trios-railway / t27 / TRAINER-IGLA-SOT missions, but works for any
deploy, smoke-test, or verification deliverable.

## When to Use This Skill

Trigger on any of these signals:

- The user asks for a "NASA report", "NASA format", "🚀 NASA",
  "отчёт в формате NASA", "докуменооборот NASA", "документооборот NASA",
  "mission report", "verification report", "FRR" (flight readiness review).
- The user asks to verify or audit a deploy, MCP server, Railway service,
  GHCR image, smoke-test, CI/CD pipeline, or any go-live and report back.
- The user pastes a deploy summary and asks "проверь и отчитайся" /
  "verify and report".
- A previous task ended with a deploy / merge / migration and you owe a
  closing verification packet (R5 gate before claiming DONE).

## Output Skeleton (always in this order)

Use Markdown headers `##` for top-level sections, `###` only inside §3 and
§4 if a long anomaly needs sub-fields. Keep table cells short. Quote raw
log lines / digests / SHAs verbatim.

```
# 🚀 NASA MISSION VERIFICATION REPORT

**Document ID:** `<MISSION-ID>-<TYPE>-<NNN>`
**Mission:** <one-line mission name>
**Verification Time:** <ISO-8601 UTC> (T+<delta> after <event>)
**Verification Agent:** <agent name> (<lane, e.g. R5-honest>)
**Anchor:** `phi^2 + phi^-2 = 3`   ← include if the mission is t27/trios/trainer-igla

---

## 1. EXECUTIVE SUMMARY

**MISSION STATUS: <🟢 GREEN | 🟡 AMBER | 🔴 RED> — <one-sentence call>**

<2–4 sentences. State pass/fail count, scope, and any caveat. No marketing.>

---

## 2. VERIFICATION MATRIX (<N> PROBES)

| # | Probe | Method | Expected | Observed | Status |
|---|---|---|---|---|---|
| P-01 | <what was checked> | <tool/curl/SQL/API> | <expected outcome> | <verbatim observation> | ✅ PASS / ❌ FAIL / ⚠️ AMBER |
| P-02 | … | … | … | … | … |

Rules:
- Number every probe `P-NN`.
- "Observed" must be verbatim — actual digest, status code, count, SHA.
- Use ✅ PASS only when a tool call produced the evidence in this session.
  Otherwise use ⚠️ AMBER (couldn't verify) or ❌ FAIL.

---

## 3. AS-FLOWN CONFIGURATION

| Subsystem | Value |
|---|---|
| Public endpoint | `<url>` |
| Service ID | `<uuid>` (`<name>`) |
| Deployment ID | `<uuid>` |
| Project / env | `<name> <uuid>` / `<env name> <uuid>` |
| Container image | `<registry/path:tag>` |
| Image digest | `sha256:<full-digest>` |
| Source SHA | `<7-char>` (link if useful) |
| Stack | <runtime + key libs + R-rules in effect> |
| Latency / metric | <one observed metric, e.g. healthz 100 ms> |

Add or remove rows to match the mission. Always show image digest if a
container is involved.

---

## 4. ANOMALY → CORRECTIVE ACTION

For each incident this session, one row in the table OR one sub-block:

| Field | Value |
|---|---|
| Anomaly ID | `ICA-<issue-#>` |
| Symptom | <observable behaviour> |
| Root cause | <why it happened> |
| Corrective action | <what was changed> |
| Issue / PR | <links> |
| Verification | <which probes confirm the fix> |

If there were no anomalies, write: `No anomalies in this verification window.`

---

## 5. RESPONSE TO PRIOR FINDINGS (optional)

Use only when this report supersedes an earlier AMBER/RED report. One row
per prior finding, with `Reality` and `Resolution` columns. Link the
specific probe number that closes it.

---

## 6. CONSTITUTIONAL COMPLIANCE

| Law | Status | Evidence |
|---|---|---|
| R1 — Rust-only (or stack rule) | ✅/❌ | <pointer> |
| R5 — Honesty lane | ✅/❌ | <pointer> |
| R7 — Triplet on every emit | ✅/❌ | <verbatim triplet> |
| R9 — Destructive confirm | ✅/N/A | <pointer> |
| NO-COMMIT-WITHOUT-ISSUE | ✅/❌ | <PR→issue links> |

Add domain-specific rules as relevant (CANON_DE_ZIGFICATION,
NUMERIC-STANDARD-001, SACRED-PHYSICS-001, etc.).

---

## 7. GO/NO-GO POLL

| Component | Call |
|---|---|
| <subsystem 1> | **GO** / **NO-GO** |
| <subsystem 2> | **GO** / **NO-GO** |
| … | … |

**FINAL CALL: 🟢 GO / 🟡 HOLD / 🔴 NO-GO — <one-sentence rationale>.**

---

## 8. ACTIVE ARTIFACTS

- Public endpoint: [<url>](<url>)
- Repo HEAD: [<owner/repo @ sha>](<commit-link>)
- Issues: [#A](<link>) · [#B](<link>)
- PRs: [#X](<link>) · [#Y](<link>)
- Image: `<registry/path@sha256:…>`
- Audit row(s): <Neon table + code, or L7 path>

— END OF REPORT —
```

## Operating Procedure

1. **Gather evidence first, write later.** Run the actual probes (`curl`,
   `gh api`, Railway GraphQL, Neon SQL) before filling the matrix. NEVER
   fabricate observations. If you cannot verify something, mark it
   `⚠️ AMBER` with reason — never `✅ PASS`.

2. **Document ID convention.** `<MISSION>-<TYPE>-<NNN>` where:
   - `<MISSION>`: stable mnemonic, e.g. `MCP-PUB-DEPLOY`, `IGLA-GATE2`,
     `T27-RING-N`.
   - `<TYPE>`: `RVR` (release verification report), `FRR` (flight
     readiness), `ICA` (incident corrective action), `SVR` (smoke-test
     verification report).
   - `<NNN>`: zero-padded counter inside the mission.

3. **Probe numbering.** Always `P-01`, `P-02`, … Reference probes by ID
   in §4 and §5 to close anomalies / prior findings.

4. **Status colour rules.**
   - 🟢 GREEN — every probe PASS, every constitutional law ✅.
   - 🟡 AMBER — at least one probe AMBER, no FAIL. Mission is operational
     but documentation/audit incomplete.
   - 🔴 RED — any probe FAIL, or any R-rule violated. Block any DONE claim.

5. **Triplet recording.** When the mission has L7 + Neon audit rows
   (TRAINER-IGLA-SOT, trios-railway, t27), copy the triplet **verbatim** in
   §6 R7 evidence cell. Format:
   `BPB=<v> @ step=<N> seed=<S> sha=<7c> jsonl_row=<L> gate_status=<g>`
   for trainer; or
   `RAIL=<verb> @ project=<8c> service=<8c> sha=<7c> ...` for Railway.

6. **Language.** Write in the user's language (Russian or English). Keep
   table headers in English even when prose is Russian — preserves the
   NASA aesthetic.

7. **Length discipline.** Aim for ≤2 screens. Tables compress more than
   prose. Cut narrative; let the matrix speak.

8. **Never overclaim.** R5 honesty gate: no `🟢 GREEN` without a probe
   row producing the evidence in this session, plus merged PR + green CI
   + ledger row when applicable.

## Probe Methods Cheat Sheet

| Goal | Tool |
|---|---|
| HTTP smoke | `curl -sS -o /dev/null -w "HTTP=%{http_code}\n" <url>` |
| MCP `initialize`/`tools/list` | `curl POST` with `Accept: application/json, text/event-stream` |
| GHCR digest | anonymous token + `HEAD /v2/<owner>/<repo>/manifests/<tag>` |
| Railway state | GraphQL `query($id){ service(id:$id){ deployments(first:1){...} } }` with `Project-Access-Token` header |
| GitHub PR/issue | `gh api /repos/<owner>/<repo>/...` (NEVER browser_task) |
| Neon audit | `neon_postgres-find-row-custom-query` (SELECT ... LIMIT 1) |

## Examples

### Minimal example — single-probe smoke

```
# 🚀 NASA MISSION VERIFICATION REPORT
**Document ID:** `MCP-PUB-SMOKE-007`
**Mission:** Public MCP /healthz hourly probe
**Verification Time:** 2026-04-27T13:00:00Z (T+30 m after deploy)
**Verification Agent:** scheduled-cron (R5-honest)
**Anchor:** `phi^2 + phi^-2 = 3`

## 1. EXECUTIVE SUMMARY
**MISSION STATUS: 🟢 GREEN — /healthz returns 200 in 100 ms.**

## 2. VERIFICATION MATRIX (1 PROBE)
| # | Probe | Method | Expected | Observed | Status |
|---|---|---|---|---|---|
| P-01 | GET /healthz | curl | HTTP 200, body `ok` | HTTP=200 TIME=0.100s `ok` | ✅ PASS |

## 7. GO/NO-GO POLL
| Component | Call |
|---|---|
| Public ingress | **GO** |

**FINAL CALL: 🟢 GO — service nominal.**

— END OF REPORT —
```

### Full deploy verification

See the public-MCP go-live verification (Document ID
`MCP-PUB-DEPLOY-RVR-001`) for the canonical 12-probe layout including ICA,
prior-findings rebuttal, and constitutional compliance.

## Common Mistakes

1. **Pasting last-session results without re-verifying.** Always re-run
   probes for the current report. Stale evidence = AMBER at best.
2. **Marking PASS without a tool call this session.** Either run the probe
   or mark AMBER.
3. **Skipping §6 Constitutional Compliance.** This is the R5 gate. No
   compliance table → no DONE.
4. **Hiding anomalies.** Every anomaly observed during the session must
   appear in §4, even if already fixed.
5. **Long prose.** NASA reports are tables + verbatim observations. Cut
   narrative paragraphs.
