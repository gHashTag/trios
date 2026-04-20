use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::board::KnownBoard;

#[derive(Debug, Clone)]
pub struct FlashConfig {
    pub board: KnownBoard,
    pub cable: Option<String>,
    pub xvc_addr: Option<String>,
    pub bitstream: Option<PathBuf>,
    pub freq: u32,
    pub flash: bool,
    pub verify: bool,
    pub reset: bool,
}

impl FlashConfig {
    pub fn new(board: KnownBoard) -> Self {
        Self {
            board,
            cable: None,
            xvc_addr: None,
            bitstream: None,
            freq: 6_000_000,
            flash: false,
            verify: false,
            reset: false,
        }
    }

    pub fn detect_cable(&self) -> &str {
        if let Some(ref c) = self.cable {
            c
        } else if self.xvc_addr.is_some() {
            "xvc-client"
        } else {
            "ft2232"
        }
    }

    pub fn detect_bitstream(&self) -> Result<PathBuf> {
        if let Some(ref p) = self.bitstream {
            if p.exists() {
                return Ok(p.clone());
            }
            anyhow::bail!("Bitstream not found: {}", p.display());
        }
        let candidates = [
            PathBuf::from("build/fpga/zerodsp_top.bit"),
            PathBuf::from("build/fpga/bitstream.bit"),
        ];
        for c in &candidates {
            if c.exists() {
                return Ok(c.clone());
            }
        }
        anyhow::bail!("No bitstream found. Run build first or specify --bitstream")
    }
}

pub struct FlashPipeline {
    config: FlashConfig,
}

impl FlashPipeline {
    pub fn new(config: FlashConfig) -> Self {
        Self { config }
    }

    pub fn run(&self) -> Result<FlashResult> {
        let bitstream = self.config.detect_bitstream()?;
        let bit_size = std::fs::metadata(&bitstream)?.len();
        let cable = self.config.detect_cable();

        println!("=== T27 FPGA Flash ===");
        println!("Board: {}", self.config.board);
        println!("Bitstream: {} ({} bytes)", bitstream.display(), bit_size);
        println!("Cable: {}", cable);
        println!("JTAG freq: {} Hz", self.config.freq);
        println!(
            "Mode: {}",
            if self.config.flash {
                "SPI flash write"
            } else {
                "SRAM load"
            }
        );

        let mut cmd = std::process::Command::new("openFPGALoader");
        cmd.arg("--cable").arg(cable);
        cmd.arg("--freq").arg(self.config.freq.to_string());

        if cable == "xvc-client" {
            let addr = self
                .config
                .xvc_addr
                .as_deref()
                .unwrap_or("192.168.4.1:2542");
            cmd.arg("--addr").arg(addr);
            println!("XVC server: {}", addr);
        }

        if self.config.verify {
            cmd.arg("--verify");
        }

        cmd.arg("--bitstream").arg(&bitstream);
        println!("Running: {:?}", cmd);
        println!();

        let status = cmd
            .status()
            .with_context(|| "openFPGALoader not found. Install: brew install openfpgaloader")?;

        if status.success() {
            println!();
            println!("=== Flash SUCCESS ===");
            println!(
                "Bitstream loaded to {} ({})",
                self.config.board,
                if self.config.flash {
                    "SPI flash"
                } else {
                    "SRAM"
                }
            );

            Ok(FlashResult {
                bitstream,
                bit_size,
                mode: if self.config.flash {
                    FlashMode::SpiFlash
                } else {
                    FlashMode::SramLoad
                },
            })
        } else {
            anyhow::bail!("openFPGALoader failed with exit code {:?}", status.code())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashMode {
    SramLoad,
    SpiFlash,
}

pub struct FlashResult {
    pub bitstream: PathBuf,
    pub bit_size: u64,
    pub mode: FlashMode,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn config_default_cable_is_ft2232() {
        let cfg = FlashConfig::new(KnownBoard::QmtechA100t);
        assert_eq!(cfg.detect_cable(), "ft2232");
    }

    #[test]
    fn config_explicit_cable() {
        let mut cfg = FlashConfig::new(KnownBoard::QmtechA100t);
        cfg.cable = Some("ft232RL".into());
        assert_eq!(cfg.detect_cable(), "ft232RL");
    }

    #[test]
    fn config_xvc_auto_detects_cable() {
        let mut cfg = FlashConfig::new(KnownBoard::QmtechA100t);
        cfg.xvc_addr = Some("192.168.1.100:2542".into());
        assert_eq!(cfg.detect_cable(), "xvc-client");
    }

    #[test]
    fn config_default_freq() {
        let cfg = FlashConfig::new(KnownBoard::QmtechA100t);
        assert_eq!(cfg.freq, 6_000_000);
    }

    #[test]
    fn config_default_not_flash_mode() {
        let cfg = FlashConfig::new(KnownBoard::QmtechA100t);
        assert!(!cfg.flash);
    }

    #[test]
    fn config_default_not_verify() {
        let cfg = FlashConfig::new(KnownBoard::QmtechA100t);
        assert!(!cfg.verify);
    }

    #[test]
    fn config_default_not_reset() {
        let cfg = FlashConfig::new(KnownBoard::QmtechA100t);
        assert!(!cfg.reset);
    }

    #[test]
    fn detect_bitstream_explicit_exists() {
        let dir = tempfile::tempdir().unwrap();
        let bit = dir.path().join("test.bit");
        std::fs::File::create(&bit)
            .unwrap()
            .write_all(b"BIT")
            .unwrap();

        let mut cfg = FlashConfig::new(KnownBoard::QmtechA100t);
        cfg.bitstream = Some(bit.clone());
        let result = cfg.detect_bitstream().unwrap();
        assert_eq!(result, bit);
    }

    #[test]
    fn detect_bitstream_explicit_missing_fails() {
        let mut cfg = FlashConfig::new(KnownBoard::QmtechA100t);
        cfg.bitstream = Some(PathBuf::from("/nonexistent/path.bit"));
        let result = cfg.detect_bitstream();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Bitstream not found"));
    }

    #[test]
    fn detect_bitstream_no_candidates_fails() {
        let cfg = FlashConfig::new(KnownBoard::QmtechA100t);
        let result = cfg.detect_bitstream();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No bitstream found"));
    }

    #[test]
    fn flash_mode_equality() {
        assert_eq!(FlashMode::SramLoad, FlashMode::SramLoad);
        assert_eq!(FlashMode::SpiFlash, FlashMode::SpiFlash);
        assert_ne!(FlashMode::SramLoad, FlashMode::SpiFlash);
    }

    #[test]
    fn config_board_preserved() {
        let cfg = FlashConfig::new(KnownBoard::ArtyA7_100t);
        assert_eq!(cfg.board, KnownBoard::ArtyA7_100t);
    }

    #[test]
    fn config_all_fields_mutable() {
        let mut cfg = FlashConfig::new(KnownBoard::QmtechA100t);
        cfg.freq = 12_000_000;
        cfg.flash = true;
        cfg.verify = true;
        cfg.reset = true;
        cfg.cable = Some("digilent_hs2".into());
        cfg.xvc_addr = Some("10.0.0.1:2542".into());
        assert_eq!(cfg.freq, 12_000_000);
        assert!(cfg.flash);
        assert!(cfg.verify);
        assert!(cfg.reset);
        assert_eq!(cfg.detect_cable(), "digilent_hs2");
    }
}
