//! HC-02 integration: fake trios-server WS implementing browser/poll and
//! browser/result, fake executor. Verifies the full poll → execute → report
//! cycle including event skipping.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;
use trios_a2a::BrowserCommand;
use trios_host_cdp_hc02::{run, CommandExecutor, PollerConfig};

struct FakeExecutor {
    seen: Arc<Mutex<Vec<String>>>,
    fail: bool,
}

#[async_trait::async_trait]
impl CommandExecutor for FakeExecutor {
    async fn execute(&self, command: &BrowserCommand) -> anyhow::Result<Value> {
        self.seen.lock().unwrap().push(command.id.clone());
        if self.fail {
            anyhow::bail!("boom");
        }
        Ok(json!({"ok": true, "echo": command.params}))
    }
}

/// Fake server: first poll returns one navigate command (preceded by a
/// broadcast event frame), later polls return nothing. Reports are recorded.
async fn spawn_fake_server(reports: Arc<Mutex<Vec<Value>>>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let reports = reports.clone();
            tokio::spawn(async move {
                let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                let mut poll_count = 0usize;
                while let Some(Ok(Message::Text(text))) = ws.next().await {
                    let req: Value = serde_json::from_str(&text).unwrap();
                    match req["method"].as_str().unwrap() {
                        "browser/poll" => {
                            assert_eq!(req["params"]["agent_id"], "host-test");
                            // Broadcast noise the poller must skip.
                            ws.send(Message::Text(
                                json!({"event": {"type": "agent_connected"}}).to_string(),
                            ))
                            .await
                            .unwrap();
                            let commands = if poll_count == 0 {
                                let cmd = BrowserCommand::from_tool_name(
                                    "browser_navigate",
                                    "host-test",
                                    json!({"url": "https://example.com"}),
                                )
                                .unwrap();
                                json!([cmd])
                            } else {
                                json!([])
                            };
                            poll_count += 1;
                            ws.send(Message::Text(
                                json!({"result": {"commands": commands}}).to_string(),
                            ))
                            .await
                            .unwrap();
                        }
                        "browser/result" => {
                            reports.lock().unwrap().push(req["params"].clone());
                            ws.send(Message::Text(
                                json!({"result": {"ok": true}}).to_string(),
                            ))
                            .await
                            .unwrap();
                        }
                        other => panic!("unexpected method {other}"),
                    }
                }
            });
        }
    });
    format!("ws://{addr}")
}

fn config(server_ws: String, max_polls: usize) -> PollerConfig {
    PollerConfig {
        server_ws,
        agent_id: "host-test".into(),
        poll_interval: Duration::from_millis(10),
        max_polls: Some(max_polls),
        reconnect_backoff: Duration::from_millis(50),
    }
}

#[tokio::test]
async fn poll_execute_report_round_trip() {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let server = spawn_fake_server(reports.clone()).await;
    let seen = Arc::new(Mutex::new(Vec::new()));
    let executor = FakeExecutor { seen: seen.clone(), fail: false };

    tokio::time::timeout(Duration::from_secs(10), run(config(server, 3), &executor))
        .await
        .expect("poller should finish max_polls rounds")
        .unwrap();

    assert_eq!(seen.lock().unwrap().len(), 1, "one command executed");
    let reports = reports.lock().unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0]["success"], true);
    assert_eq!(reports[0]["agent_id"], "host-test");
    assert_eq!(reports[0]["result"]["echo"]["url"], "https://example.com");
    assert!(reports[0]["command_id"].as_str().unwrap().len() > 10);
}

#[tokio::test]
async fn executor_failure_is_reported_not_fatal() {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let server = spawn_fake_server(reports.clone()).await;
    let executor = FakeExecutor { seen: Arc::new(Mutex::new(Vec::new())), fail: true };

    tokio::time::timeout(Duration::from_secs(10), run(config(server, 2), &executor))
        .await
        .expect("loop must survive executor errors")
        .unwrap();

    let reports = reports.lock().unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0]["success"], false);
    assert_eq!(reports[0]["error"], "boom");
}
