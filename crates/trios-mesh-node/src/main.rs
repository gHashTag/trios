//! Trinity GF16 CPU Mesh Node Daemon — v0.2.0
//! E2E Encryption: X25519 ECDH + ChaCha20-Poly1305
//! φ² + φ⁻² = 3 — Self-Sovereign dePIN Mesh

mod crypto;

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

use trios_mesh::{
    identity::NodeIdentity,
    routing::RoutingTable,
    DestHash,
};
use crypto::MeshKeypair;

// ── Node State ────────────────────────────────────────────────────────────

#[derive(Clone)]
struct NodeState {
    identity:  Arc<NodeIdentity>,
    keypair:   Arc<MeshKeypair>,
    table:     Arc<Mutex<RoutingTable>>,
    node_name: String,
    tick:      Arc<Mutex<u32>>,
}

impl NodeState {
    fn new(seed: u8, name: &str) -> Self {
        // E2E keypair — deterministic from seed (dev) or random (prod)
        let use_random = std::env::var("MESH_RANDOM_KEY").is_ok();
        let keypair = if use_random {
            MeshKeypair::random()
        } else {
            MeshKeypair::from_seed(seed)
        };

        // NodeIdentity dest_hash now derived from X25519 pubkey
        let identity = NodeIdentity::from_pubkey(&pad_to_32(&keypair.dest_hash));
        let table    = RoutingTable::new(identity.dest_hash);

        info!("🔺 Node '{}' pubkey: {}", name, &keypair.pubkey_hex()[..16]);
        info!("🔑 dest_hash: {} (X25519-derived)", hex::encode(keypair.dest_hash));

        Self {
            identity:  Arc::new(identity),
            keypair:   Arc::new(keypair),
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

fn pad_to_32(h: &[u8; 16]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[..16].copy_from_slice(h);
    out
}

// ── HTTP API types ─────────────────────────────────────────────────────

#[derive(Serialize)]
struct NodeInfo {
    name:         String,
    dest_hash:    String,
    pubkey:       String,   // X25519 pubkey hex (32 bytes) — for ECDH
    routes:       usize,
    tick:         u32,
    power_mw:     f32,
    encryption:   &'static str,
}

#[derive(Deserialize)]
struct AnnounceReq {
    dest_hash:  String,   // hex 16 bytes
    sender:     String,   // hex 16 bytes
    hops:       u8,
    quality:    u8,
    /// Sender's X25519 pubkey (hex 32 bytes) — enables ECDH on receipt
    #[serde(default)]
    pubkey:     Option<String>,
}

#[derive(Serialize)]
struct AnnounceResp {
    accepted:  bool,
    routes:    usize,
}

#[derive(Deserialize)]
struct NextHopReq {
    dest_hash: String,
}

#[derive(Serialize)]
struct NextHopResp {
    next_hop:  Option<String>,
    local:     bool,
}

/// Encrypted message envelope
#[derive(Deserialize)]
struct SendMessageReq {
    /// Recipient dest_hash (hex)
    to:             String,
    /// Sender's X25519 pubkey (hex 32 bytes)
    sender_pubkey:  String,
    /// ChaCha20-Poly1305 encrypted payload, base64(nonce||ciphertext)
    payload:        String,
}

#[derive(Serialize)]
struct SendMessageResp {
    delivered:   bool,
    decrypted:   Option<String>,  // UTF-8 plaintext if recipient is this node
    error:       Option<String>,
}

/// Encrypt a message for a known pubkey
#[derive(Deserialize)]
struct EncryptReq {
    /// Recipient X25519 pubkey hex (32 bytes)
    recipient_pubkey: String,
    /// Plaintext (UTF-8)
    plaintext:        String,
}

#[derive(Serialize)]
struct EncryptResp {
    payload:         String,  // base64(nonce||ciphertext)
    sender_pubkey:   String,  // this node's pubkey hex
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn hex_to_dest(s: &str) -> Option<DestHash> {
    let bytes = hex::decode(s).ok()?;
    if bytes.len() != 16 { return None; }
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&bytes);
    Some(arr)
}

fn hex_to_pubkey(s: &str) -> Option<[u8; 32]> {
    let bytes = hex::decode(s).ok()?;
    if bytes.len() != 32 { return None; }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(arr)
}

fn dest_to_hex(d: &DestHash) -> String { hex::encode(d) }

// ── Route handlers ────────────────────────────────────────────────────────────

async fn health() -> &'static str { "ok" }

async fn get_info(State(s): State<NodeState>) -> Json<NodeInfo> {
    let routes = s.table.lock().unwrap().len();
    let tick   = *s.tick.lock().unwrap();
    Json(NodeInfo {
        name:       s.node_name.clone(),
        dest_hash:  dest_to_hex(&s.identity.dest_hash),
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

    if let Some(pk_hex) = &req.pubkey {
        if pk_hex.len() == 64 {
            info!("📣 ANNOUNCE dest={} pubkey={}…", &req.dest_hash[..8], &pk_hex[..16]);
        }
    }

    let tick     = s.bump_tick();
    let mut tbl  = s.table.lock().unwrap();
    let accepted = tbl.process_announce(dest, via, req.hops, req.quality, tick);
    let routes   = tbl.len();
    if accepted {
        info!("✅ ANNOUNCE accepted dest={} hops={}", &req.dest_hash[..8], req.hops);
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

/// POST /encrypt — encrypt plaintext for a recipient pubkey
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
        Ok(payload) => Json(EncryptResp {
            payload,
            sender_pubkey: s.keypair.pubkey_hex(),
        }),
        Err(e) => Json(EncryptResp {
            payload:       format!("error: {e}"),
            sender_pubkey: s.keypair.pubkey_hex(),
        }),
    }
}

/// POST /message — deliver encrypted message; decrypt if recipient is this node
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

    // If message is for this node — decrypt it
    if to_dest == s.identity.dest_hash {
        match crypto::decrypt_from(&req.payload, &s.keypair, &sender_pk) {
            Ok(plaintext) => {
                let text = String::from_utf8_lossy(&plaintext).to_string();
                info!("📩 MESSAGE received and decrypted: '{}", &text[..text.len().min(40)]);
                return Json(SendMessageResp {
                    delivered: true,
                    decrypted: Some(text),
                    error: None,
                });
            }
            Err(e) => return Json(SendMessageResp {
                delivered: false, decrypted: None,
                error: Some(format!("decrypt failed: {e}")),
            }),
        }
    }

    // Not for us — check routing table for next hop
    let next = s.table.lock().unwrap().next_hop(&to_dest)
        .map(|h| dest_to_hex(&h));

    Json(SendMessageResp {
        delivered: false,
        decrypted: None,
        error: next.map(|h| format!("forward to next_hop={}", &h[..8]))
            .or(Some("no route to dest".to_owned())),
    })
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
        .unwrap_or_else(|_| format!("trinity-node-{}", seed));

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>().unwrap_or(8080);

    let state = NodeState::new(seed, &name);

    info!("🔺 Trinity Mesh Node v0.2.0 '{}' on :{}", name, port);
    info!("🔐 Encryption: X25519-ECDH + ChaCha20-Poly1305");
    info!("φ² + φ⁻² = 3  |  CPU mode (~800 mW)");

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
