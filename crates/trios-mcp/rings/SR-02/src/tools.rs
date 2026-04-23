//! MCP Tools for browser automation
//!
//! All 14 MCP tools from `mcp-server.ts`

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use tracing::{debug, info, warn};

/// HTTP client for calling SR-00 endpoints
pub struct HttpClient {
    pub host: String,
    pub port: u16,
    pub auth: (String, String),
}

impl HttpClient {
    pub fn new(host: impl Into<String>, port: u16, username: String, password: String) -> Self {
        Self {
            host: host.into(),
            port,
            auth: (username, password),
        }
    }

    /// GET request to SR-00
    async fn get(&self, path: &str) -> Result<Value> {
        let url = format!("http://{}:{}/{}", self.host, self.port, path);
        debug!("GET request to: {}", url);

        let response = Client::new()
            .get(&url)
            .basic_auth(&self.auth.0, &self.auth.1)
            .send()
            .await
            .context("HTTP GET request failed")?;

        let json = response.json().await
            .context("Failed to parse JSON response")?;

        Ok(json)
    }

    /// POST request to SR-00
    async fn post(&self, path: &str, body: Value) -> Result<Value> {
        let url = format!("http://{}:{}/{}", self.host, self.port, path);
        debug!("POST request to: {}", url);

        let response = Client::new()
            .post(&url)
            .basic_auth(&self.auth.0, &self.auth.1)
            .json(&body)
            .send()
            .await
            .context("HTTP POST request failed")?;

        let json = response.json().await
            .context("Failed to parse JSON response")?;

        Ok(json)
    }

    /// Check server health
    pub async fn health_check(&self) -> Result<bool> {
        match self.get("/.identity").await {
            Ok(json) => {
                if let Some(signature) = json.get("signature").and_then(|s| s.as_str()) {
                    Ok(signature == "mcp-browser-connector-24x7")
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }
}

/// List all available tools
pub fn list_tools() -> Value {
    json!(tools)
}

/// Tool: getConsoleLogs - Get browser console logs
pub async fn get_console_logs(http_client: &HttpClient) -> Result<Value> {
    let json = http_client.get("/console-logs").await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json)?,
        }],
    }))
}

/// Tool: getConsoleErrors - Get browser console errors
pub async fn get_console_errors(http_client: &HttpClient) -> Result<Value> {
    let json = http_client.get("/console-errors").await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json)?,
        }],
    }))
}

/// Tool: getNetworkErrors - Get network ERROR logs (status ≥ 400)
pub async fn get_network_errors(http_client: &HttpClient) -> Result<Value> {
    let json = http_client.get("/network-errors").await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json)?,
        }],
    }))
}

/// Tool: getNetworkLogs - Get ALL network logs (success + errors)
pub async fn get_network_logs(http_client: &HttpClient) -> Result<Value> {
    let json = http_client.get("/network-success").await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json)?,
        }],
    }))
}

/// Tool: getSelectedElement - Get currently selected DOM element
pub async fn get_selected_element(http_client: &HttpClient) -> Result<Value> {
    let json = http_client.get("/selected-element").await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json)?,
        }],
    }))
}

/// Tool: wipeLogs - Clear all browser logs from memory
pub async fn wipe_logs(http_client: &HttpClient) -> Result<Value> {
    let body = json!({});
    let json = http_client.post("/wipelogs", body).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": "All logs cleared from memory.",
        }],
    }))
}

/// Tool: takeScreenshot - Take screenshot of current browser tab
pub async fn take_screenshot(http_client: &HttpClient, _arguments: Option<Value>) -> Result<Value> {
    let body = json!({"timeout_ms": 10000});
    let json = http_client.post("/capture-screenshot", body).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json)?,
        }],
    }))
}

/// Tool: runAccessibilityAudit - Run Lighthouse accessibility audit
pub async fn run_accessibility_audit(http_client: &HttpClient, _arguments: Option<Value>) -> Result<Value> {
    let json = http_client.post("/accessibility-audit", json!({})).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json)?,
        }],
    }))
}

