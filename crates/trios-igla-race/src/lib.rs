pub mod asha;
pub mod invariants;
pub mod lessons;
pub mod neon;
pub mod sampler;
pub mod status;

pub use asha::{AshaConfig, AshaRung, record_checkpoint, register_trial};

pub use lessons::{generate_lesson, get_top_lessons, store_lesson, LessonType, Outcome, TrialConfig, RungData};

pub use neon::{NeonDb, LessonEntry, DashboardMeta, spawn_heartbeat};

pub use status::*;

pub use invariants::{InvTrialConfig, GradientMode, InvError, validate_config};

pub const IGLA_TARGET_BPB: f64 = 1.5;
pub const ASHA_KEEP_FRACTION: f64 = 0.33;
