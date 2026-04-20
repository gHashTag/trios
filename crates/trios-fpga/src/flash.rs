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
