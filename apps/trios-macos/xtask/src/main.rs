//! trios-app — Rust-порт бывших shell-скриптов приложения (закон L1: без .sh).
//!
//! Подкоманды (1:1 с удалёнными скриптами):
//! - `build`          ← build.sh: Queen dylib → swiftc → .app bundle → codesign → swift test;
//! - `chat-sse-e2e`   ← tests/swift/run_chat_sse_e2e.sh: сборка и запуск SSE e2e-теста;
//! - `mesh-chat-e2e`  ← rings/RUST-13/clade-meshd/tests/run_mesh_chat_transport.sh:
//!                      два clade-meshd обмениваются sealed-фреймом по UDP;
//! - `e2e-flow`       ← e2e/trios_e2e_flow.sh: health 9105, PID, скриншот, лог-скан, чек-лист.
//!
//! Запуск: `cargo run -p trios-app-xtask --bin trios-app -- <subcommand>`.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn project_dir() -> PathBuf {
    if let Ok(root) = std::env::var("TRIOS_ROOT") {
        return PathBuf::from(root);
    }
    // CARGO_MANIFEST_DIR = apps/trios-macos/xtask → родитель = apps/trios-macos.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent")
        .to_path_buf()
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Запуск команды с наследованием stdout/stderr; ошибка при ненулевом коде.
fn run(program: &str, args: &[&str], envs: &[(&str, &str)], cwd: Option<&Path>) -> Result<()> {
    let mut cmd = Command::new(program);
    cmd.args(args).envs(envs.iter().copied());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let status = cmd.status().with_context(|| format!("spawn {program}"))?;
    if !status.success() {
        bail!("{program} {args:?} failed with {status}");
    }
    Ok(())
}

