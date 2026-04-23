pub mod envelope;
pub mod payload;
pub mod types;
pub mod validate;

pub use envelope::{MessageEnvelope, RingId, PROTOCOL_VERSION};
pub use payload::IpcPayload;
pub use validate::IpcError;
