//! HC-01 — executes SR-03 `BrowserCommand`s against a CDP endpoint.
//!
//! This is the Rust replacement for the host-side command runner: every one
//! of the 12 `browser_*` host tools is mapped onto minimal CDP calls
//! (`Page.navigate`, `Runtime.evaluate`, `Page.captureScreenshot`,
//! `Target.createTarget`/`closeTarget`). DOM interactions go through
//! `Runtime.evaluate` so no extra CDP domains have to be enabled.

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use trios_a2a::{BrowserCommand, BrowserCommandType};

/// Abstract CDP transport — implemented by HC-00's `CdpClient` and by test
/// fakes.
#[async_trait::async_trait]
pub trait CdpCall: Send + Sync {
    async fn call(&self, method: &str, params: Value) -> Result<Value>;
}

#[async_trait::async_trait]
impl CdpCall for trios_host_cdp_hc00::CdpClient {
    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        trios_host_cdp_hc00::CdpClient::call(self, method, params).await
    }
}

/// Evaluate a JS expression, return its JSON value (`returnByValue`).
async fn eval(cdp: &dyn CdpCall, expression: &str) -> Result<Value> {
    let result = cdp
        .call(
            "Runtime.evaluate",
            json!({
                "expression": expression,
                "returnByValue": true,
                "awaitPromise": true,
            }),
        )
        .await?;
    if let Some(exception) = result.get("exceptionDetails") {
        let text = exception["exception"]["description"]
            .as_str()
            .or_else(|| exception["text"].as_str())
            .unwrap_or("JS exception");
        bail!("evaluate failed: {text}");
    }
    Ok(result["result"]["value"].clone())
}

/// JSON-escape a CSS selector for safe embedding into a JS expression.
fn js_str(raw: &str) -> String {
    serde_json::to_string(raw).unwrap_or_else(|_| "\"\"".into())
}

fn required_str<'a>(params: &'a Value, key: &str) -> Result<&'a str> {
    params[key]
        .as_str()
        .ok_or_else(|| anyhow!("missing required param `{key}`"))
}

