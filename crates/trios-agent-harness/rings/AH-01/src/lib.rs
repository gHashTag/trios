//! AH-01 — agent adapter catalog.
//!
//! Ported from `lib/agents/agent-catalog.ts`. Descriptor types + the catalog
//! of adapters with their default model / reasoning effort and available
//! options. Depends only on AH-00.
//!
//! The concrete model lists are volatile data; the catalog here carries the
//! adapter defaults and `model_control`, and the full option lists can be
//! overridden at runtime from config without touching this ring's logic.

use serde::{Deserialize, Serialize};
use trios_agent_harness_ah00::AgentAdapter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelControl {
    RuntimeSupported,
    BestEffort,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogOption {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub recommended: Option<bool>,
}

impl CatalogOption {
    pub fn new(id: &str, label: &str) -> Self {
        Self { id: id.into(), label: label.into(), recommended: None }
    }
    pub fn recommended(id: &str, label: &str) -> Self {
        Self { id: id.into(), label: label.into(), recommended: Some(true) }
    }
}

/// Adapter descriptor (TS `AgentAdapterDescriptor`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterDescriptor {
    pub id: AgentAdapter,
    pub name: String,
    pub default_model_id: String,
    pub default_reasoning_effort: String,
    pub model_control: ModelControl,
    pub models: Vec<CatalogOption>,
    pub reasoning_efforts: Vec<CatalogOption>,
}

fn efforts_low_to_max(recommended_medium: bool) -> Vec<CatalogOption> {
    vec![
        CatalogOption::new("low", "Low"),
        if recommended_medium {
            CatalogOption::recommended("medium", "Medium")
        } else {
            CatalogOption::new("medium", "Medium")
        },
        CatalogOption::new("high", "High"),
        CatalogOption::new("xhigh", "Extra high"),
        CatalogOption::new("max", "Max"),
    ]
}

/// The adapter catalog (ported from AGENT_ADAPTER_CATALOG).
/// Model lists are trimmed to the recommended/default entries; the full list
/// is runtime-overridable. Defaults + control flags are authoritative here.
pub fn catalog() -> Vec<AdapterDescriptor> {
    vec![
        AdapterDescriptor {
            id: AgentAdapter::Claude,
            name: "Claude Code".into(),
            default_model_id: "haiku".into(),
            default_reasoning_effort: "medium".into(),
            model_control: ModelControl::BestEffort,
            models: vec![
                CatalogOption::new("opus", "Opus (latest)"),
                CatalogOption::new("sonnet", "Sonnet (latest)"),
                CatalogOption::recommended("haiku", "Haiku (latest)"),
            ],
            reasoning_efforts: efforts_low_to_max(true),
        },
        AdapterDescriptor {
            id: AgentAdapter::Codex,
            name: "Codex".into(),
            default_model_id: "gpt-5.5".into(),
            default_reasoning_effort: "medium".into(),
            model_control: ModelControl::BestEffort,
            models: vec![CatalogOption::recommended("gpt-5.5", "GPT-5.5")],
            reasoning_efforts: efforts_low_to_max(true),
        },
        AdapterDescriptor {
            id: AgentAdapter::Openclaw,
            name: "OpenClaw".into(),
            default_model_id: "default".into(),
            default_reasoning_effort: "medium".into(),
            model_control: ModelControl::BestEffort,
            models: vec![],
            reasoning_efforts: efforts_low_to_max(true),
        },
        AdapterDescriptor {
            id: AgentAdapter::Hermes,
            name: "Hermes".into(),
            default_model_id: "default".into(),
            default_reasoning_effort: "medium".into(),
            model_control: ModelControl::BestEffort,
            models: vec![],
            reasoning_efforts: efforts_low_to_max(true),
        },
    ]
}

/// Look up a descriptor by adapter.
pub fn descriptor_for(adapter: AgentAdapter) -> Option<AdapterDescriptor> {
    catalog().into_iter().find(|d| d.id == adapter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_all_adapters() {
        let c = catalog();
        assert_eq!(c.len(), 4);
        for a in [AgentAdapter::Claude, AgentAdapter::Codex, AgentAdapter::Openclaw, AgentAdapter::Hermes] {
            assert!(descriptor_for(a).is_some(), "missing {a:?}");
        }
    }

    #[test]
    fn defaults_match_ts_catalog() {
        let claude = descriptor_for(AgentAdapter::Claude).unwrap();
        assert_eq!(claude.default_model_id, "haiku");
        let codex = descriptor_for(AgentAdapter::Codex).unwrap();
        assert_eq!(codex.default_model_id, "gpt-5.5");
    }

    #[test]
    fn descriptor_serializes_camelcase() {
        let d = descriptor_for(AgentAdapter::Claude).unwrap();
        let v = serde_json::to_value(&d).unwrap();
        assert!(v.get("defaultModelId").is_some());
        assert!(v.get("modelControl").is_some());
        assert_eq!(v["modelControl"], serde_json::json!("best-effort"));
    }
}
