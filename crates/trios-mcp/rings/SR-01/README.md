# SR-01: Lighthouse Bridge

## Ring Purpose

Bridge to execute Lighthouse audits via `std::process::Command` calling `node lighthouse`.
Replaces `browser-tools-server/lighthouse/*.ts` files.

## TypeScript References

- `lighthouse/index.ts` — Main orchestration
- `lighthouse/types.ts` — Type definitions
- `lighthouse/accessibility.ts` — Accessibility audit
- `lighthouse/performance.ts` — Performance audit
- `lighthouse/seo.ts` — SEO audit
- `lighthouse/best-practices.ts` — Best practices audit

## Rust Files

- `src/lib.rs` — Main entry point, types
- `src/lighthouse.rs` — std::process::Command → node lighthouse
- `src/audits.rs` — 4 audit parsers (accessibility, performance, seo, best-practices)

## Dependencies (workspace)

- tokio, serde, serde_json, anyhow, tracing

## Audit Types

| Audit | Lighthouse Category | Description |
|-------|------------------|-------------|
| `AccessibilityAudit` | `accessibility` | WCAG compliance, color contrast, keyboard access, ARIA attributes |
| `PerformanceAudit` | `performance` | Core Web Vitals (LCP, FCP, CLS, TBT), optimization opportunities, resource breakdown |
| `SeoAudit` | `seo` | Meta tags, robots.txt, sitemaps, crawlability, link structure |
| `BestPracticesAudit` | `best-practices` | Security, trust, user experience, deprecated APIs, compatibility |

## Lighthouse Execution

```rust
pub async fn run_lighthouse(
    url: &str,
    categories: Vec<AuditCategory>,
) -> anyhow::Result<LighthouseReport> {
    let args = vec![
        "lighthouse",
        url,
        "--output=json",
        "--only-categories",
        &categories.join(","),
        "--quiet",
    ];

    let output = Command::new("node")
        .args(&args)
        .output()?;

    serde_json::from_str(&output.stdout)?
}
```

## Smart Limits

| Impact | Limit | Description |
|--------|--------|-------------|
| Critical | Unlimited | All issues are shown |
| Serious | 15 | Up to 15 items per issue |
| Moderate | 10 | Up to 10 items per issue |
| Minor | 3 | Up to 3 items per issue |

## Response Format (AI-optimized)

```rust
pub struct LighthouseReport {
    pub metadata: Metadata,
    pub report: AuditReport,
}

pub struct AuditReport {
    pub score: u8,
    pub audit_counts: AuditCounts,
    pub issues: Vec<Issue>,
    pub categories: HashMap<String, CategoryScore>,
    pub critical_elements: Vec<CriticalElement>,
    pub prioritized_recommendations: Vec<String>,
}
```

## Ring Status

- [ ] Lighthouse called via `Command::new("node")`
- [ ] JSON report parsed correctly
- [ ] 4 audit types: accessibility, performance, seo, best-practices
- [ ] Smart limits applied (critical: unlimited, serious: 15, moderate: 10, minor: 3)
- [ ] AI-optimized response format
- [ ] `RING.md` present (R3)
- [ ] Separate `Cargo.toml` (R2)
- [ ] Tests pass (R4, ≥10 tests)