/// Execute one SR-03 command; the returned `Value` becomes
/// `BrowserResult.data`.
pub async fn execute_command(cdp: &dyn CdpCall, command: &BrowserCommand) -> Result<Value> {
    let p = &command.params;
    match command.command_type {
        BrowserCommandType::GetUrl => {
            Ok(json!({"url": eval(cdp, "location.href").await?}))
        }
        BrowserCommandType::GetTitle => {
            Ok(json!({"title": eval(cdp, "document.title").await?}))
        }
        BrowserCommandType::Navigate => {
            let url = required_str(p, "url")?;
            let nav = cdp.call("Page.navigate", json!({"url": url})).await?;
            if let Some(err) = nav["errorText"].as_str() {
                bail!("navigate failed: {err}");
            }
            Ok(json!({"ok": true, "url": url, "frame_id": nav["frameId"]}))
        }
        BrowserCommandType::GetDom => {
            let expr = match p["selector"].as_str() {
                Some(sel) => format!(
                    "(() => {{ const el = document.querySelector({}); return el ? el.outerHTML : null; }})()",
                    js_str(sel)
                ),
                None => "document.documentElement.outerHTML".to_string(),
            };
            Ok(json!({"dom": eval(cdp, &expr).await?}))
        }
        BrowserCommandType::QuerySelector => {
            let sel = required_str(p, "selector")?;
            let expr = format!(
                "(() => {{ const els = [...document.querySelectorAll({})]; \
                 return {{ count: els.length, \
                 first: els[0] ? els[0].outerHTML.slice(0, 4096) : null }}; }})()",
                js_str(sel)
            );
            Ok(eval(cdp, &expr).await?)
        }
        BrowserCommandType::Click => {
            let sel = required_str(p, "selector")?;
            let expr = format!(
                "(() => {{ const el = document.querySelector({}); \
                 if (!el) return {{ clicked: false, error: 'not found' }}; \
                 el.click(); return {{ clicked: true }}; }})()",
                js_str(sel)
            );
            Ok(eval(cdp, &expr).await?)
        }
        BrowserCommandType::Type => {
            let sel = required_str(p, "selector")?;
            let text = required_str(p, "text")?;
            let expr = format!(
                "(() => {{ const el = document.querySelector({sel}); \
                 if (!el) return {{ typed: false, error: 'not found' }}; \
                 el.focus(); el.value = {text}; \
                 el.dispatchEvent(new Event('input', {{ bubbles: true }})); \
                 el.dispatchEvent(new Event('change', {{ bubbles: true }})); \
                 return {{ typed: true }}; }})()",
                sel = js_str(sel),
                text = js_str(text),
            );
            Ok(eval(cdp, &expr).await?)
        }
        BrowserCommandType::Scroll => {
            let dx = p["x"].as_f64().unwrap_or(0.0);
            let dy = p["y"].as_f64().unwrap_or(600.0);
            let expr = format!(
                "(() => {{ window.scrollBy({dx}, {dy}); \
                 return {{ scrolled: true, x: window.scrollX, y: window.scrollY }}; }})()"
            );
            Ok(eval(cdp, &expr).await?)
        }
        BrowserCommandType::Eval => {
            let expr = p["expression"]
                .as_str()
                .or_else(|| p["code"].as_str())
                .ok_or_else(|| anyhow!("missing required param `expression`"))?;
            Ok(json!({"value": eval(cdp, expr).await?}))
        }
        BrowserCommandType::Screenshot => {
            let format = p["format"].as_str().unwrap_or("png");
            let shot = cdp
                .call("Page.captureScreenshot", json!({"format": format}))
                .await?;
            let data = shot["data"]
                .as_str()
                .ok_or_else(|| anyhow!("captureScreenshot returned no data"))?;
            Ok(json!({"data": data, "format": format}))
        }
        BrowserCommandType::OpenTab => {
            let url = p["url"].as_str().unwrap_or("about:blank");
            let target = cdp
                .call("Target.createTarget", json!({"url": url}))
                .await?;
            Ok(json!({"target_id": target["targetId"], "url": url}))
        }
        BrowserCommandType::CloseTab => {
            let target_id = required_str(p, "target_id")
                .or_else(|_| required_str(p, "targetId"))?;
            cdp.call("Target.closeTarget", json!({"targetId": target_id}))
                .await?;
            Ok(json!({"closed": true, "target_id": target_id}))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Records calls; returns canned CDP responses.
    struct FakeCdp {
        calls: Mutex<Vec<(String, Value)>>,
        eval_value: Value,
    }

    impl FakeCdp {
        fn new(eval_value: Value) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                eval_value,
            }
        }
        fn calls(&self) -> Vec<(String, Value)> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl CdpCall for FakeCdp {
        async fn call(&self, method: &str, params: Value) -> Result<Value> {
            self.calls
                .lock()
                .unwrap()
                .push((method.to_string(), params));
            Ok(match method {
                "Runtime.evaluate" => json!({"result": {"value": self.eval_value}}),
                "Page.navigate" => json!({"frameId": "F7"}),
                "Page.captureScreenshot" => json!({"data": "aGVsbG8="}),
                "Target.createTarget" => json!({"targetId": "T1"}),
                _ => json!({}),
            })
        }
    }

    fn cmd(tool: &str, params: Value) -> BrowserCommand {
        BrowserCommand::from_tool_name(tool, "host-test", params).unwrap()
    }

    #[tokio::test]
    async fn navigate_maps_to_page_navigate() {
        let fake = FakeCdp::new(json!(null));
        let out = execute_command(&fake, &cmd("browser_navigate", json!({"url": "https://x"})))
            .await
            .unwrap();
        assert_eq!(out["ok"], true);
        assert_eq!(out["frame_id"], "F7");
        assert_eq!(fake.calls()[0].0, "Page.navigate");
    }

    #[tokio::test]
    async fn get_title_uses_runtime_evaluate() {
        let fake = FakeCdp::new(json!("Trios"));
        let out = execute_command(&fake, &cmd("browser_get_title", json!({})))
            .await
            .unwrap();
        assert_eq!(out["title"], "Trios");
        let (method, params) = &fake.calls()[0];
        assert_eq!(method, "Runtime.evaluate");
        assert_eq!(params["returnByValue"], true);
    }

    #[tokio::test]
    async fn click_escapes_the_selector() {
        let fake = FakeCdp::new(json!({"clicked": true}));
        let evil = "a[href=\"x\"] ' `";
        execute_command(&fake, &cmd("browser_click", json!({"selector": evil})))
            .await
            .unwrap();
        let expr = fake.calls()[0].1["expression"].as_str().unwrap().to_string();
        assert!(expr.contains(&js_str(evil)), "selector must be JSON-escaped: {expr}");
    }

    #[tokio::test]
    async fn screenshot_returns_base64_payload() {
        let fake = FakeCdp::new(json!(null));
        let out = execute_command(&fake, &cmd("browser_screenshot", json!({})))
            .await
            .unwrap();
        assert_eq!(out["data"], "aGVsbG8=");
        assert_eq!(out["format"], "png");
    }

    #[tokio::test]
    async fn eval_requires_expression() {
        let fake = FakeCdp::new(json!(null));
        let err = execute_command(&fake, &cmd("browser_eval", json!({})))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("expression"));
    }

    #[tokio::test]
    async fn tabs_map_to_target_domain() {
        let fake = FakeCdp::new(json!(null));
        let opened = execute_command(&fake, &cmd("browser_open_tab", json!({"url": "https://y"})))
            .await
            .unwrap();
        assert_eq!(opened["target_id"], "T1");
        execute_command(&fake, &cmd("browser_close_tab", json!({"target_id": "T1"})))
            .await
            .unwrap();
        let names: Vec<String> = fake.calls().iter().map(|(m, _)| m.clone()).collect();
        assert_eq!(names, vec!["Target.createTarget", "Target.closeTarget"]);
    }
}