/// Запуск с захватом stdout (stderr наследуется).
fn run_capture(program: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(program)
        .args(args)
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("spawn {program}"))?;
    if !out.status.success() {
        bail!("{program} {args:?} failed with {}", out.status);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn find_swift_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return files };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(find_swift_files(&path));
        } else if path.extension().is_some_and(|e| e == "swift") {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn is_git_tracked(repo: &Path, relative: &str) -> bool {
    Command::new("git")
        .args(["-C"])
        .arg(repo)
        .args(["ls-files", "--error-unmatch", relative])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// build (бывший build.sh)
// ---------------------------------------------------------------------------

const BR_OUTPUT_ALLOWLIST: [&str; 15] = [
    "BR-OUTPUT/FullscreenChatWorkspace.swift",
    "BR-OUTPUT/HotkeyBar.swift",
    "BR-OUTPUT/SmoothStreamingEnhancements.swift",
    "BR-OUTPUT/ModelsTabView.swift",
    "BR-OUTPUT/QueenMasterViewModel.swift",
    "BR-OUTPUT/QueenIntelligenceEngine.swift",
    "BR-OUTPUT/TaskDelegator.swift",
    "BR-OUTPUT/PredictiveOrchestrator.swift",
    "BR-OUTPUT/TeamQueenManager.swift",
    "BR-OUTPUT/QueenPermissions.swift",
    "BR-OUTPUT/QueenAuditLog.swift",
    "BR-OUTPUT/QueenIntegrationsHub.swift",
    "BR-OUTPUT/SlackIntegration.swift",
    "BR-OUTPUT/EmailIntegration.swift",
    "BR-OUTPUT/CalendarIntegration.swift",
];

const INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key><string>trios</string>
    <key>CFBundleIdentifier</key><string>com.browseros.trios</string>
    <key>CFBundleName</key><string>Trios</string>
    <key>CFBundleVersion</key><string>1.0.0</string>
    <key>CFBundleShortVersionString</key><string>1.0.0</string>
    <key>LSMinimumSystemVersion</key><string>14.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>TRIOS_MESH_PORT</key><string>9505</string>
    <key>TRIOS_MCP_PORT</key><string>9105</string>
    <!-- TS-retirement item 3: the consolidated Rust trios-server serves MCP,
         A2A and /health on a single port. A2A is collapsed onto the MCP port
         (9105) so the client talks to one backend process. -->
    <key>TRIOS_A2A_PORT</key><string>9105</string>
    <key>TRIOS_CANARY_MCP_PORT</key><string>9205</string>
    <key>TRIOS_VARIANT</key><string>prod</string>
</dict>
</plist>
"#;

fn cmd_build() -> Result<()> {
    let project = project_dir();
    let output = project.join("trios_app");
    let log_dir = project.join(".trinity/logs");
    std::fs::create_dir_all(&log_dir)?;
    let log_file = log_dir.join(format!("build_{}.log", timestamp()));

    let user_root = project
        .parent()
        .and_then(Path::parent)
        .context("project dir has no grandparent")?
        .to_path_buf();
    let trinity_root = std::env::var("TRINITY_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| user_root.join("trinity"));
    let queen_package = trinity_root.join("apps/queen");

    if !queen_package.join("Package.swift").is_file() {
        bail!(
            "[FAIL] Canonical Queen package not found: {} — set TRINITY_ROOT to the gHashTag/trinity checkout",
            queen_package.display()
        );
    }

    println!("Building canonical Trinity Queen interface...");
    let queen_path = queen_package.display().to_string();
    run(
        "swift",
        &["build", "--package-path", &queen_path, "--product", "QueenUILib"],
        &[],
        None,
    )?;
    let queen_bin_dir = PathBuf::from(run_capture(
        "swift",
        &["build", "--package-path", &queen_path, "--show-bin-path"],
    )?);
    let queen_dylib = queen_bin_dir.join("libQueenUILib.dylib");
    if !queen_dylib.is_file() {
        bail!("[FAIL] QueenUILib was not produced: {}", queen_dylib.display());
    }

    // Отслеживаемые прод-исходники + allowlist для BR-OUTPUT (см. историю build.sh:
    // компиляция каждого untracked-черновика ломала приложение).
    let mut swift_files = vec![project.join("main.swift")];
    swift_files.extend(find_swift_files(&project.join("rings")));
    for file in find_swift_files(&project.join("BR-OUTPUT")) {
        let relative = file
            .strip_prefix(&project)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        if is_git_tracked(&project, &relative)
            || BR_OUTPUT_ALLOWLIST.contains(&relative.as_str())
        {
            swift_files.push(file);
        }
    }
    println!("Compiling {} Swift files...", swift_files.len());

    let modules_dir = queen_bin_dir.join("Modules");
    let mut args: Vec<String> = [
        "-j", "1", "-O", "-o",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    args.push(output.display().to_string());
    for framework in ["SwiftUI", "AppKit", "WebKit", "Combine"] {
        args.push("-framework".into());
        args.push(framework.into());
    }
    args.extend([
        "-I".into(), modules_dir.display().to_string(),
        "-L".into(), queen_bin_dir.display().to_string(),
        "-lQueenUILib".into(),
        "-Xlinker".into(), "-rpath".into(),
        "-Xlinker".into(), "@executable_path/Frameworks".into(),
        "-Xlinker".into(), "-rpath".into(),
        "-Xlinker".into(), "@executable_path/../Frameworks".into(),
    ]);
    args.extend(swift_files.iter().map(|p| p.display().to_string()));

    let build_out = Command::new("swiftc")
        .args(&args)
        .output()
        .context("spawn swiftc")?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&build_out.stdout),
        String::from_utf8_lossy(&build_out.stderr)
    );
    print!("{combined}");
    std::fs::write(&log_file, &combined)?;
    if !build_out.status.success() {
        bail!("[FAIL] Build failed (log: {})", log_file.display());
    }
    println!("[OK] Build successful: {}", output.display());

    // Standalone-бинарник + .app bundle (см. историю: отсутствующий/устаревший
    // Info.plist отключал single-instance активацию и вызывал каскад перезапусков).
    let standalone_frameworks = project.join("Frameworks");
    std::fs::create_dir_all(&standalone_frameworks)?;
    std::fs::copy(&queen_dylib, standalone_frameworks.join("libQueenUILib.dylib"))?;

    let app_bundle = project.join("trios.app");
    let macos_dir = app_bundle.join("Contents/MacOS");
    let resources_dir = app_bundle.join("Contents/Resources");
    let frameworks_dir = app_bundle.join("Contents/Frameworks");
    for dir in [&macos_dir, &resources_dir, &frameworks_dir] {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(app_bundle.join("Contents/Info.plist"), INFO_PLIST)?;
    std::fs::copy(&output, macos_dir.join("trios"))?;
    std::fs::copy(&queen_dylib, frameworks_dir.join("libQueenUILib.dylib"))?;

    // Замена файла внутри подписанного бандла инвалидирует подпись — dyld убьёт
    // приложение до main(). Подписываем ad-hoc после полной сборки бандла.
    let bundle = app_bundle.display().to_string();
    run("codesign", &["--force", "--deep", "--sign", "-", &bundle], &[], None)?;
    run("codesign", &["--verify", "--deep", "--strict", &bundle], &[], None)?;
    println!("[OK] Copied and signed .app bundle (bundle ID: com.browseros.trios)");

    // swift test — только при доступном XCTest (нужен Xcode).
    if std::env::var("TRIOS_SKIP_SWIFT_TEST").is_ok() {
        println!("[SKIP] TRIOS_SKIP_SWIFT_TEST is set; skipping swift test");
    } else if Command::new("xcrun")
        .args(["--find", "xctest"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| !s.success())
        .unwrap_or(true)
    {
        println!("[SKIP] XCTest not available in this toolchain (install Xcode to run swift test)");
    } else {
        println!("Running swift test...");
        let package_root = project.parent().context("no parent")?.display().to_string();
        run("swift", &["test", "--package-path", &package_root], &[], None)
            .with_context(|| format!("[FAIL] swift test failed (log: {})", log_file.display()))?;
        println!("[OK] swift test passed");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// chat-sse-e2e (бывший tests/swift/run_chat_sse_e2e.sh)
// ---------------------------------------------------------------------------

fn cmd_chat_sse_e2e() -> Result<()> {
    let project = project_dir();
    let script_dir = project.join("tests/swift");
    let output = PathBuf::from("/tmp/trios_chat_sse_e2e_test");
    let log_dir = project.join(".trinity/logs");
    std::fs::create_dir_all(&log_dir)?;
    let log_file = log_dir.join(format!("chat_sse_e2e_build_{}.log", timestamp()));

    // rings целиком + только те BR-OUTPUT-файлы, на которые rings ссылается
    // (main.swift исключён: у теста собственная @main-точка входа).
    let mut files = find_swift_files(&project.join("rings"));
    for name in [
        "BR-OUTPUT/ProjectPaths.swift",
        "BR-OUTPUT/QueenStatusViewModel.swift",
        "BR-OUTPUT/A2AMessageRouter.swift",
        "BR-OUTPUT/TriosTheme.swift",
        "BR-OUTPUT/GitHubModels.swift",
        "BR-OUTPUT/GitHubAPIClient.swift",
    ] {
        files.push(project.join(name));
    }
    files.push(script_dir.join("ChatSSETestMocks.swift"));
    files.push(script_dir.join("ChatSSEEndToEndTest.swift"));
    println!("Compiling {} Swift files...", files.len());

    let mut args: Vec<String> =
        ["-j", "1", "-O", "-o"].iter().map(|s| s.to_string()).collect();
    args.push(output.display().to_string());
    for framework in ["SwiftUI", "AppKit", "WebKit", "Combine", "Security"] {
        args.push("-framework".into());
        args.push(framework.into());
    }
    args.extend(files.iter().map(|p| p.display().to_string()));

    let build_out = Command::new("swiftc").args(&args).output().context("spawn swiftc")?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&build_out.stdout),
        String::from_utf8_lossy(&build_out.stderr)
    );
    print!("{combined}");
    std::fs::write(&log_file, &combined)?;
    if !build_out.status.success() {
        bail!("[FAIL] Build failed (log: {})", log_file.display());
    }
    println!("[OK] Build successful: {}", output.display());
    println!("Running {}...", output.display());
    run(&output.display().to_string(), &[], &[], None)
}

// ---------------------------------------------------------------------------
// mesh-chat-e2e (бывший rings/RUST-13/clade-meshd/tests/run_mesh_chat_transport.sh)
// ---------------------------------------------------------------------------

fn random_token() -> Result<String> {
    let bytes = std::fs::read("/dev/urandom").ok().filter(|b| b.len() >= 32).map_or_else(
        || -> Result<Vec<u8>> {
            use std::io::Read;
            let mut buf = vec![0u8; 32];
            std::fs::File::open("/dev/urandom")?.read_exact(&mut buf)?;
            Ok(buf)
        },
        |b| Ok(b[..32].to_vec()),
    )?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn curl_json(method: &str, url: &str, token: &str, body: Option<&str>) -> Result<String> {
    let mut args = vec!["-fs", "-X", method, url, "-H", "Content-Type: application/json"];
    let auth = format!("Authorization: Bearer {token}");
    args.push("-H");
    args.push(&auth);
    if let Some(data) = body {
        args.push("-d");
        args.push(data);
    }
    run_capture("curl", &args)
}

fn grep_log(path: &Path, prefix: &str) -> Result<String> {
    let text = std::fs::read_to_string(path)?;
    text.lines()
        .find_map(|l| l.split_whitespace().find_map(|w| w.strip_prefix(prefix)))
        .map(String::from)
        .with_context(|| format!("no `{prefix}` in {}", path.display()))
}

fn cmd_mesh_chat_e2e() -> Result<()> {
    let project = project_dir();
    // apps/trios-macos → корень репозитория.
    let repo = project.parent().and_then(Path::parent).context("no repo root")?;
    let bin = repo.join("target/debug/clade-meshd");
    let tmp = std::env::temp_dir().join(format!("mesh_chat_e2e_{}", timestamp()));
    std::fs::create_dir_all(tmp.join("keys1"))?;
    std::fs::create_dir_all(tmp.join("keys2"))?;
    let token = random_token()?;

    println!("[e2e] building clade-meshd...");
    let manifest = repo.join("Cargo.toml").display().to_string();
    run("cargo", &["build", "-p", "clade-meshd", "--manifest-path", &manifest], &[], None)?;

    let spawn_node = |node: &str, http: &str, udp: &str, log: &Path| -> Result<std::process::Child> {
        let log_file = std::fs::File::create(log)?;
        Command::new(&bin)
            .env("TRIOS_MESH_NODE_ID", node)
            .env("TRIOS_MESH_PORT", http)
            .env("TRIOS_MESH_UDP_BIND", udp)
            .env("TRIOS_MESH_KEY_DIR", tmp.join(format!("keys{node}")))
            .env("TRIOS_MESH_CHAT_STORE", tmp.join(format!("store{node}.json")))
            .env("TRIOS_MESH_API_TOKEN", &token)
            .stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file))
            .spawn()
            .context("spawn clade-meshd")
    };
    let log1 = tmp.join("d1.log");
    let log2 = tmp.join("d2.log");
    let mut d1 = spawn_node("1", "9505", "127.0.0.1:9601", &log1)?;
    let mut d2 = spawn_node("2", "9506", "127.0.0.1:9602", &log2)?;

    let cleanup = |d1: &mut std::process::Child, d2: &mut std::process::Child| {
        let _ = d1.kill();
        let _ = d2.kill();
        let _ = d1.wait();
        let _ = d2.wait();
    };

    let wait_health = |port: u16| -> bool {
        for _ in 0..50 {
            if run_capture("curl", &["-fs", &format!("http://127.0.0.1:{port}/health")]).is_ok() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        false
    };
    println!("[e2e] waiting for daemons...");
    if !wait_health(9505) || !wait_health(9506) {
        cleanup(&mut d1, &mut d2);
        bail!(
            "daemon health failed\nnode1:\n{}\nnode2:\n{}",
            std::fs::read_to_string(&log1).unwrap_or_default(),
            std::fs::read_to_string(&log2).unwrap_or_default()
        );
    }

    let result = (|| -> Result<()> {
        let pub1 = grep_log(&log1, "public_key=")?;
        let pub2 = grep_log(&log2, "public_key=")?;
        let udp1 = grep_log(&log1, "udp=")?;
        let udp2 = grep_log(&log2, "udp=")?;
        println!("[e2e] node 1 udp={udp1} pub={}...", &pub1[..16.min(pub1.len())]);
        println!("[e2e] node 2 udp={udp2} pub={}...", &pub2[..16.min(pub2.len())]);

        println!("[e2e] seeding peers...");
        curl_json(
            "POST",
            "http://127.0.0.1:9505/seed-peer",
            &token,
            Some(&format!(r#"{{"peer":2,"public_key":"{pub2}","address":"{udp2}"}}"#)),
        )?;
        curl_json(
            "POST",
            "http://127.0.0.1:9506/seed-peer",
            &token,
            Some(&format!(r#"{{"peer":1,"public_key":"{pub1}","address":"{udp1}"}}"#)),
        )?;

        println!("[e2e] sending message...");
        let send = curl_json(
            "POST",
            "http://127.0.0.1:9505/messages/send",
            &token,
            Some(r#"{"dst":2,"kind":0,"text":"hello over udp"}"#),
        )?;
        println!("[e2e] send response: {send}");
        if !send.contains(r#""queued":true"#) {
            bail!("[e2e] message was not queued/forwarded");
        }

        println!("[e2e] polling node 2...");
        for _ in 0..50 {
            let poll = curl_json(
                "GET",
                "http://127.0.0.1:9506/messages/poll?since_id=0",
                &token,
                None,
            )
            .unwrap_or_else(|_| "{}".into());
            if poll.contains("hello over udp") {
                println!("[e2e] SUCCESS: message delivered over UDP");
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        bail!(
            "[e2e] FAIL: message did not arrive at node 2\nnode1:\n{}\nnode2:\n{}",
            std::fs::read_to_string(&log1).unwrap_or_default(),
            std::fs::read_to_string(&log2).unwrap_or_default()
        );
    })();

    cleanup(&mut d1, &mut d2);
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

// ---------------------------------------------------------------------------
// e2e-flow (бывший e2e/trios_e2e_flow.sh)
// ---------------------------------------------------------------------------

fn cmd_e2e_flow() -> Result<()> {
    let log_dir = PathBuf::from("/tmp/trios_e2e");
    std::fs::create_dir_all(&log_dir)?;
    let ts = timestamp();
    let report_path = log_dir.join(format!("report_{ts}.md"));
    let mut report = format!("# TRIOS E2E Report (ts {ts})\n");

    // 1. Здоровье сервера (боевой порт 9105).
    let health = run_capture("curl", &["-s", "-m", "5", "http://127.0.0.1:9105/health"])
        .unwrap_or_else(|_| "FAIL".into());
    if health.contains(r#""status":"ok""#) {
        report.push_str(&format!("- [OK] trios-server: OK ({health})\n"));
    } else {
        report.push_str(&format!("- [FAIL] trios-server: DOWN ({health})\n"));
    }

    // 2. Приложение запущено?
    let pid = run_capture("pgrep", &["-f", "trios.app/Contents/MacOS/trios"]).unwrap_or_default();
    if !pid.is_empty() {
        report.push_str(&format!("- [OK] Trios App: PID {pid}\n"));
    } else {
        report.push_str("- [FAIL] Trios App: NOT RUNNING — restarting...\n");
        let app = project_dir().join("trios.app");
        let _ = run("open", &[&app.display().to_string()], &[], None);
        std::thread::sleep(std::time::Duration::from_secs(3));
    }

    // 3. Скриншот (только macOS).
    let shot = log_dir.join(format!("screenshot_{ts}.png"));
    if run("screencapture", &["-x", &shot.display().to_string()], &[], None).is_ok() {
        report.push_str(&format!("- Screenshot: {}\n", shot.display()));
    } else {
        report.push_str("- [SKIP] screencapture unavailable\n");
    }

    // 4. Ошибки в системном логе за 5 минут (только macOS).
    let errors = run_capture(
        "log",
        &["show", "--predicate", r#"process == "trios""#, "--last", "5m", "--style", "compact"],
    )
    .map(|text| {
        text.lines()
            .filter(|l| {
                let lower = l.to_lowercase();
                ["timed out", "transporterror", "crash", "fatal", "error"]
                    .iter()
                    .any(|needle| lower.contains(needle))
            })
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n")
    })
    .unwrap_or_default();
    if errors.is_empty() {
        report.push_str("- [OK] No critical errors in last 5m\n");
    } else {
        report.push_str(&format!("- [WARN] Recent Errors:\n```\n{errors}\n```\n"));
    }

    // 5. Чек-лист UI-аномалий.
    report.push_str(
        "\n## UI Anomaly Checklist (verify against screenshot)\n\
         - [ ] Title bar shows correct status (Online green dot, A2A blue dot)\n\
         - [ ] Tab bar icons visible and not duplicated (Chat/Git/Terminal/Queen/Settings)\n\
         - [ ] Chat input field visible at bottom with placeholder 'Ask anything...'\n\
         - [ ] No overlapping views, no black rectangles, no glitched rendering\n\
         - [ ] Glassmorphism blur visible behind panel content\n\
         - [ ] Messages scroll correctly without cutting off bubbles\n\
         - [ ] No duplicate headers or buttons outside tab bar\n",
    );

    std::fs::write(&report_path, report)?;
    println!("{}", report_path.display());
    Ok(())
}

// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    let command = std::env::args().nth(1).unwrap_or_default();
    match command.as_str() {
        "build" => cmd_build(),
        "chat-sse-e2e" => cmd_chat_sse_e2e(),
        "mesh-chat-e2e" => cmd_mesh_chat_e2e(),
        "e2e-flow" => cmd_e2e_flow(),
        _ => {
            eprintln!("usage: trios-app <build|chat-sse-e2e|mesh-chat-e2e|e2e-flow>");
            std::process::exit(2);
        }
    }
}
