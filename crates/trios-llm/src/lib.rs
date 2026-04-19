//! trios-llm — LLM inference bridge for TRIOS
//!
//! R0: Stub implementation (echo/identity)
//! R1: OpenAI/Azure API client (planned)
//! R2: Local model via trinity-brain (planned)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LLM error types
#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Token limit exceeded")]
    TokenLimit,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
        }
    }
}

/// LLM generation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOptions {
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: f32,
    pub stop: Vec<String>,
}

impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 1024,
            top_p: 1.0,
            stop: Vec::new(),
        }
    }
}

/// LLM generation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponse {
    pub content: String,
    pub tokens_used: u32,
    pub finish_reason: String,
    pub model: String,
}

/// LLM trait for pluggable backends
#[async_trait::async_trait]
pub trait LLMBackend: Send + Sync {
    async fn generate(
        &self,
        messages: &[ChatMessage],
        options: &GenerationOptions,
    ) -> Result<GenerationResponse, LLMError>;

    async fn generate_stream(
        &self,
        messages: &[ChatMessage],
        options: &GenerationOptions,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String, LLMError>> + Send>, LLMError> {
        // Default: not implemented
        Err(LLMError::InvalidRequest("Streaming not supported".to_string()))
    }
}

/// Echo backend (R0 stub)
pub struct EchoBackend {
    pub model_name: String,
}

impl EchoBackend {
    pub fn new(model_name: &str) -> Self {
        Self {
            model_name: model_name.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl LLMBackend for EchoBackend {
    async fn generate(
        &self,
        messages: &[ChatMessage],
        _options: &GenerationOptions,
    ) -> Result<GenerationResponse, LLMError> {
        // Echo the last user message
        let last_user_msg = messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.clone())
            .unwrap_or_else(|| "No message".to_string());

        let content = format!("[ECHO] {}", last_user_msg);

        Ok(GenerationResponse {
            content,
            tokens_used: last_user_msg.len() as u32,
            finish_reason: "stop".to_string(),
            model: self.model_name.clone(),
        })
    }
}

/// LLM client (main API)
pub struct LLMClient {
    backend: Box<dyn LLMBackend>,
}

impl LLMClient {
    pub fn new(backend: Box<dyn LLMBackend>) -> Self {
        Self { backend }
    }

    pub fn with_echo(model_name: &str) -> Self {
        Self::new(Box::new(EchoBackend::new(model_name)))
    }

    pub async fn generate(
        &self,
        messages: &[ChatMessage],
        options: Option<GenerationOptions>,
    ) -> Result<GenerationResponse, LLMError> {
        let opts = options.unwrap_or_default();
        self.backend.generate(messages, &opts).await
    }

    pub async fn chat(&self, prompt: &str) -> Result<String, LLMError> {
        let messages = vec![ChatMessage::user(prompt)];
        let response = self.generate(&messages, None).await?;
        Ok(response.content)
    }
}

/// Available models registry
pub struct ModelRegistry {
    models: HashMap<String, Box<dyn LLMBackend>>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, backend: Box<dyn LLMBackend>) {
        self.models.insert(name.to_string(), backend);
    }

    pub fn get(&self, name: &str) -> Option<&dyn LLMBackend> {
        self.models.get(name).map(|b| b.as_ref())
    }

    pub fn create_client(&self, name: &str) -> Result<LLMClient, LLMError> {
        let backend = self
            .get(name)
            .ok_or_else(|| LLMError::ModelNotFound(name.to_string()))?;

        // Clone the backend - for R0 this is simple
        // For R1+ we'd need a backend factory pattern
        Ok(LLMClient::with_echo(name))
    }
}

/// Global registry instance
lazy_static::lazy_static! {
    pub static ref REGISTRY: std::sync::Mutex<ModelRegistry> =
        std::sync::Mutex::new(ModelRegistry::new());
}

/// Initialize default models
pub fn init() {
    let mut registry = REGISTRY.lock().unwrap();
    registry.register("echo", Box::new(EchoBackend::new("echo")));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo_backend() {
        let backend = EchoBackend::new("echo");

        let messages = vec![ChatMessage::user("Hello, world!")];
        let response = backend.generate(&messages, &GenerationOptions::default())
            .await
            .unwrap();

        assert_eq!(response.content, "[ECHO] Hello, world!");
        assert_eq!(response.model, "echo");
        assert_eq!(response.finish_reason, "stop");
    }

    #[tokio::test]
    async fn test_llm_client() {
        let client = LLMClient::with_echo("echo");

        let messages = vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("Test message"),
        ];

        let response = client.generate(&messages, None).await.unwrap();
        assert!(response.content.contains("Test message"));
    }

    #[tokio::test]
    async fn test_chat() {
        let client = LLMClient::with_echo("echo");
        let result = client.chat("What is 2+2?").await.unwrap();
        assert_eq!(result, "[ECHO] What is 2+2?");
    }

    #[test]
    fn test_chat_message_constructors() {
        let system = ChatMessage::system("System prompt");
        assert_eq!(system.role, "system");

        let user = ChatMessage::user("User prompt");
        assert_eq!(user.role, "user");

        let assistant = ChatMessage::assistant("Assistant response");
        assert_eq!(assistant.role, "assistant");
    }

    #[test]
    fn test_generation_options_default() {
        let opts = GenerationOptions::default();
        assert_eq!(opts.temperature, 0.7);
        assert_eq!(opts.max_tokens, 1024);
        assert_eq!(opts.top_p, 1.0);
        assert!(opts.stop.is_empty());
    }

    #[tokio::test]
    async fn test_registry() {
        let mut registry = ModelRegistry::new();
        registry.register("test", Box::new(EchoBackend::new("test")));

        assert!(registry.get("test").is_some());
        assert!(registry.get("nonexistent").is_none());
    }
}
