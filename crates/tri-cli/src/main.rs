use anyhow::Result;
use clap::Parser;
use std::process::{Child, Command};
use tokio::time::{sleep, Duration};

/// tri — Unified CLI for trios-server + Tailscale Funnel
#[derive(Parser, Debug)]
#[command(name = "tri")]
#[command(about = "One command to run trios-server with cloud access", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Start trios-server + Funnel (default)
    Start {
        #[arg(long, default_value = "9005")]
        port: u16,
    },
    /// Stop everything
    Stop,
    /// Show status
    Status,
    /// Article build / QA service (docs/articles/<slug>/).
    ///
    /// Thin wrapper around the repo-native Node runner at
    /// docs/articles/_runner/src/main.mjs. The Rust binary preserves the
    /// `tri article ...` command surface declared in
    /// docs/articles/<slug>/README.md and forwards all positional and
    /// flag arguments verbatim to the runner.
    Article {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

const TRIOS_SERVER: &str = "trios-server";
const TUNNEL_CLI: &str = "tri-tunnel";

async fn start_server_and_tunnel(port: u16) -> Result<()> {
    // Start trios-server
    println!("🚀 Starting trios-server on port {}...", port);
    let server = Command::new("cargo")
        .args(["run", "-p", TRIOS_SERVER])
        .env("TRIOS_PORT", port.to_string())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    // Give server time to start
    sleep(Duration::from_secs(3)).await;

    // Check if Funnel is already running
    let status_check = Command::new("cargo")
        .args(["run", "-p", TUNNEL_CLI, "--", "status"])
        .output()?;

    let funnel_running = status_check.status.success()
        && String::from_utf8_lossy(&status_check.stdout).contains("ACTIVE");

    if funnel_running {
        println!("✅ Funnel already running!");
    } else {
        // Start Funnel
        println!("🌐 Starting Tailscale Funnel...");
        Command::new("cargo")
            .args(["run", "-p", TUNNEL_CLI, "--", "start", "--port", &port.to_string()])
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()?;
        sleep(Duration::from_secs(3)).await;
    }

    // Show status
    let status = Command::new("cargo")
        .args(["run", "-p", TUNNEL_CLI, "--", "status"])
        .output()?;

    if status.status.success() {
        println!("\n✅ tri cloud is running!");
        println!("{}", String::from_utf8_lossy(&status.stdout));
        println!("\n📡 Your trios-server is now accessible from anywhere!");
        println!("📝 Press Ctrl+C to stop");
    }

    // Keep server running
    println!("\n🟢 Server running locally at http://localhost:{}", port);
    wait_for_server(server).await?;

    Ok(())
}

async fn stop_all() -> Result<()> {
    println!("🛑 Stopping tri cloud...");

    // Stop Funnel
    let _ = Command::new("cargo")
        .args(["run", "-p", TUNNEL_CLI, "--", "stop"])
        .output();

    // Find and kill trios-server
    let _ = Command::new("pkill")
        .args(["-f", "trios-server"])
        .output();

    println!("✅ Stopped!");
    Ok(())
}

async fn show_status() -> Result<()> {
    let mut status = Command::new("cargo")
        .args(["run", "-p", TUNNEL_CLI, "--", "status"])
        .spawn()?;

    let _ = status.wait();
    Ok(())
}

async fn wait_for_server(mut server: Child) -> Result<()> {
    let _ = server.wait();
    println!("\n🛑 trios-server stopped");
    Ok(())
}

/// Locate the repo-native Node article runner.
///
/// The runner lives at `docs/articles/_runner/src/main.mjs` relative to
/// the repository root. We walk up from the manifest dir (cargo sets
/// `CARGO_MANIFEST_DIR` to `crates/tri-cli`) until we find it; we also
/// honour `TRIOS_ARTICLE_RUNNER` for ad-hoc overrides.
fn locate_article_runner() -> Result<std::path::PathBuf> {
    use std::path::PathBuf;
    if let Ok(p) = std::env::var("TRIOS_ARTICLE_RUNNER") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Ok(pb);
        }
    }
    // Manifest dir → repo root is two `..` up.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let mut here = PathBuf::from(manifest);
    for _ in 0..6 {
        let candidate = here.join("docs/articles/_runner/src/main.mjs");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !here.pop() {
            break;
        }
    }
    // Fall back to cwd-based lookup.
    let cwd = std::env::current_dir()?;
    let candidate = cwd.join("docs/articles/_runner/src/main.mjs");
    if candidate.exists() {
        return Ok(candidate);
    }
    anyhow::bail!(
        "article runner not found at docs/articles/_runner/src/main.mjs (set TRIOS_ARTICLE_RUNNER to override)"
    )
}

async fn run_article(args: Vec<String>) -> Result<()> {
    let runner = locate_article_runner()?;
    let status = Command::new("node")
        .arg(&runner)
        .args(&args)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;
    if !status.success() {
        anyhow::bail!(
            "tri article: runner exited with {}",
            status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into())
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle Ctrl+C gracefully
    let ctrl_c = tokio::signal::ctrl_c();

    tokio::select! {
        _ = ctrl_c => {
            let _ = stop_all().await;
            std::process::exit(0);
        }
        result = async {
            match cli.command.unwrap_or(Commands::Start { port: 9005 }) {
                Commands::Start { port } => start_server_and_tunnel(port).await,
                Commands::Stop => stop_all().await,
                Commands::Status => show_status().await,
                Commands::Article { args } => run_article(args).await,
            }
        } => {
            if let Err(e) = result {
                eprintln!("❌ Error: {:#}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
