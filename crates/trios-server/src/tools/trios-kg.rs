//! MCP tools for trios-kg crate
//!
//! Exposes Knowledge Graph operations through Model Context Protocol.
//!
//! ## Tools
//!
//! - `kg_create_entity`: Create an entity (node) in knowledge graph
//! - `kg_create_edge`: Create a relationship (edge) between two entities
//! - `kg_query`: Query knowledge graph with filters
//! - `kg_traverse`: Traverse graph relationships
//!
//! ## Example
//!
//! ```json
//! {
//!   "name": "kg_create_entity",
//!   "arguments": {
//!     "entity_type": "concept",
//!     "name": "GF16",
//!     "properties": {"key": "value"}
//!   }
//! }
//! ```

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use trios_kg::{Edge, Entity, KgClient, KgError, QueryParams, SearchResult};

/// Create an entity (node) in knowledge graph.
pub async fn kg_create_entity(input: &Value) -> Result<Value> {
    let entity_type = input
        .get("entity_type")
        .and_then(|v| v.as_str())
        .context("entity_type is required")?;

    let name = input
        .get("name")
        .and_then(|v| v.as_str())
        .context("name is required")?;

    let properties = input
        .get("properties")
        .and_then(|v| match v {
            Value::Object(props) => props.clone(),
            _ => json!({}),
        })
        .unwrap_or_else(|| json!({}));

    let entity = Entity {
        id: format!("entity-{}", uuid::Uuid::new_v4()),
        entity_type: entity_type.into(),
        name: name.into(),
        properties,
    };

    Ok(json!({
        "id": entity.id,
        "entity_type": entity.entity_type,
        "name": entity.name,
        "properties": entity.properties,
    }))
}

/// Create a relationship (edge) between two entities.
pub async fn kg_create_edge(input: &Value) -> Result<Value> {
    let source = input
        .get("source")
        .and_then(|v| v.as_str())
        .context("source is required")?;

    let target = input
        .get("target")
        .and_then(|v| v.as_str())
        .context("target is required")?;

    let edge_type = input
        .get("edge_type")
        .and_then(|v| v.as_str())
        .context("edge_type is required")?;

    let weight = input.get("weight").and_then(|v| match v {
        Value::Number(n) if n.as_f64().is_ok() => n.as_f64(),
        _ => None,
    });

    let edge = Edge {
        id: format!("edge-{}", uuid::Uuid::new_v4()),
        source: source.into(),
        target: target.into(),
        edge_type: edge_type.into(),
        weight,
    };

    Ok(json!({
        "id": edge.id,
        "source": edge.source,
        "target": edge.target,
        "edge_type": edge.edge_type,
        "weight": edge.weight,
    }))
}

/// Query knowledge graph with optional filters.
pub async fn kg_query(input: &Value) -> Result<Value> {
    let query = input
        .get("query")
        .and_then(|v| v.as_str())
        .context("query is required")?;

    let limit = input
        .get("limit")
        .and_then(|v| match v {
            Value::Number(n) if n.as_u64().is_ok() => n.as_u64(),
            _ => None,
        });

    let entity_type = match input.get("entity_type") {
        Some(s) => Some(s.clone()),
        _ => None,
    };

    // In a real implementation, this would query KG server
    // For now, we return a mock response
    let results = vec![
        Entity {
            id: format!("entity-{}", uuid::Uuid::new_v4()),
            entity_type: "concept".into(),
            name: "Trinity Concept".into(),
            properties: json!({}),
        },
    ];

    Ok(json!({
        "entities": results.entities,
        "edges": vec![],
        "total": results.total,
    }))
}

/// Traverse graph relationships starting from an entity.
pub async fn kg_traverse(input: &Value) -> Result<Value> {
    let source = input
        .get("source")
        .and_then(|v| v.as_str())
        .context("source is required")?;

    let max_depth = input
        .get("max_depth")
        .and_then(|v| match v {
            Value::Number(n) if n.as_u64().is_ok() => n.as_u64(),
            _ => bail!("max_depth must be a number"),
        });

    // In a real implementation, this would traverse KG graph
    // For now, we return a mock response
    let results = vec![
        json!({
            "source": source,
            "target": format!("node-{}", uuid::Uuid::new_v4()),
            "edge_type": "related_to",
        }),
    ];

    Ok(json!({
        "paths": results,
        "total": results.len(),
    }))
}
