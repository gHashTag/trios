//! trios-deploy — деплой-тулинг боевой связки (L1: никаких .sh).
//!
//! Подкоманды:
//! - `render [--repo P] [--bin-dir P] [--port N] [--cdp-http URL] [--out DIR]`
//!   — рендер launchd-плистов из deploy/launchd/*.plist.tmpl;
//! - `install` — macOS: render → ~/Library/LaunchAgents → launchctl bootstrap;
//! - `uninstall` — macOS: launchctl bootout + удаление плистов;
//! - `status` — macOS: launchctl print по обоим юнитам;
//! - `smoke [--host H] [--port N]` — проверка живого сервера:
//!   /health, /agent/tools, WS browser/poll, маршрут /agent/run.
//!
//! Прод-порт: 9105 (см. tasks/INVARIANTS.md I8), CDP BrowserOS: 9102.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const SERVER_TMPL: &str = include_str!("../../../../deploy/launchd/com.trios.server.plist.tmpl");
const HOST_CDP_TMPL: &str =
    include_str!("../../../../deploy/launchd/com.trios.host-cdp.plist.tmpl");
const UNITS: [&str; 2] = ["com.trios.server", "com.trios.host-cdp"];

const DEFAULT_PORT: u16 = 9105;
const DEFAULT_CDP_HTTP: &str = "http://127.0.0.1:9102";

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

struct RenderConfig {
    repo: PathBuf,
    bin_dir: PathBuf,
    port: u16,
    cdp_http: String,
    out: PathBuf,
    home: PathBuf,
}

impl RenderConfig {
    fn from_args(args: &[String]) -> Result<Self> {
        let home = PathBuf::from(std::env::var("HOME").context("HOME is not set")?);
        let repo = arg_value(args, "--repo")
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .context("cannot resolve repo dir")?;
        let bin_dir = arg_value(args, "--bin-dir")
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("target/release"));
        let port = arg_value(args, "--port")
            .and_then(|p| p.parse().ok())
            .unwrap_or(DEFAULT_PORT);
        let cdp_http =
            arg_value(args, "--cdp-http").unwrap_or_else(|| DEFAULT_CDP_HTTP.to_string());
        let out = arg_value(args, "--out")
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join("deploy/launchd/rendered"));
        Ok(Self { repo, bin_dir, port, cdp_http, out, home })
    }

    fn render(&self, template: &str) -> String {
        template
            .replace("__REPO__", &self.repo.display().to_string())
            .replace("__BIN_DIR__", &self.bin_dir.display().to_string())
            .replace("__PORT__", &self.port.to_string())
            .replace("__CDP_HTTP__", &self.cdp_http)
            .replace("__HOME__", &self.home.display().to_string())
    }

    fn write_units(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        std::fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
        let mut written = Vec::new();
        for (unit, template) in UNITS.iter().zip([SERVER_TMPL, HOST_CDP_TMPL]) {
            let rendered = self.render(template);
            if rendered.contains("__") {
                bail!("unresolved placeholder in {unit}");
            }
            let path = dir.join(format!("{unit}.plist"));
            std::fs::write(&path, rendered).with_context(|| format!("write {}", path.display()))?;
            written.push(path);
        }
        Ok(written)
    }
}

fn ensure_macos(action: &str) -> Result<()> {
    if !cfg!(target_os = "macos") {
        bail!("`{action}` is macOS-only (launchd); use `render` + your init system elsewhere");
    }
    Ok(())
}

fn launchctl(args: &[&str]) -> Result<String> {
    let out = Command::new("launchctl").args(args).output().context("run launchctl")?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if !out.status.success() {
        bail!("launchctl {args:?} failed: {text}");
    }
    Ok(text)
}

fn uid() -> String {
    // launchctl bootstrap gui/$UID — resolve via id -u (std has no getuid).
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "501".into())
}

fn install(config: &RenderConfig) -> Result<()> {
    ensure_macos("install")?;
    std::fs::create_dir_all(config.home.join("Library/Logs/trios")).ok();
    let agents = config.home.join("Library/LaunchAgents");
    let units = config.write_units(&agents)?;
    let domain = format!("gui/{}", uid());
    for path in &units {
        // Re-bootstrap idempotently: bootout may fail if not loaded — ignore.
        let _ = Command::new("launchctl")
            .args(["bootout", &domain, &path.display().to_string()])
            .output();
        launchctl(&["bootstrap", &domain, &path.display().to_string()])?;
        println!("bootstrapped {}", path.display());
    }
    for unit in UNITS {
        launchctl(&["kickstart", "-k", &format!("{domain}/{unit}")])?;
        println!("kickstarted {unit}");
    }
    Ok(())
}

