//! `GET /metrics` — Prometheus text exposition (version 0.0.4).
//!
//! Hand-rolled, dependency-free: values come from the SR-03 browser command
//! queue counters (`QueueStats`) plus a couple of cheap server gauges.
//! Wave 11 (loop 7, block B) of the TS-retirement consolidation.

use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use std::fmt::Write as _;

use crate::mcp_endpoints::browser;
use crate::ws_handler::AppState;

/// Prometheus text-format content type.
const PROMETHEUS_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

fn metric(out: &mut String, name: &str, help: &str, kind: &str, value: impl std::fmt::Display) {
    // Errors from write! on String are impossible; ignore via let _.
    let _ = writeln!(out, "# HELP {name} {help}");
    let _ = writeln!(out, "# TYPE {name} {kind}");
    let _ = writeln!(out, "{name} {value}");
}

/// Render the metrics page from live state.
pub async fn render(state: &AppState) -> String {
    let q = browser::queue_stats(state).await;
    let agents = state.agents.lock().await.len();
    let tasks = state.tasks.lock().await.len();

    let mut out = String::with_capacity(1536);
    metric(
        &mut out,
        "trios_browser_queue_depth",
        "Browser commands waiting to be polled (Pending).",
        "gauge",
        q.depth,
    );
    metric(
        &mut out,
        "trios_browser_queue_executing",
        "Browser commands handed to an agent, result not reported yet.",
        "gauge",
        q.executing,
    );
    metric(
        &mut out,
        "trios_browser_queue_capacity",
        "Backpressure cap on Pending browser commands (MAX_PENDING_COMMANDS).",
        "gauge",
        trios_a2a::MAX_PENDING_COMMANDS,
    );
    metric(
        &mut out,
        "trios_browser_commands_enqueued_total",
        "Browser commands accepted into the queue since server start.",
        "counter",
        q.enqueued_total,
    );
    metric(
        &mut out,
        "trios_browser_commands_polled_total",
        "Browser commands handed out to polling agents since server start.",
        "counter",
        q.polled_total,
    );
    metric(
        &mut out,
        "trios_browser_results_total",
        "Browser command results recorded since server start.",
        "counter",
        q.results_total,
    );
    metric(
        &mut out,
        "trios_browser_enqueue_rejected_total",
        "Browser command enqueues refused by the backpressure cap.",
        "counter",
        q.rejected_total,
    );
    metric(
        &mut out,
        "trios_agents_registered",
        "Agents currently registered over the /ws surface.",
        "gauge",
        agents,
    );
    metric(
        &mut out,
        "trios_tasks_tracked",
        "Task entries currently tracked by the server.",
        "gauge",
        tasks,
    );
    out
}

async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    let body = render(&state).await;
    ([(CONTENT_TYPE, PROMETHEUS_CONTENT_TYPE)], body)
}

pub fn router() -> Router<AppState> {
    Router::new().route("/metrics", get(metrics_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::json;
    use tower::ServiceExt;

    async fn get_metrics(router: &Router) -> (StatusCode, String, String) {
        let res = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = res.status();
        let content_type = res
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (status, content_type, String::from_utf8(bytes.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn metrics_empty_state_renders_zeroes() {
        let state = AppState::new();
        let router = router().with_state(state);
        let (status, content_type, body) = get_metrics(&router).await;
        assert_eq!(status, StatusCode::OK);
        assert!(content_type.starts_with("text/plain"));
        assert!(body.contains("# TYPE trios_browser_queue_depth gauge"));
        assert!(body.contains("trios_browser_queue_depth 0"));
        assert!(body.contains("trios_browser_commands_enqueued_total 0"));
        assert!(body.contains(&format!(
            "trios_browser_queue_capacity {}",
            trios_a2a::MAX_PENDING_COMMANDS
        )));
    }

    #[tokio::test]
    async fn metrics_reflect_queue_lifecycle() {
        let state = AppState::new();
        let router = router().with_state(state.clone());

        // Enqueue two commands, poll one agent's worth.
        for _ in 0..2 {
            let r = crate::mcp_endpoints::browser::enqueue_command(
                &state,
                json!({"tool": "browser_get_url", "agent_id": "m-agent"}),
            )
            .await;
            assert_eq!(r["queued"], json!(true));
        }
        let (_, _, body) = get_metrics(&router).await;
        assert!(body.contains("trios_browser_queue_depth 2"));
        assert!(body.contains("trios_browser_commands_enqueued_total 2"));

        let polled = crate::mcp_endpoints::browser::browser_commands(&state, "m-agent").await;
        assert_eq!(polled["commands"].as_array().unwrap().len(), 2);

        let (_, _, body) = get_metrics(&router).await;
        assert!(body.contains("trios_browser_queue_depth 0"));
        assert!(body.contains("trios_browser_queue_executing 2"));
        assert!(body.contains("trios_browser_commands_polled_total 2"));
    }
}
