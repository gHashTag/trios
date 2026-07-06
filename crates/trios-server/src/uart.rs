//! UART endpoint — serial-port bridge over HTTP + SSE.
//!
//! Design goals:
//! - Give remote agents (through `tri-tunnel` Funnel) controlled access to the
//!   host's USB-serial adapters (e.g. `/dev/cu.usbmodem*`, `/dev/ttyUSB*`).
//! - Never expose UART without an explicit, separate token (`TRIOS_UART_TOKEN`).
//!   Reusing `TRIOS_API_KEY` would let anyone with git access reach the wire.
//! - Read = SSE stream so bootrom output can be observed in real time.
//! - Write = one-shot POST that appends bytes to the port; base64-encoded so
//!   binary boot loaders and control chars (Ctrl-C, Ctrl-A) survive JSON.
//! - Auto-close on 30s idle to avoid leaking file descriptors when clients
//!   drop mid-stream.
//!
//! Anchor: phi^2 + phi^-2 = 3.
//!
//! Endpoints (all prefixed with `/api/uart`):
//! | Method | Path            | Purpose                                          |
//! |--------|-----------------|--------------------------------------------------|
//! | GET    | `/ports`        | List available serial ports with metadata.       |
//! | GET    | `/stream`       | SSE stream of bytes read from a port.            |
//! | POST   | `/write`        | Write base64-encoded bytes to a port.            |
//!
//! All three require `Authorization: Bearer $TRIOS_UART_TOKEN`. If the env var
//! is unset, the router is not mounted at all (fail-closed).
//!
//! ## Threat model
//!
//! - Assumes `trios-server` is either bound to `127.0.0.1` or fronted by
//!   `tri-tunnel` (Tailscale Funnel), where the tailnet ACL provides transport
//!   auth. `TRIOS_UART_TOKEN` is a defence-in-depth layer on top.
//! - `/write` is base64-only to prevent smuggling and to make Ctrl-chars
//!   explicit rather than shell-escaped.
//! - No sudo, no port privilege elevation. If the OS blocks access, the endpoint
//!   returns the raw error; it does not attempt to work around permissions.
//!
//! ## Backpressure
//!
//! `/stream` uses a bounded broadcast channel of 1024 chunks. If the client
//! reads slower than the port produces, older chunks are dropped and an SSE
//! event `{"lag": N}` is emitted so the client knows.

use axum::{
    extract::{Query, Request},
    http::{header, StatusCode},
    middleware::Next,
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    convert::Infallible,
    io::{ErrorKind, Read, Write},
    time::Duration,
};
use tokio::sync::broadcast;
use tokio_stream::{
    wrappers::{errors::BroadcastStreamRecvError, BroadcastStream},
    Stream, StreamExt,
};
use tracing::{debug, info, warn};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const ENV_TOKEN: &str = "TRIOS_UART_TOKEN";
const DEFAULT_BAUD: u32 = 115_200;
const READ_CHUNK: usize = 512;
const READ_POLL: Duration = Duration::from_millis(20);
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const CHANNEL_CAPACITY: usize = 1024;

