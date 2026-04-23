//! HR-01 — Route handlers for trios-http
//! Ring 01 — POST /api/chat, GET /api/status, GET /health

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};
use hr_00::{AppState, ChatRequest, ChatResponse, StatusResponse};
use tracing::info;

/// GET /health
pub async fn health() -> &'static str {
    "ok"
}

/// GET /api/status
pub async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    let agents = *state.agents.read().await;
    info!("status: agents={}, tools={}", agents, state.tools);
    Json(StatusResponse {
        status: "ok",
        agents,
        tools: state.tools,
    })
}

/// POST /api/chat
pub async fn chat(
    State(_state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> impl IntoResponse {
    info!("chat: method={}", req.method);

    // Dispatch to known methods
    let result = match req.method.as_str() {
        "agents/list" => serde_json::json!([]),
        "agents/chat" => {
            let msg = req.params.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            serde_json::json!({ "reply": format!("echo: {}", msg) })
        }
        "tools/list" => serde_json::json!({ "tools": [] }),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ChatResponse {
                    result: None,
                    error: Some(format!("unknown method: {}", req.method)),
                }),
            ).into_response();
        }
    };

    (
        StatusCode::OK,
        Json(ChatResponse {
            result: Some(result),
            error: None,
        }),
    ).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use br_output::build_router;

    fn test_state() -> AppState {
        AppState::new(19)
    }

    #[tokio::test]
    async fn test_health() {
        let app = build_router(test_state());
        let resp = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_status() {
        let app = build_router(test_state());
        let resp = app
            .oneshot(Request::builder().uri("/api/status").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_chat_agents_list() {
        let app = build_router(test_state());
        let body = r##"{"method":"agents/list","params":{}}"##;
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
