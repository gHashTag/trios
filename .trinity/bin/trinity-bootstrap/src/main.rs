//! trinity-bootstrap — provision one of 27 Coptic-letter agents on one node for EPIC #446.
//!
//! Anchor: phi^2 + phi^-2 = 3
//! Constitutional reference: gHashTag/trios LAWS.md v2.0, AGENTS.md (codenames + I-SCOPE),
//! ONE-SHOT v2.0 dispatch (#236), EPIC #446.
//!
//! Anti-collision is *network-enforced* via Tailscale tags:
//! - tag:trinity-<codename> on each node (27 tags total)
//! - one node per codename, one workspace per node
//! - each agent owns one git clone + one GitButler virtual branch
//! - inter-agent traffic routes via but-server :7777 on tailscale0 (L24)
//! - LEAD (Omega) is the only agent that may reach the others on :7777

use anyhow::{Context, Result, bail};
use chrono::Utc;
use clap::{Parser, ValueEnum};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// 27 Coptic-letter agent domains. Each maps to one Tailscale tag + one crate scope.
#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "UPPER")]
enum Codename {
    Alpha,
    Beta,
    Gamma,
    Delta,
    Epsilon,
    Zeta,
    Eta,
    Theta,
    Iota,
    Kappa,
    Lambda,
    Mu,
    Nu,
    Xi,
    Omicron,
    Pi,
    Koppa,
    Rho,
    Sigma,
    Tau,
    Upsilon,
    Phi,
    Khi,
    Psi,
    Omega,
    Sampi,
    Sho,
}

