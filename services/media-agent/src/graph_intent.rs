//! Graph Runtime Intent — shared contract between Node (Control Plane) and Rust
//! (Hardware Plane). SoT mandates a typed contract, NOT string-JSON guessing:
//! the TypeScript `GraphRuntimeIntent` is mirrored here and deserialized via serde.
//! CI `cargo test` validates that the Rust side parses the canonical JSON schema.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphRuntimeIntent {
    pub version: String,
    pub devices: Vec<DeviceIntent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceIntent {
    pub device_id: String,
    pub role: String,
    pub pipeline: PipelineIntent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipelineIntent {
    pub source: SourceIntent,
    pub sink: SinkIntent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum SourceIntent {
    /// Existing DeckLink wire shape remains `{kind, device_id, port_id}`.
    #[serde(rename = "decklink")]
    Decklink {
        device_id: String,
        port_id: Option<String>,
    },
    /// Network source identity is independent of hardware DeviceId/PortId.
    #[serde(rename = "rtmp")]
    Rtmp {
        source_id: crate::source::NetworkSourceId,
        endpoint: crate::source::NetworkEndpoint,
    },
    #[serde(rename = "self_test")]
    SelfTest,
}

impl SourceIntent {
    pub fn decklink(device_id: impl Into<String>, port_id: Option<String>) -> Self {
        Self::Decklink {
            device_id: device_id.into(),
            port_id,
        }
    }

    pub fn rtmp(
        source_id: crate::source::NetworkSourceId,
        endpoint: crate::source::NetworkEndpoint,
    ) -> Self {
        Self::Rtmp {
            source_id,
            endpoint,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Decklink { .. } => "decklink",
            Self::Rtmp { .. } => "rtmp",
            Self::SelfTest => "self_test",
        }
    }

    pub fn device_id(&self) -> Option<&str> {
        match self {
            Self::Decklink { device_id, .. } => Some(device_id),
            Self::Rtmp { .. } | Self::SelfTest => None,
        }
    }

    pub fn port_id(&self) -> Option<&str> {
        match self {
            Self::Decklink { port_id, .. } => port_id.as_deref(),
            Self::Rtmp { .. } | Self::SelfTest => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SinkIntent {
    pub kind: String,
}

/// Parse a GraphRuntimeIntent from JSON (the contract guardrail).
pub fn from_json(s: &str) -> Result<GraphRuntimeIntent, serde_json::Error> {
    serde_json::from_str(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "version": "1.0",
      "devices": [
        {
          "device_id": "decklink-0",
          "role": "CAPTURE",
          "pipeline": {
            "source": { "kind": "decklink", "device_id": "dev-a" },
            "sink":   { "kind": "rtmp" }
          }
        }
      ]
    }"#;

    #[test]
    fn deserializes_canonical_intent() {
        let intent = from_json(SAMPLE).expect("must parse canonical GraphRuntimeIntent");
        assert_eq!(intent.version, "1.0");
        assert_eq!(intent.devices.len(), 1);
        let d = &intent.devices[0];
        assert_eq!(d.device_id, "decklink-0");
        assert_eq!(d.role, "CAPTURE");
        assert_eq!(d.pipeline.source.kind(), "decklink");
        assert_eq!(d.pipeline.source.device_id(), Some("dev-a"));
        assert_eq!(d.pipeline.sink.kind, "rtmp");
    }

    #[test]
    fn network_source_uses_typed_identity_and_credential_free_wire_shape() {
        let source_id = crate::source::NetworkSourceId(uuid::Uuid::new_v4());
        let source = SourceIntent::rtmp(
            source_id,
            crate::source::NetworkEndpoint {
                protocol: crate::source::NetworkProtocol::Rtmp,
                host: "127.0.0.1".into(),
                port: 1935,
                path: "/live/input".into(),
            },
        );
        assert_eq!(source.kind(), "rtmp");
        assert_eq!(source.device_id(), None);
        assert_eq!(source.port_id(), None);
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(json["kind"], "rtmp");
        assert!(json.get("device_id").is_none());
        assert!(json.get("port_id").is_none());
        assert_eq!(
            serde_json::from_value::<SourceIntent>(json).unwrap(),
            source
        );
    }

    #[test]
    fn roundtrips_through_json() {
        let intent = from_json(SAMPLE).unwrap();
        let serialized = serde_json::to_string(&intent).unwrap();
        let again = from_json(&serialized).unwrap();
        assert_eq!(intent, again);
    }
}
