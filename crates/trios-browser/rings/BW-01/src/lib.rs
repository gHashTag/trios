//! BW-01 — browser action protocol.
//!
//! Models the `Browser` class methods (`goto`, `goBack`, `goForward`,
//! `reload`, `closePage`, `screenshot`, `snapshot`, `content`, `evaluate`,
//! `click`, `listPages`, `getActivePage`) as a serializable command/response
//! envelope. trios-server routes these to the CDP driver over A2A; the driver
//! executes them next to the live browser. This ring is transport-agnostic
//! and pure — no CDP, no I/O. Depends on BW-00 for result payloads.

use serde::{Deserialize, Serialize};
use trios_browser_bw00::PageInfo;

/// A browser action request, tagged by `action` (camelCase to match TS).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum BrowserCommand {
    ListPages,
    GetActivePage,
    Goto { page: i64, url: String },
    GoBack { page: i64 },
    GoForward { page: i64 },
    Reload { page: i64 },
    ClosePage { page: i64 },
    Snapshot { page: i64 },
    Content { page: i64, #[serde(skip_serializing_if = "Option::is_none", default)] selector: Option<String> },
    Screenshot { page: i64, #[serde(default)] full_page: bool },
    Evaluate { page: i64, expression: String },
    Click { page: i64, #[serde(skip_serializing_if = "Option::is_none", default)] selector: Option<String>, #[serde(skip_serializing_if = "Option::is_none", default)] node_id: Option<i64> },
}

impl BrowserCommand {
    /// The page id this command targets, if any (None for list/active queries).
    pub fn target_page(&self) -> Option<i64> {
        match self {
            BrowserCommand::ListPages | BrowserCommand::GetActivePage => None,
            BrowserCommand::Goto { page, .. }
            | BrowserCommand::GoBack { page }
            | BrowserCommand::GoForward { page }
            | BrowserCommand::Reload { page }
            | BrowserCommand::ClosePage { page }
            | BrowserCommand::Snapshot { page }
            | BrowserCommand::Content { page, .. }
            | BrowserCommand::Screenshot { page, .. }
            | BrowserCommand::Evaluate { page, .. }
            | BrowserCommand::Click { page, .. } => Some(*page),
        }
    }
}

/// A screenshot result (mirrors the TS screenshot return shape).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotResult {
    pub data: String,
    pub mime_type: String,
    pub device_pixel_ratio: f64,
}

/// A click point result.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClickPoint {
    pub x: f64,
    pub y: f64,
}

/// The response to a [`BrowserCommand`], tagged by `result`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "camelCase")]
pub enum BrowserResponse {
    Pages { pages: Vec<PageInfo> },
    Page { page: Option<PageInfo> },
    Ok,
    Text { text: String },
    Screenshot(ScreenshotResult),
    Evaluated { value: serde_json::Value },
    Clicked { point: Option<ClickPoint> },
    Error { message: String, #[serde(skip_serializing_if = "Option::is_none", default)] code: Option<String> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_tagged_camelcase() {
        let c = BrowserCommand::Goto { page: 3, url: "https://x".into() };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["action"], serde_json::json!("goto"));
        assert_eq!(v["page"], serde_json::json!(3));
        let back: BrowserCommand = serde_json::from_value(v).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn target_page_routing() {
        assert_eq!(BrowserCommand::ListPages.target_page(), None);
        assert_eq!(BrowserCommand::Reload { page: 7 }.target_page(), Some(7));
        assert_eq!(
            BrowserCommand::Click { page: 2, selector: Some("button".into()), node_id: None }.target_page(),
            Some(2)
        );
    }

    #[test]
    fn response_roundtrip() {
        let r = BrowserResponse::Screenshot(ScreenshotResult {
            data: "AAAA".into(),
            mime_type: "image/png".into(),
            device_pixel_ratio: 2.0,
        });
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["result"], serde_json::json!("screenshot"));
        assert_eq!(v["mimeType"], serde_json::json!("image/png"));
        let back: BrowserResponse = serde_json::from_value(v).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn error_response() {
        let r = BrowserResponse::Error { message: "no such page".into(), code: Some("ENOENT".into()) };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["result"], serde_json::json!("error"));
        assert_eq!(v["message"], serde_json::json!("no such page"));
    }
}
