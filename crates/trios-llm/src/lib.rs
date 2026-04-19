//! # trios-llm
//!
//! LLm Inference bridge connecting TRIOS to Claude via MCP.
//!
//! Provides a structured interface for LLM inference requests,
//! handling model routing, context management, and response formatting.
//!
//! ## Example
//!
//! ```ignore
//! use trios_llm::{InferenceRequest, InferenceResponse};
//!
//! let request = InferenceRequest {
//!     model: "claude-opus-4",
//!     prompt: "Explain quantum computing in simple terms",
//!     max_tokens: 500,
//! };
//! let response = trios_llm::infer(request)?;
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Model identifier for LLM inference.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Model {
    ClaudeOpus4,
    ClaudeSonnet4,
    ClaudeHaiku4,
    ClaudeOpus3,
    ClaudeSonnet3,
}

/// Inference request payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub model: Model,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
}

/// Inference response payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub text: String,
    pub model: String,
    pub tokens_used: usize,
    pub finish_reason: Option<String>,
}

/// Perform LLM inference via Claude MCP bridge.
pub fn infer(request: InferenceRequest) -> Result<InferenceResponse> {
    // TODO: Implement actual MCP bridge to Claude
    // For now, return stub response
    Ok(InferenceResponse {
        text: format!("TODO: Implement inference for model: {:?}", request.model),
        model: format!("{:?}", request.model),
        tokens_used: 0,
        finish_reason: Some("not_implemented".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_request() {
        let request = InferenceRequest {
            model: Model::ClaudeOpus4,
            prompt: "test".to_string(),
            max_tokens: Some(100),
            temperature: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test"));
    }

    #[test]
    fn test_stub_inference() {
        let request = InferenceRequest {
            model: Model::ClaudeSonnet4,
            prompt: "hello".to_string(),
            max_tokens: None,
            temperature: None,
        };
        let response = infer(request).unwrap();
        assert!(response.finish_reason.as_ref().unwrap() == "not_implemented");
    }
}
