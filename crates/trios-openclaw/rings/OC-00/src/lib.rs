//! OC-00 — OpenClaw gateway config + ACP command builder.
//!
//! Ported 1:1 from `lib/agents/openclaw/acp-command.ts`. The heavy VM /
//! container execution (lima-cli, container-cli, managed-container) drives
//! host processes and stays in a host-runtime layer next to the machine;
//! this ring ports the *pure* pieces:
//!   - the gateway accessor contract (`GatewayConfig`)
//!   - `resolve_acp_command` — deterministic argv construction (the part most
//!     prone to silent breakage, now fully unit-tested)
//!
//! No process spawning, no I/O.

use serde::{Deserialize, Serialize};

/// Gateway port inside the container (TS `OPENCLAW_GATEWAY_CONTAINER_PORT`).
pub const OPENCLAW_GATEWAY_CONTAINER_PORT: u16 = 18789;

/// Accessor for the BrowserOS-owned OpenClaw gateway (TS
/// `OpenclawGatewayAccessor`). Concrete values are provided by the host
/// runtime; this struct is the pure data snapshot the command builder reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayConfig {
    pub container_name: String,
    pub lima_home_dir: String,
    pub limactl_path: String,
    pub vm_name: String,
}

/// Normalize a harness/session key into the gateway bridge session key.
///
/// - `None` → `None`
/// - already `agent:*` → passed through unchanged
/// - anything else → `agent:main:<sanitized>` where non `[A-Za-z0-9-]` chars
///   become `-`
pub fn bridge_session_key(session_key: Option<&str>) -> Option<String> {
    let key = session_key?;
    if key.starts_with("agent:") {
        Some(key.to_string())
    } else {
        let sanitized: String = key
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
            .collect();
        Some(format!("agent:main:{sanitized}"))
    }
}

/// Build the ACP bridge command line (TS `resolveOpenclawAcpCommand`).
///
/// Prefixes `env LIMA_HOME=<path>` so the spawned `limactl` finds the
/// BrowserOS-owned VM, shells into the VM, `nerdctl exec`s into the container
/// with banner suppression, and runs `openclaw acp --url ws://...`. Appends
/// `--session <key>` when a bridge session key resolves.
pub fn resolve_acp_command(gateway: &GatewayConfig, session_key: Option<&str>) -> String {
    let gateway_url = format!("ws://127.0.0.1:{OPENCLAW_GATEWAY_CONTAINER_PORT}");
    let bridge_key = bridge_session_key(session_key);

    let mut argv: Vec<String> = vec![
        "env".into(),
        format!("LIMA_HOME={}", gateway.lima_home_dir),
        gateway.limactl_path.clone(),
        "shell".into(),
        "--workdir".into(),
        "/".into(),
        gateway.vm_name.clone(),
        "--".into(),
        "nerdctl".into(),
        "exec".into(),
        "-i".into(),
        "-e".into(),
        "OPENCLAW_HIDE_BANNER=1".into(),
        "-e".into(),
        "OPENCLAW_SUPPRESS_NOTES=1".into(),
        gateway.container_name.clone(),
        "openclaw".into(),
        "acp".into(),
        "--url".into(),
        gateway_url,
    ];
    if let Some(key) = bridge_key {
        argv.push("--session".into());
        argv.push(key);
    }
    argv.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> GatewayConfig {
        GatewayConfig {
            container_name: "oc-container".into(),
            lima_home_dir: "/home/browseros/.lima".into(),
            limactl_path: "/opt/lima/bin/limactl".into(),
            vm_name: "browseros-vm".into(),
        }
    }

    #[test]
    fn bridge_key_variants() {
        assert_eq!(bridge_session_key(None), None);
        assert_eq!(bridge_session_key(Some("agent:x:main")), Some("agent:x:main".into()));
        assert_eq!(
            bridge_session_key(Some("legacy key/with:chars")),
            Some("agent:main:legacy-key-with-chars".into())
        );
    }

    #[test]
    fn command_contains_core_parts() {
        let cmd = resolve_acp_command(&cfg(), Some("agent:h1:main"));
        assert!(cmd.starts_with("env LIMA_HOME=/home/browseros/.lima /opt/lima/bin/limactl shell"));
        assert!(cmd.contains("browseros-vm -- nerdctl exec -i"));
        assert!(cmd.contains("OPENCLAW_HIDE_BANNER=1"));
        assert!(cmd.contains("OPENCLAW_SUPPRESS_NOTES=1"));
        assert!(cmd.contains("oc-container openclaw acp --url ws://127.0.0.1:18789"));
        assert!(cmd.ends_with("--session agent:h1:main"));
    }

    #[test]
    fn command_without_session_has_no_flag() {
        let cmd = resolve_acp_command(&cfg(), None);
        assert!(!cmd.contains("--session"));
    }
}
