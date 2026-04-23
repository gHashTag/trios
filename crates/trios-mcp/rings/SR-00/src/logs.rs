//! In-memory log storage for browser events
//!
//! Stores console logs, errors, network requests, selected element, and current URL.

use serde::{Deserialize, Serialize};
use tracing::debug;

/// Console log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleLog {
    #[serde(rename = "type")]
    pub log_type: String,
    #[serde(default)]
    pub level: Option<String>,
    pub message: Option<String>,
    pub url: Option<String>,
    pub timestamp: String,
}

/// Console error entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub level: String,
    pub message: String,
    pub timestamp: String,
}

/// Network request entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    #[serde(rename = "type")]
    pub request_type: String,
    pub url: String,
    pub method: Option<String>,
    #[serde(rename = "status")]
    pub status_code: Option<i64>,
    pub timestamp: String,
}

/// Selected DOM element
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SelectedElement {
    #[serde(rename = "element")]
    Element(Element),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Element {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub class_name: Option<String>,
    #[serde(rename = "snippet")]
    pub html_snippet: Option<String>,
}

/// In-memory log store with configurable limits
#[derive(Debug)]
pub struct LogStore {
    console_logs: Vec<ConsoleLog>,
    console_errors: Vec<ConsoleError>,
    network_errors: Vec<NetworkRequest>,
    network_success: Vec<NetworkRequest>,
    selected_element: Option<SelectedElement>,
    current_url: Option<String>,
}

impl LogStore {
    /// Create new empty log store
    pub fn new() -> Self {
        Self {
            console_logs: Vec::with_capacity(50),
            console_errors: Vec::with_capacity(50),
            network_errors: Vec::with_capacity(50),
            network_success: Vec::with_capacity(50),
            selected_element: None,
            current_url: None,
        }
    }

    /// Add console log
    pub fn add_console_log(&mut self, log: ConsoleLog) {
        self.console_logs.push(log);
        self.trim_if_needed();
    }

    /// Add console error
    pub fn add_console_error(&mut self, error: ConsoleError) {
        self.console_errors.push(error);
        self.trim_if_needed();
    }

    /// Add network request (routes to success or error based on status)
    pub fn add_network_request(&mut self, request: NetworkRequest) {
        match request.status_code {
            Some(code) if code >= 400 => {
                self.network_errors.push(request);
            }
            _ => {
                self.network_success.push(request);
            }
        }
        self.trim_if_needed();
    }

    /// Update selected element
    pub fn set_selected_element(&mut self, element: SelectedElement) {
        self.selected_element = Some(element);
        debug!("Selected element updated: {:?}", element);
    }

    /// Update current URL
    pub fn set_current_url(&mut self, url: String) {
        self.current_url = Some(url);
        debug!("Current URL updated: {}", url);
    }

    /// Get current URL (waits up to 10 seconds)
    pub async fn get_current_url(&self) -> Option<String> {
        // In original, this waits up to 10 seconds for URL from extension
        // For now, return stored URL immediately
        self.current_url.clone()
    }

    /// Clear all logs
    pub fn clear_all(&mut self) {
        self.console_logs.clear();
        self.console_errors.clear();
        self.network_errors.clear();
        self.network_success.clear();
        self.selected_element = None;
        debug!("All logs cleared");
    }

    /// Trim logs if they exceed capacity
    fn trim_if_needed(&mut self) {
        const MAX_ENTRIES: usize = 50;

        if self.console_logs.len() > MAX_ENTRIES {
            self.console_logs = self.console_logs.split_off(MAX_ENTRIES - 5);
            debug!("Trimmed console_logs to {}", self.console_logs.len());
        }
        if self.console_errors.len() > MAX_ENTRIES {
            self.console_errors = self.console_errors.split_off(MAX_ENTRIES - 5);
            debug!("Trimmed console_errors to {}", self.console_errors.len());
        }
        if self.network_errors.len() > MAX_ENTRIES {
            self.network_errors = self.network_errors.split_off(MAX_ENTRIES - 5);
            debug!("Trimmed network_errors to {}", self.network_errors.len());
        }
        if self.network_success.len() > MAX_ENTRIES {
            self.network_success = self.network_success.split_off(MAX_ENTRIES - 5);
            debug!("Trimmed network_success to {}", self.network_success.len());
        }
    }

    /// Truncate logs to fit within query limit (30,000 chars)
    pub fn truncate_to_query_limit(&self, max_chars: usize) -> String {
        let logs = serde_json::to_string(&self.get_all_logs()).unwrap_or_default();
        if logs.len() > max_chars {
            format!("... (truncated, {} chars total)", max_chars)
        } else {
            logs
        }
    }

    /// Get all logs as JSON
    pub fn get_all_logs(&self) -> serde_json::Value {
        serde_json::json!({
            "consoleLogs": self.console_logs,
            "consoleErrors": self.console_errors,
            "networkErrors": self.network_errors,
            "networkSuccess": self.network_success,
            "selectedElement": self.selected_element,
            "currentUrl": self.current_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_store_new() {
        let store = LogStore::new();
        assert_eq!(store.console_logs.len(), 0);
        assert_eq!(store.current_url, None);
    }

    #[test]
    fn test_add_console_log() {
        let mut store = LogStore::new();
        let log = ConsoleLog {
            log_type: "console-log".to_string(),
            level: Some("info".to_string()),
            message: Some("test".to_string()),
            url: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
        };

        store.add_console_log(log);
        assert_eq!(store.console_logs.len(), 1);
    }

    #[test]
    fn test_network_routing() {
        let mut store = LogStore::new();

        // Error (status 400) goes to network_errors
        let err_req = NetworkRequest {
            request_type: "xhr".to_string(),
            url: "https://example.com".to_string(),
            method: Some("GET".to_string()),
            status_code: Some(404),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
        };
        store.add_network_request(err_req);
        assert_eq!(store.network_errors.len(), 1);
        assert_eq!(store.network_success.len(), 0);

        // Success (status 200) goes to network_success
        let ok_req = NetworkRequest {
            request_type: "xhr".to_string(),
            url: "https://example.com".to_string(),
            method: Some("GET".to_string()),
            status_code: Some(200),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
        };
        store.add_network_request(ok_req);
        assert_eq!(store.network_success.len(), 1);
    }

    #[test]
    fn test_selected_element() {
        let mut store = LogStore::new();
        let element = SelectedElement::Element(Element {
            tag_name: Some("div".to_string()),
            id: Some("test-id".to_string()),
            class_name: None,
            html_snippet: Some("<div>test</div>".to_string()),
        });

        store.set_selected_element(element);
        assert!(store.selected_element.is_some());
    }

    #[test]
    fn test_clear_all() {
        let mut store = LogStore::new();
        store.add_console_log(ConsoleLog {
            log_type: "console-log".to_string(),
            level: None,
            message: Some("test".to_string()),
            url: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
        });

        assert_eq!(store.console_logs.len(), 1);
        store.clear_all();
        assert_eq!(store.console_logs.len(), 0);
    }
}
