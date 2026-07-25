//! Host CDP agent binary.
//!
//! Env:
//! - `TRIOS_SERVER_WS`        — trios-server WS (default `ws://127.0.0.1:9005/ws`)
//! - `TRIOS_CDP_HTTP`         — DevTools HTTP endpoint (default `http://127.0.0.1:9102`)
//! - `TRIOS_BROWSER_AGENT_ID` — agent id (default `host-cdp`)
//! - `TRIOS_POLL_INTERVAL_MS` — poll cadence (default 1000)

use anyhow::Result;
use tracing::info;
use trios_host_cdp::{discover_page_ws, run, CdpClient, CdpExecutor, PollerConfig};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cdp_http =
        std::env::var("TRIOS_CDP_HTTP").unwrap_or_else(|_| "http://127.0.0.1:9102".into());
    let config = PollerConfig::from_env();

    info!("discovering page target at {cdp_http} …");
    let ws_url = discover_page_ws(&cdp_http).await?;
    info!("attaching to {ws_url}");
    let cdp = CdpClient::connect(&ws_url).await?;
    let executor = CdpExecutor { cdp };

    info!(
        "host CDP agent `{}` → {} (poll every {:?})",
        config.agent_id, config.server_ws, config.poll_interval
    );
    run(config, &executor).await
}
