//! # trios-kg
//!
//! HTTP client for the [zig-knowledge-graph](https://github.com/gHashTag/zig-knowledge-graph) REST API.
//!
//! Provides typed access to knowledge graph operations: entity CRUD,
//! relationship queries, semantic search, and graph traversal.
//!
//! ## Example
//!
//! ```ignore
//! use trios_kg::KgClient;
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = KgClient::new("http://localhost:8080");
//!     let entity = client.create_entity("concept", "GF16").await.unwrap();
//!     println!("Created entity: {:?}", entity);
//! }
//! ```

mod client;
mod types;

pub use client::KgClient;
pub use types::{Edge, Entity, KgError, QueryParams, SearchResult};