/// Tool: runPerformanceAudit - Run Lighthouse performance audit
pub async fn run_performance_audit(http_client: &HttpClient, _arguments: Option<Value>) -> Result<Value> {
    let json = http_client.post("/performance-audit", json!({})).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json)?,
        }],
    }))
}

/// Tool: runSeoAudit - Run Lighthouse SEO audit
pub async fn run_seo_audit(http_client: &HttpClient, _arguments: Option<Value>) -> Result<Value> {
    let json = http_client.post("/seo-audit", json!({})).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json)?,
        }],
    }))
}

/// Tool: runBestPracticesAudit - Run Lighthouse best practices audit
pub async fn run_best_practices_audit(http_client: &HttpClient, _arguments: Option<Value>) -> Result<Value> {
    let json = http_client.post("/best-practices-audit", json!({})).await?;
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json)?,
        }],
    }))
}

/// Tool: runNextJSAudit - Comprehensive Next.js SEO audit (static checklist)
pub async fn run_nextjs_audit(http_client: &HttpClient, _arguments: Option<Value>) -> Result<Value> {
    let checklist = r#"
## Next.js SEO Audit Checklist

### Meta Tags
- [ ] Unique `<title>` tag on each page
- [ ] Accurate `<meta name="description">` tag
- [ ] Open Graph `<meta property="og:*">` tags for social sharing
- [ ] Twitter Card `<meta name="twitter:*">` tags
- [ ] Canonical URL `<link rel="canonical">` tag
- [ ] `<meta name="robots">` tag with correct value

### Structured Data
- [ ] JSON-LD schema for type-safe data
- [ ] Article schema with `@type: "Article"`
- [ ] Product schema with `@type: "Product"`
- [ ] FAQ page with FAQPage schema

### Sitemap and Robots
- [ ] `sitemap.xml` or `sitemap-index.xml` submitted to search engines
- [ ] `robots.txt` allows crawling of important pages
- [ ] `robots.txt` disallows admin/unnecessary pages
- [ ] Sitemap includes all important pages (max 50K URLs)
- [ ] Sitemap follows XML sitemap protocol

### Page Speed
- [ ] Images optimized (WebP, AVIF)
- [ ] Lazy loading for below-fold images
- [ ] Font optimization (variable fonts, subset)
- [ ] Script optimization (minify, defer non-critical)
- [ ] Core Web Vitals in green zone

### Routing
- [ ] Clean URL structure
- [ ] Trailing slashes handled correctly
- [ ] 404 page exists with proper redirect
- [ ] Dynamic routes have meta tags
- [ ] API routes excluded from indexing

### Technical SEO
- [ ] Proper heading hierarchy (`<h1>` → `<h2>` → `<h3>`)
- [ ] Descriptive URLs with keywords
- [ ] HTTPS enabled
- [ ] Mobile-friendly and responsive
- [ ] Proper semantic HTML tags
- [ ] Image `alt` attributes on all images
- [ ] Link text descriptive (not "click here")
"#;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": checklist.to_string(),
        }],
    }))
}

/// Tool: runDebuggerMode - Guide systematic debugging process (8-step workflow)
pub async fn run_debugger_mode(http_client: &HttpClient, _arguments: Option<Value>) -> Result<Value> {
    let guide = r#"
## 🐛 Systematic Debugger Mode - 8-Step Workflow

### Step 1: Check Console Errors
Run: `getConsoleErrors`
Look for: JavaScript errors, uncaught exceptions, failed promises.

### Step 2: Check Network Errors
Run: `getNetworkErrors`
Look for: 404, 500 status codes, failed API calls.

### Step 3: Review Network Logs
Run: `getNetworkLogs`
Look for: Slow requests, unusual endpoints, request patterns.

### Step 4: Check Console Logs
Run: `getConsoleLogs`
Look for: Warnings, info messages, deprecation notices.

### Step 5: Take Screenshot
Run: `takeScreenshot`
Capture: Current page state for visual inspection.

### Step 6: Review Selected Element
Run: `getSelectedElement`
Inspect: DOM structure, styles, event listeners.

### Step 7: Identify Issue Type
Based on findings:
- Console error → JavaScript logic bug
- Network error → API/server issue
- UI problem → CSS/styling bug
- Data issue → State management bug

### Step 8: Form Hypothesis
Write: Clear statement of what's wrong and expected behavior.
Example: "User clicks button but console shows TypeError - likely null reference in click handler."

## 📋 Next Actions
1. Reproduce the issue in browser DevTools
2. Check relevant source files (component, API endpoint)
3. Add breakpoints or console.log statements
4. Test fix and verify resolution
"#;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": guide.to_string(),
        }],
    }))
}

