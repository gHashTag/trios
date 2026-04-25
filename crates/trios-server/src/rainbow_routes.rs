// Rainbow Bridge route mount — re-export of `trios-rainbow-bridge` (L13 / INV-8).
//
// ONE SHOT: https://github.com/gHashTag/trios/issues/267 §6 deliverable 7.
// Coq source: trinity-clara/proofs/igla/rainbow_bridge_consistency.v
// Anchor: φ² + φ⁻² = 3 (Zenodo DOI 10.5281/zenodo.19227877).
//
// Honesty (R5):
//   - `/rainbow/status` is fully wired and returns the L-R14 anchors served
//     by the in-process bridge crate.
//   - `/rainbow/ws`, `/rainbow/sse`, `/rainbow/publish` are stubbed at HTTP
//     501 Not Implemented with a typed JSON body that references the ONE
//     SHOT. The live tailnet transport is a follow-up substrate landing —
//     the crate's pure state machine is already covered by the 23 tests in
//     `trios-rainbow-bridge` (CI gate `rainbow-bridge.yml`). Stubbing here
//     preserves R6 (no rewrite of `ws_handler.rs`) and R11 (conservative
//     reading: do not invent infrastructure outside the lane file map).
//
// Why a separate module: ONE SHOT §3.3 explicitly marks
// `crates/trios-server/src/ws_handler.rs` as read-only for L13. Mounting a
// new sibling module satisfies the "APPEND only" rule.

use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use serde_json::{json, Value};
use trios_rainbow_bridge as rainbow;

/// Build the `/rainbow/*` sub-router. Mount with `app.merge(rainbow_routes())`.
pub fn rainbow_routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/rainbow/status", get(rainbow_status))
        .route("/rainbow/ws", get(rainbow_ws_stub))
        .route("/rainbow/sse", get(rainbow_sse_stub))
        .route("/rainbow/publish", post(rainbow_publish_stub))
}

/// `GET /rainbow/status` — L-R14 numeric anchors from `trios-rainbow-bridge`.
///
/// The values are imported as Rust constants from the crate, so they are
/// pinned to `assertions/igla_assertions.json::INV-8.numeric_anchor` and to
/// `rainbow_bridge_consistency.v` by the build-time `anchors_match_coq`
/// test in the bridge crate.
async fn rainbow_status() -> Json<Value> {
    Json(json!({
        "service": "rainbow-bridge",
        "lane": "L13",
        "invariant": "INV-8",
        "trinity_identity": "phi^2 + phi^-2 = 3",
        "anchors": {
            "latency_p95_ms": rainbow::LATENCY_P95_MS,
            "heartbeat_max_s": rainbow::HEARTBEAT_MAX_S,
            "channel_count": rainbow::CHANNEL_COUNT,
            "layer_count": rainbow::LAYER_COUNT,
        },
        "doi": "10.5281/zenodo.19227877",
        "one_shot": "https://github.com/gHashTag/trios/issues/267",
        "coq_source": "trinity-clara/proofs/igla/rainbow_bridge_consistency.v",
        "rust_source": "crates/trios-rainbow-bridge/",
    }))
}

fn not_implemented(channel: &str) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error": "not_implemented",
            "channel": channel,
            "reason": "Live tailnet transport is a follow-up; in-process bridge is operational via the trios-rainbow-bridge crate API.",
            "one_shot": "https://github.com/gHashTag/trios/issues/267",
            "lane": "L13",
            "invariant": "INV-8",
        })),
    )
}

async fn rainbow_ws_stub() -> (StatusCode, Json<Value>) {
    not_implemented("ws")
}

async fn rainbow_sse_stub() -> (StatusCode, Json<Value>) {
    not_implemented("sse")
}

async fn rainbow_publish_stub() -> (StatusCode, Json<Value>) {
    not_implemented("publish")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchors_re_exported() {
        // Mirror: rainbow_bridge_consistency.v (numeric anchors).
        assert_eq!(rainbow::LATENCY_P95_MS, 2000);
        assert_eq!(rainbow::HEARTBEAT_MAX_S, 14_400);
        assert_eq!(rainbow::CHANNEL_COUNT, 7);
        assert_eq!(rainbow::LAYER_COUNT, 3);
    }
}
