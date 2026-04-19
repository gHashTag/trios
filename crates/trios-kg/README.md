# trios-kg

HTTP client for [zig-knowledge-graph](https://github.com/gHashTag/zig-knowledge-graph) REST API.

## Usage

```rust
use trios_kg::KgClient;

let client = KgClient::localhost();

// Create an entity
let entity = client.create_entity("concept", "GF16", serde_json::json!({})).await?;

// Search
let results = client.search(&QueryParams {
    query: "golden ratio".into(),
    limit: 10,
    ..Default::default()
}).await?;
```
