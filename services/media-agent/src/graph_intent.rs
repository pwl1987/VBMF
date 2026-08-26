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
    /// GStreamer decklinkvideosrc 的 `device-number` 属性 (可选).
    /// canonical 身份由 `DeviceIntent.device_id` (DeviceHandle 派生 UUID) 承载;
    /// 此处仅作临时 probe / fallback. **GStreamer decklink 插件无 `persistent-id`
    /// 属性**, 因此物化时必须由 canonical identity 解析出 `device-number`
    /// (见 `pipeline::materialize`).
    pub device_number: Option<u32>,
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
            "source": { "kind": "decklink", "device_number": 0 },
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
        assert_eq!(d.pipeline.source.device_number, Some(0));
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
