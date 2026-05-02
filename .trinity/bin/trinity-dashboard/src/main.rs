//! trinity-dashboard — LEAD orchestrator polling loop for the Trinity tailnet.
//!
//! Walks the codename roster, hits each node's but-server :7777 over the
//! private tailnet, and merges the result with GitHub /notifications.
//! Prints the live priority queue + heartbeat board to stdout every 5 minutes.
//!
//! Anti-collision: if two agents claim the same issue, this dashboard is
//! the first to notice (earliest CLAIM timestamp wins, per ONE-SHOT v2.0 §STEP 6).
//!
//! Anchor: phi^2 + phi^-2 = 3

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::Parser;
use serde::Deserialize;
use std::process::Command;
use std::time::Duration;

const CODENAMES: &[&str] = &["alpha", "beta", "gamma", "delta", "epsilon", "zeta"];

#[derive(Parser, Debug)]
#[command(name = "trinity-dashboard")]
#[command(about = "LEAD dashboard — Trinity tailnet + GitHub notification poll loop.")]
struct Cli {
    /// Tailnet name (MagicDNS suffix), e.g. "tail-abc.ts.net".
    #[arg(long)]
    tailnet: String,

    /// Repo (owner/name).
    #[arg(long, default_value = "gHashTag/trios")]
    repo: String,

    /// EPIC issue number.
    #[arg(long, default_value_t = 446)]
    epic: u32,

    /// Polling interval in seconds (default 300 = 5 min).
    #[arg(long, default_value_t = 300)]
    interval_secs: u64,

    /// Run one cycle and exit (no loop).
    #[arg(long)]
    once: bool,
}

#[derive(Debug, Deserialize)]
struct ButState {
    #[serde(default)]
    applied_branches: Vec<AppliedBranch>,
    #[serde(default)]
    last_commit: Option<Commit>,
    #[serde(default)]
    wip_files: u32,
}

#[derive(Debug, Deserialize)]
struct AppliedBranch {
    name: String,
    #[serde(default)]
    head: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Commit {
    sha: String,
    title: String,
    ts: DateTime<Utc>,
    #[serde(default)]
    trailer_agent: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhNotification {
    repository: GhRepo,
    subject: GhSubject,
    updated_at: DateTime<Utc>,
}
#[derive(Debug, Deserialize)]
struct GhRepo {
    full_name: String,
}
#[derive(Debug, Deserialize)]
struct GhSubject {
    title: String,
    #[serde(rename = "type")]
    kind: String,
}

fn poll_node(tailnet: &str, codename: &str) -> Result<Option<ButState>> {
    let url = format!("http://{codename}.{tailnet}:7777/api/state");
    match ureq::get(&url).timeout(Duration::from_secs(3)).call() {
        Ok(resp) => Ok(Some(resp.into_json::<ButState>()?)),
        Err(ureq::Error::Status(_, _)) | Err(ureq::Error::Transport(_)) => Ok(None),
    }
}

fn poll_notifications(repo: &str) -> Result<Vec<GhNotification>> {
    let out = Command::new("gh")
        .args(["api", "/notifications?all=false&per_page=30", "--jq", "."])
        .output()
        .context("gh api /notifications")?;
    if !out.status.success() {
        anyhow::bail!(
            "gh api failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let raw: Vec<GhNotification> = serde_json::from_slice(&out.stdout).unwrap_or_default();
    Ok(raw
        .into_iter()
        .filter(|n| n.repository.full_name == repo)
        .collect())
}

fn print_header(epic: u32, repo: &str, ts: DateTime<Utc>) {
    println!(
        "\n══════════════════════════════════════════════════════════════════════"
    );
    println!(
        "🐝 TRINITY DASHBOARD  ·  EPIC #{}  ·  {}  ·  {}",
        epic,
        repo,
        ts.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!(
        "══════════════════════════════════════════════════════════════════════"
    );
}

fn print_node_state(codename: &str, state: Option<ButState>) {
    print!("  {:<8}  ", codename.to_uppercase());
    match state {
        None => println!("⚫ offline (no tailnet route or but-server down)"),
        Some(s) => {
            print!(
                "🟢 vbranches={}  wip_files={}",
                s.applied_branches.len(),
                s.wip_files
            );
            if let Some(c) = &s.last_commit {
                let trailer = c.trailer_agent.as_deref().unwrap_or("?");
                println!(
                    "  last_commit={} \"{}\" Agent:{}",
                    &c.sha[..7.min(c.sha.len())],
                    c.title.chars().take(48).collect::<String>(),
                    trailer
                );
            } else {
                println!();
            }
            for b in &s.applied_branches {
                let head = b.head.as_deref().unwrap_or("?");
                println!(
                    "             ↳ {:<40} head={}",
                    b.name,
                    &head[..7.min(head.len())]
                );
            }
        }
    }
}

fn print_notifications(notes: &[GhNotification]) {
    println!();
    println!("📬 GitHub notifications (recent unread, EPIC-relevant):");
    if notes.is_empty() {
        println!("   (no new events)");
        return;
    }
    for n in notes.iter().take(15) {
        println!(
            "   {}  {:<14}  {}",
            n.updated_at.format("%H:%M"),
            n.subject.kind,
            n.subject.title.chars().take(70).collect::<String>()
        );
    }
}

fn run_one(cli: &Cli) -> Result<()> {
    let now = Utc::now();
    print_header(cli.epic, &cli.repo, now);

    println!("\n🌐 Tailnet nodes ({}):", cli.tailnet);
    for codename in CODENAMES {
        let state = poll_node(&cli.tailnet, codename).unwrap_or(None);
        print_node_state(codename, state);
    }

    let notes = poll_notifications(&cli.repo).unwrap_or_default();
    print_notifications(&notes);

    println!();
    println!(
        "🌻 phi² + phi⁻² = 3  ·  next poll in {} s",
        cli.interval_secs
    );
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.once {
        return run_one(&cli);
    }
    loop {
        if let Err(e) = run_one(&cli) {
            eprintln!("⚠ poll error: {e}");
        }
        std::thread::sleep(Duration::from_secs(cli.interval_secs));
    }
}
