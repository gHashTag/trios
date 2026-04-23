//! SSE (Server-Sent Events) transport for MCP protocol
//! Compatible with Claude Desktop, Cursor, VSCode MCP clients
//!
//! Protocol:
//!   GET /sse          — open SSE stream, receive session endpoint
//!   POST /sse/message — send MCP JSON-RPC messages

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use serde_json::{json, Value};
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _;
use tracing::info;
use uuid::Uuid;

use crate::ws_handler::AppState;

/// GET /sse — open SSE stream
/// On connect, sends `endpoint` event with the POST URL for this session
pub async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let session_id = Uuid::new_v4().to_string();
    info!("SSE client connected, session={}", session_id);

    let rx = state.event_tx.subscribe();
    let post_url = format!("/sse/message?session={}", session_id);

    // First event: endpoint advertisement (MCP spec)
    let endpoint_event = Event::default()
        .event("endpoint")
        .data(post_url.clone());

    // Convert broadcast channel to SSE stream
    let event_stream = BroadcastStream::new(rx).filter_map(move |msg| {
        match msg {
            Ok(event) => {
                let data = serde_json::to_string(&json!({"event": event}))
                    .unwrap_or_default();
                Some(Ok(Event::default().event("message").data(data)))
            }
            Err(_) => None,
        }
    });

    // Prepend endpoint event to the stream
    let stream = tokio_stream::once(Ok(endpoint_event)).chain(event_stream);

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// POST /sse/message — receive MCP JSON-RPC from client
/// Routes to same handle_message() as WebSocket
pub async fn sse_message(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    info!("SSE message received: {:?}", body.get("method"));
    let text = serde_json::to_string(&body).unwrap_or_default();
    let response = crate::ws_handler::handle_message(&text, &state).await;
    Json(json!({
        "jsonrpc": "2.0",
        "id": body.get("id").cloned().unwrap_or(json!(null)),
        "result": response.result,
    }))
}
