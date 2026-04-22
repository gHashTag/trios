//! trios-http — HTTP REST gateway
//!
//! Ring architecture:
//!   HR-00 — core types (AppState, request/response structs)
//!   HR-01 — routes (POST /api/chat, GET /api/status, GET /health)
//!   BR-OUTPUT — axum Router builder

pub use hr_00 as types;
pub use hr_01 as routes;
pub use br_output as router;
