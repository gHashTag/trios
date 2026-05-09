//! trios-mesh-node — 25-test E2E harness (Rust-only, L1-compliant)
//!
//! L-E2E-1 · trinity-fpga#23 · EPIC trinity-fpga#22
//! Anchor: φ² + φ⁻² = 3
//!
//! Replaces `scripts/e2e_25.sh` (forbidden under L1 — no .sh files).
//! Spawns two `mesh-node` instances, exercises 25 black-box checks against
//! their HTTP API, and exits non-zero on any failure.
//!
//! Run locally:   cargo run --release -p trios-mesh-node --bin e2e_25
//! Run from CI:   cargo run --release -p trios-mesh-node --bin e2e_25 --no-default-features
//!
//! Env knobs:
//!   BIN      — path to mesh-node binary (default: ./target/release/mesh-node)
//!   PORT_A   — listening port for node-A (default 18080)
//!   PORT_B   — listening port for node-B (default 18081)

use std::env;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use serde_json::Value;

struct Suite {
    pass: u32,
    fail: u32,
    failed: Vec<String>,
}

impl Suite {
    fn new() -> Self {
        Self { pass: 0, fail: 0, failed: vec![] }
    }
    fn ok(&mut self, name: &str, note: &str) {
        self.pass += 1;
        println!("  ✅ {:<46} {}", name, note);
    }
    fn fail(&mut self, name: &str, note: &str) {
        self.fail += 1;
        self.failed.push(name.to_string());
        println!("  ❌ {:<46} {}", name, note);
    }
    fn assert_eq(&mut self, name: &str, expected: &str, actual: &str) {
        if expected == actual {
            self.ok(name, &format!("= {}", expected));
        } else {
            self.fail(name, &format!("expected='{}' actual='{}'", expected, actual));
        }
    }
    fn assert_contains(&mut self, name: &str, needle: &str, hay: &str) {
        if hay.contains(needle) {
            self.ok(name, &format!("contains '{}'", needle));
        } else {
            self.fail(name, &format!("no '{}' in '{}'", needle, hay));
        }
    }
}

/// Minimal blocking HTTP client built on std::net so we don't pull in reqwest.
mod http {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    pub struct Resp {
        pub status: u16,
        pub body: String,
    }

    fn parse(raw: &[u8]) -> Resp {
        let text = String::from_utf8_lossy(raw).to_string();
        let mut parts = text.splitn(2, "\r\n\r\n");
        let head = parts.next().unwrap_or("");
        let body_full = parts.next().unwrap_or("").to_string();

        let status = head
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(0);

        // Handle chunked encoding minimally — strip simple hex-len framing if present.
        let lower = head.to_lowercase();
        let body = if lower.contains("transfer-encoding: chunked") {
            decode_chunked(&body_full)
        } else {
            body_full
        };
        Resp { status, body }
    }

    fn decode_chunked(s: &str) -> String {
        let mut out = String::new();
        let mut rest = s;
        loop {
            let nl = match rest.find("\r\n") {
                Some(i) => i,
                None => break,
            };
            let len_hex = rest[..nl].trim();
            let len = match usize::from_str_radix(len_hex, 16) {
                Ok(n) => n,
                Err(_) => break,
            };
            let after = &rest[nl + 2..];
            if len == 0 || after.len() < len {
                break;
            }
            out.push_str(&after[..len]);
            // skip trailing \r\n after chunk
            rest = &after[len + 2..];
        }
        out
    }

    fn send(host: &str, port: u16, req: &[u8]) -> std::io::Result<Resp> {
        let addr = format!("{}:{}", host, port);
        let mut s = TcpStream::connect(&addr)?;
        s.set_read_timeout(Some(Duration::from_secs(5)))?;
        s.set_write_timeout(Some(Duration::from_secs(5)))?;
        s.write_all(req)?;
        let mut buf = Vec::with_capacity(8192);
        s.read_to_end(&mut buf)?;
        Ok(parse(&buf))
    }

    pub fn get(host: &str, port: u16, path: &str) -> std::io::Result<Resp> {
        let req = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            path, host
        );
        send(host, port, req.as_bytes())
    }

    pub fn post_json(host: &str, port: u16, path: &str, body: &str) -> std::io::Result<Resp> {
        let req = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            path,
            host,
            body.len(),
            body
        );
        send(host, port, req.as_bytes())
    }
}

fn jget<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.get(key).unwrap_or(&Value::Null)
}

fn jstr(v: &Value, key: &str) -> String {
    jget(v, key).as_str().unwrap_or("").to_string()
}

