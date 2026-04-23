//! Basic Auth middleware for axum HTTP routes
//!
//! Handles Basic Authentication with configurable username/password.
//! Skips auth for `/.identity` and `/.port` endpoints.

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use tower_http::request::Request;
use tracing::{debug, warn};

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub username: String,
    pub password: String,
    pub enabled: bool,
}

impl AuthConfig {
    /// Create from environment variables
    pub fn from_env() -> Self {
        let username = std::env::var("AUTH_USERNAME")
            .unwrap_or_else(|_| "admin".to_string());
        let password = std::env::var("AUTH_PASSWORD")
            .unwrap_or_else(|_| String::new());
        let enabled = std::env::var("ENABLE_AUTH")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        Self {
            username,
            password,
            enabled,
        }
    }

    /// Check if authentication is required for given path
    pub fn requires_auth(&self, path: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // Skip auth for identity and port endpoints
        !path.starts_with("/.identity") && !path.starts_with("/.port")
    }

    /// Convert to Basic Auth header value (Base64)
    pub fn to_header_value(&self) -> String {
        let credentials = format!("{}:{}", self.username, self.password);
        format!("Basic {}", BASE64.encode(credentials))
    }

    /// Extract Basic Auth credentials from header
    pub fn from_header(auth_header: &str) -> Result<(String, String)> {
        let auth_header = auth_header.strip_prefix("Basic ")
            .context("Invalid Basic Auth header format")?;

        let decoded = BASE64.decode(auth_header)
            .context("Failed to decode Base64")?;

        let credentials = String::from_utf8(decoded)
            .context("Invalid UTF-8 in credentials")?;

        let parts: Vec<&str> = credentials.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid credentials format");
        }

        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Extract Basic Auth header from HTTP request
    pub fn extract_basic_auth<B>(req: &Request<B>) -> Option<String> {
        req.headers()
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            username: "admin".to_string(),
            password: String::new(),
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_env() {
        std::env::set_var("AUTH_USERNAME", "testuser");
        std::env::set_var("AUTH_PASSWORD", "testpass");
        std::env::set_var("ENABLE_AUTH", "true");

        let auth = AuthConfig::from_env();
        assert_eq!(auth.username, "testuser");
        assert_eq!(auth.password, "testpass");
        assert!(auth.enabled);
    }

    #[test]
    fn test_to_header_value() {
        let auth = AuthConfig {
            username: "user".to_string(),
            password: "pass".to_string(),
            enabled: true,
        };

        let header = auth.to_header_value();
        assert!(header.starts_with("Basic "));

        let decoded = BASE64.decode(header.strip_prefix("Basic ").unwrap()).unwrap();
        let credentials = String::from_utf8(decoded).unwrap();
        assert_eq!(credentials, "user:pass");
    }

    #[test]
    fn test_requires_auth() {
        let auth = AuthConfig {
            username: "user".to_string(),
            password: "pass".to_string(),
            enabled: true,
        };

        assert!(auth.requires_auth("/console-logs"));
        assert!(auth.requires_auth("/.identity"));
        assert!(auth.requires_auth("/.port"));
    }

    #[test]
    fn test_auth_disabled() {
        let auth = AuthConfig {
            username: "user".to_string(),
            password: "pass".to_string(),
            enabled: false,
        };

        assert!(!auth.requires_auth("/console-logs"));
    }

    #[test]
    fn test_from_header() {
        let header = "Basic dXNlc3JhcGFzcw==";
        let (user, pass) = AuthConfig::from_header(header).unwrap();
        assert_eq!(user, "dima");
        assert_eq!(pass, "password");
    }
}
