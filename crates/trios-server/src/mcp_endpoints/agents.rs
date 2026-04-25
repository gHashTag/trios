use crate::ws_handler::AppState;
use serde_json::{json, Value};
use tracing::{info, warn, error};

pub async fn list(state: &AppState) -> Value {
    let agents = state.agents.lock().await;
    serde_json::to_value(&*agents).unwrap_or(json!([]))
}

pub async fn chat(state: &AppState, params: Option<Value>) -> Value {
    let params = params.unwrap_or(json!({}));
    let agent_id = params.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
    let message = params.get("message").and_then(|v| v.as_str()).unwrap_or("");

    // No agent specified — use z.ai as default provider
    if agent_id.is_empty() {
        return call_zai(state, message).await;
    }

    let agents = state.agents.lock().await;
    let exists = agents.iter().any(|a| a.id == agent_id);
    drop(agents);

    if !exists {
        return json!({"error": format!("agent '{}' not found. Register an agent first.", agent_id)});
    }

    json!({"response": format!("[{}] {}", agent_id, message)})
}

/// Call z.ai API using Anthropic Messages format
async fn call_zai(state: &AppState, message: &str) -> Value {
    if state.zai_api.is_empty() || state.zai_keys.is_empty() {
        return json!({"response": format!("[echo] {}", message)});
    }

    // Round-robin key selection (use first key for now)
    let key = &state.zai_keys[0];

    let body = json!({
        "model": "claude-3-7-sonnet-20250219",
        "max_tokens": 1024,
        "messages": [
            {"role": "user", "content": message}
        ]
    });

    info!("[z.ai] POST {} (msg={} bytes)", state.zai_api, message.len());

    let response = state.http_client
        .post(&state.zai_api)
        .header("x-api-key", key.as_str())
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            match resp.text().await {
                Ok(text) => {
                    if !status.is_success() {
                        error!("[z.ai] HTTP {} — {}", status, &text[..text.len().min(200)]);
                        return json!({"error": format!("z.ai HTTP {}: {}", status, &text[..text.len().min(100)])});
                    }
                    // Parse Anthropic Messages response
                    if let Ok(val) = serde_json::from_str::<Value>(&text) {
                        if let Some(content) = val.get("content").and_then(|c| c.as_array()) {
                            let text_parts: Vec<&str> = content.iter()
                                .filter_map(|block| {
                                    if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                                        block.get("text").and_then(|t| t.as_str())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            let reply = text_parts.join("\n");
                            info!("[z.ai] response: {} bytes", reply.len());
                            return json!({"response": reply});
                        }
                    }
                    // Fallback: return raw text
                    warn!("[z.ai] unexpected response format, returning raw");
                    json!({"response": text})
                }
                Err(e) => {
                    error!("[z.ai] failed to read response body: {}", e);
                    json!({"error": format!("z.ai read error: {}", e)})
                }
            }
        }
        Err(e) => {
            error!("[z.ai] request failed: {}", e);
            json!({"error": format!("z.ai request failed: {}", e)})
        }
    }
}