fn uninstall(config: &RenderConfig) -> Result<()> {
    ensure_macos("uninstall")?;
    let domain = format!("gui/{}", uid());
    for unit in UNITS {
        let path = config.home.join(format!("Library/LaunchAgents/{unit}.plist"));
        let _ = Command::new("launchctl")
            .args(["bootout", &domain, &path.display().to_string()])
            .output();
        let _ = std::fs::remove_file(&path);
        println!("removed {unit}");
    }
    Ok(())
}

fn status() -> Result<()> {
    ensure_macos("status")?;
    let domain = format!("gui/{}", uid());
    for unit in UNITS {
        match launchctl(&["print", &format!("{domain}/{unit}")]) {
            Ok(text) => {
                let state = text
                    .lines()
                    .find(|l| l.contains("state ="))
                    .unwrap_or("state = ?")
                    .trim();
                println!("{unit}: {state}");
            }
            Err(_) => println!("{unit}: not loaded"),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// smoke
// ---------------------------------------------------------------------------

async fn smoke(host: &str, port: u16) -> Result<()> {
    let base = format!("http://{host}:{port}");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let mut failures = 0usize;

    // 1. /health
    match client.get(format!("{base}/health")).send().await {
        Ok(r) if r.status().is_success() => println!("OK   /health"),
        other => {
            failures += 1;
            println!("FAIL /health: {other:?}");
        }
    }

    // 2. /agent/tools must advertise builtin + browser tools.
    match client.get(format!("{base}/agent/tools")).send().await {
        Ok(r) if r.status().is_success() => {
            let body: serde_json::Value = r.json().await.unwrap_or_default();
            let names: Vec<String> = body["tools"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|t| t["name"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            if names.iter().any(|n| n == "browser_goto") && names.iter().any(|n| n == "echo") {
                println!("OK   /agent/tools ({} tools)", names.len());
            } else {
                failures += 1;
                println!("FAIL /agent/tools: unexpected tool set {names:?}");
            }
        }
        other => {
            failures += 1;
            println!("FAIL /agent/tools: {other:?}");
        }
    }

    // 3. WS /ws: browser/poll round trip (empty queue is fine).
    match smoke_ws_poll(host, port).await {
        Ok(()) => println!("OK   ws browser/poll"),
        Err(err) => {
            failures += 1;
            println!("FAIL ws browser/poll: {err}");
        }
    }

    // 4. /agent/run is wired (bogus provider → 4xx/5xx JSON error, not 404).
    match client
        .post(format!("{base}/agent/run"))
        .json(&serde_json::json!({
            "prompt": "smoke",
            "max_steps": 1,
            "provider": {"base_url": "http://127.0.0.1:1/v1", "model": "smoke"}
        }))
        .send()
        .await
    {
        Ok(r) if r.status().as_u16() == 404 => {
            failures += 1;
            println!("FAIL /agent/run: 404 (route not mounted)");
        }
        Ok(r) => println!("OK   /agent/run route (status {})", r.status()),
        Err(err) => {
            failures += 1;
            println!("FAIL /agent/run: {err}");
        }
    }

    if failures > 0 {
        bail!("smoke failed: {failures} check(s)");
    }
    println!("SMOKE PASSED on {base}");
    Ok(())
}

async fn smoke_ws_poll(host: &str, port: u16) -> Result<()> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{host}:{port}/ws")).await?;
    ws.send(Message::Text(
        serde_json::json!({"method": "browser/poll", "params": {"agent_id": "smoke-test"}})
            .to_string(),
    ))
    .await?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if std::time::Instant::now() > deadline {
            bail!("no poll response within 10s");
        }
        let Some(msg) = ws.next().await else { bail!("ws closed") };
        let Message::Text(text) = msg? else { continue };
        let value: serde_json::Value = serde_json::from_str(&text)?;
        if value.get("event").is_some() {
            continue;
        }
        let commands = &value["result"]["commands"];
        if commands.is_array() {
            return Ok(());
        }
        bail!("unexpected poll response: {value}");
    }
}

// ---------------------------------------------------------------------------

fn usage() -> ! {
    eprintln!(
        "usage: trios-deploy <render|install|uninstall|status|smoke> \
         [--repo P] [--bin-dir P] [--port N] [--cdp-http URL] [--out DIR] [--host H]"
    );
    std::process::exit(2);
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else { usage() };
    match command.as_str() {
        "render" => {
            let config = RenderConfig::from_args(&args)?;
            let out = config.out.clone();
            for path in config.write_units(&out)? {
                println!("rendered {}", path.display());
            }
            Ok(())
        }
        "install" => install(&RenderConfig::from_args(&args)?),
        "uninstall" => uninstall(&RenderConfig::from_args(&args)?),
        "status" => status(),
        "smoke" => {
            let host = arg_value(&args, "--host").unwrap_or_else(|| "127.0.0.1".into());
            let port = arg_value(&args, "--port")
                .and_then(|p| p.parse().ok())
                .unwrap_or(DEFAULT_PORT);
            smoke(&host, port).await
        }
        _ => usage(),
    }
}
