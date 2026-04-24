use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reqwest::Client;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

const DWAGENT_URL: &str = "https://www.dwservice.net/download/dwagent_x86_64.sh";
const INSTALL_PATH: &str = "/tmp/dwagent.sh";

#[derive(Debug, Parser)]
#[command(name = "trios-dwagent")]
#[command(about = "DWService Agent installer for Railway deployment", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Download DWAgent installer
    Download,
    /// Install DWAgent (requires sudo)
    Install,
    /// Clean up downloaded files
    Cleanup,
    /// Full installation flow (download + instructions)
    InstallAll {
        /// Auto-install with sudo (not recommended for security)
        #[arg(long)]
        auto: bool,
    },
}

/// Downloads the DWService agent installer
async fn download_installer() -> Result<()> {
    println!("📥 Downloading DWAgent installer from {}...", DWAGENT_URL);

    let client = Client::new();
    let response = client
        .get(DWAGENT_URL)
        .send()
        .await
        .context("Failed to fetch DWAgent installer")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to download DWAgent: HTTP {}",
            response.status()
        );
    }

    let content = response
        .bytes()
        .await
        .context("Failed to read installer content")?;

    let mut file = File::create(INSTALL_PATH)
        .context("Failed to create installer file")?;

    file.write_all(&content)
        .context("Failed to write installer file")?;

    println!("✅ Downloaded to {}", INSTALL_PATH);
    Ok(())
}

/// Sets executable permissions on the installer
fn make_executable() -> Result<()> {
    println!("🔧 Setting executable permissions...");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let path = Path::new(INSTALL_PATH);
        let mut perms = fs::metadata(path)
            .context("Failed to get file permissions")?
            .permissions();

        perms.set_mode(0o755); // rwxr-xr-x
        fs::set_permissions(path, perms)
            .context("Failed to set executable permissions")?;
    }

    println!("✅ Executable permissions set");
    Ok(())
}

/// Provides instructions for running the installer
fn print_install_instructions() {
    println!("\n🚀 To complete installation, run:");
    println!("   sudo {}", INSTALL_PATH);
    println!("\n💡 After installation, check status at:");
    println!("   https://www.dwservice.net\n");
}

/// Cleans up the downloaded installer
fn cleanup() -> Result<()> {
    if Path::new(INSTALL_PATH).exists() {
        fs::remove_file(INSTALL_PATH)
            .context("Failed to remove installer file")?;
        println!("🧹 Cleaned up installer file");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Download => {
            download_installer().await?;
            make_executable()?;
            println!("\nRun: sudo /tmp/dwagent.sh");
        }
        Commands::Install => {
            println!("❌ Direct install requires manual execution:");
            println!("   sudo /tmp/dwagent.sh");
        }
        Commands::Cleanup => {
            cleanup()?;
        }
        Commands::InstallAll { auto } => {
            download_installer().await?;
            make_executable()?;

            if auto {
                println!("⚠️  Auto-install mode - ensure you trust the source!");
                println!("   Use --no-auto for manual review first");
                println!("\nTo install manually: sudo /tmp/dwagent.sh");
            } else {
                print_install_instructions();
            }
        }
    }

    Ok(())
}
