//! Trinity GF16 CPU Mesh Node Daemon
//! Railway deployment: 3 nodes in a mesh network
//! φ² + φ⁻² = 3 — Self-Sovereign dePIN Mesh

use anyhow::Result;
use axum::{
    extract::State,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tracing::info;

// Re-export trios-mesh with std feature
use trios_mesh::{
    identity::NodeIdentity,
    routing::RoutingTable,
    DestHash,
};

// ── Node State ────────────────────────────────────────────────────────────

#[derive(Clone)]
struct NodeState {
    identity:  Arc<NodeIdentity>,
    table:     Arc<Mutex<RoutingTable>>,
    node_name: String,
    tick:      Arc<Mutex<u32>>,
}

impl NodeState {
    fn new(seed: u8, name: &str) -> Self {
        // Deterministic identity from seed (dev mode)
        let mut pubkey = [0u8; 32];
        pubkey[0] = seed;
        pubkey[1] = 0xA3; // Trinity marker (φ-derived constant)
        let h = Sha256::digest(&pubkey);
        pubkey[2..18].copy_from_slice(&h[..16]); // φ-stretch

        let identity = NodeIdentity::from_pubkey(&pubkey);
        let table    = RoutingTable::new(identity.dest_hash);

        info!("🔺 Node '{}' identity: {:02x?}", name, &identity.dest_hash[..4]);

        Self {
            identity:  Arc::new(identity),
            table:     Arc::new(Mutex::new(table)),
            node_name: name.to_owned(),
            tick:      Arc::new(Mutex::new(0)),
        }
    }

    fn bump_tick(&self) -> u32 {
        let mut t = self.tick.lock().unwrap();
        *t += 1;
        *t
    }
}

// ── HTTP API types ─────────────────────────────────────────────────────

#[derive(Serialize)]
struct NodeInfo {
    name:      String,
    dest_hash: String,
    routes:    usize,
    tick:      u32,
    power_mw:  f32, // simulated: CPU mesh at ~800 mW vs ASIC at 25 mW
}

#[derive(Serialize)]
struct RouteView {
    dest:      String,
    next_hop:  String,
    hops:      u8,
    quality:   u8,
    last_seen: u32,
}

#[derive(Deserialize)]
struct AnnounceReq {
    dest_hash: String,   // hex
    sender:    String,   // hex
    hops:      u8,
    quality:   u8,
}

#[derive(Serialize)]
struct AnnounceResp {
    accepted:  bool,
    routes:    usize,
}

#[derive(Deserialize)]
struct NextHopReq {
    dest_hash: String,   // hex
}

#[derive(Serialize)]
struct NextHopResp {
    next_hop: Option<String>,
    local:    bool,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn hex_to_dest(s: &str) -> Option<DestHash> {
    let bytes = hex::decode(s).ok()?;
    if bytes.len() != 16 { return None; }
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&bytes);
    Some(arr)
}

fn dest_to_hex(d: &DestHash) -> String {
    hex::encode(d)
}

// ── Route handlers ────────────────────────────────────────────────────────────

async fn health() -> &'static str { "ok" }

async fn get_info(State(s): State<NodeState>) -> Json<NodeInfo> {
    let routes = s.table.lock().unwrap().len();
    let tick   = *s.tick.lock().unwrap();
    Json(NodeInfo {
        name:      s.node_name.clone(),
        dest_hash: dest_to_hex(&s.identity.dest_hash),
        routes,
        tick,
        // CPU node draws ~800 mW; ASIC target <25 mW
        power_mw:  800.0,
    })
}

async fn get_routes(State(_s): State<NodeState>) -> Json<Vec<RouteView>> {
    // expose table via debug dump — real impl needs table iterator
    Json(vec![])
}

async fn post_announce(
    State(s): State<NodeState>,
    Json(req): Json<AnnounceReq>,
) -> Json<AnnounceResp> {
    let dest = match hex_to_dest(&req.dest_hash) {
        Some(d) => d,
        None    => return Json(AnnounceResp { accepted: false, routes: 0 }),
    };
    let via = match hex_to_dest(&req.sender) {
        Some(d) => d,
        None    => return Json(AnnounceResp { accepted: false, routes: 0 }),
    };
    let tick = s.bump_tick();
    let mut tbl = s.table.lock().unwrap();
    let accepted = tbl.process_announce(dest, via, req.hops, req.quality, tick);
    let routes   = tbl.len();
    if accepted {
        info!("📡 ANNOUNCE accepted dest={} via={} hops={}",
              &req.dest_hash[..8], &req.sender[..8], req.hops);
    }
    Json(AnnounceResp { accepted, routes })
}

async fn post_next_hop(
    State(s): State<NodeState>,
    Json(req): Json<NextHopReq>,
) -> Json<NextHopResp> {
    let dest = match hex_to_dest(&req.dest_hash) {
        Some(d) => d,
        None    => return Json(NextHopResp { next_hop: None, local: false }),
    };
    let tbl = s.table.lock().unwrap();
    match tbl.next_hop(&dest) {
        None if dest == s.identity.dest_hash =>
            Json(NextHopResp { next_hop: None, local: true }),
        None =>
            Json(NextHopResp { next_hop: None, local: false }),
        Some(hop) =>
            Json(NextHopResp {
                next_hop: Some(dest_to_hex(&hop)),
                local:    false,
            }),
    }
}

// ── Main ───────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("trios_mesh_node=info".parse()?))
        .init();

    let seed = std::env::var("MESH_SEED")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<u8>().unwrap_or(1);

    let name = std::env::var("MESH_NODE_NAME")
        .unwrap_or_else(|_| format!("node-{}", seed));

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>().unwrap_or(8080);

    let state = NodeState::new(seed, &name);

    info!("🔺 Trinity Mesh Node '{}' starting on :{}", name, port);
    info!("φ² + φ⁻² = 3  |  CPU mode (~800 mW)  |  target: ASIC <25 mW");

    let app = Router::new()
        .route("/health",       get(health))
        .route("/info",         get(get_info))
        .route("/routes",       get(get_routes))
        .route("/announce",     post(post_announce))
        .route("/next-hop",     post(post_next_hop))
        .with_state(state);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("✅ Listening on 0.0.0.0:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