/// Tool: runAuditMode - Automated optimization workflow
pub async fn run_audit_mode(http_client: &HttpClient, _arguments: Option<Value>) -> Result<Value> {
    let workflow = r#"
## 🤖 Automated Audit Mode - Full Optimization Workflow

### Phase 1: Accessibility Audit
Run: `runAccessibilityAudit`
Priority: Critical for user inclusion.
Key metrics: WCAG 2.1 AA compliance, color contrast, keyboard navigation.

### Phase 2: Performance Audit
Run: `runPerformanceAudit`
Priority: Core user experience metrics.
Key metrics: LCP < 2.5s, FID < 1s, CLS < 0.1, TBT < 200ms.

### Phase 3: Best Practices Audit
Run: `runBestPracticesAudit`
Priority: Security and compatibility.
Key metrics: HTTPS, CSP headers, modern API usage, no deprecated APIs.

### Phase 4: SEO Audit
Run: `runSeoAudit`
Priority: Search engine visibility.
Key metrics: Meta tags, structured data, sitemap, robots.txt.

### Phase 5: Review and Prioritize
For each audit:
1. Review all issues (critical first)
2. Categorize by impact and effort
3. Create implementation plan
4. Estimate time-to-fix

### Phase 6: Implement Fixes
Priority order:
1. Critical accessibility issues (blocking users)
2. High-impact performance issues
3. Security vulnerabilities
4. Medium-impact SEO improvements
5. Minor code quality issues

### Phase 7: Verify
After each fix:
1. Re-run specific audit
2. Manual testing with screen readers (a11y)
3. Cross-browser compatibility testing
4. Lighthouse score improvement validation

### Phase 8: Document
For each fix:
1. Update issue tracker with fix details
2. Add comments explaining the change
3. Note any related issues discovered
4. Update documentation if affected

## 📊 Success Criteria
- All critical issues resolved
- Accessibility score > 90
- Performance score > 85
- Best practices score > 90
- SEO score > 85
- Zero security vulnerabilities
- Core Web Vitals passing
"#;

    Ok(json!({
        "content": [{
            "type": "text",
            "text": workflow.to_string(),
        }],
    }))
}

/// Call tool by name
pub async fn call_tool(
    name: &str,
    http_client: &HttpClient,
) -> Result<Value> {
    match name {
        "getConsoleLogs" => get_console_logs(http_client).await,
        "getConsoleErrors" => get_console_errors(http_client).await,
        "getNetworkErrors" => get_network_errors(http_client).await,
        "getNetworkLogs" => get_network_logs(http_client).await,
        "getSelectedElement" => get_selected_element(http_client).await,
        "wipeLogs" => wipe_logs(http_client).await,
        "takeScreenshot" => take_screenshot(http_client, None).await,
        "runAccessibilityAudit" => run_accessibility_audit(http_client, None).await,
        "runPerformanceAudit" => run_performance_audit(http_client, None).await,
        "runSeoAudit" => run_seo_audit(http_client, None).await,
        "runBestPracticesAudit" => run_best_practices_audit(http_client, None).await,
        "runNextJSAudit" => run_nextjs_audit(http_client, None).await,
        "runDebuggerMode" => run_debugger_mode(http_client, None).await,
        "runAuditMode" => run_audit_mode(http_client, None).await,
        _ => {
            anyhow::bail!("Unknown tool: {}", name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_client_health_check() {
        let client = HttpClient::new("127.0.0.1", 3025, "admin", "admin");
        let healthy = client.health_check().await.unwrap();
        assert!(!healthy); // Server not running
    }

    #[test]
    fn test_list_tools() {
        let tools = list_tools();
        let tools_array = tools.get("tools").and_then(|t| t.as_array()).unwrap();
        assert_eq!(tools_array.len(), 14);
    }
}
