use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::path::PathBuf;
use std::time::Duration;
use tracing::warn;

/// Validates that a repository path is safe to access.
///
/// Checks:
/// - Path is absolute
/// - Path exists and is a directory
/// - Canonicalized path does not escape allowed roots (if configured)
/// - Path does not contain suspicious components
pub fn validate_repo_path(repo_path: &str, allowed_roots: &[PathBuf]) -> Result<PathBuf, String> {
    let path = PathBuf::from(repo_path);

    // Must be absolute
    if !path.is_absolute() {
        return Err("repo_path must be an absolute path".into());
    }

    // Must exist and be a directory
    if !path.exists() {
        return Err(format!("repo_path does not exist: {}", repo_path));
    }
    if !path.is_dir() {
        return Err(format!("repo_path is not a directory: {}", repo_path));
    }

    // Canonicalize to resolve symlinks and `..`
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("failed to canonicalize repo_path: {}", e))?;

    // If allowed_roots is configured, verify the path is under one of them
    if !allowed_roots.is_empty() {
        let permitted = allowed_roots.iter().any(|root| {
            let canonical_root = root.canonicalize().unwrap_or_else(|_| root.clone());
            canonical.starts_with(&canonical_root)
        });
        if !permitted {
            return Err(format!(
                "repo_path '{}' is outside allowed directories",
                repo_path
            ));
        }
    }

    Ok(canonical)
}

/// Bearer token authentication middleware for axum.
///
/// Reads the expected token from the `TRIOS_API_KEY` environment variable.
/// If the variable is not set, auth is disabled (for development).
/// Skips auth for GET requests to `/` and `/health`.
pub async fn auth_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    let expected_token = std::env::var("TRIOS_API_KEY").unwrap_or_default();
    let path = request.uri().path().to_owned();

    // Skip auth for health endpoints
    if matches!(path.as_str(), "/" | "/health") {
        return Ok(next.run(request).await);
    }

    // If no API key is configured, allow all requests (dev mode)
    if expected_token.is_empty() {
        return Ok(next.run(request).await);
    }

    // Check Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(value) if value.starts_with("Bearer ") => {
            let token = &value[7..];
            if token == expected_token {
                Ok(next.run(request).await)
            } else {
                warn!(path = %path, "invalid API key provided");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => {
            warn!(path = %path, "missing or malformed Authorization header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Default trusted origins when `TRIOS_TRUSTED_ORIGINS` is unset.
/// Mirrors the trusted-app-origin allowlist enforced by the legacy Hono
/// server's `requireTrustedAppOrigin` on `/agents`, `/claw`, `/terminal`.
/// Wave 1 extends the same guard to cover ALL routes (incl. `/a2a`),
/// closing the P0 finding where the TS `/a2a` routes accepted any Origin.
const DEFAULT_TRUSTED_ORIGINS: &[&str] = &[
    "http://localhost",
    "http://127.0.0.1",
    // BrowserOS / trios app custom schemes
    "chrome-extension://",
    "trios://",
    "app://",
];

/// Returns the configured trusted-origin prefixes. Reads a comma-separated
/// `TRIOS_TRUSTED_ORIGINS` env var; falls back to [`DEFAULT_TRUSTED_ORIGINS`].
fn trusted_origins() -> Vec<String> {
    match std::env::var("TRIOS_TRUSTED_ORIGINS") {
        Ok(v) if !v.trim().is_empty() => v
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => DEFAULT_TRUSTED_ORIGINS.iter().map(|s| s.to_string()).collect(),
    }
}

/// Pure predicate: is `origin` trusted against `allowed` prefixes?
/// An origin is trusted if it starts with any allowed prefix.
pub fn is_trusted_origin(origin: &str, allowed: &[String]) -> bool {
    allowed.iter().any(|a| origin.starts_with(a.as_str()))
}

/// Origin-guard middleware (Wave 1 — closes P0).
///
/// - Requests WITHOUT an `Origin` header pass (native app / server-to-server
///   clients like the Swift `A2ARegistryClient` and curl do not send Origin).
/// - Requests WITH an `Origin` header must match the trusted allowlist,
///   otherwise they are rejected with 403 Forbidden. This blocks a foreign
///   web page from driving the A2A/agent endpoints via the browser.
/// - `/` and `/health` always pass (liveness probes).
pub async fn origin_guard(request: Request, next: Next) -> Result<Response, StatusCode> {
    let path = request.uri().path().to_owned();
    if matches!(path.as_str(), "/" | "/health") {
        return Ok(next.run(request).await);
    }

    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match origin {
        // No Origin header -> native/CLI client, allow.
        None => Ok(next.run(request).await),
        Some(o) => {
            if is_trusted_origin(&o, &trusted_origins()) {
                Ok(next.run(request).await)
            } else {
                warn!(path = %path, origin = %o, "rejected untrusted Origin");
                Err(StatusCode::FORBIDDEN)
            }
        }
    }
}

/// Request timeout middleware using `tokio::time::timeout`.
/// Returns 504 Gateway Timeout if the handler takes too long.
pub async fn timeout_middleware(request: Request, next: Next) -> Response {
    let timeout_secs = std::env::var("TRIOS_REQUEST_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30u64);
    let path = request.uri().path().to_owned();

    match tokio::time::timeout(Duration::from_secs(timeout_secs), next.run(request)).await {
        Ok(response) => response,
        Err(_) => {
            warn!(path = %path, "request timed out");
            StatusCode::GATEWAY_TIMEOUT.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn origin_guard_trusts_localhost_and_app_schemes() {
        let allowed = trusted_origins();
        assert!(is_trusted_origin("http://localhost:9005", &allowed));
        assert!(is_trusted_origin("http://127.0.0.1:9200", &allowed));
        assert!(is_trusted_origin("chrome-extension://abc123", &allowed));
        assert!(is_trusted_origin("trios://app", &allowed));
    }

    #[test]
    fn origin_guard_rejects_foreign_origin() {
        let allowed = trusted_origins();
        assert!(!is_trusted_origin("https://evil.example.com", &allowed));
        assert!(!is_trusted_origin("http://attacker.local", &allowed));
    }

    #[test]
    fn origin_guard_respects_env_override() {
        std::env::set_var("TRIOS_TRUSTED_ORIGINS", "https://my.app, https://ok.dev");
        let allowed = trusted_origins();
        assert!(is_trusted_origin("https://my.app", &allowed));
        assert!(is_trusted_origin("https://ok.dev/page", &allowed));
        assert!(!is_trusted_origin("http://localhost", &allowed));
        std::env::remove_var("TRIOS_TRUSTED_ORIGINS");
    }

    #[test]
    fn test_reject_relative_path() {
        let result = validate_repo_path("./some/relative/path", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("absolute"));
    }

    #[test]
    fn test_reject_nonexistent_path() {
        let result = validate_repo_path("/nonexistent/path/xyz123", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_accept_valid_absolute_path() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_str().unwrap();
        let result = validate_repo_path(path, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reject_path_outside_allowed_roots() {
        let dir = TempDir::new().unwrap();
        let allowed = vec![PathBuf::from("/tmp/some_other_dir")];
        let result = validate_repo_path(dir.path().to_str().unwrap(), &allowed);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("outside allowed"));
    }

    #[test]
    fn test_accept_path_within_allowed_root() {
        let dir = TempDir::new().unwrap();
        let allowed = vec![dir.path().to_path_buf()];
        let result = validate_repo_path(dir.path().to_str().unwrap(), &allowed);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reject_file_not_directory() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello").unwrap();
        let result = validate_repo_path(file_path.to_str().unwrap(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a directory"));
    }
}
