//! `POST /chat` — the BrowserOS sidepanel chat endpoint (Wave 16).
//!
//! The sidepanel (`useChatSession` → `DefaultChatTransport` from the
//! AI SDK v6) posts the body built by `buildChatRequestBody` to
//! `${agentServerUrl}/chat` and expects an **AI SDK UI Message Stream**
//! over SSE (`data: {chunk}` lines, terminated by `data: [DONE]`, with
//! the `x-vercel-ai-ui-message-stream: v1` header).
//!
//! The legacy TS Hono server owned this route; it died with the TS
//! retirement, which silently broke chat in the app (the panel got 404).
//! This module re-implements it on top of the Rust agent loop
//! (`trios-agent-loop`, rings AL-00..02):
//!
//! - provider resolution reuses the same mapping as `/test-provider`
//!   (`rest_browseros::provider_base_url`) — OpenAI-compatible only;
//! - `previousConversation` (role/content pairs) and `userSystemPrompt`
//!   are folded into the system prompt;
//! - browser tools are exposed when a host browser agent is actively
//!   polling the SR-03 queue (or when the body carries an explicit
//!   `browserAgentId`), mirroring `/agent/run`;
//! - loop `StepEvent`s are translated into UI-stream chunks
//!   (`text-*`, `tool-input-available`, `tool-output-available`, …).

use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{AppendHeaders, IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;

use trios_agent_loop::{
    register_browser_tools, register_builtin_tools, AgentLoop, AgentLoopConfig, HttpLlmClient,
    LlmConfig, StepEvent, ToolRegistry,
};

use crate::rest_agent::{browser_bridge_timeout, QueueBrowserBridge};
use crate::rest_browseros::provider_base_url;
use crate::ws_handler::AppState;

/// A browser agent that polled the SR-03 queue within this window is
/// considered "connected" and gets browser tools wired into the chat.
const ACTIVE_POLLER_WINDOW: Duration = Duration::from_secs(60);

/// UI-stream protocol header expected by the AI SDK client.
const UI_STREAM_HEADER: (&str, &str) = ("x-vercel-ai-ui-message-stream", "v1");

fn chunk(value: Value) -> Event {
    Event::default().data(value.to_string())
}

/// Fold `userSystemPrompt` + `previousConversation` into one system prompt.
fn build_system_prompt(body: &Value) -> Option<String> {
    let mut sections: Vec<String> = Vec::new();
    if let Some(sp) = body
        .get("userSystemPrompt")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
    {
        sections.push(sp.trim().to_string());
    }
    if let Some(history) = body.get("previousConversation").and_then(Value::as_array) {
        let lines: Vec<String> = history
            .iter()
            .filter_map(|m| {
                let role = m.get("role").and_then(Value::as_str)?;
                let content = m.get("content").and_then(Value::as_str)?;
                if content.trim().is_empty() {
                    return None;
                }
                Some(format!("{role}: {content}"))
            })
            .collect();
        if !lines.is_empty() {
            sections.push(format!(
                "Previous conversation (for context):\n{}",
                lines.join("\n")
            ));
        }
    }
    if sections.is_empty() {
        None
    } else {
        Some(sections.join("\n\n"))
    }
}

fn resolve_llm_config(body: &Value) -> Result<LlmConfig, String> {
    let mut config = LlmConfig::from_env();
    match provider_base_url(body) {
        Some(base) => config.base_url = base,
        None => {
            let provider = body
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            return Err(format!(
                "provider '{provider}' needs an OpenAI-compatible baseUrl"
            ));
        }
    }
    if let Some(key) = body
        .get("apiKey")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
    {
        config.api_key = Some(key.to_string());
    }
    if let Some(model) = body
        .get("model")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty() && *s != "default")
    {
        config.model = model.to_string();
    }
    Ok(config)
}

async fn resolve_browser_agent(state: &AppState, body: &Value) -> Option<String> {
    if let Some(id) = body
        .get("browserAgentId")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
    {
        return Some(id.to_string());
    }
    state.browser.active_agent(ACTIVE_POLLER_WINDOW).await
}

