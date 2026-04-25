//! Integration tests for tri-tunnel
//!
//! Note: These tests require Tailscale to be installed and logged in.
//! They will be skipped if Tailscale is not available.

use std::process::Command;

#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(["run", "-p", "tri-tunnel", "--", "--help"])
        .output()
        .expect("Failed to run tri-tunnel --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tri-tunnel"));
    assert!(stdout.contains("start") || stdout.contains("Start"));
    assert!(stdout.contains("stop") || stdout.contains("Stop"));
    assert!(stdout.contains("status") || stdout.contains("Status"));
}

#[test]
fn test_status_command() {
    let output = Command::new("cargo")
        .args(["run", "-p", "tri-tunnel", "--", "status"])
        .output()
        .expect("Failed to run tri-tunnel status");

    // Status command should succeed even if funnel is not active
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Device") || stdout.contains("Error"));
}

#[test]
fn test_tailscale_cli_path() {
    let cli_path = "/Applications/Tailscale.app/Contents/MacOS/Tailscale";
    let path = std::path::Path::new(cli_path);
    // Test will pass if CLI doesn't exist (skip gracefully)
    if path.exists() {
        let output = Command::new(cli_path)
            .args(["status", "--json"])
            .output()
            .expect("Failed to run Tailscale status");
        assert!(output.status.success());
    }
}