/// Returns the configured UART token, or None if the endpoint should be
/// disabled entirely.
fn configured_token() -> Option<String> {
    match std::env::var(ENV_TOKEN) {
        Ok(v) if !v.is_empty() => Some(v),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Router assembly
// ---------------------------------------------------------------------------

/// Build the `/api/uart/*` router.
///
/// Returns `None` if `TRIOS_UART_TOKEN` is unset — in that case the caller
/// should not mount UART at all. This is the fail-closed default.
///
/// The router is generic over the parent's state type so it can be nested
/// into a `Router<AppState>` without requiring UART handlers to know about
/// `AppState`. UART handlers themselves are stateless.
pub fn router<S>() -> Option<Router<S>>
where
    S: Clone + Send + Sync + 'static,
{
    let _ = configured_token()?;
    Some(
        Router::new()
            .route("/ports", get(list_ports))
            .route("/stream", get(stream_port))
            .route("/write", post(write_port))
            .layer(axum::middleware::from_fn(uart_auth_middleware)),
    )
}

/// Log whether the endpoint is enabled at startup. Called from `main` so the
/// operator sees the state on boot without leaking the token.
pub fn log_startup_state() {
    match configured_token() {
        Some(_) => info!("UART endpoint ENABLED at /api/uart (auth: TRIOS_UART_TOKEN)"),
        None => warn!(
            "UART endpoint DISABLED — set {} to enable /api/uart/*",
            ENV_TOKEN
        ),
    }
}

// ---------------------------------------------------------------------------
// Auth middleware — separate from global auth
// ---------------------------------------------------------------------------

async fn uart_auth_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    let expected = match configured_token() {
        Some(t) => t,
        None => {
            // Should be unreachable — router() returns None in this case — but
            // fail closed just in case.
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    let provided = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_owned);

    match provided {
        Some(t) if t == expected => Ok(next.run(request).await),
        Some(_) => {
            warn!("UART: invalid bearer token");
            Err(StatusCode::UNAUTHORIZED)
        }
        None => {
            warn!("UART: missing Authorization header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

// ---------------------------------------------------------------------------
// GET /ports — metadata only, no port is opened
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct PortInfo {
    device: String,
    port_type: &'static str,
    vid: Option<u16>,
    pid: Option<u16>,
    serial_number: Option<String>,
    manufacturer: Option<String>,
    product: Option<String>,
}

async fn list_ports() -> Response {
    let ports = match serialport::available_ports() {
        Ok(p) => p,
        Err(e) => {
            warn!("serialport::available_ports failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let listed: Vec<PortInfo> = ports
        .into_iter()
        .map(|p| {
            use serialport::SerialPortType;
            let (kind, vid, pid, sn, mfr, prod) = match p.port_type {
                SerialPortType::UsbPort(info) => (
                    "usb",
                    Some(info.vid),
                    Some(info.pid),
                    info.serial_number.clone(),
                    info.manufacturer.clone(),
                    info.product.clone(),
                ),
                SerialPortType::BluetoothPort => ("bluetooth", None, None, None, None, None),
                SerialPortType::PciPort => ("pci", None, None, None, None, None),
                SerialPortType::Unknown => ("unknown", None, None, None, None, None),
            };
            PortInfo {
                device: p.port_name,
                port_type: kind,
                vid,
                pid,
                serial_number: sn,
                manufacturer: mfr,
                product: prod,
            }
        })
        .collect();

    Json(json!({ "ports": listed })).into_response()
}

// ---------------------------------------------------------------------------
// GET /stream?port=/dev/cu.usbmodemXXXX&baud=115200 — SSE of raw bytes
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct StreamQuery {
    port: String,
    baud: Option<u32>,
}

async fn stream_port(
    Query(q): Query<StreamQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, Response> {
    let baud = q.baud.unwrap_or(DEFAULT_BAUD);
    let port_name = q.port.clone();
    debug!(port = %port_name, baud, "UART stream open");

    // Open port in a blocking task; keep it in a thread for its lifetime.
    let (tx, rx) = broadcast::channel::<Vec<u8>>(CHANNEL_CAPACITY);
    let port_thread = std::thread::Builder::new()
        .name(format!("uart-read-{}", port_name))
        .spawn(move || read_loop(port_name, baud, tx));

    if let Err(e) = port_thread {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("thread spawn failed: {}", e) })),
        )
            .into_response());
    }

    // Wrap broadcast rx into a stream; each item = one chunk of bytes.
    let stream = BroadcastStream::new(rx)
        .timeout(STREAM_IDLE_TIMEOUT)
        .map(|item| {
            let ev = match item {
                Ok(Ok(bytes)) => Event::default().event("data").data(B64.encode(bytes)),
                Ok(Err(BroadcastStreamRecvError::Lagged(n))) => {
                    Event::default().event("lag").data(n.to_string())
                }
                Err(_elapsed) => Event::default().event("idle").data("30s no data"),
            };
            Ok::<_, Infallible>(ev)
        });

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// Blocking read loop — runs in a dedicated OS thread because `serialport`
/// is sync. Broadcasts every chunk to all SSE subscribers.
fn read_loop(port_name: String, baud: u32, tx: broadcast::Sender<Vec<u8>>) {
    let mut port = match serialport::new(&port_name, baud)
        .timeout(READ_POLL)
        .open()
    {
        Ok(p) => p,
        Err(e) => {
            warn!(port = %port_name, "UART open failed: {}", e);
            return;
        }
    };
    info!(port = %port_name, baud, "UART reader started");

    let mut buf = vec![0u8; READ_CHUNK];
    loop {
        // Stop when no more receivers exist (all clients disconnected).
        if tx.receiver_count() == 0 {
            debug!(port = %port_name, "UART reader stopping — no receivers");
            return;
        }
        match port.read(&mut buf) {
            Ok(0) => {
                std::thread::sleep(READ_POLL);
            }
            Ok(n) => {
                let chunk = buf[..n].to_vec();
                let _ = tx.send(chunk);
            }
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                // Normal — no data in the polling window.
                continue;
            }
            Err(e) => {
                warn!(port = %port_name, "UART read error: {}", e);
                return;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// POST /write — one-shot byte write
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct WriteBody {
    port: String,
    baud: Option<u32>,
    /// base64-encoded bytes to send.
    data: String,
}

#[derive(Serialize)]
struct WriteResult {
    written: usize,
}

async fn write_port(Json(body): Json<WriteBody>) -> Response {
    let bytes = match B64.decode(&body.data) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("invalid base64: {}", e) })),
            )
                .into_response();
        }
    };
    let baud = body.baud.unwrap_or(DEFAULT_BAUD);

    let write_result = tokio::task::spawn_blocking(move || -> Result<usize, String> {
        let mut port = serialport::new(&body.port, baud)
            .timeout(Duration::from_secs(2))
            .open()
            .map_err(|e| format!("open {}: {}", body.port, e))?;
        port.write_all(&bytes)
            .map_err(|e| format!("write: {}", e))?;
        port.flush().map_err(|e| format!("flush: {}", e))?;
        Ok(bytes.len())
    })
    .await;

    match write_result {
        Ok(Ok(n)) => Json(WriteResult { written: n }).into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("join: {}", e) })),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_disabled_without_token() {
        // Ensure env var is not set — override any inherited value from CI.
        std::env::remove_var(ENV_TOKEN);
        assert!(router::<()>().is_none());
    }

    #[test]
    fn router_enabled_with_token() {
        std::env::set_var(ENV_TOKEN, "test-token-abc");
        assert!(router::<()>().is_some());
        std::env::remove_var(ENV_TOKEN);
    }

    #[test]
    fn base64_write_body_roundtrip() {
        // Round-trip through the same base64 config the handler uses.
        let raw: &[u8] = b"\r\nroot\r\nanalog\r\n";
        let encoded = B64.encode(raw);
        let decoded = B64.decode(&encoded).unwrap();
        assert_eq!(decoded, raw);
    }

    #[test]
    fn control_chars_survive_base64() {
        // Ctrl-C = 0x03, Ctrl-A = 0x01 — must not be mangled.
        let raw: &[u8] = &[0x03, 0x01, 0x1b, b'[', b'B'];
        let encoded = B64.encode(raw);
        let decoded = B64.decode(&encoded).unwrap();
        assert_eq!(decoded, raw);
    }

    #[test]
    fn default_baud_is_115200() {
        assert_eq!(DEFAULT_BAUD, 115_200);
    }

    #[test]
    fn env_var_name_is_stable() {
        // If this ever changes, docs and tri-tunnel wiring must be updated too.
        assert_eq!(ENV_TOKEN, "TRIOS_UART_TOKEN");
    }
}
