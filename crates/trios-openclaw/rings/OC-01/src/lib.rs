//! OC-01 — Hermes provider mapping.
//!
//! Ported 1:1 from `lib/agents/hermes/hermes-provider-map.ts`. Pure lookup
//! mapping a BrowserOS provider type to the Hermes provider name, its API-key
//! env var, whether a base URL is required, and an optional default base URL.
//! No I/O. Depends on nothing (leaf ring).

use serde::{Deserialize, Serialize};

/// A Hermes provider mapping (TS `HermesProviderMapping`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HermesProviderMapping {
    pub hermes_provider: String,
    pub env_var_name: String,
    pub requires_base_url: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub default_base_url: Option<String>,
}

impl HermesProviderMapping {
    fn new(provider: &str, env: &str, requires_base_url: bool, default_base_url: Option<&str>) -> Self {
        Self {
            hermes_provider: provider.into(),
            env_var_name: env.into(),
            requires_base_url,
            default_base_url: default_base_url.map(|s| s.into()),
        }
    }
}

/// Supported BrowserOS provider types (TS
/// `HERMES_SUPPORTED_BROWSEROS_PROVIDER_TYPES`).
pub const SUPPORTED_PROVIDER_TYPES: [&str; 4] =
    ["anthropic", "openai", "openrouter", "openai-compatible"];

pub fn is_supported(provider_type: &str) -> bool {
    SUPPORTED_PROVIDER_TYPES.contains(&provider_type)
}

/// Look up the Hermes mapping for a BrowserOS provider type.
///
/// Note (from upstream): Hermes v2026.4.x has no `"openai"` provider key —
/// OpenAI-compatible endpoints use `provider: custom` + `base_url`. So both
/// `openai` and `openai-compatible` map to the `custom` Hermes provider.
pub fn get_mapping(provider_type: &str) -> Option<HermesProviderMapping> {
    match provider_type {
        "anthropic" => Some(HermesProviderMapping::new("anthropic", "ANTHROPIC_API_KEY", false, None)),
        "openai" => Some(HermesProviderMapping::new(
            "custom",
            "OPENAI_API_KEY",
            false,
            Some("https://api.openai.com/v1"),
        )),
        "openrouter" => Some(HermesProviderMapping::new("openrouter", "OPENROUTER_API_KEY", false, None)),
        "openai-compatible" => {
            Some(HermesProviderMapping::new("custom", "OPENAI_API_KEY", true, None))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_set() {
        assert!(is_supported("anthropic"));
        assert!(is_supported("openai-compatible"));
        assert!(!is_supported("cohere"));
    }

    #[test]
    fn anthropic_mapping() {
        let m = get_mapping("anthropic").unwrap();
        assert_eq!(m.hermes_provider, "anthropic");
        assert_eq!(m.env_var_name, "ANTHROPIC_API_KEY");
        assert!(!m.requires_base_url);
        assert!(m.default_base_url.is_none());
    }

    #[test]
    fn openai_maps_to_custom_with_default_url() {
        let m = get_mapping("openai").unwrap();
        assert_eq!(m.hermes_provider, "custom");
        assert_eq!(m.default_base_url.as_deref(), Some("https://api.openai.com/v1"));
    }

    #[test]
    fn openai_compatible_requires_base_url() {
        let m = get_mapping("openai-compatible").unwrap();
        assert_eq!(m.hermes_provider, "custom");
        assert!(m.requires_base_url);
        assert!(m.default_base_url.is_none());
    }

    #[test]
    fn unknown_returns_none() {
        assert!(get_mapping("cohere").is_none());
    }
}
