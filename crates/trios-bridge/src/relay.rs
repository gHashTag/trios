//! HTTP relay with SSE for Comet ↔ agent real-time communication.
//!
//! Endpoints:
//! - `GET /events` — SSE stream of all agent messages (for sidepanel)
//! - `GET /agents` — JSON list of all registered agents
//! - `POST /dispatch` — dispatch a task to an agent
//! - `GET /health` — health check

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::protocol::AgentState;
use crate::server::BridgeServer;

/// Shared state for the relay HTTP server.
#[derive(Clone)]
pub struct RelayState {
    pub server: Arc<BridgeServer>,
}

/// Task dispatch request body.
#[derive(Debug, Deserialize)]
pub struct DispatchRequest {
    /// Agent ID to dispatch to (or "broadcast" for all)
    pub agent_id: String,
    /// Task description
    pub task: String,
    /// Optional issue number
    pub issue: Option<u64>,
    /// Optional branch name
    pub branch: Option<String>,
}

/// Task dispatch response.
#[derive(Debug, Serialize)]
pub struct DispatchResponse {
    pub ok: bool,
    pub message: String,
}

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub agents_count: usize,
}

/// Build the relay HTTP router.
pub fn relay_app(state: RelayState) -> Router {
    Router::new()
        .route("/events", get(sse_events))
        .route("/agents", get(list_agents))
        .route("/dispatch", post(dispatch_task))
        .route("/health", get(health_check))
        .with_state(state)
}

/// Start the relay HTTP server alongside the WebSocket bridge.
pub async fn serve_relay(server: Arc<BridgeServer>, addr: SocketAddr) -> anyhow::Result<()> {
    let state = RelayState {
        server: server.clone(),
    };

    let app = relay_app(state);

    tracing::info!("📡 Relay HTTP+SSE starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("✅ Relay listening on port {}", addr.port());

    axum::serve(listener, app).await?;

    Ok(())
}

/// SSE endpoint — streams all agent messages to connected sidepanels.
///
/// The sidepanel connects with `EventSource`:
/// ```js
/// const es = new EventSource('http://localhost:7475/events');
/// es.onmessage = (e) => console.log(JSON.parse(e.data));
/// ```
async fn sse_events(State(state): State<RelayState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.server.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| {
        match result {
            Ok(msg) => {
                let event = Event::default().data(msg);
                Some(Ok(event))
            }
            Err(_) => None, // Skip lagged messages
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// List all registered agents.
async fn list_agents(State(state): State<RelayState>) -> Json<Vec<AgentState>> {
    let agents = state.server.router().list().await;
    Json(agents)
}

/// Dispatch a task to an agent.
async fn dispatch_task(
    State(state): State<RelayState>,
    Json(req): Json<DispatchRequest>,
) -> Json<DispatchResponse> {
    let router = state.server.router();

    if req.agent_id == "broadcast" {
        // Broadcast to all agents
        let msg = crate::protocol::BridgeMessage::send_command(
            "broadcast".to_string(),
            req.task,
            false,
        );
        match state.server.broadcast_message(&msg) {
            Ok(()) => Json(DispatchResponse {
                ok: true,
                message: "Task broadcast to all agents".to_string(),
            }),
            Err(e) => Json(DispatchResponse {
                ok: false,
                message: format!("Broadcast failed: {}", e),
            }),
        }
    } else {
        // Dispatch to specific agent
        if !router.is_registered(&req.agent_id).await {
            return Json(DispatchResponse {
                ok: false,
                message: format!("Agent '{}' not registered", req.agent_id),
            });
        }

        if let (Some(issue), Some(branch)) = (req.issue, req.branch.clone()) {
            router.claim_issue(&req.agent_id, issue, branch).await;
        }

        router
            .update_status(&req.agent_id, crate::protocol::AgentStatus::Working, req.task.clone())
            .await;

        let msg = crate::protocol::BridgeMessage::send_command(
            req.agent_id.clone(),
            req.task,
            false,
        );
        match state.server.broadcast_message(&msg) {
            Ok(()) => Json(DispatchResponse {
                ok: true,
                message: format!("Task dispatched to agent '{}'", req.agent_id),
            }),
            Err(e) => Json(DispatchResponse {
                ok: false,
                message: format!("Dispatch failed: {}", e),
            }),
        }
    }
}

/// Health check endpoint.
async fn health_check(State(state): State<RelayState>) -> Json<HealthResponse> {
    let agents = state.server.router().list().await;
    Json(HealthResponse {
        status: "ok".to_string(),
        agents_count: agents.len(),
    })
}
