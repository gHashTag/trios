//! SR-00 — Config & Identity Types (zero deps except serde)

#[derive(Debug, Clone, Copy)]
pub struct ServerPort(pub u16);

impl Default for ServerPort {
    fn default() -> Self { Self(3026) }
}

#[derive(Debug, Clone)]
pub struct AuthCredentials {
    pub username: String,
    pub password: String,
    pub enabled: bool,
}

impl Default for AuthCredentials {
    fn default() -> Self {
        Self {
            username: std::env::var("AUTH_USERNAME").unwrap_or_else(|_| "perplexity".into()),
            password: std::env::var("AUTH_PASSWORD").unwrap_or_default(),
            enabled: std::env::var("ENABLE_AUTH").unwrap_or_else(|_| "true".into()) != "false",
        }
    }
}

#[derive(Debug, Clone)]
pub struct McpConfig {
    pub port: ServerPort,
    pub auth: AuthCredentials,
    pub server_host: String,
    pub log_limit: usize,
    pub query_limit: usize,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            port: ServerPort::default(),
            auth: AuthCredentials::default(),
            server_host: "127.0.0.1".to_string(),
            log_limit: 50,
            query_limit: 100,
        }
    }
}

impl McpConfig {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3026);
        Self {
            port: ServerPort(port),
            auth: AuthCredentials::default(),
            server_host: std::env::var("SERVER_HOST")
                .unwrap_or_else(|_| "127.0.0.1".into()),
            log_limit: std::env::var("LOG_LIMIT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(50),
            query_limit: std::env::var("QUERY_LIMIT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(100),
        }
    }
}

pub const IDENTITY_SIGNATURE: &str = "mcp-browser-connector-24x7";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_port() {
        assert_eq!(ServerPort::default().0, 3026);
    }

    #[test]
    fn test_identity_signature() {
        assert!(!IDENTITY_SIGNATURE.is_empty());
        assert_eq!(IDENTITY_SIGNATURE, "mcp-browser-connector-24x7");
    }

    #[test]
    fn test_config_from_env() {
        let c = McpConfig::from_env();
        assert!(c.port.0 > 0);
        assert!(!c.server_host.is_empty());
    }

    #[test]
    fn test_auth_credentials_default() {
        let auth = AuthCredentials::default();
        assert_eq!(auth.username, "perplexity");
        assert!(auth.enabled);
    }
}
