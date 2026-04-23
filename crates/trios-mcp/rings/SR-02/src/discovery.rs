//! Server discovery for MCP stdio
//!
//! Discovers SR-00 HTTP server by checking ports 3025-3035 sequentially.
//! Uses `/.identity` endpoint with signature `"mcp-browser-connector-24x7"`.

use anyhow::{Context, Result};
use reqwest::Client;
use tracing::{debug, info, warn};

/// Server identity signature
pub const SIGNATURE: &'static str = "mcp-browser-connector-24x7";

/// Server identity response
#[derive(Debug, serde::Deserialize)]
pub struct IdentityResponse {
    pub signature: String,
}

/// Discover SR-00 server
pub async fn discover_server(
    host: &str,
    username: &str,
    password: &str,
) -> Result<Option<String>> {
    let client = Client::new();
    let auth = format!("{}:{}", username, password);

    // Check ports 3025-3035 sequentially
    for port in 3025u16..=3035 {
        let url = format!("http://{}:{}/.identity", host, port);
        debug!("Checking server at: {}", url);

        let response = match client
            .get(&url)
            .basic_auth(&auth)
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(text) = resp.text().await {
                    if let Ok(identity) = serde_json::from_str::<IdentityResponse>(&text) {
                        if identity.signature == SIGNATURE {
                            info!("Found server at {}:{}", host);
                            return Ok(Some(format!("{}:{}", host, port)));
                        }
                    }
                }
            }
            Err(e) => {
                debug!("Failed to connect to {}:{} - {}", host, port, e);
            }
        }
    }

    warn!("SR-00 server not found on {}:{} - {}", host, host, username);
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature() {
        assert_eq!(SIGNATURE, "mcp-browser-connector-24x7");
    }

    #[test]
    fn test_identity_response() {
        let json = r#"{"signature": "mcp-browser-connector-24x7"}"#;
        let resp: IdentityResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.signature, "mcp-browser-connector-24x7");
    }
}
