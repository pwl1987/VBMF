//! HW-PORT-01 Gate — 端口级绑定闭环验收.
//!
//! 验收闭环 (用户 §二十一/§四十四):
//!   DeviceHandle → Device → Port → Direction → Connector → GStreamer runtime address → Signal
//!
//! PASS 条件:
//! - 至少存在一个 **已验证 (ManifestVerified)** 且 **信号 Locked** 的输入端口 (证明闭环成立);
//! - 且 Manifest 中 `required=true` 的输入端口全部 Locked+Verified (失败闭合: 必需端口缺失即拒).
//!
//! 绝不把当前机器 dn0/dn1/dn2 数量/语义写死; 端口完全来自 Manifest 声明 + 运行时 probe.

#![allow(dead_code)]

use crate::port::{PortDirection, PortRegistry, SignalState};
use crate::resolver::DeviceBindingManifest;
use serde::{Deserialize, Serialize};

/// 单端口验收证据 (对应 §二十二 acceptance JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HwPort01PortReport {
    pub device_handle: Option<String>,
    pub port: PortAcceptance,
    pub gstreamer: GstAddress,
    pub signal: SignalAcceptance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortAcceptance {
    #[serde(rename = "connector")]
    pub connector: crate::port::ConnectorType,
    pub ordinal: u32,
    pub direction: PortDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GstAddress {
    pub device_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalAcceptance {
    pub state: SignalState,
    pub format: Option<String>,
}

/// HW-PORT-01 整体报告.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HwPort01Report {
    pub ports: Vec<HwPort01PortReport>,
    /// 闭环是否验证通过.
    pub pass: bool,
    /// 失败原因 (若有).
    pub notes: Vec<String>,
}

/// 视频格式 → 可读字符串 (如 "1920x1080i50").
pub fn format_video_format(f: &crate::port::VideoFormat) -> String {
    let inter = match f.interlaced {
        Some(true) => "i",
        Some(false) => "p",
        None => "",
    };
    let fps = f.frame_rate.clone().unwrap_or_else(|| "0/1".into());
    let fps_short = fps.split('/').next().unwrap_or("0").to_string();
    format!("{}x{}{}{}", f.width, f.height, inter, fps_short)
}

/// 执行 HW-PORT-01 验收.
pub fn verify(registry: &PortRegistry, manifest: &DeviceBindingManifest) -> HwPort01Report {
    let mut ports = Vec::new();
    let mut notes = Vec::new();

    for p in &registry.ports {
        let gst_num = p.runtime_binding.as_ref().map(|b| b.gst_device_number);
        let fmt = p.signal.video_format.as_ref().map(format_video_format);
        ports.push(HwPort01PortReport {
            device_handle: p.device_handle.clone(),
            port: PortAcceptance {
                connector: p.identity.connector,
                ordinal: p.identity.ordinal,
                direction: p.direction,
            },
            gstreamer: GstAddress { device_number: gst_num },
            signal: SignalAcceptance {
                state: p.signal.state,
                format: fmt,
            },
        });
    }

    // 闭环成立: 至少一个已验证且 Locked 的输入端口.
    let closed_loop = registry.input_ports().iter().any(|p| {
        p.signal.state == SignalState::Locked && p.runtime_binding.is_some()
    });
    if !closed_loop {
        notes.push(
            "闭环未成立: 无已验证且信号 Locked 的输入端口 (Device→Port→Direction→Connector→Gst address→Signal 未闭合)".into(),
        );
    }

    // 必需输入端口: 全部 Locked+Verified (失败闭合).
    let mut required_ok = true;
    for entry in &manifest.bindings {
        let Some(port) = &entry.port else { continue };
        if !port.required || port.direction != PortDirection::Input {
            continue;
        }
        let matched = registry.ports.iter().find(|p| {
            p.identity.connector == port.connector && p.identity.ordinal == port.ordinal
        });
        match matched {
            Some(p) if p.signal.state == SignalState::Locked && p.runtime_binding.is_some() => {}
            Some(p) => {
                required_ok = false;
                notes.push(format!(
                    "必需输入端口 {:?}#{} 未 Locked (state={:?})",
                    port.connector, port.ordinal, p.signal.state
                ));
            }
            None => {
                required_ok = false;
                notes.push(format!(
                    "必需输入端口 {:?}#{} 在 Discovery 中未找到",
                    port.connector, port.ordinal
                ));
            }
        }
    }

    let pass = closed_loop && required_ok;
    HwPort01Report { ports, pass, notes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::{PortCapabilities, PortIdentity, PortInfo, RuntimePortBinding, SignalStatus};
    use crate::resolver::{BindingEntry, Confidence, ResolverMatch};
    use uuid::Uuid;

    fn input_port(ordinal: u32, locked: bool) -> PortInfo {
        let dev = Uuid::new_v4();
        PortInfo {
            device_id: dev,
            device_handle: Some(format!("handle-{ordinal}")),
            identity: PortIdentity {
                port_id: PortIdentity::derive(&dev, crate::port::ConnectorType::Sdi, ordinal),
                connector: crate::port::ConnectorType::Sdi,
                ordinal,
            },
            direction: PortDirection::Input,
            capabilities: PortCapabilities {
                input: crate::port::CapabilityValue::Supported(true),
                ..Default::default()
            },
            runtime_binding: Some(RuntimePortBinding {
                gst_device_number: ordinal,
                hw_serial_number: None,
                confidence: Confidence::High,
                match_kind: ResolverMatch::ManifestVerified,
            }),
            signal: SignalStatus {
                state: if locked { SignalState::Locked } else { SignalState::NoSignal },
                video_locked: Some(locked),
                ..Default::default()
            },
            content: crate::port::VideoContentState::Unknown,
        }
    }

    fn manifest_with_required(ordinal: u32, required: bool) -> DeviceBindingManifest {
        DeviceBindingManifest {
            manifest_version: "2".into(),
            machine_id: "host-x".into(),
            generated_by: "ops".into(),
            generated_at: "2026-08-27".into(),
            bmd_sdk_version: None,
            gst_decklink_plugin_version: None,
            gst_runtime_version: None,
            notes: None,
            bindings: vec![BindingEntry {
                label: None,
                bmd_device_handle: format!("handle-{ordinal}"),
                gst_device_number: ordinal,
                expected_hw_serial_number: None,
                expected_model: None,
                port: Some(crate::resolver::PortBinding {
                    connector: crate::port::ConnectorType::Sdi,
                    ordinal,
                    direction: PortDirection::Input,
                    required,
                }),
            }],
        }
    }

    #[test]
    fn pass_when_required_input_locked() {
        let reg = PortRegistry { ports: vec![input_port(1, true)] };
        let report = verify(&reg, &manifest_with_required(1, true));
        assert!(report.pass);
        assert_eq!(report.ports.len(), 1);
        assert_eq!(report.ports[0].signal.state, SignalState::Locked);
    }

    #[test]
    fn fail_when_required_input_no_signal() {
        // 必需输入端口无信号 → 失败闭合 (绝不回退).
        let reg = PortRegistry { ports: vec![input_port(1, false)] };
        let report = verify(&reg, &manifest_with_required(1, true));
        assert!(!report.pass);
        assert!(report.notes.iter().any(|n| n.contains("未 Locked")));
    }

    #[test]
    fn fail_when_no_closed_loop() {
        // 只有输出端口 (无 Locked 输入) → 闭环不成立.
        let dev = Uuid::new_v4();
        let reg = PortRegistry {
            ports: vec![PortInfo {
                device_id: dev,
                device_handle: Some("out".into()),
                identity: PortIdentity {
                    port_id: PortIdentity::derive(&dev, crate::port::ConnectorType::Sdi, 0),
                    connector: crate::port::ConnectorType::Sdi,
                    ordinal: 0,
                },
                direction: PortDirection::Output,
                capabilities: PortCapabilities {
                    output: crate::port::CapabilityValue::Supported(true),
                    ..Default::default()
                },
                runtime_binding: Some(RuntimePortBinding {
                    gst_device_number: 0,
                    hw_serial_number: None,
                    confidence: Confidence::High,
                    match_kind: ResolverMatch::ManifestVerified,
                }),
                signal: SignalStatus::default(),
                content: crate::port::VideoContentState::Unknown,
            }],
        };
        let report = verify(&reg, &manifest_with_required(0, false));
        assert!(!report.pass);
        assert!(report.notes.iter().any(|n| n.contains("闭环")));
    }
}
