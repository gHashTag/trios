//! HTTP client for the zig-knowledge-graph REST API.

use crate::types::*;
use anyhow::Result;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::time::Duration;
use tracing::{debug, instrument};

/// Default base URL for the Knowledge Graph server.
pub const DEFAULT_KG_URL: &str = "http://localhost:8080";

/// REST client for the Knowledge Graph API.
#[derive(Debug, Clone)]
pub struct KgClient {
    base_url: String,
    client: Client,
}

impl KgClient {
    /// Create a new KG client pointing at the given base URL.
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Create a client with default URL (`http://localhost:8080`).
    pub fn localhost() -> Self {
        Self::new(DEFAULT_KG_URL)
    }

    // ── Entity operations ──────────────────────────────────────

    /// Create a new entity in the knowledge graph.
    #[instrument(skip(self), fields(base_url = %self.base_url))]
    pub async fn create_entity(
        &self,
        entity_type: &str,
        name: &str,
        properties: serde_json::Value,
    ) -> Result<Entity> {
        let body = CreateEntityRequest {
            entity_type: entity_type.to_string(),
            name: name.to_string(),
            properties,
        };
        debug!("Creating entity: {:?}", body);
        self.post("/api/entities", &body).await
    }

    /// Get an entity by ID.
    #[instrument(skip(self))]
    pub async fn get_entity(&self, id: &str) -> Result<Entity> {
        self.get(&format!("/api/entities/{id}")).await
    }

    /// List all entities, optionally filtered by type.
    #[instrument(skip(self))]
    pub async fn list_entities(&self, entity_type: Option<&str>) -> Result<Vec<Entity>> {
        let mut url = "/api/entities".to_string();
        if let Some(t) = entity_type {
            url = format!("/api/entities?type={t}");
        }
        self.get(&url).await
    }

    /// Delete an entity by ID.
    #[instrument(skip(self))]
    pub async fn delete_entity(&self, id: &str) -> Result<()> {
        let url = format!("/api/entities/{id}");
        self.client
            .delete(format!("{}{url}", self.base_url))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    // ── Edge operations ────────────────────────────────────────

    /// Create a new edge (relationship) between two entities.
    #[instrument(skip(self))]
    pub async fn create_edge(
        &self,
        source: &str,
        target: &str,
        edge_type: &str,
        weight: Option<f64>,
    ) -> Result<Edge> {
        let body = CreateEdgeRequest {
            source: source.to_string(),
            target: target.to_string(),
            edge_type: edge_type.to_string(),
            weight,
            properties: serde_json::Value::Null,
        };
        self.post("/api/edges", &body).await
    }

    /// Get all edges connected to an entity.
    #[instrument(skip(self))]
    pub async fn get_entity_edges(&self, entity_id: &str) -> Result<Vec<Edge>> {
        self.get(&format!("/api/entities/{entity_id}/edges")).await
    }

    // ── Search ─────────────────────────────────────────────────

    /// Search the knowledge graph.
    #[instrument(skip(self))]
    pub async fn search(&self, params: &QueryParams) -> Result<SearchResult> {
        self.post("/api/search", params).await
    }

    /// Semantic search using vector similarity.
    #[instrument(skip(self))]
    pub async fn semantic_search(&self, query: &str, limit: usize) -> Result<SearchResult> {
        let params = QueryParams {
            query: query.to_string(),
            limit,
            offset: 0,
            entity_type: None,
            min_weight: None,
        };
        self.post("/api/search/semantic", &params).await
    }

    // ── Graph traversal ────────────────────────────────────────

    /// Get neighbors of an entity within `depth` hops.
    #[instrument(skip(self))]
    pub async fn traverse(&self, entity_id: &str, depth: usize) -> Result<SearchResult> {
        self.get(&format!("/api/entities/{entity_id}/traverse?depth={depth}"))
            .await
    }

    /// Find shortest path between two entities.
    #[instrument(skip(self))]
    pub async fn shortest_path(&self, from: &str, to: &str) -> Result<Vec<Edge>> {
        self.get(&format!("/api/path?from={from}&to={to}")).await
    }

    // ── Health ─────────────────────────────────────────────────

    /// Check if the KG server is healthy.
    pub async fn health(&self) -> Result<bool> {
        let resp = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await;
        match resp {
            Ok(r) => Ok(r.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    // ── Internal helpers ───────────────────────────────────────

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        debug!("GET {}{}", self.base_url, path);
        let resp = self
            .client
            .get(format!("{}{path}", self.base_url))
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("KG API error {status}: {text}");
        }
        Ok(resp.json().await?)
    }

    async fn post<T: Serialize, R: DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
        debug!("POST {}{}", self.base_url, path);
        let resp = self
            .client
            .post(format!("{}{path}", self.base_url))
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("KG API error {status}: {text}");
        }
        Ok(resp.json().await?)
    }
}

use serde::Serialize;