impl Codename {
    fn label(self) -> &'static str {
        match self {
            Codename::Alpha => "ALPHA",
            Codename::Beta => "BETA",
            Codename::Gamma => "GAMMA",
            Codename::Delta => "DELTA",
            Codename::Epsilon => "EPSILON",
            Codename::Zeta => "ZETA",
            Codename::Eta => "ETA",
            Codename::Theta => "THETA",
            Codename::Iota => "IOTA",
            Codename::Kappa => "KAPPA",
            Codename::Lambda => "LAMBDA",
            Codename::Mu => "MU",
            Codename::Nu => "NU",
            Codename::Xi => "XI",
            Codename::Omicron => "OMICRON",
            Codename::Pi => "PI",
            Codename::Koppa => "KOPPA",
            Codename::Rho => "RHO",
            Codename::Sigma => "SIGMA",
            Codename::Tau => "TAU",
            Codename::Upsilon => "UPSILON",
            Codename::Phi => "PHI",
            Codename::Khi => "KHI",
            Codename::Psi => "PSI",
            Codename::Omega => "OMEGA",
            Codename::Sampi => "SAMPI",
            Codename::Sho => "SHO",
        }
    }
    fn coptic(self) -> &'static str {
        match self {
            Codename::Alpha => "Ⲁⲁ",
            Codename::Beta => "Ⲃⲃ",
            Codename::Gamma => "Ⲅⲅ",
            Codename::Delta => "Ⲇⲇ",
            Codename::Epsilon => "Ⲉⲉ",
            Codename::Zeta => "Ⲋⲋ",
            Codename::Eta => "Ⲍⲍ",
            Codename::Theta => "Ⲏⲏ",
            Codename::Iota => "Ⲑⲑ",
            Codename::Kappa => "Ⲓⲓ",
            Codename::Lambda => "Ⲕⲕ",
            Codename::Mu => "Ⲗⲗ",
            Codename::Nu => "Ⲙⲙ",
            Codename::Xi => "Ⲛⲛ",
            Codename::Omicron => "Ⲝⲝ",
            Codename::Pi => "Ⲟⲟ",
            Codename::Koppa => "Ⲡⲡ",
            Codename::Rho => "Ⲣⲣ",
            Codename::Sigma => "Ⲥⲥ",
            Codename::Tau => "Ⲧⲧ",
            Codename::Upsilon => "Ⲩⲩ",
            Codename::Phi => "Ⲫⲫ",
            Codename::Khi => "Ⲭⲭ",
            Codename::Psi => "Ⲯⲯ",
            Codename::Omega => "Ⲱⲱ",
            Codename::Sampi => "Ϣⲳ",
            Codename::Sho => "Ϥϥ",
        }
    }
    fn tag(self) -> String {
        format!("tag:trinity-{}", self.label().to_lowercase())
    }
    fn domain(self) -> &'static str {
        match self {
            Codename::Alpha => "Core bootstrapping, agent lifecycle",
            Codename::Beta => "Benchmarking, metrics collection",
            Codename::Gamma => "Git operations, version control",
            Codename::Delta => "Database, persistence layer",
            Codename::Epsilon => "Error handling, recovery",
            Codename::Zeta => "Zig compilation, VIBEE pipeline",
            Codename::Eta => "Event orchestration, hooks",
            Codename::Theta => "Testing, validation",
            Codename::Iota => "I18n, localization",
            Codename::Kappa => "Knowledge base, VSA operations",
            Codename::Lambda => "Learning, experience persistence",
            Codename::Mu => "Memory management, allocators",
            Codename::Nu => "Notification systems (Telegram)",
            Codename::Xi => "MCP server integration",
            Codename::Omicron => "Optimization, ASHA+PBT",
            Codename::Pi => "Pipeline orchestration",
            Codename::Koppa => "Compression, GF16 format",
            Codename::Rho => "Railway cloud deployment",
            Codename::Sigma => "Swarm intelligence",
            Codename::Tau => "Ternary VM execution",
            Codename::Upsilon => "UI components (Queen)",
            Codename::Phi => "Math, phi^2 + 1/phi^2 = 3",
            Codename::Khi => "CLI commands (310+)",
            Codename::Psi => "Privacy, PII detection",
            Codename::Omega => "Orchestration, final assembly (LEAD)",
            Codename::Sampi => "SACred intelligence, physics",
            Codename::Sho => "FPGA synthesis, Verilog",
        }
    }
    /// Default crate domain — enforces I-SCOPE.
    fn allowed_crate_globs(self) -> &'static [&'static str] {
        match self {
            Codename::Alpha => &["crates/trios-igla-race-pipeline/**", "crates/trios-trinity-init/**"],
            Codename::Beta => &["crates/trios-algorithm-arena/**", "crates/trios-bench/**"],
            Codename::Gamma => &["crates/trios-git/**", "crates/trios-gb/**"],
            Codename::Delta => &["crates/trios-data/**", ".trinity/state/**"],
            Codename::Epsilon => &["crates/trios-doctor/rings/SILVER-RING-DR-04/**"],
            Codename::Zeta => &["crates/trios-zig-agents/**", "crates/zig-knowledge-graph/**"],
            Codename::Eta => &["crates/trios-bridge/**", ".github/workflows/**"],
            Codename::Theta => &["crates/trios-igla-race-hack/**", "tests/**"],
            Codename::Iota => &["docs/i18n/**"],
            Codename::Kappa => &["crates/trios-kg/**", "crates/trios-vsa/**", "crates/trios-agent-memory/**"],
            Codename::Lambda => &["crates/trios-igla-race/src/lessons.rs", "crates/trios-agent-memory/rings/SR-MEM-05/**"],
            Codename::Mu => &["crates/trios-agent-memory/rings/SR-MEM-00/**", "crates/trios-agent-memory/rings/SR-MEM-01/**"],
            Codename::Nu => &["crates/trios-rainbow-bridge/**"],
            Codename::Xi => &["crates/trios-mcp/**", "crates/trios-server/**"],
            Codename::Omicron => &["crates/trios-igla-race/src/asha.rs", "crates/trios-igla-race-pipeline/rings/SR-04/**"],
            Codename::Pi => &["crates/trios-igla-race-pipeline/rings/BR-OUTPUT/**"],
            Codename::Koppa => &["crates/trios-golden-float/**", "crates/trios-igla-race-pipeline/rings/SR-02/**"],
            Codename::Rho => &["crates/trios-railway-audit/**", "bin/tri-railway/**", "bin/tri-gardener/**"],
            Codename::Sigma => &["crates/trios-a2a/**", "crates/trios-agents/**"],
            Codename::Tau => &["crates/trios-tri/**", "crates/trios-ternary/**"],
            Codename::Upsilon => &["crates/trios-ui/**"],
            Codename::Phi => &["crates/trios-phi-schedule/**", "crates/trios-physics/**", "docs/phd/theorems/**"],
            Codename::Khi => &["crates/trios-cli/**"],
            Codename::Psi => &["crates/trios-ca-mask/**", "crates/trios-crypto/**"],
            Codename::Omega => &[".trinity/**", "docs/golden-sunflowers/**"],
            Codename::Sampi => &["crates/trios-sacred/**", "crates/trios-phd/**"],
            Codename::Sho => &["crates/trios-fpga/**", "crates/trios-hdc/**"],
        }
    }
    fn primary_scope_dir(self) -> &'static str {
        // first glob without wildcard suffix
        let g = self.allowed_crate_globs().first().copied().unwrap_or(".trinity");
        g.trim_end_matches("/**").trim_end_matches("/*").trim_end_matches("/")
    }
}

