//! trinity-bootstrap — provision one agent on one node for the EPIC #446 ring-refactor.
//!
//! Anchor: phi^2 + phi^-2 = 3
//! Constitutional reference: gHashTag/trios LAWS.md v2.0, AGENTS.md (codenames + I-SCOPE),
//! ONE-SHOT v2.0 dispatch (#236), EPIC #446.
//!
//! Anti-collision is *network-enforced* via Tailscale tags:
//! - tag:trinity-<codename> on each node
//! - one node per codename, one workspace per node
//! - each agent owns one git clone + one GitButler virtual branch
//! - inter-agent traffic routes via but-server :7777 on tailscale0 (L24)

use anyhow::{Context, Result, bail};
use chrono::Utc;
use clap::{Parser, ValueEnum};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "UPPER")]
enum Codename {
    Alpha,
    Beta,
    Gamma,
    Delta,
    Epsilon,
    Zeta,
    Lead,
}

impl Codename {
    fn tag(self) -> &'static str {
        match self {
            Codename::Alpha => "tag:trinity-alpha",
            Codename::Beta => "tag:trinity-beta",
            Codename::Gamma => "tag:trinity-gamma",
            Codename::Delta => "tag:trinity-delta",
            Codename::Epsilon => "tag:trinity-epsilon",
            Codename::Zeta => "tag:trinity-zeta",
            Codename::Lead => "tag:trinity-lead",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Codename::Alpha => "ALPHA",
            Codename::Beta => "BETA",
            Codename::Gamma => "GAMMA",
            Codename::Delta => "DELTA",
            Codename::Epsilon => "EPSILON",
            Codename::Zeta => "ZETA",
            Codename::Lead => "LEAD",
        }
    }
    /// Default crate domain — enforces I-SCOPE.
    fn allowed_crate_globs(self) -> &'static [&'static str] {
        match self {
            Codename::Alpha => &["crates/trios-igla-race-pipeline/**"],
            Codename::Beta => &["crates/trios-algorithm-arena/**"],
            Codename::Gamma => &["crates/trios-igla-race-hack/**"],
            Codename::Delta => &["crates/trios-doctor/rings/SILVER-RING-DR-04/**"],
            Codename::Epsilon => &["crates/trios-ext/**"],
            Codename::Zeta => &["crates/trios-agent-memory/**"],
            Codename::Lead => &[".trinity/**", "docs/golden-sunflowers/**"],
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "trinity-bootstrap")]
#[command(about = "Provision one Trinity agent on one Tailscale node for EPIC #446.")]
#[command(version)]
struct Cli {
    /// Agent codename (ALPHA|BETA|GAMMA|DELTA|EPSILON|ZETA|LEAD).
    #[arg(long, value_enum)]
    codename: Codename,

    /// GitHub issue number this agent will claim (e.g. 448 for SR-00 scarab-types).
    #[arg(long)]
    issue: u32,

    /// Soul-name (humorous English, one or two words). Required by L11.
    #[arg(long)]
    soul: String,

    /// Repository owner/name (default: gHashTag/trios).
    #[arg(long, default_value = "gHashTag/trios")]
    repo: String,

    /// EPIC issue number (default: 446).
    #[arg(long, default_value_t = 446)]
    epic: u32,

    /// Tailscale tailnet name (e.g. "tail-abc.ts.net"). If unset, no DNS exposure.
    #[arg(long)]
    tailnet: Option<String>,

    /// Tailscale auth-key for non-interactive `tailscale up`. If unset, skip the up step.
    #[arg(long)]
    ts_authkey: Option<String>,

    /// Local but-server port (always bound to tailscale0). Default 7777.
    #[arg(long, default_value_t = 7777)]
    but_server_port: u16,

    /// Workspace root (where the clone lands). Default: ~/work/trinity-<CODENAME>-<issue>
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Dry run — print what would happen, do not mutate the system.
    #[arg(long)]
    dry_run: bool,
}

