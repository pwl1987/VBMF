//! 测试 Fixture 模型 — 当前真实 SDI loopback 作为可复用 Fixture.
//!
//! Boundary (用户 §二十三/§二十四): 绝不写死 "Mini Monitor 4K → 双路卡".
//! Fixture 只声明 `source device/port` → `sink device/port` (medium=SDI),
//! 物理接线变更只需替换 device/port 引用, 不触动代码/Schema. 所有具体 device_id/port_id
//! 属于 `evidence/bmd-10.30.15.10/` 的 `HOST_SPECIFIC` 观察, 不得进入架构事实.

#![allow(dead_code)]

use crate::port::{PortRegistry, SignalState, VideoContentState};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

/// 传输介质.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportMedium {
    Sdi,
    Hdmi,
    Optical,
    Analog,
    Unknown,
}

/// 端口端点引用 (由 operator 在 host-specific 证据中填具体 UUID).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PortRef {
    /// VBMF 设备 ID (UUID 字符串).
    pub device_id: Option<String>,
    /// VBMF 端口 ID (UUID 字符串).
    pub port_id: Option<String>,
}

/// 期望信号 (验收判据).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpectedSignal {
    /// 期望信号状态 (loopback 应为 locked).
    pub state: SignalState,
    /// 期望视频格式 (如 "1080i50"); 仅报告/告警, 不强制.
    pub format: Option<String>,
    /// 期望内容态 (loopback 应为 Active); 仅当 `Some` 时强制验收, `None` 不检查内容.
    /// 旧版 Fixture JSON 缺此字段时反序列化为 `None` (向后兼容, 见 STEP 9).
    #[serde(default)]
    pub content: Option<VideoContentState>,
}

/// 单条测试 Fixture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fixture {
    pub fixture_id: String,
    /// 信号源 (BMD 输出能力端口).
    pub source: PortRef,
    /// 信号汇 (BMD 输入能力端口).
    pub sink: PortRef,
    pub transport: TransportMedium,
    pub expected: ExpectedSignal,
    /// 备注 (host-specific 观察说明).
    pub notes: Option<String>,
}

impl Fixture {
    /// 保存为 JSON 证据文件.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let s = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, s)
    }

    /// 从 JSON 证据文件加载.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let s = std::fs::read_to_string(path)?;
        serde_json::from_str(&s)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// 从目录批量加载全部 `*.json` Fixture 证据文件 (loopback 验收多 fixture 入口).
    ///
    /// 跳过子目录与非 `.json` 文件; 任一文件解析失败即整体返回 `Err` (失败闭合, 不静默跳过坏证据).
    pub fn load_dir(dir: &Path) -> std::io::Result<Vec<Fixture>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                out.push(Fixture::load(&path)?);
            }
        }
        Ok(out)
    }

    /// 由 PortRegistry 解析出 loopback 的 source(输出端口)/sink(已锁定输入端口).
    /// 返回 (source_port_id, sink_port_id) 若可解析; 否则 None (诊断信息不足).
    /// 按 Fixture 显式声明的 `source`/`sink` `PortRef` 精确匹配端口; 仅当 Fixture 为空模板
    /// (port_id 均未填, 即 host-specific provisioning 阶段) 才回退到 "第一个 Locked 输入 / 第一个输出"
    /// 的启发式 (绝不在已声明拓扑时猜, §二十一 P1#4).
    pub fn resolve(&self, registry: &PortRegistry) -> Option<(String, String)> {
        let sink = if let Some(pid) = &self.sink.port_id {
            registry
                .get(&Uuid::parse_str(pid).ok()?)
                .and_then(|p| p.identity.port_id)
                .map(|u| u.to_string())
        } else {
            // 未声明拓扑 (host-specific 模板 fixture): 确定性回退到第一个输入端口.
            // 注意: 不要求 signal==Locked —— loopback 诊断早于信号探测, 运行时端口 signal 尚未填充;
            // 由 probe_signal 实测该端口实际信号态 (Locked/NoSignal).
            registry
                .input_ports()
                .first()
                .and_then(|p| p.identity.port_id)
                .map(|u| u.to_string())
        }?;
        let source = if let Some(pid) = &self.source.port_id {
            registry
                .get(&Uuid::parse_str(pid).ok()?)
                .and_then(|p| p.identity.port_id)
                .map(|u| u.to_string())
        } else {
            registry
                .output_ports()
                .first()
                .and_then(|p| p.identity.port_id)
                .map(|u| u.to_string())
        }?;
        Some((source, sink))
    }
}

