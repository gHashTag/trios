use crate::{AuditCategory, CategoryScores, Issue, IssueElement, Metadata, PriorityLimit};
use serde_json::{json, Value};
use anyhow::Result;
use tracing::{debug, info};

/// Parse accessibility audit results
pub fn parse_accessibility(raw: &Value) -> Result<Value> {
    let categories = raw.get("categories").and_then(|c| c.get("accessibility")).context("Missing accessibility category")?;

    let score = categories.get("score").and_then(|s| s.as_u64()).map(|s| s as u8).unwrap_or(0);

    let audit_counts = raw.get("accessibility").and_then(|a| {
        Some(a.get("audit-counts"))
    }).context("Missing accessibility audit counts")?;

    let mut issues: Vec<Issue> = Vec::new();

    if let Some(audits) = raw.get("audits").and_then(|a| a.as_object()) {
        for (audit_id, audit_value) in audits.iter().take(15) {
            if let Some(ref_obj) = audit_value.get("details").and_then(|d| d.get("items").and_then(|i| i.as_array()) {
                for item in items {
                    if let Some(node) = item.as_object() {
                        if let Some(issue) = parse_lighthouse_issue(audit_id, node, "accessibility", "a11y") {
                            issues.push(issue);
                        }
                    }
                }
            }
        }
    }

    let limits = PriorityLimit::default();
    limits.limit_issues(&mut issues);

    let category_scores = json!({
        "a11y-navigation": json!({"score": 0, "issues_count": issues.len()}),
        "a11y-aria": json!({"score": 0, "issues_count": count_by_category(&issues, "aria")}),
        "a11y-best-practices": json!({"score": 0, "issues_count": count_by_category(&issues, "best-practices")}),
    });

    let critical_elements: Vec<IssueElement> = issues.iter().filter(|i| i.impact == "critical").map(|i| IssueElement {
        selector: i.elements.get(0).and_then(|e| e.as_str()).unwrap().to_string(),
        snippet: i.elements.get(0).and_then(|s| s.as_str()).unwrap().to_string(),
        label: "a11y".to_string(),
        issue_description: format!("Fix: {}", i.title),
    }).collect();

    let prioritized_recommendations = vec![
        "Fix ARIA attributes and roles",
        "Fix keyboard navigation traps",
        "Improve color contrast ratios",
    ];

    let report = json!({
        "metadata": {
            "url": raw.get("finalUrl").and_then(|u| u.as_str()).unwrap_or(""),
            "timestamp": "2025-01-01T00:00:00Z".to_string(),
            "device": "desktop",
            "lighthouseVersion": "11.6.0".to_string(),
        },
        "report": {
            "score": score,
            "audit_counts": audit_counts,
            "issues": issues,
            "categories": category_scores,
            "critical_elements": critical_elements,
            "prioritized_recommendations": prioritized_recommendations,
        },
    });

    info!("Parsed {} accessibility issues", issues.len());
    Ok(report)
}
