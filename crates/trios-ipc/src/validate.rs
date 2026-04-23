use crate::envelope::{MessageEnvelope, RingId, PROTOCOL_VERSION};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcError {
    VersionMismatch { expected: u8, got: u8 },
    MissingId,
    InvalidPayload(String),
    UnauthorizedRoute { from: RingId, to: RingId },
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpcError::VersionMismatch { expected, got } => {
                write!(f, "version mismatch: expected {}, got {}", expected, got)
            }
            IpcError::MissingId => write!(f, "missing message id"),
            IpcError::InvalidPayload(msg) => write!(f, "invalid payload: {}", msg),
            IpcError::UnauthorizedRoute { from, to } => {
                write!(f, "unauthorized route: {:?} -> {:?}", from, to)
            }
        }
    }
}

impl std::error::Error for IpcError {}

impl MessageEnvelope {
    pub fn validate(&self) -> Result<(), IpcError> {
        if self.version != PROTOCOL_VERSION {
            return Err(IpcError::VersionMismatch {
                expected: PROTOCOL_VERSION,
                got: self.version,
            });
        }
        if self.id.is_empty() {
            return Err(IpcError::MissingId);
        }
        validate_route(&self.from, &self.to)?;
        Ok(())
    }
}

fn validate_route(from: &RingId, to: &RingId) -> Result<(), IpcError> {
    let allowed = matches!(
        (from, to),
        (RingId::UrApiClient, RingId::ExtBackground)
        | (RingId::ExtBackground, RingId::UrApiClient)
        | (RingId::ExtBackground, RingId::ExtDom)
        | (RingId::ExtBackground, RingId::SvBrowser)
    );
    if !allowed {
        return Err(IpcError::UnauthorizedRoute {
            from: from.clone(),
            to: to.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::IpcPayload;

    fn valid_envelope(from: RingId, to: RingId, version: u8) -> MessageEnvelope {
        MessageEnvelope {
            version,
            id: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            payload: IpcPayload::GetConnectionStatus,
        }
    }

    #[test]
    fn test_valid_route_ui_to_ext() {
        let env = valid_envelope(RingId::UrApiClient, RingId::ExtBackground, PROTOCOL_VERSION);
        assert!(env.validate().is_ok());
    }

    #[test]
    fn test_valid_route_ext_to_ui() {
        let env = valid_envelope(RingId::ExtBackground, RingId::UrApiClient, PROTOCOL_VERSION);
        assert!(env.validate().is_ok());
    }

    #[test]
    fn test_valid_route_ext_to_dom() {
        let env = valid_envelope(RingId::ExtBackground, RingId::ExtDom, PROTOCOL_VERSION);
        assert!(env.validate().is_ok());
    }

    #[test]
    fn test_valid_route_ext_to_server() {
        let env = valid_envelope(RingId::ExtBackground, RingId::SvBrowser, PROTOCOL_VERSION);
        assert!(env.validate().is_ok());
    }

    #[test]
    fn test_version_mismatch() {
        let env = valid_envelope(RingId::UrApiClient, RingId::ExtBackground, 0);
        let err = env.validate().unwrap_err();
        assert_eq!(
            err,
            IpcError::VersionMismatch {
                expected: PROTOCOL_VERSION,
                got: 0
            }
        );
    }

    #[test]
    fn test_missing_id() {
        let mut env = valid_envelope(RingId::UrApiClient, RingId::ExtBackground, PROTOCOL_VERSION);
        env.id = String::new();
        let err = env.validate().unwrap_err();
        assert_eq!(err, IpcError::MissingId);
    }

    #[test]
    fn test_unauthorized_route() {
        let env = valid_envelope(RingId::UrState, RingId::ExtDom, PROTOCOL_VERSION);
        let err = env.validate().unwrap_err();
        assert!(matches!(err, IpcError::UnauthorizedRoute { .. }));
    }

    #[test]
    fn test_unauthorized_route_dom_to_server() {
        let env = valid_envelope(RingId::ExtDom, RingId::SvBrowser, PROTOCOL_VERSION);
        let err = env.validate().unwrap_err();
        assert!(matches!(err, IpcError::UnauthorizedRoute { .. }));
    }

    #[test]
    fn test_envelope_new_has_correct_version() {
        let env = MessageEnvelope::new(
            RingId::UrApiClient,
            RingId::ExtBackground,
            IpcPayload::GetConnectionStatus,
        );
        assert_eq!(env.version, PROTOCOL_VERSION);
        assert!(!env.id.is_empty());
        assert_eq!(env.from, RingId::UrApiClient);
        assert_eq!(env.to, RingId::ExtBackground);
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let env = MessageEnvelope::new(
            RingId::UrApiClient,
            RingId::ExtBackground,
            IpcPayload::SendChatMessage {
                text: "hello".to_string(),
                agent_id: None,
            },
        );
        let json = serde_json::to_string(&env).unwrap();
        let decoded: MessageEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.version, env.version);
        assert_eq!(decoded.id, env.id);
        assert_eq!(decoded.from, env.from);
        assert_eq!(decoded.to, env.to);
    }
}
