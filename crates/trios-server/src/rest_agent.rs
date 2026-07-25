//! REST surface for the Rust agent tool-loop (Wave 8).
//!
//! Endpoints:
//! - `GET  /agent/tools`      — tool definitions the loop advertises
//! - `POST /agent/run`        — run the loop, JSON result
//! - `POST /agent/run/stream` — run the loop, step events over SSE
//!
//! The loop itself lives in `crates/trios-agent-loop` (rings AL-00..02),
//! ported from the retired TS agent core. Browser tools are wired to the
//! existing SR-03 host-runtime queue: the server enqueues commands, the
//! host CDP agent polls `browser_commands` and reports results back.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use futures::stream::Stream;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;

use trios_agent_loop::{
    register_browser_tools, register_builtin_tools, AgentLoop, AgentLoopConfig, BrowserBridge,
    HttpLlmClient, LlmConfig, StepEvent, ToolRegistry,
};
use trios_browser::proto::{BrowserCommand as BwCommand, BrowserResponse as BwResponse};

use crate::ws_handler::AppState;

// ---------------------------------------------------------------------------
// Request/response contract
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ProviderOverride {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentRunBody {
    pub prompt: String,
    #[serde(default)]
    pub system: Option<String>,
    #[serde(default)]
    pub max_steps: Option<usize>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub provider: Option<ProviderOverride>,
    /// When set, browser tools are exposed and routed to this host agent
    /// through the SR-03 command queue.
    #[serde(default)]
    pub browser_agent_id: Option<String>,
    /// Include the full transcript in the response (default: false).
    #[serde(default)]
    pub include_transcript: bool,
}

fn resolve_config(body: &AgentRunBody) -> LlmConfig {
    let mut config = LlmConfig::from_env();
    if let Some(p) = &body.provider {
        if let Some(url) = &p.base_url {
            config.base_url = url.clone();
        }
        if let Some(key) = &p.api_key {
            config.api_key = Some(key.clone());
        }
        if let Some(model) = &p.model {
            config.model = model.clone();
        }
    }
    config
}

fn loop_config(body: &AgentRunBody) -> AgentLoopConfig {
    let mut config = AgentLoopConfig::default();
    if let Some(system) = &body.system {
        config.system_prompt = system.clone();
    }
    if let Some(max_steps) = body.max_steps {
        config.max_steps = max_steps.clamp(1, trios_agent_loop::agent_loop::MAX_TURNS_DEFAULT);
    }
    config.temperature = body.temperature;
    config
}

fn build_registry(state: &AppState, body: &AgentRunBody) -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    register_builtin_tools(&mut registry);
    if let Some(agent_id) = &body.browser_agent_id {
        let bridge = Arc::new(QueueBrowserBridge {
            state: state.clone(),
            agent_id: agent_id.clone(),
            timeout: browser_bridge_timeout(),
        });
        register_browser_tools(&mut registry, bridge);
    }
    registry
}

