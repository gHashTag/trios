//! `trios-scarab-types` — facade for the 5-ring parallel-execution
//! foundation. Re-exports the typed scarab enums from each ring.
//!
//! L-RING-FACADE-001: this file MUST NOT contain business logic — only
//! re-exports. Closes #479 · Part of #446.

pub use trios_scarab_types_sr_00::{Term, TermScarab};
pub use trios_scarab_types_sr_01::{LaneScarab, LaneScarabType};
pub use trios_scarab_types_sr_02::{SoulScarab, SoulScarabType};
pub use trios_scarab_types_sr_03::{GateScarab as GateScarabFour, GateScarabFourType};
pub use trios_scarab_types_sr_04::{GateScarab as GateScarabFive, GateScarabFiveType};
