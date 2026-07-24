//! BW-00 — browser data types.
//!
//! Ported 1:1 from `apps/server/src/browser/browser.ts` (`PageInfo`,
//! `WindowInfo`, ...). Pure data + serde, camelCase wire form matching the
//! TS/CDP driver. No CDP, no I/O — the live Chrome driver stays next to the
//! browser process; these types are the contract trios-server proxies over.

use serde::{Deserialize, Serialize};

/// A browser page/tab (TS `PageInfo`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub page_id: i64,
    pub target_id: String,
    pub tab_id: i64,
    pub url: String,
    pub title: String,
    pub is_active: bool,
    pub is_loading: bool,
    pub load_progress: f64,
    pub is_pinned: bool,
    pub is_hidden: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub window_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub index: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowType {
    Normal,
    Popup,
    App,
    Devtools,
    AppPopup,
    PictureInPicture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowBounds {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub left: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub top: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub width: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub height: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub window_state: Option<WindowState>,
}

/// A browser window (TS `WindowInfo`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub window_id: i64,
    pub window_type: WindowType,
    pub bounds: WindowBounds,
    pub is_active: bool,
    pub is_visible: bool,
    pub tab_count: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub active_tab_id: Option<i64>,
}

/// Result of setting window visibility (TS `SetWindowVisibilityResult`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetWindowVisibilityResult {
    pub window: WindowInfo,
    pub replaced: bool,
    pub previous_window_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_info_camelcase() {
        let p = PageInfo {
            page_id: 1,
            target_id: "T1".into(),
            tab_id: 5,
            url: "https://x".into(),
            title: "X".into(),
            is_active: true,
            is_loading: false,
            load_progress: 1.0,
            is_pinned: false,
            is_hidden: false,
            window_id: Some(2),
            index: None,
            group_id: None,
        };
        let v = serde_json::to_value(&p).unwrap();
        assert!(v.get("pageId").is_some());
        assert!(v.get("targetId").is_some());
        assert!(v.get("isActive").is_some());
        assert!(v.get("index").is_none()); // None skipped
        let back: PageInfo = serde_json::from_value(v).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn window_type_snake_case() {
        let v = serde_json::to_value(WindowType::PictureInPicture).unwrap();
        assert_eq!(v, serde_json::json!("picture_in_picture"));
        let v = serde_json::to_value(WindowType::AppPopup).unwrap();
        assert_eq!(v, serde_json::json!("app_popup"));
    }

    #[test]
    fn window_state_lowercase() {
        let v = serde_json::to_value(WindowState::Fullscreen).unwrap();
        assert_eq!(v, serde_json::json!("fullscreen"));
    }
}
