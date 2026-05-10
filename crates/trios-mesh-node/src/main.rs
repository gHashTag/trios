//! Trinity GF16 CPU Mesh Node Daemon — v0.2.1
//! E2E Encryption: X25519 ECDH + ChaCha20-Poly1305
//! φ² + φ⁻² = 3 — Self-Sovereign dePIN Mesh
//!
//! dest_hash = SHA256(x25519_pubkey)[..16] — single canonical derivation

mod crypto;
#[cfg(feature = "persist")]
mod persist;

use anyhow::Result;
use axum::{
    extract::State,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tracing::info;
#[cfg(feature = "persist")]
use tracing::warn;

use trios_mesh::routing::RoutingTable;
use crypto::MeshKeypair;

#[cfg(feature = "persist")]
use sqlx::PgPool;

type DestHash = [u8; 16];

// ── Node State ────────────────────────────────────────────────────────────

#[derive(Clone)]
struct NodeState {
    /// SHA256(x25519_pubkey)[..16] — single source of truth
    dest_hash: DestHash,
    keypair:   Arc<MeshKeypair>,
    table:     Arc<Mutex<RoutingTable>>,
    node_name: String,
    tick:      Arc<Mutex<u32>>,
    #[cfg(feature = "persist")]
    db:        Option<PgPool>,
}

impl NodeState {
    fn new(seed: u8, name: &str) -> Self {
        let use_random = std::env::var("MESH_RANDOM_KEY").is_ok();
        let keypair = if use_random {
            MeshKeypair::random()
        } else {
            MeshKeypair::from_seed(seed)
        };

        // keypair.dest_hash = SHA256(pubkey)[..16] — used everywhere
        let dest_hash = keypair.dest_hash;
        let table = RoutingTable::new(dest_hash);

        info!("🔺 Node '{}' pubkey: {}...", name, &keypair.pubkey_hex()[..16]);
        info!("🔑 dest_hash: {} (SHA256(X25519_pubkey)[..16])", hex::encode(dest_hash));

        Self {
            dest_hash,
            keypair:   Arc::new(keypair),
            table:     Arc::new(Mutex::new(table)),
            node_name: name.to_owned(),
            tick:      Arc::new(Mutex::new(0)),
            #[cfg(feature = "persist")]
            db:        None,
        }
    }

    fn bump_tick(&self) -> u32 {
        let mut t = self.tick.lock().unwrap();
        *t += 1;
        *t
    }
}

// ── API types ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct NodeInfo {
    name:       String,
    dest_hash:  String,
    pubkey:     String,
    routes:     usize,
    tick:       u32,
    power_mw:   f32,
    encryption: &'static str,
}

#[derive(Deserialize)]
struct AnnounceReq {
    dest_hash: String,
    sender:    String,
    hops:      u8,
    quality:   u8,
    #[serde(default)]
    pubkey:    Option<String>,
}

#[derive(Serialize)]
struct AnnounceResp { accepted: bool, routes: usize }

#[derive(Deserialize)]
struct NextHopReq { dest_hash: String }

#[derive(Serialize)]
struct NextHopResp { next_hop: Option<String>, local: bool }

#[derive(Deserialize)]
struct EncryptReq {
    recipient_pubkey: String,
    plaintext:        String,
}

#[derive(Serialize)]
struct EncryptResp {
    payload:       String,
    sender_pubkey: String,
}

#[derive(Deserialize)]
struct SendMessageReq {
    to:            String,
    sender_pubkey: String,
    payload:       String,
}

