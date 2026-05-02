//! API — public HTTP API surface.
//!
//! Provides a REST API for external systems to query
//! and control IGLA RACE.

use std::collections::HashMap;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::sync::RwLock;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::info;

/// API server.
pub struct ApiServer {
    bind_addr: String,
    port: u16,
}

/// Shared API state.
#[derive(Debug, Default)]
pub struct ApiState {
    best_bpb: f64,
    total_trials: u64,
    active_workers: u32,
    last_update: Option<chrono::DateTime<chrono::Utc>>,
}

use chrono::{DateTime, Utc};

/// API configuration.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub bind_addr: String,
    pub port: u16,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1".into(),
            port: 8080,
        }
    }
}

/// Health check response.
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

/// Race status response.
#[derive(Debug, Serialize)]
struct RaceStatusResponse {
    best_bpb: f64,
    total_trials: u64,
    active_workers: u32,
    best_trial_id: u64,
}

/// Start race request.
#[derive(Debug, Deserialize)]
struct StartRaceRequest {
    workers: u32,
    trials_per_worker: u32,
}

/// Start race response.
#[derive(Debug, Serialize)]
struct StartRaceResponse {
    race_id: String,
    status: String,
}

impl ApiServer {
    /// Create a new API server.
    pub fn new(config: ApiConfig) -> Self {
        Self {
            bind_addr: config.bind_addr,
            port: config.port,
        }
    }

    /// Run the API server.
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let state = Arc::new(RwLock::new(ApiState::default()));
        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/api/status", get(status_handler))
            .route("/api/race/start", post(start_race_handler))
            .with_state(state);

        let addr = format!("{}:{}", self.bind_addr, self.port);
        info!("API server listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let socket_addr: SocketAddr = listener.local_addr()?;
        info!("Server listening on {}", socket_addr);

        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// Health check handler.
async fn health_handler() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".into(),
        version: "0.1.0".into(),
    })
}

/// Status handler.
async fn status_handler(
    State(state): State<Arc<RwLock<ApiState>>>,
) -> impl IntoResponse {
    let state = state.read().await;

    Json(RaceStatusResponse {
        best_bpb: state.best_bpb,
        total_trials: state.total_trials,
        active_workers: state.active_workers,
        best_trial_id: 0, // TODO: track
    })
}

/// Start race handler.
async fn start_race_handler(
    State(state): State<Arc<RwLock<ApiState>>>,
    Json(req): Json<StartRaceRequest>,
) -> impl IntoResponse {
    info!("Start race request: {} workers, {} trials per worker",
            req.workers, req.trials_per_worker);

    let race_id = uuid::Uuid::new_v4().to_string();

    // Update state
    let mut state = state.write().await;
    state.active_workers = req.workers;
    state.last_update = Some(chrono::Utc::now());

    Json(StartRaceResponse {
        race_id,
        status: "started".into(),
    })
}
