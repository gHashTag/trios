//! Types for the Knowledge Graph API.

use serde::{Deserialize, Serialize};

/// Error type for KG operations.
#[derive(Debug, thiserror::Error)]
pub enum KgError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {status} - {message}")]
    Api { status: u16, message: String },
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// An entity (node) in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique entity ID.
    pub id: String,
    /// Entity type (e.g., "concept", "person", "technology").
    #[serde(rename = "type")]
    pub entity_type: String,
    /// Entity name/label.
    pub name: String,
    /// Optional properties as JSON.
    #[serde(default)]
    pub properties: serde_json::Value,
    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
}

/// An edge (relationship) between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Unique edge ID.
    pub id: String,
    /// Source entity ID.
    pub source: String,
    /// Target entity ID.
    pub target: String,
    /// Relationship type (e.g., "depends_on", "related_to").
    #[serde(rename = "type")]
    pub edge_type: String,
    /// Optional weight for the relationship.
    #[serde(default)]
    pub weight: Option<f64>,
    /// Optional properties as JSON.
    #[serde(default)]
    pub properties: serde_json::Value,
}

/// Query parameters for graph searches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParams {
    /// Search query string.
    pub query: String,
    /// Maximum number of results.
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Offset for pagination.
    #[serde(default)]
    pub offset: usize,
    /// Optional entity type filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    /// Minimum relationship weight filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_weight: Option<f64>,
}

fn default_limit() -> usize {
    50
}

/// Search result from the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Matching entities.
    pub entities: Vec<Entity>,
    /// Matching edges.
    pub edges: Vec<Edge>,
    /// Total number of matches (for pagination).
    pub total: usize,
}

/// Request body for creating an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntityRequest {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub name: String,
    #[serde(default)]
    pub properties: serde_json::Value,
}

/// Request body for creating an edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEdgeRequest {
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub edge_type: String,
    #[serde(default)]
    pub weight: Option<f64>,
    #[serde(default)]
    pub properties: serde_json::Value,
}
