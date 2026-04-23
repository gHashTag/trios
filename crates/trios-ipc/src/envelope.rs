use serde::{Deserialize, Serialize};
use crate::payload::IpcPayload;

pub const PROTOCOL_VERSION: u8 = 1;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageEnvelope {
    pub version: u8,
    pub id: String,
    pub from: RingId,
    pub to: RingId,
    pub payload: IpcPayload,
}

impl MessageEnvelope {
    pub fn new(from: RingId, to: RingId, payload: IpcPayload) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            id: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            payload,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RingId {
    UrState,
    UrApiClient,
    ExtBackground,
    ExtDom,
    ExtComet,
    SvBrowser,
}