fn run(cmd: &str, args: &[&str], dry: bool) -> Result<String> {
    println!("$ {} {}", cmd, args.join(" "));
    if dry {
        return Ok(String::new());
    }
    let out = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("spawn `{}`", cmd))?;
    if !out.status.success() {
        bail!("`{} {}` exited with {}", cmd, args.join(" "), out.status);
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn require(tool: &str) -> Result<()> {
    which::which(tool).with_context(|| format!("required tool `{tool}` not on PATH"))?;
    Ok(())
}

fn slug(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .replace("--", "-")
}

fn task_md_seal(issue: u32, codename: Codename, soul: &str, allowed: &[&str]) -> (String, String) {
    let body = format!(
        "# TASK #{issue}\n\nSoul: {soul}\nCodename: {label}\nTailscale tag: {tag}\nEPIC: #446\n\n## Allowed scope (I-SCOPE)\n\n{scope}\n\n## PHI LOOP step\n\nCLAIM\n",
        label = codename.label(),
        tag = codename.tag(),
        scope = allowed.iter().map(|p| format!("- `{p}`")).collect::<Vec<_>>().join("\n"),
    );
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    (body, hex::encode(h.finalize()))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let started = Utc::now().to_rfc3339();
    println!("🐝 trinity-bootstrap v0.1.0  ·  Codename: {}  ·  Issue: #{}  ·  Soul: {}",
             cli.codename.label(), cli.issue, cli.soul);
    println!("   started:  {started}");
    println!("   anchor:   phi^2 + phi^-2 = 3");
    println!();

    // 0. Sanity check — required tools.
    println!("== Step 0: required tooling ==");
    for tool in ["git", "gh", "but"] {
        match require(tool) {
            Ok(_) => println!("   ✓ {tool}"),
            Err(e) => {
                if cli.dry_run {
                    println!("   (dry) would require {tool}: {e}");
                } else {
                    bail!(e);
                }
            }
        }
    }
    println!();

    let workspace = cli
        .workspace
        .clone()
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(format!("work/trinity-{}-{}", cli.codename.label(), cli.issue))
        });
    let branch = format!("bee/{}-{}", cli.issue, slug(&cli.soul));

    // 1. Tailscale up with the agent's tag (network-enforced isolation).
    println!("== Step 1: Tailscale tag ==");
    if let Some(authkey) = cli.ts_authkey.as_deref() {
        run(
            "sudo",
            &[
                "tailscale",
                "up",
                "--reset",
                "--ssh",
                &format!("--advertise-tags={}", cli.codename.tag()),
                &format!("--authkey={authkey}"),
            ],
            cli.dry_run,
        )?;
    } else {
        println!("   (skipping `tailscale up` — no --ts-authkey given)");
    }
    println!();

    // 2. Clone repo into private workspace.
    println!("== Step 2: clone {} → {} ==", cli.repo, workspace.display());
    run(
        "git",
        &[
            "clone",
            &format!("https://github.com/{}.git", cli.repo),
            workspace.to_str().unwrap(),
        ],
        cli.dry_run,
    )?;
    println!();

    // 3. Verify issue is claimable + post CLAIM (L11/§STEP 1).
    println!("== Step 3: verify issue and post CLAIM ==");
    let claim_msg = format!(
        "IN-FLIGHT — Soul: {soul} · Codename: {label} · Branch: {branch}",
        soul = cli.soul,
        label = cli.codename.label(),
    );
    println!("   probing #{} comments for prior IN-FLIGHT…", cli.issue);
    let comments = run(
        "gh",
        &[
            "issue",
            "view",
            &cli.issue.to_string(),
            "--repo",
            &cli.repo,
            "--json",
            "comments",
            "--jq",
            ".comments[] | .body",
        ],
        cli.dry_run,
    )?;
    if comments.contains("IN-FLIGHT —") {
        bail!(
            "issue #{} already has an IN-FLIGHT claim — choose another issue",
            cli.issue
        );
    }
    run(
        "gh",
        &[
            "issue",
            "comment",
            &cli.issue.to_string(),
            "--repo",
            &cli.repo,
            "--body",
            &claim_msg,
        ],
        cli.dry_run,
    )?;
    println!("   ✓ CLAIM posted: {claim_msg}");
    println!();

    // 4. cd into workspace.
    if !cli.dry_run {
        std::env::set_current_dir(&workspace).context("cd workspace")?;
    }

    // 5. Create branch + write TASK.md (L12) + seal SHA-256 (LAWS.md §7 step 4).
    println!("== Step 4: SPEC + SEAL ==");
    run("git", &["checkout", "-b", &branch], cli.dry_run)?;
    let allowed = cli.codename.allowed_crate_globs();
    let scope_dir = match cli.codename {
        Codename::Alpha => "crates/trios-igla-race-pipeline",
        Codename::Beta => "crates/trios-algorithm-arena",
        Codename::Gamma => "crates/trios-igla-race-hack",
        Codename::Zeta => "crates/trios-agent-memory",
        Codename::Delta => "crates/trios-doctor/rings/SILVER-RING-DR-04",
        _ => ".trinity",
    };
    run("mkdir", &["-p", scope_dir], cli.dry_run)?;
    let (task_body, task_sha) = task_md_seal(cli.issue, cli.codename, &cli.soul, allowed);
    let task_path = format!("{scope_dir}/TASK.md");
    if !cli.dry_run {
        std::fs::write(&task_path, &task_body)?;
    }
    println!("   ✓ wrote {task_path}  sha256={task_sha}");
    println!();

    // 6. GitButler init + virtual branch + but-server on tailscale0.
    println!("== Step 5: GitButler virtual branch ==");
    run("but", &["project", "add", "."], cli.dry_run)?;
    let vbranch = format!("{}/{}", cli.codename.label().to_lowercase(), branch);
    run("but", &["vbranch", "create", &vbranch], cli.dry_run)?;
    run("but", &["vbranch", "apply", &vbranch], cli.dry_run)?;

    let listen_iface = match cli.tailnet.as_deref() {
        Some(_) => "tailscale0",
        None => "127.0.0.1",
    };
    let port_arg = format!("--port={}", cli.but_server_port);
    let listen_arg = format!("--listen={listen_iface}");
    println!("   starting but-server (background)…");
    if cli.dry_run {
        println!("   (dry) would run: but server start {port_arg} {listen_arg}");
    } else {
        Command::new("but")
            .args(["server", "start", &port_arg, &listen_arg])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("spawn but-server")?;
    }
    println!();

    // 7. Initial commit (L8 push-first + L14 trailer).
    println!("== Step 6: COMMIT + PUSH (initial) ==");
    run("git", &["add", "-A"], cli.dry_run)?;
    let commit_msg = format!(
        "feat({scope}): claim #{issue} via trinity-bootstrap\n\nSoul: {soul}\nTASK.md sha256: {sha}\n\nPart of #{epic}\n\nAgent: {label}\n",
        scope = scope_dir.split('/').nth(1).unwrap_or("trinity"),
        issue = cli.issue,
        soul = cli.soul,
        sha = task_sha,
        epic = cli.epic,
        label = cli.codename.label(),
    );
    run("git", &["commit", "-m", &commit_msg], cli.dry_run)?;
    run("git", &["push", "-u", "origin", &branch], cli.dry_run)?;
    println!();

    // 8. Final heartbeat.
    println!("== Step 7: HEARTBEAT ==");
    let hb = format!(
        "loop: {label} | 🟢 ACTIVE | {soul} · CLAIM/SEAL · provisioned via trinity-bootstrap\n\nevidence:\n  ts:               {ts}\n  issue:            #{issue}\n  epic:             #{epic}\n  loop:             CLAIM → SPEC → SEAL\n  task_md_sha256:   {sha}\n  branch:           {branch}\n  vbranch:          {vbranch}\n  but_server:       http://0.0.0.0:{port}/  (listen on {iface})\n  next:             GEN — implement acceptance criteria for #{issue}\n",
        label = cli.codename.label(),
        soul = cli.soul,
        ts = started,
        issue = cli.issue,
        epic = cli.epic,
        sha = task_sha,
        branch = branch,
        vbranch = vbranch,
        port = cli.but_server_port,
        iface = listen_iface,
    );
    run(
        "gh",
        &[
            "issue",
            "comment",
            &cli.issue.to_string(),
            "--repo",
            &cli.repo,
            "--body",
            &format!("```\n{hb}\n```\n\n🌻 phi² + phi⁻² = 3"),
        ],
        cli.dry_run,
    )?;
    println!("   ✓ heartbeat posted to #{}", cli.issue);
    println!();

    println!("🌻 trinity-bootstrap done. Workspace: {}", workspace.display());
    println!("   next:  cd {workspace}  &&  enter PHI LOOP step GEN.", workspace = workspace.display());
    Ok(())
}
