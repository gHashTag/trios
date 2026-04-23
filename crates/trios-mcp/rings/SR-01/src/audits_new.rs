use crate::{AuditCategory, CategoryScores, Issue, IssueElement, Metadata, PriorityLimit};
use serde_json::{json, Value};
use anyhow::Result;
use tracing::{debug, info};

/// Parse accessibility audit results
pub fn parse_accessibility(raw: &Value) -> Result<Value> {
    let categories = raw
        .get("categories")
        .and_then(|c| c.get("accessibility"))
        .context("Missing accessibility category")?;

    let score = categories
        .get("score")
        .and_then(|s| s.as_u64())
        .map(|s| s as u8)
        .unwrap_or(0);

    let audit_counts = raw
        .get("accessibility")
        .and_then(|a| {
            Some(a.get("audit-counts"))
        })
        .context("Missing accessibility audit counts")?;

    let mut issues: Vec<Issue> = Vec::new();

    if let Some(audits) = raw.get("audits").and_then(|a| a.as_object()) {
        for (audit_id, audit_value) in audits.iter().take(15) {
            if let Some(ref_obj) = audit_value.get("details") {
                if let Some(items) = ref_obj.get("items").and_then(|i| i.as_array()) {
                    for item in items.iter().take(15) {
                        if let Some(node) = item.as_object() {
                            if let Some(issue) = parse_lighthouse_issue(
                                audit_id,
                                node,
                                "accessibility",
                                "a11y",
                            ) {
                                issues.push(issue);
                            }
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

    let report = json!({
        "metadata": Metadata {
            "url": raw.get("finalUrl")
                .and_then(|u| u.as_str())
                .unwrap_or(""),
            "timestamp": "2025-01-01T00:00:00Z",
            "device": "desktop",
            "lighthouseVersion": "11.6.0",
        },
        "report": {
            "score": score,
            "audit_counts": audit_counts,
            "issues": issues,
            "categories": category_scores,
        },
    });

    info!("Parsed {} accessibility issues", issues.len());
    Ok(report)
}

/// Parse issue from Lighthouse audit
fn parse_lighthouse_issue(
    audit_id: &str,
    node: &Value,
    category: &str,
    impact_category: &str,
) -> Option<Issue> {
    let title = node.get("title").and_then(|t| t.as_str());

    let title = match title {
        Some(t) if !t.is_empty() => t.clone(),
        _ => return None,
    };

    let impact = node
        .get("impact")
        .and_then(|i| i.as_str())
        .unwrap_or_else(|_| "moderate");

    let impact_level = match impact.as_str() {
        "critical" | "high" => "critical",
        "serious" => "serious",
        "medium" | "low" => "moderate",
        "low" => "minor",
        _ => "moderate",
    };

    let mut elements: Vec<IssueElement> = Vec::new();

    if let Some(ref_obj) = node.get("details") {
        if let Some(items) = ref_obj.get("items").and_then(|i| i.as_array()) {
            for item in items.iter().take(3) {
                if let Some(selector) = node.get("selector").and_then(|s| s.as_str()) {
                    if let Some(snippet) = node.get("snippet").and_then(|s| s.as_str()) {
                        elements.push(IssueElement {
                            selector: selector,
                            snippet: snippet,
                            label: impact_category,
                        });
                    }
                }
            }
        }
    }

    Some(Issue {
        id: audit_id.to_string(),
        title,
        impact: impact_level,
        category: category.to_string(),
        elements,
        score: node.get("score").and_then(|s| s.as_u64()).unwrap_or(0) as u8,
    })
}

/// Count issues by category
fn count_by_category(issues: &[Issue], category: &str) -> u16 {
    issues.iter()
        .filter(|i| i.category == category)
        .count() as u16
}

/// Convert category to label
fn category_to_label(category: &str) -> &str {
    match category {
        "accessibility" => "a11y",
        "seo" => "seo",
        "best-practices" => "best-practices",
        _ => category,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_accessibility() {
        let json = json!({
            "categories": {
                "accessibility": {
                    "score": 85,
                },
            },
        });

        let result = parse_accessibility(&json);
        assert!(result.is_ok());
    }
}
