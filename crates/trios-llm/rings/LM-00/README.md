# LM-00 — LLM Client Interface

## Purpose
Abstract LLM client trait + HTTP client implementation.

## API
- `LlmClient` trait: `complete(prompt) -> Result<String>`
- `LlmConfig` — model, endpoint, api_key
- `AnthropicClient` — Claude API implementation
