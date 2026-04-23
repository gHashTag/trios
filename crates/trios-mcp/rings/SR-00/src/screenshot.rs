//! Screenshot capture and save functionality
//!
//! Handles base64 PNG data from Chrome Extension:
//! - Decode base64 to bytes
//! - Save to configured directory (default: ~/.trios/screenshots/)
//! - Optional macOS auto-paste via AppleScript

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::Utc;
use std::path::PathBuf;
use tracing::{debug, info};

/// Screenshot configuration
#[derive(Debug, Clone)]
pub struct ScreenshotConfig {
    pub directory: PathBuf,
    pub auto_paste: bool,
}

impl Default for ScreenshotConfig {
    fn default() -> Self {
        Self {
            directory: dirs_next::home_dir()
                .unwrap()
                .join(".trios")
                .join("screenshots"),
            auto_paste: cfg!(target_os = "macos"),
        }
    }
}

impl ScreenshotConfig {
    /// Create from environment variables
    pub fn from_env() -> Self {
        let directory = std::env::var("SCREENSHOT_DIR")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs_next::home_dir()
                    .unwrap()
                    .join(".trios")
                    .join("screenshots")
            });

        let auto_paste = std::env::var("AUTO_PASTE")
            .unwrap_or_else(|_| if cfg!(target_os = "macos") { "true".to_string() } else { "false".to_string() })
            .parse()
            .unwrap_or(cfg!(target_os = "macos"));

        Self {
            directory,
            auto_paste,
        }
    }

    /// Ensure screenshot directory exists
    pub fn ensure_directory(&self) -> Result<()> {
        std::fs::create_dir_all(&self.directory)
            .with_context(|| format!("Failed to create screenshot directory: {:?}", self.directory))
    }
}

/// Screenshot metadata
#[derive(Debug, Clone)]
pub struct Screenshot {
    pub filename: String,
    pub path: PathBuf,
    pub size_bytes: usize,
    pub timestamp: String,
}

/// Capture and save screenshot from base64 data
pub fn save_screenshot(
    base64_data: &str,
    config: &ScreenshotConfig,
) -> Result<Screenshot> {
    // Remove data URL prefix if present
    let base64_data = base64_data
        .strip_prefix("data:image/png;base64,")
        .unwrap_or(base64_data);

    // Decode base64 to PNG bytes
    let png_bytes = BASE64.decode(base64_data)
        .context("Failed to decode base64 screenshot data")?;

    // Ensure directory exists
    config.ensure_directory()?;

    // Generate filename with timestamp
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("screenshot_{}.png", timestamp);
    let path = config.directory.join(&filename);

    // Write PNG file
    std::fs::write(&path, &png_bytes)
        .with_context(|| format!("Failed to write screenshot to {:?}", path))?;

    info!("Screenshot saved: {:?} ({} bytes)", path, png_bytes.len());

    // Optional macOS auto-paste
    if config.auto_paste {
        #[cfg(target_os = "macos")]
        auto_paste_to_cursor(&path)?;
    }

    Ok(Screenshot {
        filename,
        path,
        size_bytes: png_bytes.len(),
        timestamp: timestamp.to_string(),
    })
}

/// Auto-paste image to Cursor on macOS using AppleScript
#[cfg(target_os = "macos")]
fn auto_paste_to_cursor(image_path: &std::path::Path) -> Result<()> {
    use std::process::Command;

    let applescript = format!(
        r#"
        set theImage to POSIX file "{}"
        tell application "System Events"
            set theClipboard to (read theImage as JPEG picture)
        end tell
        "#,
        image_path.display()
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&applescript)
        .output()
        .context("Failed to execute AppleScript for auto-paste")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Auto-paste AppleScript failed: {}", stderr);
    } else {
        debug!("Auto-paste completed");
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn auto_paste_to_cursor(_image_path: &std::path::Path) -> Result<()> {
    debug!("Auto-paste only supported on macOS");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screenshot_config_default() {
        let config = ScreenshotConfig::default();
        assert!(config.directory.ends_with("screenshots"));
    }

    #[test]
    fn test_save_screenshot() {
        let config = ScreenshotConfig {
            directory: std::env::temp_dir().join("test_screenshots"),
            auto_paste: false,
        };

        // Create minimal PNG (1x1 transparent pixel)
        let minimal_png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

        let result = save_screenshot(minimal_png, &config);
        assert!(result.is_ok());

        let screenshot = result.unwrap();
        assert!(screenshot.filename.starts_with("screenshot_"));
        assert!(screenshot.filename.ends_with(".png"));
        assert!(screenshot.path.exists());

        // Cleanup
        let _ = std::fs::remove_file(screenshot.path);
    }
}
