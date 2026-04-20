pub mod bridge;
pub mod process;

pub use bridge::ClaudeBridge;
pub use process::{AgentId, AgentStatus, ChildProcess};

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