fn browser_bridge_timeout() -> Duration {
    // SR-03 commands expire after 30s; stay under that.
    let secs = std::env::var("TRIOS_BROWSER_BRIDGE_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(25u64);
    Duration::from_secs(secs.min(29))
}

// ---------------------------------------------------------------------------
// SR-03 queue bridge (server enqueues, host CDP agent polls + reports)
// ---------------------------------------------------------------------------

struct QueueBrowserBridge {
    state: AppState,
    agent_id: String,
    timeout: Duration,
}

/// Map a BW-01 command to the SR-03 tool name + params the host agent speaks.
fn to_queue_tool(command: &BwCommand) -> Option<(&'static str, Value)> {
    match command {
        BwCommand::Goto { page, url } => {
            Some(("browser_navigate", json!({"page": page, "url": url})))
        }
        BwCommand::Content { page, selector } => {
            Some(("browser_get_dom", json!({"page": page, "selector": selector})))
        }
        BwCommand::Screenshot { page, full_page } => {
            Some(("browser_screenshot", json!({"page": page, "full_page": full_page})))
        }
        BwCommand::Evaluate { page, expression } => {
            Some(("browser_eval", json!({"page": page, "expression": expression})))
        }
        BwCommand::Click { page, selector, node_id } => Some((
            "browser_click",
            json!({"page": page, "selector": selector, "node_id": node_id}),
        )),
        // Not representable in the SR-03 host-tool surface (yet).
        _ => None,
    }
}

#[async_trait::async_trait]
impl BrowserBridge for QueueBrowserBridge {
    async fn execute(&self, command: BwCommand) -> anyhow::Result<BwResponse> {
        let (tool, params) = to_queue_tool(&command).ok_or_else(|| {
            anyhow::anyhow!("command not supported by the SR-03 host bridge: {command:?}")
        })?;
        let queued = trios_a2a::BrowserCommand::from_tool_name(tool, &self.agent_id, params)
            .ok_or_else(|| anyhow::anyhow!("unknown SR-03 tool: {tool}"))?;
        let command_id = queued.id.clone();
        {
            let mut queue = self.state.browser.queue.lock().await;
            queue.enqueue(queued);
        }
        let deadline = tokio::time::Instant::now() + self.timeout;
        loop {
            tokio::time::sleep(Duration::from_millis(50)).await;
            {
                let queue = self.state.browser.queue.lock().await;
                if let Some(result) = queue.get_result(&command_id) {
                    if !result.ok {
                        let message = result
                            .error
                            .clone()
                            .unwrap_or_else(|| "browser command failed".into());
                        return Ok(BwResponse::Error { message, code: None });
                    }
                    // Prefer the typed BW-01 shape; fall back to a raw value.
                    let data = result.data.clone();
                    return Ok(serde_json::from_value::<BwResponse>(data.clone())
                        .unwrap_or(BwResponse::Evaluated { value: data }));
                }
            }
            if tokio::time::Instant::now() >= deadline {
                anyhow::bail!(
                    "browser command {command_id} timed out after {:?} (host agent `{}` not reporting)",
                    self.timeout,
                    self.agent_id
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn list_tools(State(state): State<AppState>) -> Json<Value> {
    // Advertise the full surface: builtins + browser tools (bridge-backed).
    let body = AgentRunBody {
        prompt: String::new(),
        system: None,
        max_steps: None,
        temperature: None,
        provider: None,
        browser_agent_id: Some("host-cdp".into()),
        include_transcript: false,
    };
    let registry = build_registry(&state, &body);
    let defs: Vec<Value> = registry
        .defs()
        .iter()
        .map(|d| json!({"name": d.name, "description": d.description, "parameters": d.parameters}))
        .collect();
    Json(json!({ "tools": defs }))
}

async fn run_agent(
    State(state): State<AppState>,
    Json(body): Json<AgentRunBody>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if body.prompt.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "prompt is required"})),
        ));
    }
    let llm_config = resolve_config(&body);
    let model = llm_config.model.clone();
    let client = HttpLlmClient::new(llm_config);
    let registry = build_registry(&state, &body);
    let agent = AgentLoop::new(&client, &registry, model.clone(), loop_config(&body));

    let result = agent.run(&body.prompt, None).await.map_err(|err| {
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({"error": err.to_string()})),
        )
    })?;

    let mut payload = json!({
        "finalText": result.final_text,
        "steps": result.steps,
        "stopReason": result.stop_reason,
        "model": model,
        "usage": {
            "promptTokens": result.prompt_tokens,
            "completionTokens": result.completion_tokens,
        },
    });
    if body.include_transcript {
        payload["transcript"] = serde_json::to_value(&result.transcript).unwrap_or(json!([]));
    }
    Ok(Json(payload))
}