/// 默认 loopback Fixture 模板 (字段留空, 由 host-specific 证据填充; 不硬编码拓扑).
pub fn default_sdi_loopback() -> Fixture {
    Fixture {
        fixture_id: "BMD-SDI-LOOPBACK-01".into(),
        source: PortRef::default(),
        sink: PortRef::default(),
        transport: TransportMedium::Sdi,
        expected: ExpectedSignal {
            state: SignalState::Locked,
            format: Some("1080i50".into()),
            content: Some(VideoContentState::Active),
        },
        notes: Some(
            "HOST_SPECIFIC / OBSERVED: 真实 SDI 环路 (BMD 输出端口 → BMD 输入端口). 具体 device/port UUID 由 Discovery 运行时填充.".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::{
        ConnectorType, PortCapabilities, PortDirection, PortIdentity, PortInfo, PortOrdinal,
        RuntimePortBinding,
    };
    use crate::resolver::{Confidence, ResolverMatch};
    use uuid::Uuid;

    fn port(
        device_id: Uuid,
        connector: ConnectorType,
        ordinal: u32,
        dir: PortDirection,
        signal: SignalState,
    ) -> PortInfo {
        PortInfo {
            device_id,
            provider_binding_ref: None,
            identity: PortIdentity {
                port_id: PortIdentity::derive(&device_id, connector, PortOrdinal::Known(ordinal)),
                connector,
                ordinal: PortOrdinal::Known(ordinal),
            },
            direction: dir,
            capabilities: PortCapabilities {
                input: if matches!(dir, PortDirection::Input) {
                    crate::port::CapabilityValue::Supported(true)
                } else {
                    crate::port::CapabilityValue::Unknown
                },
                ..Default::default()
            },
            runtime_binding: Some(RuntimePortBinding {
                gst_device_number: ordinal,
                hw_serial_number: None,
                confidence: Confidence::High,
                match_kind: ResolverMatch::ManifestVerified,
            }),
            signal: crate::port::SignalStatus {
                state: signal,
                video_locked: Some(signal == SignalState::Locked),
                ..Default::default()
            },
            content: crate::port::VideoContentState::Unknown,
        }
    }

    #[test]
    fn default_fixture_is_template_without_hardcoded_topology() {
        let f = default_sdi_loopback();
        assert_eq!(f.fixture_id, "BMD-SDI-LOOPBACK-01");
        assert_eq!(f.transport, TransportMedium::Sdi);
        // 默认模板不得硬编码具体 device/port (避免把当前拓扑写死).
        assert!(f.source.device_id.is_none());
        assert!(f.sink.device_id.is_none());
    }

    #[test]
    fn fixture_roundtrips_through_json() {
        let f = default_sdi_loopback();
        let s = serde_json::to_string(&f).unwrap();
        let back: Fixture = serde_json::from_str(&s).unwrap();
        assert_eq!(f, back);
    }

    #[test]
    fn expected_signal_deserializes_old_json_without_content_as_none() {
        // 向后兼容 (STEP 9, Spec 评审需求 4): 旧 Fixture JSON 缺 content 字段须解析为 None, 不报错.
        let json = r#"{"state":"locked","format":"1080i50"}"#;
        let es: ExpectedSignal =
            serde_json::from_str(json).expect("旧 JSON 无 content 须可反序列化");
        assert_eq!(es.content, None);
        assert_eq!(es.state, SignalState::Locked);
    }

    #[test]
    fn resolve_finds_locked_input_as_sink() {
        let d = Uuid::new_v4();
        let reg = PortRegistry {
            ports: vec![
                port(
                    d,
                    ConnectorType::Sdi,
                    0,
                    PortDirection::Output,
                    SignalState::Unknown,
                ),
                port(
                    d,
                    ConnectorType::Sdi,
                    1,
                    PortDirection::Input,
                    SignalState::Locked,
                ),
            ],
        };
        let mut f = default_sdi_loopback();
        // 已声明拓扑: source=输出端口(SDI#0), sink=Locked 输入端口(SDI#1) → 精确匹配, 不猜.
        f.source.port_id = Some(
            reg.ports[0]
                .identity
                .port_id
                .map(|u| u.to_string())
                .unwrap(),
        );
        f.sink.port_id = Some(
            reg.ports[1]
                .identity
                .port_id
                .map(|u| u.to_string())
                .unwrap(),
        );
        let (source, sink) = f.resolve(&reg).expect("应解析出 source/sink");
        assert_eq!(
            source,
            reg.ports[0]
                .identity
                .port_id
                .map(|u| u.to_string())
                .unwrap()
        );
        assert_eq!(
            sink,
            reg.ports[1]
                .identity
                .port_id
                .map(|u| u.to_string())
                .unwrap()
        );
    }
}
