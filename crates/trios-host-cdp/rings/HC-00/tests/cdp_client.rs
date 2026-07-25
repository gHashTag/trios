//! HC-00 integration: mock CDP WS server + mock DevTools HTTP endpoint.

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;
use trios_host_cdp_hc00::{discover_page_ws, CdpClient};

/// Mock CDP target: replies to every call; emits a noise event before each
/// response to prove event skipping works.
async fn spawn_mock_cdp() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                while let Some(Ok(Message::Text(text))) = ws.next().await {
                    let req: Value = serde_json::from_str(&text).unwrap();
                    let id = req["id"].as_u64().unwrap();
                    let method = req["method"].as_str().unwrap().to_string();
                    // Noise: a CDP event without an id.
                    ws.send(Message::Text(
                        json!({"method": "Page.frameNavigated", "params": {}}).to_string(),
                    ))
                    .await
                    .unwrap();
                    let response = match method.as_str() {
                        "Page.navigate" => json!({"id": id, "result": {"frameId": "F1"}}),
                        "Runtime.evaluate" => json!({"id": id, "result":
                            {"result": {"type": "string", "value": "mock-title"}}}),
                        "Bad.method" => json!({"id": id, "error": {"message": "no such method"}}),
                        _ => json!({"id": id, "result": {}}),
                    };
                    ws.send(Message::Text(response.to_string())).await.unwrap();
                }
            });
        }
    });
    format!("ws://{addr}")
}

#[tokio::test]
async fn call_correlates_ids_and_skips_events() {
    let ws_url = spawn_mock_cdp().await;
    let client = CdpClient::connect(&ws_url).await.unwrap();

    let nav = client
        .call("Page.navigate", json!({"url": "https://example.com"}))
        .await
        .unwrap();
    assert_eq!(nav["frameId"], "F1");

    let eval = client
        .call("Runtime.evaluate", json!({"expression": "document.title"}))
        .await
        .unwrap();
    assert_eq!(eval["result"]["value"], "mock-title");
}

#[tokio::test]
async fn call_surfaces_cdp_errors() {
    let ws_url = spawn_mock_cdp().await;
    let client = CdpClient::connect(&ws_url).await.unwrap();
    let err = client.call("Bad.method", json!({})).await.unwrap_err();
    assert!(err.to_string().contains("no such method"), "{err}");
}

#[tokio::test]
async fn concurrent_calls_do_not_cross_wires() {
    let ws_url = spawn_mock_cdp().await;
    let client = CdpClient::connect(&ws_url).await.unwrap();
    let (a, b) = tokio::join!(
        client.call("Page.navigate", json!({"url": "https://a"})),
        client.call("Runtime.evaluate", json!({"expression": "1"})),
    );
    assert_eq!(a.unwrap()["frameId"], "F1");
    assert_eq!(b.unwrap()["result"]["value"], "mock-title");
}

#[tokio::test]
async fn discovery_picks_the_page_target() {
    use axum::routing::get;
    let ws_url = "ws://127.0.0.1:1/devtools/page/AAAA";
    let payload = json!([
        {"type": "iframe", "webSocketDebuggerUrl": "ws://127.0.0.1:1/devtools/iframe/X"},
        {"type": "page", "webSocketDebuggerUrl": ws_url},
    ]);
    let app = axum::Router::new().route(
        "/json/list",
        get(move || {
            let payload = payload.clone();
            async move { axum::Json(payload) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let found = discover_page_ws(&format!("http://{addr}")).await.unwrap();
    assert_eq!(found, ws_url);
}
