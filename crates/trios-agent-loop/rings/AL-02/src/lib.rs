//! AL-02 — AgentLoop state machine.
//!
//! Ported from the retired TS agent core (`agent/ai-sdk-agent.ts`, the
//! AI-SDK `ToolLoopAgent`): system prompt + user prompt, then alternate
//! LLM turns and tool executions until the model stops calling tools or
//! the step budget is exhausted (TS parity: `AGENT_LIMITS.MAX_TURNS`).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc::UnboundedSender;
use trios_agent_loop_al00::{AssistantTurn, ChatMessage, ChatRequest, LlmClient};
use trios_agent_loop_al01::ToolRegistry;

/// TS parity: `AGENT_LIMITS.MAX_TURNS = 100`.
pub const MAX_TURNS_DEFAULT: usize = 100;
/// Tool results longer than this are truncated before entering the transcript.
pub const MAX_TOOL_RESULT_BYTES_DEFAULT: usize = 32 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLoopConfig {
    pub system_prompt: String,
    pub max_steps: usize,
    pub max_tool_result_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub temperature: Option<f64>,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            system_prompt: "You are a helpful agent. Use the available tools when needed."
                .into(),
            max_steps: MAX_TURNS_DEFAULT,
            max_tool_result_bytes: MAX_TOOL_RESULT_BYTES_DEFAULT,
            temperature: None,
        }
    }
}

/// Why the loop stopped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The model produced a final answer (no tool calls).
    Completed,
    /// The step budget was exhausted.
    MaxSteps,
}

/// Streamed loop progress (SSE-friendly).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepEvent {
    AssistantText { step: usize, text: String },
    ToolCallStarted { step: usize, id: String, name: String, arguments: Value },
    ToolResult { step: usize, id: String, name: String, result: Value, is_error: bool },
    Done { steps: usize, stop_reason: StopReason },
}

/// Final result of a loop run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunResult {
    pub final_text: Option<String>,
    pub steps: usize,
    pub stop_reason: StopReason,
    pub transcript: Vec<ChatMessage>,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

pub struct AgentLoop<'a> {
    client: &'a dyn LlmClient,
    registry: &'a ToolRegistry,
    config: AgentLoopConfig,
    model: String,
}

impl<'a> AgentLoop<'a> {
    pub fn new(
        client: &'a dyn LlmClient,
        registry: &'a ToolRegistry,
        model: impl Into<String>,
        config: AgentLoopConfig,
    ) -> Self {
        Self { client, registry, config, model: model.into() }
    }