async fn run_agent_stream(
    State(state): State<AppState>,
    Json(body): Json<AgentRunBody>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<Value>)> {
    if body.prompt.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "prompt is required"})),
        ));
    }
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<StepEvent>();
    let llm_config = resolve_config(&body);
    let model = llm_config.model.clone();
    let loop_cfg = loop_config(&body);
    let state_clone = state.clone();

    tokio::spawn(async move {
        let client = HttpLlmClient::new(llm_config);
        let registry = build_registry(&state_clone, &body);
        let agent = AgentLoop::new(&client, &registry, model, loop_cfg);
        if let Err(err) = agent.run(&body.prompt, Some(tx.clone())).await {
            // Surface loop errors as a terminal SSE event.
            let _ = tx.send(StepEvent::AssistantText {
                step: 0,
                text: format!("[agent error] {err}"),
            });
            let _ = tx.send(StepEvent::Done {
                steps: 0,
                stop_reason: trios_agent_loop::StopReason::Completed,
            });
        }
    });

    let stream = UnboundedReceiverStream::new(rx).map(|event| {
        let name = match &event {
            StepEvent::AssistantText { .. } => "assistant_text",
            StepEvent::ToolCallStarted { .. } => "tool_call",
            StepEvent::ToolResult { .. } => "tool_result",
            StepEvent::Done { .. } => "done",
        };
        Ok(Event::default()
            .event(name)
            .data(serde_json::to_string(&event).unwrap_or_else(|_| "{}".into())))
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/agent/tools", get(list_tools))
        .route("/agent/run", post(run_agent))
        .route("/agent/run/stream", post(run_agent_stream))
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tower::ServiceExt;

    fn app() -> (Router, AppState) {
        let state = AppState::new();
        let router = router().with_state(state.clone());
        (router, state)
    }

    async fn post_json(router: &Router, path: &str, body: Value) -> (StatusCode, Value) {
        let res = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let value = serde_json::from_slice(&bytes).unwrap_or(json!(null));
        (status, value)
    }

    /// Mock OpenAI endpoint: first N-1 turns call `echo`, then a final text.
    async fn spawn_mock_llm(tool_turns: usize) -> String {
        use axum::routing::post as axum_post;
        let calls = Arc::new(AtomicUsize::new(0));
        let app = Router::new().route(
            "/v1/chat/completions",
            axum_post(move |Json(_body): Json<Value>| {
                let calls = calls.clone();
                async move {
                    let n = calls.fetch_add(1, Ordering::SeqCst);
                    if n < tool_turns {
                        Json(json!({
                            "choices": [{
                                "message": {
                                    "role": "assistant",
                                    "content": null,
                                    "tool_calls": [{
                                        "id": format!("call_{n}"),
                                        "type": "function",
                                        "function": {"name": "echo", "arguments": "{\"text\":\"ping\"}"}
                                    }]
                                },
                                "finish_reason": "tool_calls"
                            }]
                        }))
                    } else {
                        Json(json!({
                            "choices": [{
                                "message": {"role": "assistant", "content": "final answer"},
                                "finish_reason": "stop"
                            }],
                            "usage": {"prompt_tokens": 11, "completion_tokens": 5}
                        }))
                    }
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{addr}/v1")
    }

    #[tokio::test]
    async fn agent_tools_lists_builtin_and_browser_tools() {
        let (router, _state) = app();
        let res = router
            .clone()
            .oneshot(Request::builder().uri("/agent/tools").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let names: Vec<&str> = value["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"time_now"));
        assert!(names.contains(&"browser_goto"));
    }

    #[tokio::test]
    async fn run_requires_prompt() {
        let (router, _state) = app();
        let (status, body) = post_json(&router, "/agent/run", json!({"prompt": "  "})).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("prompt"));
    }

    #[tokio::test]
    async fn run_completes_with_tool_round_trip() {
        let base_url = spawn_mock_llm(1).await;
        let (router, _state) = app();
        let (status, body) = post_json(
            &router,
            "/agent/run",
            json!({
                "prompt": "echo ping then finish",
                "provider": {"base_url": base_url, "model": "mock"},
                "include_transcript": true
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "body: {body}");
        assert_eq!(body["finalText"], "final answer");
        assert_eq!(body["steps"], 2);
        assert_eq!(body["stopReason"], "completed");
        assert_eq!(body["usage"]["promptTokens"], 11);
        let roles: Vec<&str> = body["transcript"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["role"].as_str().unwrap())
            .collect();
        assert_eq!(roles, vec!["system", "user", "assistant", "tool", "assistant"]);
    }

    #[tokio::test]
    async fn run_respects_max_steps() {
        // The mock always asks for tools; the loop must stop at max_steps.
        let base_url = spawn_mock_llm(usize::MAX).await;
        let (router, _state) = app();
        let (status, body) = post_json(
            &router,
            "/agent/run",
            json!({
                "prompt": "loop",
                "max_steps": 2,
                "provider": {"base_url": base_url, "model": "mock"}
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["stopReason"], "max_steps");
        assert_eq!(body["steps"], 2);
        assert_eq!(body["finalText"], Value::Null);
    }

    #[tokio::test]
    async fn run_reports_llm_errors_as_bad_gateway() {
        let (router, _state) = app();
        let (status, body) = post_json(
            &router,
            "/agent/run",
            json!({
                "prompt": "hi",
                // Nothing listens here.
                "provider": {"base_url": "http://127.0.0.1:1/v1", "model": "mock"}
            }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert!(body["error"].as_str().is_some());
    }

    /// Full host-runtime round trip: the model calls `browser_goto`; a fake
    /// host agent polls the SR-03 queue and reports success; the loop feeds
    /// the BW-01 response back to the model.
    #[tokio::test]
    async fn run_bridges_browser_tools_through_the_queue() {
        std::env::set_var("TRIOS_BROWSER_BRIDGE_TIMEOUT_SECS", "5");
        use axum::routing::post as axum_post;
        let calls = Arc::new(AtomicUsize::new(0));
        let app_llm = Router::new().route(
            "/v1/chat/completions",
            axum_post(move |Json(_): Json<Value>| {
                let calls = calls.clone();
                async move {
                    if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                        Json(json!({
                            "choices": [{"message": {"role": "assistant", "content": null,
                                "tool_calls": [{"id": "b1", "type": "function",
                                    "function": {"name": "browser_goto",
                                        "arguments": "{\"page\":1,\"url\":\"https://example.com\"}"}}]},
                                "finish_reason": "tool_calls"}]
                        }))
                    } else {
                        Json(json!({
                            "choices": [{"message": {"role": "assistant", "content": "navigated"},
                                "finish_reason": "stop"}]
                        }))
                    }
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app_llm).await.unwrap() });

        let (router, state) = app();

        // Fake host agent: poll the queue, report ok for everything.
        let host_state = state.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(20)).await;
                let mut queue = host_state.browser.queue.lock().await;
                for cmd in queue.poll("host-test") {
                    assert_eq!(cmd.params["url"], "https://example.com");
                    queue.record_result(trios_a2a::BrowserResult::ok(&cmd.id, json!({"ok": true})));
                }
            }
        });

        let (status, body) = post_json(
            &router,
            "/agent/run",
            json!({
                "prompt": "open example.com",
                "provider": {"base_url": format!("http://{addr}/v1"), "model": "mock"},
                "browser_agent_id": "host-test",
                "include_transcript": true
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "body: {body}");
        assert_eq!(body["finalText"], "navigated");
        assert_eq!(body["steps"], 2);
        // The tool message carries the bridged response.
        let tool_msg = body["transcript"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["role"] == "tool")
            .unwrap();
        assert!(tool_msg["content"].as_str().unwrap().contains("ok"));
    }
}
