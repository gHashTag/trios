//! Integration: HttpLlmClient against a local OpenAI-compatible mock.

use axum::{extract::State, routing::post, Json, Router};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use trios_agent_loop_al00::{ChatMessage, ChatRequest, HttpLlmClient, LlmClient, LlmConfig};

#[derive(Clone, Default)]
struct MockState {
    last_request: Arc<Mutex<Option<Value>>>,
    last_auth: Arc<Mutex<Option<String>>>,
}

async fn completions(
    State(state): State<MockState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    *state.last_request.lock().unwrap() = Some(body);
    *state.last_auth.lock().unwrap() = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    Json(json!({
        "choices": [{
            "message": {"role": "assistant", "content": "mock says hi"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 5, "completion_tokens": 4}
    }))
}

#[tokio::test]
async fn http_client_round_trip() {
    let state = MockState::default();
    let app = Router::new()
        .route("/v1/chat/completions", post(completions))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = HttpLlmClient::new(LlmConfig {
        base_url: format!("http://{addr}/v1"),
        api_key: Some("sk-test".into()),
        model: "mock-model".into(),
    });
    let request = ChatRequest {
        model: "mock-model".into(),
        messages: vec![ChatMessage::system("sys"), ChatMessage::user("hello")],
        tools: vec![],
        temperature: Some(0.0),
    };
    let turn = client.chat(&request).await.unwrap();

    assert_eq!(turn.content.as_deref(), Some("mock says hi"));
    assert_eq!(turn.finish_reason.as_deref(), Some("stop"));
    assert_eq!(turn.prompt_tokens, Some(5));

    let seen = state.last_request.lock().unwrap().clone().unwrap();
    assert_eq!(seen["model"], "mock-model");
    assert_eq!(seen["messages"][1]["content"], "hello");
    // tools omitted when empty
    assert!(seen.get("tools").is_none());
    assert_eq!(
        state.last_auth.lock().unwrap().as_deref(),
        Some("Bearer sk-test")
    );
}