    /// Run the loop for one user prompt. Events are emitted to `events`
    /// (if provided) as the loop progresses.
    pub async fn run(
        &self,
        user_prompt: &str,
        events: Option<UnboundedSender<StepEvent>>,
    ) -> Result<AgentRunResult> {
        let emit = |event: StepEvent| {
            if let Some(tx) = &events {
                let _ = tx.send(event);
            }
        };

        let tool_defs: Vec<Value> = self.registry.defs().iter().map(|d| d.to_openai()).collect();
        let mut messages = vec![
            ChatMessage::system(&self.config.system_prompt),
            ChatMessage::user(user_prompt),
        ];
        let mut steps = 0usize;
        let mut prompt_tokens = 0u64;
        let mut completion_tokens = 0u64;

        loop {
            steps += 1;
            let request = ChatRequest {
                model: self.model.clone(),
                messages: messages.clone(),
                tools: tool_defs.clone(),
                temperature: self.config.temperature,
            };
            let turn: AssistantTurn = self.client.chat(&request).await?;
            prompt_tokens += turn.prompt_tokens.unwrap_or(0);
            completion_tokens += turn.completion_tokens.unwrap_or(0);

            if let Some(text) = &turn.content {
                if !text.is_empty() {
                    emit(StepEvent::AssistantText { step: steps, text: text.clone() });
                }
            }

            if turn.tool_calls.is_empty() {
                // Final answer.
                let final_text = turn.content.clone();
                messages.push(ChatMessage::assistant(final_text.clone().unwrap_or_default()));
                emit(StepEvent::Done { steps, stop_reason: StopReason::Completed });
                return Ok(AgentRunResult {
                    final_text,
                    steps,
                    stop_reason: StopReason::Completed,
                    transcript: messages,
                    prompt_tokens,
                    completion_tokens,
                });
            }

            // Echo the assistant tool-call message back verbatim, then run tools.
            messages.push(ChatMessage::assistant_tool_calls(
                turn.content.clone(),
                turn.raw_tool_calls.clone(),
            ));

            for call in &turn.tool_calls {
                emit(StepEvent::ToolCallStarted {
                    step: steps,
                    id: call.id.clone(),
                    name: call.name.clone(),
                    arguments: call.arguments.clone(),
                });
                let (result, is_error) =
                    match self.registry.execute(&call.name, call.arguments.clone()).await {
                        Ok(value) => (value, false),
                        Err(err) => (json!({ "error": err.to_string() }), true),
                    };
                emit(StepEvent::ToolResult {
                    step: steps,
                    id: call.id.clone(),
                    name: call.name.clone(),
                    result: result.clone(),
                    is_error,
                });
                let mut rendered = result.to_string();
                if rendered.len() > self.config.max_tool_result_bytes {
                    rendered.truncate(self.config.max_tool_result_bytes);
                    rendered.push_str("… [truncated]");
                }
                messages.push(ChatMessage::tool_result(call.id.clone(), rendered));
            }

            if steps >= self.config.max_steps {
                tracing::warn!(steps, "agent loop hit max_steps");
                emit(StepEvent::Done { steps, stop_reason: StopReason::MaxSteps });
                return Ok(AgentRunResult {
                    final_text: None,
                    steps,
                    stop_reason: StopReason::MaxSteps,
                    transcript: messages,
                    prompt_tokens,
                    completion_tokens,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use trios_agent_loop_al01::register_builtin_tools;

    /// Scripted LLM: returns canned turns in order.
    struct ScriptedLlm {
        turns: Mutex<Vec<AssistantTurn>>,
        requests: Mutex<Vec<ChatRequest>>,
    }

    impl ScriptedLlm {
        fn new(turns: Vec<AssistantTurn>) -> Self {
            Self { turns: Mutex::new(turns), requests: Mutex::new(vec![]) }
        }
    }

    #[async_trait::async_trait]
    impl LlmClient for ScriptedLlm {
        async fn chat(&self, request: &ChatRequest) -> Result<AssistantTurn> {
            self.requests.lock().unwrap().push(request.clone());
            let mut turns = self.turns.lock().unwrap();
            if turns.is_empty() {
                anyhow::bail!("script exhausted");
            }
            Ok(turns.remove(0))
        }
    }

    fn text_turn(text: &str) -> AssistantTurn {
        AssistantTurn {
            content: Some(text.into()),
            tool_calls: vec![],
            finish_reason: Some("stop".into()),
            raw_tool_calls: vec![],
            prompt_tokens: Some(7),
            completion_tokens: Some(3),
        }
    }

    fn echo_call_turn(id: &str, text: &str) -> AssistantTurn {
        let raw = json!({
            "id": id,
            "type": "function",
            "function": {"name": "echo", "arguments": format!("{{\"text\":\"{text}\"}}")}
        });
        AssistantTurn {
            content: None,
            tool_calls: vec![trios_agent_loop_al00::ToolCallRequest {
                id: id.into(),
                name: "echo".into(),
                arguments: json!({"text": text}),
            }],
            finish_reason: Some("tool_calls".into()),
            raw_tool_calls: vec![raw],
            prompt_tokens: None,
            completion_tokens: None,
        }
    }

    #[tokio::test]
    async fn completes_after_tool_round_trip() {
        let llm = ScriptedLlm::new(vec![echo_call_turn("c1", "ping"), text_turn("pong")]);
        let mut reg = ToolRegistry::new();
        register_builtin_tools(&mut reg);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let agent = AgentLoop::new(&llm, &reg, "test-model", AgentLoopConfig::default());

        let result = agent.run("please echo ping", Some(tx)).await.unwrap();
        assert_eq!(result.stop_reason, StopReason::Completed);
        assert_eq!(result.final_text.as_deref(), Some("pong"));
        assert_eq!(result.steps, 2);
        assert_eq!(result.prompt_tokens, 7);

        // Transcript: system, user, assistant(tool_calls), tool, assistant(final)
        let roles: Vec<_> = result.transcript.iter().map(|m| m.role.as_str()).collect();
        assert_eq!(roles, vec!["system", "user", "assistant", "tool", "assistant"]);
        assert_eq!(result.transcript[3].tool_call_id.as_deref(), Some("c1"));

        // Events: ToolCallStarted, ToolResult, AssistantText, Done
        let mut kinds = vec![];
        while let Ok(e) = rx.try_recv() {
            kinds.push(match e {
                StepEvent::AssistantText { .. } => "text",
                StepEvent::ToolCallStarted { .. } => "call",
                StepEvent::ToolResult { is_error, .. } => {
                    assert!(!is_error);
                    "result"
                }
                StepEvent::Done { .. } => "done",
            });
        }
        assert_eq!(kinds, vec!["call", "result", "text", "done"]);
    }

    #[tokio::test]
    async fn second_request_carries_tool_transcript() {
        let llm = ScriptedLlm::new(vec![echo_call_turn("c1", "hi"), text_turn("done")]);
        let mut reg = ToolRegistry::new();
        register_builtin_tools(&mut reg);
        let agent = AgentLoop::new(&llm, &reg, "m", AgentLoopConfig::default());
        agent.run("go", None).await.unwrap();

        let requests = llm.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        // Both requests advertise the tools.
        assert!(!requests[0].tools.is_empty());
        // Second request contains the assistant tool-call echo and the tool result.
        let roles: Vec<_> = requests[1].messages.iter().map(|m| m.role.as_str()).collect();
        assert_eq!(roles, vec!["system", "user", "assistant", "tool"]);
        assert!(requests[1].messages[3].content.as_ref().unwrap().contains("hi"));
    }

    #[tokio::test]
    async fn stops_at_max_steps() {
        let llm = ScriptedLlm::new(vec![
            echo_call_turn("c1", "a"),
            echo_call_turn("c2", "b"),
            echo_call_turn("c3", "c"),
        ]);
        let mut reg = ToolRegistry::new();
        register_builtin_tools(&mut reg);
        let config = AgentLoopConfig { max_steps: 2, ..Default::default() };
        let agent = AgentLoop::new(&llm, &reg, "m", config);

        let result = agent.run("loop forever", None).await.unwrap();
        assert_eq!(result.stop_reason, StopReason::MaxSteps);
        assert_eq!(result.steps, 2);
        assert!(result.final_text.is_none());
    }

    #[tokio::test]
    async fn tool_errors_are_reported_not_fatal() {
        // echo without `text` fails; the loop must surface the error to the
        // model and continue.
        let raw = json!({"id": "c1", "function": {"name": "echo", "arguments": "{}"}});
        let bad_call = AssistantTurn {
            content: None,
            tool_calls: vec![trios_agent_loop_al00::ToolCallRequest {
                id: "c1".into(),
                name: "echo".into(),
                arguments: json!({}),
            }],
            finish_reason: Some("tool_calls".into()),
            raw_tool_calls: vec![raw],
            prompt_tokens: None,
            completion_tokens: None,
        };
        let llm = ScriptedLlm::new(vec![bad_call, text_turn("recovered")]);
        let mut reg = ToolRegistry::new();
        register_builtin_tools(&mut reg);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let agent = AgentLoop::new(&llm, &reg, "m", AgentLoopConfig::default());

        let result = agent.run("go", Some(tx)).await.unwrap();
        assert_eq!(result.stop_reason, StopReason::Completed);
        assert_eq!(result.final_text.as_deref(), Some("recovered"));

        let mut saw_error_result = false;
        while let Ok(e) = rx.try_recv() {
            if let StepEvent::ToolResult { is_error, result, .. } = e {
                saw_error_result = true;
                assert!(is_error);
                assert!(result["error"].as_str().unwrap().contains("text"));
            }
        }
        assert!(saw_error_result);
        // The tool message fed back to the model carries the error JSON.
        let tool_msg = result.transcript.iter().find(|m| m.role == "tool").unwrap();
        assert!(tool_msg.content.as_ref().unwrap().contains("error"));
    }

    #[tokio::test]
    async fn long_tool_results_are_truncated() {
        let text = "x".repeat(64);
        let llm = ScriptedLlm::new(vec![echo_call_turn("c1", &text), text_turn("ok")]);
        let mut reg = ToolRegistry::new();
        register_builtin_tools(&mut reg);
        let config = AgentLoopConfig { max_tool_result_bytes: 16, ..Default::default() };
        let agent = AgentLoop::new(&llm, &reg, "m", config);

        let result = agent.run("go", None).await.unwrap();
        let tool_msg = result.transcript.iter().find(|m| m.role == "tool").unwrap();
        let content = tool_msg.content.as_ref().unwrap();
        assert!(content.contains("[truncated]"));
        assert!(content.len() < 64);
    }
}
