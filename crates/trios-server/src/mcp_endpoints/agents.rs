use crate::ws_handler::AppState;
use serde_json::{json, Value};
use tracing::{info, warn, error};

const SYSTEM_PROMPT: &str = "You are Trinity — an AI agent orchestrator built on the trios stack. \
You help developers manage agents, tasks, and tools. \
Be concise, technical, and precise. \
When unsure, ask for clarification rather than guessing.";

const MAX_RETRIES: usize = 3;

pub async fn list(state: &AppState) -> Value {
    let agents = state.agents.lock().await;
    serde_json::to_value(&*agents).unwrap_or(json!([]))
}

pub async fn chat(state: &AppState, params: Option<Value>) -> Value {
    let params = params.unwrap_or(json!({}));
    let agent_id = params.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
    let message = params.get("message").and_then(|v| v.as_str()).unwrap_or("");

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

/// Call z.ai API with round-robin key rotation and retry on 429/5xx
async fn call_zai(state: &AppState, message: &str) -> Value {
    if state.zai_api.is_empty() || state.zai_keys.is_empty() {
        return json!({"response": format!("[echo] {}", message)});
    }

    let body = json!({
        "model": "claude-3-7-sonnet-20250219",
        "max_tokens": 1024,
        "system": SYSTEM_PROMPT,
        "messages": [
            {"role": "user", "content": message}
        ]
    });

    let n_keys = state.zai_keys.len();

    for attempt in 0..MAX_RETRIES {
        // Round-robin: each retry picks the next key
        let idx = state.zai_key_idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % n_keys;
        let key = &state.zai_keys[idx];

        info!("[z.ai] POST {} attempt={} key_idx={} msg={} bytes",
            state.zai_api, attempt, idx, message.len());

        let resp = state.http_client
            .post(&state.zai_api)
            .header("x-api-key", key.as_str())
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                error!("[z.ai] request failed attempt={}: {}", attempt, e);
                if attempt + 1 == MAX_RETRIES {
                    return json!({"error": format!("z.ai request failed: {}", e)});
                }
                continue;
            }
        };

        let status = resp.status();

        // Retry on rate-limit or server errors
        if status.as_u16() == 429 || status.is_server_error() {
            warn!("[z.ai] HTTP {} attempt={} — retrying with next key", status, attempt);
            if attempt + 1 == MAX_RETRIES {
                return json!({"error": format!("z.ai HTTP {} after {} retries", status, MAX_RETRIES)});
            }
            continue;
        }

        let text = match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                error!("[z.ai] failed to read body attempt={}: {}", attempt, e);
                return json!({"error": format!("z.ai read error: {}", e)});
            }
        };

        if !status.is_success() {
            error!("[z.ai] HTTP {} — {}", status, &text[..text.len().min(200)]);
            return json!({"error": format!("z.ai HTTP {}: {}", status, &text[..text.len().min(100)])});
        }

        if let Ok(val) = serde_json::from_str::<Value>(&text) {
            if let Some(content) = val.get("content").and_then(|c| c.as_array()) {
                let reply: String = content.iter()
                    .filter_map(|block| {
                        if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                            block.get("text").and_then(|t| t.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                info!("[z.ai] response: {} bytes (attempt={})", reply.len(), attempt);
                return json!({"response": reply});
            }
        }

        warn!("[z.ai] unexpected response format, returning raw");
        return json!({"response": text});
    }

    json!({"error": "z.ai: all retries exhausted"})
}
