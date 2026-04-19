# trios-agents

HTTP client + MCP proxy for [zig-agents](https://github.com/gHashTag/zig-agents) — AI agent orchestration.

## Usage

```rust
use trios_agents::AgentsClient;

let client = AgentsClient::localhost();
let agent = client.spawn("researcher", "Analyze GF16 compression").await?;
let status = client.status(&agent.id).await?;
client.send_message(&agent.id, "Focus on weight distributions").await?;
```
