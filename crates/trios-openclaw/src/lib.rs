//! trios-openclaw — OpenClaw gateway contracts.
//!
//! Re-export facade only (L-ARCH-001). Ported from the TS backend
//! `lib/agents/openclaw/*` and `lib/agents/hermes/*` during Wave 3.
//!
//! The heavy VM / container execution (lima-cli, container-cli,
//! managed-container) drives host processes and stays in a host-runtime layer
//! next to the machine. These rings port the pure logic that was most prone
//! to silent breakage:
//! - OC-00 `gateway` — GatewayConfig + resolve_acp_command (argv builder)
//! - OC-01 `hermes`  — Hermes provider mapping (pure lookup)

pub use trios_openclaw_oc00 as gateway;
pub use trios_openclaw_oc01 as hermes;

pub use trios_openclaw_oc00::{
    bridge_session_key, resolve_acp_command, GatewayConfig, OPENCLAW_GATEWAY_CONTAINER_PORT,
};
pub use trios_openclaw_oc01::{
    get_mapping, is_supported, HermesProviderMapping, SUPPORTED_PROVIDER_TYPES,
};