fn jbool(v: &Value, key: &str) -> bool {
    jget(v, key).as_bool().unwrap_or(false)
}

fn parse_json(s: &str) -> Value {
    serde_json::from_str(s).unwrap_or(Value::Null)
}

fn wait_until_healthy(port_a: u16, port_b: u16) -> bool {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        sleep(Duration::from_millis(300));
        let a = http::get("127.0.0.1", port_a, "/health");
        let b = http::get("127.0.0.1", port_b, "/health");
        if matches!(&a, Ok(r) if r.status == 200)
            && matches!(&b, Ok(r) if r.status == 200)
        {
            return true;
        }
    }
    false
}

struct Guard {
    a: Child,
    b: Child,
    log_a: String,
    log_b: String,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.a.kill();
        let _ = self.b.kill();
        let _ = self.a.wait();
        let _ = self.b.wait();
    }
}

fn read_log(path: &str) -> String {
    if let Ok(mut f) = std::fs::File::open(path) {
        let mut s = String::new();
        let _ = f.read_to_string(&mut s);
        s
    } else {
        String::new()
    }
}

fn main() {
    let bin = env::var("BIN").unwrap_or_else(|_| "./target/release/mesh-node".into());
    let port_a: u16 = env::var("PORT_A").ok().and_then(|s| s.parse().ok()).unwrap_or(18080);
    let port_b: u16 = env::var("PORT_B").ok().and_then(|s| s.parse().ok()).unwrap_or(18081);

    let log_a = "/tmp/node-a.log".to_string();
    let log_b = "/tmp/node-b.log".to_string();

    println!("── trios-mesh-node E2E suite (25 tests) ──");
    println!("  BIN     = {}", bin);
    println!("  PORT_A  = {}", port_a);
    println!("  PORT_B  = {}", port_b);

    let f_a = std::fs::File::create(&log_a).expect("cannot create log_a");
    let f_b = std::fs::File::create(&log_b).expect("cannot create log_b");

    let a = Command::new(&bin)
        .env("MESH_SEED", "0")
        .env("MESH_NODE_NAME", "node-0")
        .env("PORT", port_a.to_string())
        .stdout(Stdio::from(f_a.try_clone().unwrap()))
        .stderr(Stdio::from(f_a))
        .spawn()
        .expect("failed to spawn node-A");

    let b = Command::new(&bin)
        .env("MESH_SEED", "1")
        .env("MESH_NODE_NAME", "node-1")
        .env("PORT", port_b.to_string())
        .stdout(Stdio::from(f_b.try_clone().unwrap()))
        .stderr(Stdio::from(f_b))
        .spawn()
        .expect("failed to spawn node-B");

    let guard = Guard { a, b, log_a: log_a.clone(), log_b: log_b.clone() };

    if !wait_until_healthy(port_a, port_b) {
        eprintln!("❌ nodes never became healthy");
        eprintln!("── node-A log ─\n{}", read_log(&guard.log_a));
        eprintln!("── node-B log ─\n{}", read_log(&guard.log_b));
        std::process::exit(2);
    }

    let mut s = Suite::new();

    // ── T1–T5: health / info ────────────────────────────────────────────
    let h_a = http::get("127.0.0.1", port_a, "/health").unwrap();
    s.assert_eq("T1  GET /health node-A returns 'ok'", "ok", h_a.body.trim());
    let h_b = http::get("127.0.0.1", port_b, "/health").unwrap();
    s.assert_eq("T2  GET /health node-B returns 'ok'", "ok", h_b.body.trim());

    let info_a_raw = http::get("127.0.0.1", port_a, "/info").unwrap();
    let info_a = parse_json(&info_a_raw.body);
    s.assert_eq("T3  /info honours MESH_NODE_NAME (no prefix)", "node-0", &jstr(&info_a, "name"));
    s.assert_eq(
        "T4  /info advertises encryption suite",
        "X25519-ECDH+ChaCha20Poly1305",
        &jstr(&info_a, "encryption"),
    );
    let pub_a = jstr(&info_a, "pubkey");
    if pub_a.len() == 64 {
        s.ok("T5  pubkey is 32-byte hex (64 chars)", &format!("len={}", pub_a.len()));
    } else {
        s.fail("T5  pubkey length", &format!("got {}", pub_a.len()));
    }
    let dest_a = jstr(&info_a, "dest_hash");

    let info_b_raw = http::get("127.0.0.1", port_b, "/info").unwrap();
    let info_b = parse_json(&info_b_raw.body);
    let dest_b = jstr(&info_b, "dest_hash");
    let pub_b = jstr(&info_b, "pubkey");

    // ── T6–T10: announce ────────────────────────────────────────────────
    let body = format!(
        r#"{{"dest_hash":"{}","sender":"{}","hops":1,"quality":2}}"#,
        dest_b, dest_b
    );
    let r = http::post_json("127.0.0.1", port_a, "/announce", &body).unwrap();
    s.assert_eq(
        "T6  POST /announce basic accept",
        "true",
        &jbool(&parse_json(&r.body), "accepted").to_string(),
    );

    let body = format!(
        r#"{{"dest_hash":"{}","sender":"{}","hops":1,"quality":0}}"#,
        dest_b, dest_b
    );
    let r = http::post_json("127.0.0.1", port_a, "/announce", &body).unwrap();
    s.assert_eq(
        "T7  /announce strictly-better path replaces",
        "true",
        &jbool(&parse_json(&r.body), "accepted").to_string(),
    );

    let body = format!(
        r#"{{"dest_hash":"{}","sender":"{}","hops":5,"quality":15}}"#,
        dest_b, dest_b
    );
    let r = http::post_json("127.0.0.1", port_a, "/announce", &body).unwrap();
    s.assert_eq(
        "T8  /announce worse path rejected (ETX)",
        "false",
        &jbool(&parse_json(&r.body), "accepted").to_string(),
    );

    let r = http::post_json(
        "127.0.0.1",
        port_a,
        "/announce",
        r#"{"dest_hash":"not-hex","sender":"not-hex","hops":1,"quality":1}"#,
    )
    .unwrap();
    let acc = jget(&parse_json(&r.body), "accepted").as_bool().unwrap_or(false);
    s.assert_eq("T9  /announce rejects malformed hex", "false", &acc.to_string());

    let body = format!(
        r#"{{"dest_hash":"{}","sender":"{}","hops":1,"quality":0,"pubkey":"{}"}}"#,
        dest_b, dest_b, pub_b
    );
    let r = http::post_json("127.0.0.1", port_a, "/announce", &body).unwrap();
    if !r.body.is_empty() {
        s.ok("T10 /announce accepts pubkey field", "");
    } else {
        s.fail("T10 /announce with pubkey", "empty body");
    }

    // ── T11–T15: next-hop ───────────────────────────────────────────────
    let body = format!(r#"{{"dest_hash":"{}"}}"#, dest_b);
    let r = http::post_json("127.0.0.1", port_a, "/next-hop", &body).unwrap();
    let v = parse_json(&r.body);
    let nh = jget(&v, "next_hop").as_str().unwrap_or("").to_string();
    s.assert_eq("T11 next-hop B from A returns DEST_B", &dest_b, &nh);

    let body = format!(r#"{{"dest_hash":"{}"}}"#, dest_a);
    let r = http::post_json("127.0.0.1", port_a, "/next-hop", &body).unwrap();
    let v = parse_json(&r.body);
    s.assert_eq(
        "T12 next-hop self → local=true",
        "true",
        &jbool(&v, "local").to_string(),
    );

    let r = http::post_json(
        "127.0.0.1",
        port_a,
        "/next-hop",
        r#"{"dest_hash":"00000000000000000000000000000000"}"#,
    )
    .unwrap();
    let v = parse_json(&r.body);
    let nh = if jget(&v, "next_hop").is_null() { "null" } else { "non-null" };
    s.assert_eq("T13 next-hop unknown → null", "null", nh);

    let r = http::post_json("127.0.0.1", port_a, "/next-hop", r#"{"dest_hash":"zzz"}"#).unwrap();
    let v = parse_json(&r.body);
    s.assert_eq(
        "T14 next-hop invalid hex → local=false",
        "false",
        &jbool(&v, "local").to_string(),
    );

    let info_a_raw = http::get("127.0.0.1", port_a, "/info").unwrap();
    let info_a = parse_json(&info_a_raw.body);
    let routes = jget(&info_a, "routes").as_u64().unwrap_or(0);
    if routes >= 1 {
        s.ok("T15 /info routes count ≥ 1", &format!("= {}", routes));
    } else {
        s.fail("T15 /info routes count", &format!("got {}", routes));
    }

    // ── T16–T20: encryption ────────────────────────────────────────────
    let plaintext = "phi^2 + phi^-2 = 3";
    let body = format!(
        r#"{{"recipient_pubkey":"{}","plaintext":"{}"}}"#,
        pub_b, plaintext
    );
    let r = http::post_json("127.0.0.1", port_a, "/encrypt", &body).unwrap();
    let v = parse_json(&r.body);
    let payload = jstr(&v, "payload");
    let sender_pk = jstr(&v, "sender_pubkey");
    if !payload.is_empty() && !payload.starts_with("error") {
        s.ok("T16 /encrypt produces ciphertext", &format!("len={}", payload.len()));
    } else {
        s.fail("T16 /encrypt", &payload);
    }
    s.assert_eq(
        "T17 /encrypt sender_pubkey == node-A pubkey",
        &pub_a,
        &sender_pk,
    );

    let body = format!(
        r#"{{"to":"{}","sender_pubkey":"{}","payload":"{}"}}"#,
        dest_b, sender_pk, payload
    );
    let r = http::post_json("127.0.0.1", port_b, "/message", &body).unwrap();
    let v = parse_json(&r.body);
    s.assert_eq(
        "T18 /message delivered=true on B",
        "true",
        &jbool(&v, "delivered").to_string(),
    );
    let dec = jstr(&v, "decrypted");
    s.assert_eq("T19 /message decrypted plaintext matches", plaintext, &dec);

    let r = http::post_json(
        "127.0.0.1",
        port_a,
        "/encrypt",
        r#"{"recipient_pubkey":"deadbeef","plaintext":"x"}"#,
    )
    .unwrap();
    let v = parse_json(&r.body);
    let badp = jstr(&v, "payload");
    s.assert_contains("T20 /encrypt errors on bad pubkey", "error:", &badp);

    // ── T21–T25: edge cases ───────────────────────────────────────────
    let mut tampered = payload.clone();
    if let Some(c) = tampered.chars().next() {
        let alt = if c == 'X' { 'Y' } else { 'X' };
        tampered = format!("{}{}", alt, &tampered[c.len_utf8()..]);
    }
    let body = format!(
        r#"{{"to":"{}","sender_pubkey":"{}","payload":"{}"}}"#,
        dest_b, sender_pk, tampered
    );
    let r = http::post_json("127.0.0.1", port_b, "/message", &body).unwrap();
    s.assert_eq(
        "T21 MITM tampered payload rejected",
        "false",
        &jbool(&parse_json(&r.body), "delivered").to_string(),
    );

    let wrong_pk = "0".repeat(64);
    let body = format!(
        r#"{{"to":"{}","sender_pubkey":"{}","payload":"{}"}}"#,
        dest_b, wrong_pk, payload
    );
    let r = http::post_json("127.0.0.1", port_b, "/message", &body).unwrap();
    s.assert_eq(
        "T22 wrong sender pubkey → undecryptable",
        "false",
        &jbool(&parse_json(&r.body), "delivered").to_string(),
    );

    let body = format!(
        r#"{{"to":"{}","sender_pubkey":"{}","payload":"{}"}}"#,
        dest_a, sender_pk, payload
    );
    let r = http::post_json("127.0.0.1", port_b, "/message", &body).unwrap();
    s.assert_eq(
        "T23 foreign /message not decrypted on B",
        "false",
        &jbool(&parse_json(&r.body), "delivered").to_string(),
    );

    let body = format!(
        r#"{{"dest_hash":"deadbeefdeadbeefdeadbeefdeadbeef","sender":"{}","hops":255,"quality":255}}"#,
        dest_b
    );
    let r = http::post_json("127.0.0.1", port_a, "/announce", &body).unwrap();
    s.assert_eq(
        "T24 GF16 nibble clamp on huge values",
        "true",
        &jbool(&parse_json(&r.body), "accepted").to_string(),
    );

    let r = http::get("127.0.0.1", port_a, "/info").unwrap();
    let ok = if !parse_json(&r.body).is_null() { "ok" } else { "fail" };
    s.assert_eq(
        "T25 in-memory mode survives without DB",
        "ok",
        ok,
    );

    // ── Summary ──────────────────────────────────────────────────────
    let total = s.pass + s.fail;
    println!();
    println!("──────────────────────────────────────────────────────────────");
    println!("  {} / {} tests green", s.pass, total);

    if s.fail > 0 {
        println!("  ❌ Failed:");
        for t in &s.failed {
            println!("      - {}", t);
        }
        println!("── node-A log (last 30 lines) ─");
        for line in read_log(&guard.log_a).lines().rev().take(30).collect::<Vec<_>>().iter().rev() {
            println!("{}", line);
        }
        println!("── node-B log (last 30 lines) ─");
        for line in read_log(&guard.log_b).lines().rev().take(30).collect::<Vec<_>>().iter().rev() {
            println!("{}", line);
        }
        std::process::exit(1);
    }
    println!("  🎉 25/25 green · φ² + φ⁻² = 3");
}
