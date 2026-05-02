//! Dashboard — live dashboard streaming.
//!
//! Provides a real-time dashboard for monitoring IGLA RACE progress.

use std::time::Duration;
use tokio::time::interval;
use tracing::{info, error};

use super::status::{RaceStatus, QueryStatus};

/// Dashboard event for streaming.
#[derive(Debug, Clone)]
pub enum DashboardEvent {
    /// Trial started
    TrialStarted { trial_id: u64 },
    /// Trial checkpoint
    TrialCheckpoint { trial_id: u64, rung: u32, bpb: f64 },
    /// Trial completed
    TrialCompleted { trial_id: u64, bpb: f64 },
    /// Victory achieved
    Victory { trial_id: u64, bpb: f64 },
    /// Worker status update
    WorkerUpdate { worker_id: u32, active: bool },
}

/// Dashboard configuration.
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    /// Update interval in milliseconds
    pub update_interval_ms: u64,
    /// Maximum recent events to keep
    pub max_events: usize,
    /// Whether to show verbose output
    pub verbose: bool,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            update_interval_ms: 1000,  // 1 second
            max_events: 100,
            verbose: false,
        }
    }
}

/// Live dashboard for IGLA RACE.
pub struct Dashboard {
    config: DashboardConfig,
    events: Vec<DashboardEvent>,
    status_query: QueryStatus,
}

impl Dashboard {
    /// Create a new dashboard.
    pub fn new(config: DashboardConfig) -> Result<Self, anyhow::Error> {
        let status_query = QueryStatus::new()?;
        Ok(Self {
            config,
            events: Vec::new(),
            status_query,
        })
    }

    /// Run the dashboard.
    ///
    /// Continuously polls for status updates and displays them.
    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        info!("Starting dashboard (interval: {}ms)", self.config.update_interval_ms);

        let mut tick = interval(Duration::from_millis(self.config.update_interval_ms));

        loop {
            tick.tick().await;

            // Query status
            match self.status_query.execute().await {
                Ok(status) => {
                    self.display_status(&status);
                }
                Err(e) => {
                    error!("Failed to query status: {}", e);
                }
            }

            // Display events
            self.display_events();
        }
    }

    /// Add a dashboard event.
    pub fn add_event(&mut self, event: DashboardEvent) {
        self.events.push(event);
        // Keep only max_events
        if self.events.len() > self.config.max_events {
            self.events.drain(0..self.events.len() - self.config.max_events);
        }
    }

    /// Display current status.
    fn display_status(&self, status: &RaceStatus) {
        // Clear screen and redraw
        print!("\x1B[2J");  // ANSI clear screen
        print!("\x1B[H");   // Move cursor to home

        println!("IGLA RACE Dashboard");
        println!("══════════════════");
        println!("Best BPB:   {:.4} (trial {})",
                 status.best_bpb, status.best_trial_id);
        println!("Total:       {} trials", status.total_trials);
        println!("Active:       {} workers", status.active_workers);
        println!();
        println!("Recent Activity:");
        println!("─────────────────");

        for trial in status.recent_trials.iter().take(10) {
            println!("  Trial {}: BPB={:.4}, rung={}",
                     trial.trial_id, trial.bpb, trial.rung);
        }
    }

    /// Display events.
    fn display_events(&self) {
        if self.config.verbose {
            println!();
            println!("Event Log ({} recent):", self.events.len());
            println!("───────────────────────────");

            for event in self.events.iter().rev().take(20) {
                match event {
                    DashboardEvent::TrialStarted { trial_id } => {
                        println!("  [{}] Trial {} started",
                                 chrono::Utc::now().format("%H:%M:%S"), trial_id);
                    }
                    DashboardEvent::TrialCheckpoint { trial_id, rung, bpb } => {
                        println!("  [{}] Trial {} checkpoint: rung {}, BPB={:.4}",
                                 chrono::Utc::now().format("%H:%M:%S"), trial_id, rung, bpb);
                    }
                    DashboardEvent::TrialCompleted { trial_id, bpb } => {
                        println!("  [{}] Trial {} completed: BPB={:.4}",
                                 chrono::Utc::now().format("%H:%M:%S"), trial_id, bpb);
                    }
                    DashboardEvent::Victory { trial_id, bpb } => {
                        println!("  [{}] VICTORY! Trial {} BPB={:.4}",
                                 chrono::Utc::now().format("%H:%M:%S"), trial_id, bpb);
                    }
                    DashboardEvent::WorkerUpdate { worker_id, active } => {
                        println!("  [{}] Worker {} {}",
                                 chrono::Utc::now().format("%H:%M:%S"), worker_id,
                                 if *active { "active" } else { "idle" });
                    }
                }
            }
        }
    }
}
