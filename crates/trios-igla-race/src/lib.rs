pub mod asha;
pub mod lessons;
pub mod neon;
pub mod status;

// Re-export from asha
pub use asha::{AshaConfig, AshaRung, record_checkpoint, register_trial};

// Re-export from lessons
pub use lessons::{generate_lesson, get_top_lessons, store_lesson, LessonType, Outcome, TrialConfig, RungData};

// Re-export from neon
pub use neon::{NeonDb, LessonEntry};

pub use status::*;

pub const IGLA_TARGET_BPB: f64 = 1.5;
pub const ASHA_KEEP_FRACTION: f64 = 0.33;
