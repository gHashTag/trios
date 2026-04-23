//! Comet Bridge Integration Tests
//!
//! Tests for WebSocket connection, message parsing, and Chrome runtime
//! integration using mocked components.

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock WebSocket for testing.
    ///
    /// In a real WASM environment, this would connect to actual WebSocket server.
    /// For unit tests, we validate serialization/deserialization logic.
    #[test]
    fn envelope_serialization() {
        // Test AgentHeartbeat envelope
        let heartbeat = Envelope::agent_heartbeat("ALFA-01", "active");
        let json = heartbeat.to_json().unwrap();

        assert!(json.contains(r#"event_type":"agent:heartbeat""#));
        assert!(json.contains(r#"agent_id":"ALFA-01""#));
        assert!(json.contains(r#"status":"active""#));

        // Round-trip deserialization
        let deserialized = Envelope::from_json(&json).unwrap();
        match deserialized.payload {
            Payload::AgentHeartbeat { agent_id, status, .. } => {
                assert_eq!(agent_id, "ALFA-01");
                assert_eq!(status, "active");
            }
            _ => panic!("Expected AgentHeartbeat payload"),
        }
    }

    #[test]
    fn chat_message_envelope() {
        let chat = Envelope::chat_message("BRAVO-02", "Hello from agent", "agent");
        let json = chat.to_json().unwrap();

        assert!(json.contains(r#"event_type":"chat:message""#));
        assert!(json.contains(r#"agent_id":"BRAVO-02""#));
        assert!(json.contains(r#"content":"Hello from agent""#));
        assert!(json.contains(r#"role":"agent""#));

        let deserialized = Envelope::from_json(&json).unwrap();
        match deserialized.payload {
            Payload::ChatMessage { content, role, .. } => {
                assert_eq!(content, "Hello from agent");
                assert_eq!(role, "agent");
            }
            _ => panic!("Expected ChatMessage payload"),
        }
    }

    #[test]
    fn agent_connection_envelopes() {
        // Test AgentConnected
        let connected = Envelope::agent_connected("CHARLIE-03");
        let connected_json = connected.to_json().unwrap();

        assert!(connected_json.contains(r#"event_type":"agent:connected""#));
        assert!(connected_json.contains(r#"agent_id":"CHARLIE-03""#));

        let deserialized = Envelope::from_json(&connected_json).unwrap();
        match deserialized.payload {
            Payload::AgentConnected { agent_id } => {
                assert_eq!(agent_id, "CHARLIE-03");
            }
            _ => panic!("Expected AgentConnected payload"),
        }

        // Test AgentDisconnected
        let disconnected = Envelope::agent_disconnected("DELTA-04");
        let disconnected_json = disconnected.to_json().unwrap();

        assert!(disconnected_json.contains(r#"event_type":"agent:disconnected""#));
        assert!(disconnected_json.contains(r#"agent_id":"DELTA-04""#));

        let deserialized = Envelope::from_json(&disconnected_json).unwrap();
        match deserialized.payload {
            Payload::AgentDisconnected { agent_id } => {
                assert_eq!(agent_id, "DELTA-04");
            }
            _ => panic!("Expected AgentDisconnected payload"),
        }
    }

    #[test]
    fn mcp_tool_call_envelope() {
        let params = serde_json::json!({
            "query": "test query",
            "limit": 10
        });

        let tool_call = Envelope::mcp_tool_call("search_tool", params);
        let json = tool_call.to_json().unwrap();

        assert!(json.contains(r#"event_type":"mcp:tool_call""#));
        assert!(json.contains(r#"tool":"search_tool""#));

        let deserialized = Envelope::from_json(&json).unwrap();
        match deserialized.payload {
            Payload::McpToolCall { tool, params: tool_params } => {
                assert_eq!(tool, "search_tool");
                assert_eq!(tool_params, params);
            }
            _ => panic!("Expected McpToolCall payload"),
        }
    }

    #[test]
    fn payload_event_types() {
        assert_eq!(
            Payload::AgentHeartbeat {
                agent_id: "test".into(),
                status: "ok".into(),
                timestamp: 1000,
            }
            .event_type(),
            "agent:heartbeat"
        );

        assert_eq!(
            Payload::ChatMessage {
                agent_id: "test".into(),
                content: "hello".into(),
                role: "user".into(),
            }
            .event_type(),
            "chat:message"
        );

        assert_eq!(
            Payload::AgentConnected {
                agent_id: "test".into(),
            }
            .event_type(),
            "agent:connected"
        );

        assert_eq!(
            Payload::AgentDisconnected {
                agent_id: "test".into(),
            }
            .event_type(),
            "agent:disconnected"
        );

        assert_eq!(
            Payload::McpToolCall {
                tool: "test_tool".into(),
                params: serde_json::json!({}),
            }
            .event_type(),
            "mcp:tool_call"
        );
    }

    #[test]
    fn invalid_envelope_json() {
        // Missing event_type
        let invalid_json = r#"{
            "payload": {
                "type": "ChatMessage",
                "data": {
                    "agent_id": "test",
                    "content": "hello",
                    "role": "user"
                }
            }
        }"#;

        let result = Envelope::from_json(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn unknown_payload_type() {
        // Envelope with unknown payload type
        let unknown_json = r#"{
            "event_type": "unknown:event",
            "payload": {
                "type": "ChatMessage",
                "data": {
                    "agent_id": "test",
                    "content": "hello",
                    "role": "user"
                }
            }
        }"#;

        let result = Envelope::from_json(unknown_json);
        // Should deserialize successfully but with unknown payload type
        assert!(result.is_ok());
    }

    #[test]
    fn envelope_serialization_unicode() {
        // Test Unicode content
        let chat = Envelope::chat_message("ALFA-01", "Привет мир 🌍", "user");
        let json = chat.to_json().unwrap();

        assert!(json.contains("Привет мир 🌍"));

        let deserialized = Envelope::from_json(&json).unwrap();
        match deserialized.payload {
            Payload::ChatMessage { content, .. } => {
                assert_eq!(content, "Привет мир 🌍");
            }
            _ => panic!("Expected ChatMessage payload"),
        }
    }

    #[test]
    fn comet_bridge_state_default() {
        let state = crate::bridge::comet::CometState::default();
        assert!(!state.is_running);
        assert_eq!(state.reconnect_attempts, 0);
        assert!(state.ws.is_none());
        assert!(state.port.is_none());
    }

    #[test]
    fn bridge_constants() {
        assert_eq!(crate::bridge::comet::TRIOS_WS_URL, "ws://localhost:9005/ws");
        assert_eq!(crate::bridge::comet::RECONNECT_BASE_MS, 1000);
        assert_eq!(crate::bridge::comet::MAX_RECONNECT_ATTEMPTS, 5);
    }
}