#[derive(Parser, Debug)]
#[command(name = "trinity-bootstrap")]
#[command(about = "Provision one of 27 Coptic Trinity agents on one Tailscale node for EPIC #446.")]
#[command(version)]
struct Cli {
    /// Agent codename — one of 27 Coptic letters.
    #[arg(long, value_enum, required_unless_present = "list_grid")]
    codename: Option<Codename>,

    /// GitHub issue number this agent will claim (e.g. 448 for SR-00 scarab-types).
    #[arg(long, required_unless_present = "list_grid")]
    issue: Option<u32>,

    /// Soul-name (humorous English, one or two words). Required by L11.
    #[arg(long, required_unless_present = "list_grid")]
    soul: Option<String>,

    /// Repository owner/name.
    #[arg(long, default_value = "gHashTag/trios")]
    repo: String,

    /// EPIC issue number.
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

    /// Workspace root. Default: ~/work/trinity-<CODENAME>-<issue>
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Dry run — print what would happen, do not mutate the system.
    #[arg(long)]
    dry_run: bool,

    /// List the 27 Coptic agent grid and exit.
    #[arg(long)]
    list_grid: bool,
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
        "# TASK #{issue}\n\nSoul: {soul}\nCodename: {label}  ({coptic} {domain})\nTailscale tag: {tag}\nEPIC: #446\n\n## Allowed scope (I-SCOPE)\n\n{scope}\n\n## PHI LOOP step\n\nCLAIM\n",
        label = codename.label(),
        coptic = codename.coptic(),
        domain = codename.domain(),
        tag = codename.tag(),
        scope = allowed.iter().map(|p| format!("- `{p}`")).collect::<Vec<_>>().join("\n"),
    );
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    (body, hex::encode(h.finalize()))
}