#[derive(Serialize)]
struct SendMessageResp {
    delivered: bool,
    decrypted: Option<String>,
    error:     Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn hex_to_dest(s: &str) -> Option<DestHash> {
    let b = hex::decode(s).ok()?;
    if b.len() != 16 { return None; }
    let mut a = [0u8; 16]; a.copy_from_slice(&b); Some(a)
}

fn hex_to_pubkey(s: &str) -> Option<[u8; 32]> {
    let b = hex::decode(s).ok()?;
    if b.len() != 32 { return None; }
    let mut a = [0u8; 32]; a.copy_from_slice(&b); Some(a)
}

fn to_hex(d: &[u8]) -> String { hex::encode(d) }

// ── Handlers ──────────────────────────────────────────────────────────────

async fn health() -> &'static str { "ok" }

async fn get_info(State(s): State<NodeState>) -> Json<NodeInfo> {
    let routes = s.table.lock().unwrap().len();
    let tick   = *s.tick.lock().unwrap();
    Json(NodeInfo {
        name:       s.node_name.clone(),
        dest_hash:  to_hex(&s.dest_hash),
        pubkey:     s.keypair.pubkey_hex(),
        routes,
        tick,
        power_mw:   800.0,
        encryption: "X25519-ECDH+ChaCha20Poly1305",
    })
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
    if let Some(pk) = &req.pubkey {
        if pk.len() == 64 {
            info!("📣 ANNOUNCE dest={} pubkey={}...", &req.dest_hash[..8], &pk[..16]);
        }
    }
    let tick = s.bump_tick();
    let accepted;
    let routes;
    let pubkey_bytes = req.pubkey.as_deref().and_then(hex_to_pubkey);
    {
        let mut tbl = s.table.lock().unwrap();
        accepted = tbl.process_announce(dest, via, req.hops, req.quality, tick);
        routes   = tbl.len();
    }
    if accepted {
        info!("✅ ANNOUNCE accepted dest={} hops={}", &req.dest_hash[..8], req.hops);
        // L-E2E-4: best-effort mirror to Neon. Spawned so the announce hot
        // path never blocks on DB I/O — errors are logged inside upsert_route.
        #[cfg(feature = "persist")]
        if let Some(pool) = s.db.clone() {
            let self_dest = s.dest_hash;
            tokio::spawn(async move {
                persist::upsert_route(
                    &pool,
                    &self_dest,
                    &dest,
                    &via,
                    req.hops,
                    req.quality,
                    pubkey_bytes.as_ref(),
                )
                .await;
            });
        }
        #[cfg(not(feature = "persist"))]
        let _ = pubkey_bytes; // silence unused warning when feature is off
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
    if dest == s.dest_hash {
        return Json(NextHopResp { next_hop: None, local: true });
    }
    let tbl = s.table.lock().unwrap();
    match tbl.next_hop(&dest) {
        Some(hop) => Json(NextHopResp { next_hop: Some(to_hex(&hop)), local: false }),
        None      => Json(NextHopResp { next_hop: None, local: false }),
    }
}

async fn post_encrypt(
    State(s): State<NodeState>,
    Json(req): Json<EncryptReq>,
) -> Json<EncryptResp> {
    let recipient = match hex_to_pubkey(&req.recipient_pubkey) {
        Some(pk) => pk,
        None => return Json(EncryptResp {
            payload:       "error: invalid pubkey".to_owned(),
            sender_pubkey: s.keypair.pubkey_hex(),
        }),
    };
    match crypto::encrypt_for(req.plaintext.as_bytes(), &s.keypair, &recipient) {
        Ok(p)  => Json(EncryptResp { payload: p, sender_pubkey: s.keypair.pubkey_hex() }),
        Err(e) => Json(EncryptResp {
            payload:       format!("error: {e}"),
            sender_pubkey: s.keypair.pubkey_hex(),
        }),
    }
}

async fn post_message(
    State(s): State<NodeState>,
    Json(req): Json<SendMessageReq>,
) -> Json<SendMessageResp> {
    let to_dest = match hex_to_dest(&req.to) {
        Some(d) => d,
        None => return Json(SendMessageResp {
            delivered: false, decrypted: None,
            error: Some("invalid dest_hash".to_owned()),
        }),
    };
    let sender_pk = match hex_to_pubkey(&req.sender_pubkey) {
        Some(pk) => pk,
        None => return Json(SendMessageResp {
            delivered: false, decrypted: None,
            error: Some("invalid sender_pubkey".to_owned()),
        }),
    };

    // This node is the recipient — decrypt
    if to_dest == s.dest_hash {
        match crypto::decrypt_from(&req.payload, &s.keypair, &sender_pk) {
            Ok(plain) => {
                let text = String::from_utf8_lossy(&plain).to_string();
                info!("📩 MESSAGE decrypted: '{}'", &text[..text.len().min(60)]);
                return Json(SendMessageResp {
                    delivered: true,
                    decrypted: Some(text),
                    error:     None,
                });
            }
            Err(e) => return Json(SendMessageResp {
                delivered: false, decrypted: None,
                error: Some(format!("decrypt failed: {e}")),
            }),
        }
    }

    // Not for us — suggest next hop
    let next = s.table.lock().unwrap().next_hop(&to_dest).map(|h| to_hex(&h));
    Json(SendMessageResp {
        delivered: false,
        decrypted: None,
        error: Some(next
            .map(|h| format!("forward to next_hop={}", &h[..8]))
            .unwrap_or_else(|| "no route to dest".to_owned())),
    })
}

// ── Main ──────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("trios_mesh_node=info".parse()?),
        )
        .init();

    let seed = std::env::var("MESH_SEED")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<u8>().unwrap_or(1);
    let name = std::env::var("MESH_NODE_NAME")
        .unwrap_or_else(|_| format!("trinity-node-{}", seed));
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>().unwrap_or(8080);

    #[cfg_attr(not(feature = "persist"), allow(unused_mut))]
    let mut state = NodeState::new(seed, &name);

    info!("🔺 Trinity Mesh Node v0.2.2 '{}' on :{}", name, port);
    info!("🔐 X25519-ECDH + ChaCha20-Poly1305 (E2E default)");
    info!("φ² + φ⁻² = 3");

    // L-E2E-4 — optional Neon-backed persistence (feature `persist`, default ON).
    #[cfg(feature = "persist")]
    {
        match persist::try_open_from_env().await {
            Ok(Some(pool)) => {
                if let Err(e) = persist::migrate(&pool).await {
                    warn!("💾 migrations failed: {e:?} — continuing without persistence");
                } else {
                    match persist::load_routes(&pool, &state.dest_hash).await {
                        Ok(restored) => {
                            let mut tbl = state.table.lock().unwrap();
                            tbl.restore_from(restored);
                            info!("💾 restored {} route(s) on boot", tbl.len());
                        }
                        Err(e) => warn!("💾 load_routes failed: {e:?}"),
                    }
                    state.db = Some(pool);
                }
            }
            Ok(None) => {} // already logged inside try_open_from_env
            Err(e) => warn!("💾 Neon connection failed: {e:?} — continuing in-memory"),
        }
    }

    let app = Router::new()
        .route("/health",   get(health))
        .route("/info",     get(get_info))
        .route("/announce", post(post_announce))
        .route("/next-hop", post(post_next_hop))
        .route("/encrypt",  post(post_encrypt))
        .route("/message",  post(post_message))
        .with_state(state);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("✅ Listening on 0.0.0.0:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
