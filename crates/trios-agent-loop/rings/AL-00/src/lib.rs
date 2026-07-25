//! AL-00 — OpenAI-compatible chat contract + HTTP LLM client.
//!
//! Ported from the retired TS agent core (`agent/provider-factory.ts` +
//! the AI-SDK chat transport). The wire format is the OpenAI
//! `/chat/completions` contract, which every provider the TS server
//! supported (openai, openrouter, ollama, lmstudio, zai) speaks.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// A tool definition advertised to the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    /// JSON Schema of the tool arguments.
    pub parameters: Value,
}

impl ToolDef {
    /// OpenAI wire shape: `{"type":"function","function":{...}}`.
    pub fn to_openai(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            }
        })
    }
}

/// A tool call requested by the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    /// Parsed arguments (the wire carries them as a JSON string).
    pub arguments: Value,
}

/// One message of the conversation transcript (OpenAI wire shape).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_calls: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: "system".into(), content: Some(content.into()), tool_calls: None, tool_call_id: None }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: "user".into(), content: Some(content.into()), tool_calls: None, tool_call_id: None }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: "assistant".into(), content: Some(content.into()), tool_calls: None, tool_call_id: None }
    }
    /// Assistant message that carries tool calls (echoed back verbatim).
    pub fn assistant_tool_calls(content: Option<String>, tool_calls: Vec<Value>) -> Self {
        Self { role: "assistant".into(), content, tool_calls: Some(tool_calls), tool_call_id: None }
    }
    /// Tool result message referencing the call id.
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self { role: "tool".into(), content: Some(content.into()), tool_calls: None, tool_call_id: Some(tool_call_id.into()) }
    }
}

/// A chat request to an OpenAI-compatible endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tools: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
}

/// The distilled assistant turn returned by an LLM client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantTurn {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCallRequest>,
    pub finish_reason: Option<String>,
    /// Raw `tool_calls` wire objects — echoed back into the transcript so
    /// providers see exactly what they emitted.
    pub raw_tool_calls: Vec<Value>,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
}

/// Parse an OpenAI `/chat/completions` response body into an [`AssistantTurn`].
pub fn parse_chat_response(body: &Value) -> Result<AssistantTurn> {
    let choice = body
        .get("choices")
        .and_then(|c| c.get(0))
        .ok_or_else(|| anyhow!("LLM response has no choices: {body}"))?;
    let message = choice
        .get("message")
        .ok_or_else(|| anyhow!("LLM choice has no message"))?;
    let content = message.get("content").and_then(Value::as_str).map(String::from);
    let raw_tool_calls: Vec<Value> = message
        .get("tool_calls")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut tool_calls = Vec::with_capacity(raw_tool_calls.len());
    for tc in &raw_tool_calls {
        let id = tc.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
        let function = tc.get("function").ok_or_else(|| anyhow!("tool call has no function"))?;
        let name = function
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("tool call has no name"))?
            .to_string();
        let arguments = match function.get("arguments") {
            // The wire carries arguments as a JSON string.
            Some(Value::String(s)) if !s.trim().is_empty() => {
                serde_json::from_str(s).with_context(|| format!("tool `{name}`: bad arguments JSON: {s}"))?
            }
            Some(Value::Object(o)) => Value::Object(o.clone()),
            _ => json!({}),
        };
        tool_calls.push(ToolCallRequest { id, name, arguments });
    }
    let finish_reason = choice.get("finish_reason").and_then(Value::as_str).map(String::from);
    let usage = body.get("usage");
    let prompt_tokens = usage.and_then(|u| u.get("prompt_tokens")).and_then(Value::as_u64);
    let completion_tokens = usage.and_then(|u| u.get("completion_tokens")).and_then(Value::as_u64);
    Ok(AssistantTurn { content, tool_calls, finish_reason, raw_tool_calls, prompt_tokens, completion_tokens })
}

/// An LLM chat client. Implemented by [`HttpLlmClient`] in production and by
/// scripted mocks in tests.
#[async_trait::async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, request: &ChatRequest) -> Result<AssistantTurn>;
}

/// Provider configuration (mirrors the TS `ResolvedAgentConfig` provider part).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Base URL of the OpenAI-compatible API, e.g. `http://127.0.0.1:11434/v1`.
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub api_key: Option<String>,
    pub model: String,
}