async fn chat(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    let message = body
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if message.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "message is required"})),
        )
            .into_response();
    }

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Event>();

    // Protocol preamble.
    let _ = tx.send(chunk(json!({"type": "start"})));
    let _ = tx.send(chunk(json!({"type": "start-step"})));

    match resolve_llm_config(&body) {
        Ok(llm_config) => {
            let model = llm_config.model.clone();
            let mut loop_config = AgentLoopConfig::default();
            if let Some(system) = build_system_prompt(&body) {
                loop_config.system_prompt = system;
            }
            loop_config.temperature = body.get("temperature").and_then(Value::as_f64);

            let browser_agent = resolve_browser_agent(&state, &body).await;
            let state_clone = state.clone();
            let events = tx.clone();

            tokio::spawn(async move {
                let client = HttpLlmClient::new(llm_config);
                let mut registry = ToolRegistry::new();
                register_builtin_tools(&mut registry);
                if let Some(agent_id) = &browser_agent {
                    let bridge = Arc::new(QueueBrowserBridge {
                        state: state_clone,
                        agent_id: agent_id.clone(),
                        timeout: browser_bridge_timeout(),
                    });
                    register_browser_tools(&mut registry, bridge);
                }

                let (step_tx, mut step_rx) =
                    tokio::sync::mpsc::unbounded_channel::<StepEvent>();
                let forward = tokio::spawn({
                    let events = events.clone();
                    async move {
                        let mut text_seq = 0usize;
                        while let Some(event) = step_rx.recv().await {
                            match event {
                                StepEvent::AssistantText { text, .. } => {
                                    if text.is_empty() {
                                        continue;
                                    }
                                    text_seq += 1;
                                    let id = format!("txt-{text_seq}");
                                    let _ = events
                                        .send(chunk(json!({"type": "text-start", "id": id})));
                                    let _ = events.send(chunk(
                                        json!({"type": "text-delta", "id": id, "delta": text}),
                                    ));
                                    let _ = events
                                        .send(chunk(json!({"type": "text-end", "id": id})));
                                }
                                StepEvent::ToolCallStarted {
                                    id, name, arguments, ..
                                } => {
                                    let _ = events.send(chunk(json!({
                                        "type": "tool-input-available",
                                        "toolCallId": id,
                                        "toolName": name,
                                        "input": arguments,
                                        "dynamic": true,
                                    })));
                                }
                                StepEvent::ToolResult {
                                    id, result, is_error, ..
                                } => {
                                    let payload = if is_error {
                                        json!({
                                            "type": "tool-output-error",
                                            "toolCallId": id,
                                            "errorText": result.to_string(),
                                            "dynamic": true,
                                        })
                                    } else {
                                        json!({
                                            "type": "tool-output-available",
                                            "toolCallId": id,
                                            "output": result,
                                            "dynamic": true,
                                        })
                                    };
                                    let _ = events.send(chunk(payload));
                                }
                                StepEvent::Done { .. } => {}
                            }
                        }
                    }
                });

                let agent = AgentLoop::new(&client, &registry, model, loop_config);
                if let Err(err) = agent.run(&message, Some(step_tx)).await {
                    let _ = events.send(chunk(json!({
                        "type": "error",
                        "errorText": err.to_string(),
                    })));
                }
                let _ = forward.await;
                let _ = events.send(chunk(json!({"type": "finish-step"})));
                let _ = events.send(chunk(json!({"type": "finish"})));
                let _ = events.send(Event::default().data("[DONE]"));
            });
        }
        Err(msg) => {
            let _ = tx.send(chunk(json!({"type": "error", "errorText": msg})));
            let _ = tx.send(chunk(json!({"type": "finish-step"})));
            let _ = tx.send(chunk(json!({"type": "finish"})));
            let _ = tx.send(Event::default().data("[DONE]"));
        }
    }

    let stream = UnboundedReceiverStream::new(rx).map(Ok::<Event, std::convert::Infallible>);
    (
        AppendHeaders([UI_STREAM_HEADER]),
        Sse::new(stream).keep_alive(KeepAlive::default()),
    )
        .into_response()
}

pub fn router() -> Router<AppState> {
    Router::new().route("/chat", post(chat))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn app() -> Router {
        let state = AppState::new();
        router().with_state(state)
    }

    async fn post_chat(router: &Router, body: Value) -> (StatusCode, String) {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/chat")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&bytes).to_string())
    }

    /// Minimal OpenAI-compatible mock: always answers with plain text.
    async fn spawn_mock_llm(reply: &'static str) -> String {
        use axum::routing::post as axum_post;
        let mock = Router::new().route(
            "/v1/chat/completions",
            axum_post(move || async move {
                Json(json!({
                    "choices": [{
                        "message": {"role": "assistant", "content": reply},
                        "finish_reason": "stop",
                    }],
                    "usage": {"prompt_tokens": 1, "completion_tokens": 1},
                }))
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, mock).await.unwrap();
        });
        format!("http://{addr}/v1")
    }

    #[tokio::test]
    async fn chat_requires_message() {
        let (status, body) = post_chat(&app(), json!({"provider": "openai"})).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.contains("message is required"));
    }

    #[tokio::test]
    async fn chat_streams_ui_message_chunks() {
        let base = spawn_mock_llm("привет из rust-чата").await;
        let (status, body) = post_chat(
            &app(),
            json!({
                "message": "hi",
                "provider": "custom",
                "baseUrl": base,
                "model": "fake",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(r#""type":"start""#), "body: {body}");
        assert!(body.contains(r#""type":"text-delta""#), "body: {body}");
        assert!(body.contains("привет из rust-чата"), "body: {body}");
        assert!(body.contains(r#""type":"finish""#), "body: {body}");
        assert!(body.contains("[DONE]"), "body: {body}");
    }

    #[tokio::test]
    async fn chat_reports_unresolvable_provider_as_error_chunk() {
        let (status, body) = post_chat(
            &app(),
            json!({"message": "hi", "provider": "browseros", "model": "default"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(r#""type":"error""#), "body: {body}");
        assert!(body.contains("OpenAI-compatible baseUrl"), "body: {body}");
        assert!(body.contains("[DONE]"), "body: {body}");
    }

    #[tokio::test]
    async fn system_prompt_folds_history() {
        let system = build_system_prompt(&json!({
            "userSystemPrompt": "будь краток",
            "previousConversation": [
                {"role": "user", "content": "раз"},
                {"role": "assistant", "content": "два"},
            ],
        }))
        .unwrap();
        assert!(system.starts_with("будь краток"));
        assert!(system.contains("user: раз"));
        assert!(system.contains("assistant: два"));
    }
}