fn print_grid() {
    println!("🐝 Trinity 27-Coptic agent grid");
    println!("{:<3} {:<4}  {:<10}  {:<60}  {}", "#", "Cop", "Codename", "Domain", "Tag");
    println!("{}", "─".repeat(120));
    let all: &[Codename] = &[
        Codename::Alpha, Codename::Beta, Codename::Gamma, Codename::Delta, Codename::Epsilon,
        Codename::Zeta, Codename::Eta, Codename::Theta, Codename::Iota, Codename::Kappa,
        Codename::Lambda, Codename::Mu, Codename::Nu, Codename::Xi, Codename::Omicron,
        Codename::Pi, Codename::Koppa, Codename::Rho, Codename::Sigma, Codename::Tau,
        Codename::Upsilon, Codename::Phi, Codename::Khi, Codename::Psi, Codename::Omega,
        Codename::Sampi, Codename::Sho,
    ];
    for (i, c) in all.iter().enumerate() {
        println!("{:<3} {:<4}  {:<10}  {:<60}  {}", i + 1, c.coptic(), c.label(), c.domain(), c.tag());
    }
    println!();
    println!("🌻 phi² + phi⁻² = 3 · TRINITY · 27 = 3³ = phi^6 - phi^4 + 1 (Lucas)");
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.list_grid {
        print_grid();
        return Ok(());
    }
    let codename = cli.codename.expect("codename required");
    let issue = cli.issue.expect("issue required");
    let soul = cli.soul.clone().expect("soul required");
    let started = Utc::now().to_rfc3339();
    println!(
        "🐝 trinity-bootstrap v0.2.0  ·  Codename: {} ({} {})  ·  Issue: #{}  ·  Soul: {}",
        codename.label(),
        codename.coptic(),
        codename.domain(),
        issue,
        soul
    );
    println!("   started:  {started}");
    println!("   anchor:   phi^2 + phi^-2 = 3   (27 = 3^3)");
    println!();

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

    let workspace = cli.workspace.clone().unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(format!("work/trinity-{}-{}", codename.label(), issue))
    });
    let branch = format!("bee/{}-{}", issue, slug(&soul));

    println!("== Step 1: Tailscale tag ==");
    let tag = codename.tag();
    if let Some(authkey) = cli.ts_authkey.as_deref() {
        run(
            "sudo",
            &[
                "tailscale", "up", "--reset", "--ssh",
                &format!("--advertise-tags={}", tag),
                &format!("--authkey={authkey}"),
            ],
            cli.dry_run,
        )?;
    } else {
        println!("   (skipping `tailscale up` — no --ts-authkey given)  tag would be: {tag}");
    }
    println!();

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

    println!("== Step 3: verify issue and post CLAIM ==");
    let claim_msg = format!(
        "IN-FLIGHT — Soul: {soul} · Codename: {label} ({coptic}) · Branch: {branch}",
        soul = soul, label = codename.label(), coptic = codename.coptic(),
    );
    println!("   probing #{} comments for prior IN-FLIGHT…", issue);
    let comments = run(
        "gh",
        &[
            "issue", "view", &issue.to_string(),
            "--repo", &cli.repo,
            "--json", "comments",
            "--jq", ".comments[] | .body",
        ],
        cli.dry_run,
    )?;
    if comments.contains("IN-FLIGHT —") {
        bail!("issue #{} already has an IN-FLIGHT claim — choose another issue", issue);
    }
    run(
        "gh",
        &[
            "issue", "comment", &issue.to_string(),
            "--repo", &cli.repo,
            "--body", &claim_msg,
        ],
        cli.dry_run,
    )?;
    println!("   ✓ CLAIM posted: {claim_msg}");
    println!();

    if !cli.dry_run {
        std::env::set_current_dir(&workspace).context("cd workspace")?;
    }

    println!("== Step 4: SPEC + SEAL ==");
    run("git", &["checkout", "-b", &branch], cli.dry_run)?;
    let allowed = codename.allowed_crate_globs();
    let scope_dir = codename.primary_scope_dir();
    run("mkdir", &["-p", scope_dir], cli.dry_run)?;
    let (task_body, task_sha) = task_md_seal(issue, codename, &soul, allowed);
    let task_path = format!("{scope_dir}/TASK.md");
    if !cli.dry_run {
        std::fs::write(&task_path, &task_body)?;
    }
    println!("   ✓ wrote {task_path}  sha256={task_sha}");
    println!();

    println!("== Step 5: GitButler virtual branch ==");
    run("but", &["project", "add", "."], cli.dry_run)?;
    let vbranch = format!("{}/{}", codename.label().to_lowercase(), branch);
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

    println!("== Step 6: COMMIT + PUSH (initial) ==");
    run("git", &["add", "-A"], cli.dry_run)?;
    let crate_short = scope_dir.split('/').nth(1).unwrap_or("trinity");
    let commit_msg = format!(
        "feat({crate}): claim #{issue} via trinity-bootstrap\n\nSoul: {soul}\nCodename: {label} ({coptic} — {domain})\nTASK.md sha256: {sha}\n\nPart of #{epic}\n\nAgent: {label}\n",
        crate = crate_short,
        issue = issue,
        soul = soul,
        label = codename.label(),
        coptic = codename.coptic(),
        domain = codename.domain(),
        sha = task_sha,
        epic = cli.epic,
    );
    run("git", &["commit", "-m", &commit_msg], cli.dry_run)?;
    run("git", &["push", "-u", "origin", &branch], cli.dry_run)?;
    println!();

    println!("== Step 7: HEARTBEAT ==");
    let hb = format!(
        "loop: {label} | 🟢 ACTIVE | {soul} · CLAIM/SEAL · {coptic} {domain}\n\nevidence:\n  ts:               {ts}\n  issue:            #{issue}\n  epic:             #{epic}\n  loop:             CLAIM → SPEC → SEAL\n  task_md_sha256:   {sha}\n  branch:           {branch}\n  vbranch:          {vbranch}\n  but_server:       http://0.0.0.0:{port}/  (listen on {iface})\n  next:             GEN — implement acceptance criteria for #{issue}\n",
        label = codename.label(),
        coptic = codename.coptic(),
        domain = codename.domain(),
        soul = soul,
        ts = started,
        issue = issue,
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
            "issue", "comment", &issue.to_string(),
            "--repo", &cli.repo,
            "--body", &format!("```\n{hb}\n```\n\n🌻 phi² + phi⁻² = 3 · 27 = 3³"),
        ],
        cli.dry_run,
    )?;
    println!("   ✓ heartbeat posted to #{}", issue);
    println!();

    println!("🌻 trinity-bootstrap done.");
    println!("   workspace: {}", workspace.display());
    println!("   codename:  {} ({} — {})", codename.label(), codename.coptic(), codename.domain());
    println!("   next:      cd {workspace} && enter PHI LOOP step GEN.", workspace = workspace.display());
    Ok(())
}