impl LlmConfig {
    /// Read the provider config from the environment.
    ///
    /// - `TRIOS_LLM_BASE_URL` (default `http://127.0.0.1:11434/v1`, ollama)
    /// - `TRIOS_LLM_API_KEY`  (optional)
    /// - `TRIOS_LLM_MODEL`    (default `llama3.2`)
    pub fn from_env() -> Self {
        Self {
            base_url: std::env::var("TRIOS_LLM_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".into()),
            api_key: std::env::var("TRIOS_LLM_API_KEY").ok().filter(|s| !s.is_empty()),
            model: std::env::var("TRIOS_LLM_MODEL").unwrap_or_else(|_| "llama3.2".into()),
        }
    }
}

/// Production client for any OpenAI-compatible `/chat/completions` endpoint.
pub struct HttpLlmClient {
    config: LlmConfig,
    http: reqwest::Client,
}

impl HttpLlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self { config, http: reqwest::Client::new() }
    }

    pub fn model(&self) -> &str {
        &self.config.model
    }

    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.config.base_url.trim_end_matches('/'))
    }
}

#[async_trait::async_trait]
impl LlmClient for HttpLlmClient {
    async fn chat(&self, request: &ChatRequest) -> Result<AssistantTurn> {
        let mut req = self.http.post(self.endpoint()).json(request);
        if let Some(key) = &self.config.api_key {
            req = req.bearer_auth(key);
        }
        let res = req.send().await.context("LLM request failed")?;
        let status = res.status();
        let body: Value = res.json().await.context("LLM response is not JSON")?;
        if !status.is_success() {
            return Err(anyhow!("LLM returned {status}: {body}"));
        }
        parse_chat_response(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_def_serializes_to_openai_shape() {
        let def = ToolDef {
            name: "echo".into(),
            description: "Echo the input".into(),
            parameters: json!({"type":"object","properties":{"text":{"type":"string"}},"required":["text"]}),
        };
        let wire = def.to_openai();
        assert_eq!(wire["type"], "function");
        assert_eq!(wire["function"]["name"], "echo");
        assert_eq!(wire["function"]["parameters"]["required"][0], "text");
    }

    #[test]
    fn parses_final_text_response() {
        let body = json!({
            "choices": [{"message": {"role": "assistant", "content": "done"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 2}
        });
        let turn = parse_chat_response(&body).unwrap();
        assert_eq!(turn.content.as_deref(), Some("done"));
        assert!(turn.tool_calls.is_empty());
        assert_eq!(turn.finish_reason.as_deref(), Some("stop"));
        assert_eq!(turn.prompt_tokens, Some(10));
    }

    #[test]
    fn parses_tool_calls_with_string_arguments() {
        let body = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {"name": "echo", "arguments": "{\"text\":\"hi\"}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });
        let turn = parse_chat_response(&body).unwrap();
        assert_eq!(turn.tool_calls.len(), 1);
        assert_eq!(turn.tool_calls[0].name, "echo");
        assert_eq!(turn.tool_calls[0].arguments["text"], "hi");
        assert_eq!(turn.raw_tool_calls.len(), 1);
    }

    #[test]
    fn rejects_bad_arguments_json() {
        let body = json!({
            "choices": [{"message": {"tool_calls": [{"id": "c", "function": {"name": "echo", "arguments": "{oops"}}]}}]
        });
        assert!(parse_chat_response(&body).is_err());
    }

    #[test]
    fn empty_and_object_arguments_are_accepted() {
        let body = json!({
            "choices": [{"message": {"tool_calls": [
                {"id": "a", "function": {"name": "t1", "arguments": ""}},
                {"id": "b", "function": {"name": "t2", "arguments": {"x": 1}}}
            ]}}]
        });
        let turn = parse_chat_response(&body).unwrap();
        assert_eq!(turn.tool_calls[0].arguments, json!({}));
        assert_eq!(turn.tool_calls[1].arguments["x"], 1);
    }

    #[test]
    fn llm_config_from_env_defaults() {
        // Не трогаем реальные env в других тестах: проверяем только дефолты,
        // когда переменные не заданы.
        std::env::remove_var("TRIOS_LLM_BASE_URL");
        std::env::remove_var("TRIOS_LLM_API_KEY");
        std::env::remove_var("TRIOS_LLM_MODEL");
        let cfg = LlmConfig::from_env();
        assert_eq!(cfg.base_url, "http://127.0.0.1:11434/v1");
        assert_eq!(cfg.model, "llama3.2");
        assert!(cfg.api_key.is_none());
    }
}
