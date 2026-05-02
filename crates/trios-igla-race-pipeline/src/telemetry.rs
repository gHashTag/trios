//! Telemetry streaming — CSV and Neon event streaming.
//!
//! Collects trial results and streams them to CSV files and Neon database.

use anyhow::Result;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;
use tracing::debug;

use super::trial::TrialResult;

/// Telemetry event type.
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    /// Trial started
    TrialStarted { trial_id: u64, config: TrialConfig },
    /// Trial checkpoint (rung reached)
    TrialCheckpoint { trial_id: u64, rung: u32, bpb: f64 },
    /// Trial completed
    TrialCompleted { result: TrialResult },
    /// Victory achieved
    Victory { trial_id: u64, bpb: f64 },
}

/// Configuration for TrialConfig in TelemetryEvent.
#[derive(Debug, Clone)]
pub struct TrialConfig {
    pub trial_id: u64,
    pub lr: f64,
    pub d_model: usize,
    pub gradient_mode: String,
}

/// Telemetry sink for streaming events.
pub struct TelemetrySink {
    csv_writer: Option<BufWriter<File>>,
    neon_enabled: bool,
}

impl TelemetrySink {
    /// Create a new telemetry sink.
    pub fn new(csv_path: Option<PathBuf>, neon_enabled: bool) -> Result<Self> {
        let csv_writer = match csv_path {
            Some(path) => {
                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)?;
                Some(BufWriter::new(file))
            }
            None => None,
        };

        Ok(Self {
            csv_writer,
            neon_enabled,
        })
    }

    /// Stream a telemetry event.
    pub fn emit(&mut self, event: TelemetryEvent) -> Result<()> {
        debug!("Emitting telemetry event: {:?}", event);

        match event {
            TelemetryEvent::TrialStarted { trial_id, config } => {
                self.write_csv(&[
                    "started",
                    &trial_id.to_string(),
                    &config.lr.to_string(),
                    &config.d_model.to_string(),
                ])?;
            }
            TelemetryEvent::TrialCheckpoint { trial_id, rung, bpb } => {
                self.write_csv(&[
                    "checkpoint",
                    &trial_id.to_string(),
                    &rung.to_string(),
                    &bpb.to_string(),
                ])?;
            }
            TelemetryEvent::TrialCompleted { result } => {
                self.write_csv(&[
                    "completed",
                    &result.trial_id.to_string(),
                    &result.final_bpb.to_string(),
                    &result.final_rung.to_string(),
                    &result.total_steps.to_string(),
                    &if result.victory { "1" } else { "0" },
                    &format!("{:.2?}", result.duration),
                ])?;
            }
            TelemetryEvent::Victory { trial_id, bpb } => {
                self.write_csv(&[
                    "victory",
                    &trial_id.to_string(),
                    &bpb.to_string(),
                    &Instant::now().elapsed().as_millis().to_string(),
                ])?;
            }
        }

        // TODO: Stream to Neon if enabled
        if self.neon_enabled {
            // self.emit_to_neon(event)?;
        }

        Ok(())
    }

    /// Flush any buffered events.
    pub fn flush(&mut self) -> Result<()> {
        if let Some(writer) = &mut self.csv_writer {
            writer.flush()?;
        }
        Ok(())
    }

    fn write_csv(&mut self, fields: &[&str]) -> Result<()> {
        if let Some(writer) = &mut self.csv_writer {
            writeln!(writer, "{}", fields.join(","))?;
        }
        Ok(())
    }
}

impl Drop for TelemetrySink {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
