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
pub struct SourceIntent {
    pub kind: String,
    /// VBMF canonical device identity (DeviceHandle 派生 UUID). **不**携带任何
    /// GStreamer 专属属性 (如 `device-number` / `persistent-id`); 这些由 Media
    /// Agent 经 Device Registry 物化得到 (见 `pipeline::materialize`). Control
    /// Plane 无需感知底层采集硬件选择细节.
    pub device_id: String,
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
        assert_eq!(d.pipeline.source.kind, "decklink");
        assert_eq!(d.pipeline.source.device_id, "dev-a");
        assert_eq!(d.pipeline.sink.kind, "rtmp");
    }

    #[test]
    fn roundtrips_through_json() {
        let intent = from_json(SAMPLE).unwrap();
        let serialized = serde_json::to_string(&intent).unwrap();
        let again = from_json(&serialized).unwrap();
        assert_eq!(intent, again);
    }
}
