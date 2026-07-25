//! Scripted OpenAI-compatible fake LLM for the CI browser e2e (L1: Rust-only
//! tooling, no shell/python helpers).
//!
//! Serves `/v1/chat/completions` and walks a fixed script regardless of the
//! request body:
//!   1. `browser_goto` → `TRIOS_E2E_TARGET_URL` (default: the local
//!      trios-server `/health` page)
//!   2. `browser_content` → page 1 (bridged to `browser_get_dom` via SR-03)
//!   3. final assistant message `E2E_OK`
//!
//! Env:
//! - `TRIOS_FAKE_LLM_PORT`   — listen port (default 9301)
//! - `TRIOS_E2E_TARGET_URL`  — URL for step 1 (default `http://127.0.0.1:9207/health`)

use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let port: u16 = std::env::var("TRIOS_FAKE_LLM_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9301);
    let target = std::env::var("TRIOS_E2E_TARGET_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9207/health".into());

    let calls = Arc::new(AtomicUsize::new(0));
    let app = Router::new().route(
        "/v1/chat/completions",
        post(move |Json(_): Json<Value>| {
            let calls = calls.clone();
            let target = target.clone();
            async move {
                let n = calls.fetch_add(1, Ordering::SeqCst);
                let body = match n {
                    0 => json!({
                        "choices": [{"message": {"role": "assistant", "content": null,
                            "tool_calls": [{"id": "e2e-goto", "type": "function",
                                "function": {"name": "browser_goto",
                                    "arguments": json!({"page": 1, "url": target}).to_string()}}]},
                            "finish_reason": "tool_calls"}]
                    }),
                    1 => json!({
                        "choices": [{"message": {"role": "assistant", "content": null,
                            "tool_calls": [{"id": "e2e-content", "type": "function",
                                "function": {"name": "browser_content",
                                    "arguments": "{\"page\":1}"}}]},
                            "finish_reason": "tool_calls"}]
                    }),
                    _ => json!({
                        "choices": [{"message": {"role": "assistant", "content": "E2E_OK"},
                            "finish_reason": "stop"}]
                    }),
                };
                Json(body)
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    eprintln!("trios-e2e-llm listening on 127.0.0.1:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
