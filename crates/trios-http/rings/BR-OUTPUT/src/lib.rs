//! BR-OUTPUT — Router assembly
//! Final ring: assembles HR-00 types + HR-01 handlers into axum Router

use axum::{Router, routing::{get, post}};
use hr_00::AppState;
use hr_01::{health, status, chat};

/// Build the complete axum Router for trios-http
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/status", get(status))
        .route("/api/chat", post(chat))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_builds() {
        let state = AppState::new(19);
        let _ = build_router(state);
    }
}
