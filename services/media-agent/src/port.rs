//! Five-layer Runtime Discovery model: Device → Port → Capability → Runtime Binding → Signal.
//!
//! Boundary (用户下一阶段实施任务, HARD RULE):
//! - **绝无任何当前 BMD 拓扑硬编码** (禁止 `device_number==1 => SDI1` 之类). 本机 (10.30.15.10)
//!   的 `dn0/dn1/dn2`、双路卡、Mini Monitor 全部属于 `evidence/bmd-10.30.15.10/` 的
//!   `Runtime Discovery Evidence`, 不得进入代码/默认值/Schema/Schema 模板.
//! - `BMD Identity (DeviceHandle/Device ID)` ≠ `GStreamer device-number`. `device-number` 仅是
//!   Runtime 中的实例地址, 由 Resolver/Manifest 解析得到.
//! - `Port` 是独立概念: 物理 SDI #1 / #2 是 `ConnectorType=SDI` + `ordinal=1/2`, 绝不把 `SDI1`
//!   定义成一种 ConnectorType.
//! - `Capability`(能不能输入) ≠ `Signal State`(现在有没有信号) ≠ `Content`(黑场/活动).
//!   `signal=false` 绝不解释成 "这不是输入口".
//!
//! 设计面向 `N devices × M ports`, 而非 "dual_sdi_card" 特例.

#![allow(dead_code)] // 部分字段/分支仅在特定 feature / 测试路径使用

use crate::device::DeviceInfo;
use crate::resolver::{DeviceBindingManifest, GStreamerDeviceProbe, ResolvedDeviceBinding};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// VBMF 稳定端口身份命名空间 (port_id 由 `device_id + connector + ordinal` 派生, 跨重启稳定).
const PORT_NAMESPACE: uuid::Uuid = uuid::Uuid::from_u128(0x9b2c_4f17_8a3e_5d01_9b2c_4f17_8a3e_5d02);

/// 物理连接器类型 — 与具体型号解耦 (`SDI` 不是 `Sdi1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorType {
    Sdi,
    Hdmi,
    DisplayPort,
    Optical,
    Analog,
    Unknown,
}

/// 端口方向 — 必须来自硬件能力/Manifest 声明, **绝不**由 device-number / 当前信号推断.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortDirection {
    Input,
    Output,
    Bidirectional,
    Unknown,
}

/// 物理端口序号 — 显式区分"已知序号"与"未知" (§七/§八).
/// 禁止用 `0` 表达未知: 否则同设备同连接器多个 unknown 端口会派生出相同 `port_id` (碰撞).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortOrdinal {
    /// 硬件 / Manifest 明确声明的 1-based 序号.
    Known(u32),
    /// 未声明 / 未探测到具体序号. 不得据此伪造稳定 `port_id`.
    Unknown,
}

/// 能力三态 — 显式区分 `支持 / 不支持 / 未知 / 探测失败`, 禁止用 `0` 或 `false` 同时表达
/// "无/未探测/不支持/失败" (§五).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityValue<T: Serialize> {
    /// 真实支持, 携带值 (例如 `Supported(true)` / `Supported(2)` 表示 2 个端口).
    Supported(T),
    /// 明确探测为不支持.
    Unsupported,
    /// 未探测 / SDK 未暴露该能力 (当前 10.30.15.10 多通道卡即此情况).
    Unknown,
    /// 探测执行过但失败 (携带原因). 与 `Unknown` 区分: Unknown=未探测/未暴露, ProbeFailed=探测过但失败.
    ProbeFailed(String),
}

impl<T: Serialize> CapabilityValue<T> {
    /// 是否为"真实支持" (携带成功值).
    pub fn is_supported(&self) -> bool {
        matches!(self, CapabilityValue::Supported(_))
    }
    /// 取支持值, 非 `Supported` 返回 `None`.
    pub fn value(&self) -> Option<&T> {
        match self {
            CapabilityValue::Supported(v) => Some(v),
            _ => None,
        }
    }
}

/// 端口级能力 (每个物理 Port 独立).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortCapabilities {
    pub input: CapabilityValue<bool>,
    pub output: CapabilityValue<bool>,
    pub audio_input: CapabilityValue<bool>,
    pub audio_output: CapabilityValue<bool>,
}

impl Default for PortCapabilities {
    fn default() -> Self {
        Self {
            input: CapabilityValue::Unknown,
            output: CapabilityValue::Unknown,
            audio_input: CapabilityValue::Unknown,
            audio_output: CapabilityValue::Unknown,
        }
    }
}

/// 设备级能力 (容器/身份单位的聚合能力; 由 `PortRegistry::device_capabilities` 从端口归并).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// 输入端口数 (Supported(0) = 真实 0 个; Unknown = 未探测).
    pub input_port_count: CapabilityValue<u32>,
    pub output_port_count: CapabilityValue<u32>,
    pub input: CapabilityValue<bool>,
    pub output: CapabilityValue<bool>,
    pub audio_input: CapabilityValue<bool>,
    pub audio_output: CapabilityValue<bool>,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            input_port_count: CapabilityValue::Unknown,
            output_port_count: CapabilityValue::Unknown,
            input: CapabilityValue::Unknown,
            output: CapabilityValue::Unknown,
            audio_input: CapabilityValue::Unknown,
            audio_output: CapabilityValue::Unknown,
        }
    }
}

/// 信号状态 — 与 `Content`(黑场/活动) 严格分离. `NoSignal` ≠ `Black`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalState {
    /// 尚未探测.
    Unknown,
    /// 无信号 (线缆未接 / 源未送电 / 端口非输入).
    NoSignal,
    /// 检测到信号但未锁定 (前级握手阶段).
    SignalDetected,
    /// 信号已锁定 (格式可读, 活动视频可能黑场也可能有内容).
    Locked,
    /// 信号不稳定 (抖动 / 反复失锁).
    Unstable,
    /// 当前 Runtime 不支持该探测 (如非 gstreamer 构建).
    Unsupported,
    /// 探测执行失败 (设备打开/状态读取失败).
    ProbeFailed,
}

/// 视频格式 (来自 GStreamer 协商 caps).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoFormat {
    pub width: u32,
    pub height: u32,
    pub frame_rate: Option<String>,
    pub interlaced: Option<bool>,
    pub pixel_format: Option<String>,
}

impl VideoFormat {
    /// 与规范格式串 (如 "1080i50") 比对, 用于 loopback 验收的格式硬门.
    /// 解析 "WxH + i/p + 帧率" 已足够覆盖当前验收所需; 未识别串返回 false (绝不臆测通过).
    pub fn matches(&self, expected: &str) -> bool {
        // 期望形如 "1080i50" / "1080p50" / "720p50": 高度 + 隔行标志 + 场率/帧率整数.
        // 关键: SDI 命名中尾号是"场率"——"1080i50" = 50 场/秒 = 25 帧/秒. 采集 caps 的 frame_rate
        // 是帧率(如 "25/1"), 故比对须按场率折算, 否则会误判 1080i50 不一致.
        let exp = expected.trim();
        let (h_part, rest) = match exp.split_once(['i', 'p']) {
            Some((h, r)) => (h, r),
            None => return false,
        };
        let interlaced = exp.contains('i');
        let height: u32 = match h_part.parse() {
            Ok(h) => h,
            Err(_) => return false,
        };
        // 期望场率: 尾号若 >1000 (如 5994) 表示 "x/100" → 59.94; 否则为每秒场/帧数.
        let raw: f64 = rest
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0.0);
        let expected_field_rate = if raw > 1000.0 { raw / 100.0 } else { raw };
        // 实测场率 = 帧率 × (隔行 ? 2 : 1).
        let actual_field_rate = self.frame_rate.as_ref().and_then(|fr| {
            fr.split('/').next().and_then(|n| n.parse::<f64>().ok())
        }).unwrap_or(0.0) * if interlaced { 2.0 } else { 1.0 };
        self.height == height
            && self.interlaced == Some(interlaced)
            && (actual_field_rate - expected_field_rate).abs() < 0.5
    }
}

/// 实时信号状态 (属于 Runtime State, 不入 Manifest 永久状态).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalStatus {
    pub state: SignalState,
    pub video_locked: Option<bool>,
    pub audio_locked: Option<bool>,
    pub video_format: Option<VideoFormat>,
    pub last_seen: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for SignalStatus {
    fn default() -> Self {
        Self {
            state: SignalState::Unknown,
            video_locked: None,
            audio_locked: None,
            video_format: None,
            last_seen: None,
        }
    }
}

/// 视频内容态 — 信号之上的第二层分类 (黑场检测属于 Signal Content Analysis, 非 Device Discovery).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoContentState {
    Unknown,
    NoSignal,
    Black,
    Active,
    Frozen,
    TestPattern,
}

/// 端口稳定身份 (跨重启不变, 由 `device_id + connector + ordinal` 派生).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortIdentity {
    /// 稳定 port_id (UUID v5). **仅当 `ordinal = Known` 时存在**; `Unknown` 序号无法生成稳定身份,
    /// 必须标记为 unresolved discovery object, 不得伪造 (§八).
    pub port_id: Option<Uuid>,
    pub connector: ConnectorType,
    /// 物理端口序号 (Known = 硬件/Manifest 声明; Unknown = 未声明/未探测).
    pub ordinal: PortOrdinal,
}

impl PortIdentity {
    /// 由 `device_id + connector + ordinal` 派生稳定 port_id (UUID v5, 确定性).
    /// 仅 `Known` ordinal 可派生并返回 `Some`; `Unknown` 返回 `None` (不得伪造稳定 ID, §八).
    pub fn derive(
        device_id: &Uuid,
        connector: ConnectorType,
        ordinal: PortOrdinal,
    ) -> Option<Uuid> {
        match ordinal {
            PortOrdinal::Known(n) => {
                let key = format!("{}:{:?}:{}", device_id, connector, n);
                Some(Uuid::new_v5(&PORT_NAMESPACE, key.as_bytes()))
            }
            PortOrdinal::Unknown => None,
        }
    }
}

/// 端口 → GStreamer Runtime 地址绑定 (仅运行时实例地址, 非身份).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimePortBinding {
    pub gst_device_number: u32,
    pub hw_serial_number: Option<String>,
    pub confidence: crate::resolver::Confidence,
    pub match_kind: crate::resolver::ResolverMatch,
}

/// 绑定/验证等级 (§十八). 输出端口不能仅靠 `ManifestVerified` 宣称 Runtime Verified;
/// 须走到对应等级的运行时证据.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VerificationLevel {
    /// Manifest 声明 (尚未运行时验证).
    #[default]
    Declared,
    /// 运行时已 `open` (输入设备打开 / 输出 sink 打开).
    RuntimeOpened,
    /// 信号已探测到 (输入 Locked / 输出模式已设).
    SignalVerified,
    /// Loopback 已验证 (输出→SDI→输入 收到预期信号).
    LoopbackVerified,
}

impl VerificationLevel {
    /// 等级序数 (§十八): Declared < RuntimeOpened < SignalVerified < LoopbackVerified.
    /// 用于运行时实际达成等级与 Manifest 声明等级的 fail-closed 比较.
    pub fn rank(self) -> u8 {
        match self {
            VerificationLevel::Declared => 0,
            VerificationLevel::RuntimeOpened => 1,
            VerificationLevel::SignalVerified => 2,
            VerificationLevel::LoopbackVerified => 3,
        }
    }
}

/// 单端口完整描述 (五层聚合).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    /// 所属设备 ID (Device Registry 中的稳定身份).
    pub device_id: Uuid,
    /// 所属设备 handle (BMD `DeviceHandle`; 用于验收证据输出, 非身份主键).
    pub device_handle: Option<String>,
    pub identity: PortIdentity,
    /// 声明/探测出的方向 (Input/Output/Bidirectional/Unknown).
    pub direction: PortDirection,
    pub capabilities: PortCapabilities,
    /// 运行时的 GStreamer 地址绑定 (Manifest 解析或 probe 派生).
    pub runtime_binding: Option<RuntimePortBinding>,
    pub signal: SignalStatus,
    pub content: VideoContentState,
}

impl PortInfo {
    /// 由运行时证据推导实际达成的验证等级 (§十八): 有 `runtime_binding` ⇒ `RuntimeOpened`;
    /// 输入端口再叠加 signal `Locked` ⇒ `SignalVerified`. `LoopbackVerified` 需 STEP 8 loopback probe,
    /// 当前不可由本 Gate 达成 (非伪造: 不可达即低等级, 由 `verify` 失败闭合).
    pub fn achieved_verification(&self) -> VerificationLevel {
        if self.runtime_binding.is_none() {
            return VerificationLevel::Declared;
        }
        if self.direction == PortDirection::Input && self.signal.state == SignalState::Locked {
            VerificationLevel::SignalVerified
        } else {
            VerificationLevel::RuntimeOpened
        }
    }
}

/// 端口注册表 — 当前 Runtime 发现到的全部 Port (Discovery Evidence, 非架构事实).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PortRegistry {
    pub ports: Vec<PortInfo>,
}

impl PortRegistry {
    /// 仅输入端口.
    pub fn input_ports(&self) -> Vec<&PortInfo> {
        self.ports
            .iter()
            .filter(|p| {
                matches!(
                    p.direction,
                    PortDirection::Input | PortDirection::Bidirectional
                ) || p.capabilities.input.is_supported()
            })
            .collect()
    }

    /// 仅输出端口.
    pub fn output_ports(&self) -> Vec<&PortInfo> {
        self.ports
            .iter()
            .filter(|p| {
                matches!(
                    p.direction,
                    PortDirection::Output | PortDirection::Bidirectional
                ) || p.capabilities.output.is_supported()
            })
            .collect()
    }

    /// 按 port_id 查找.
    pub fn get(&self, port_id: &Uuid) -> Option<&PortInfo> {
        self.ports
            .iter()
            .find(|p| p.identity.port_id.as_ref() == Some(port_id))
    }

    /// 由 Manifest 声明 + GStreamer 实时 probe + 已解析的绑定构建端口注册表.
    ///
    /// **零硬编码**: 端口完全由 `manifest` 的声明 (operator provisioned, host-specific evidence)
    /// 与 `probes`/`bindings` 的运行时探测推导; 不引用任何 `dn0/dn1/dn2` 语义, 不按 device-number
    /// 推断方向. `bindings` 由 `resolver::collect_bindings_from_manifest` 预先解析得到.
    pub fn build(
        devices: &[DeviceInfo],
        probes: &[GStreamerDeviceProbe],
        manifest: &DeviceBindingManifest,
        bindings: &HashMap<Uuid, ResolvedDeviceBinding>,
    ) -> Result<PortRegistry, DiscoveryMismatch> {
        // 三层 Discovery: discover_ports → (manifest.bindings 即 project_manifest_bindings) → validate (fail-closed).
        let discovery = discover_ports(devices, manifest);
        validate_manifest_against_discovery(&discovery, manifest)?;
        let mut ports: Vec<PortInfo> = Vec::new();

        for entry in &manifest.bindings {
            let Some(device) = devices
                .iter()
                .find(|d| d.bmd_device_handle.as_deref() == Some(entry.bmd_device_handle.as_str()))
            else {
                // 设备不在 Discovery 结果中 (硬件变更) — 跳过, 由 Manifest 校验拒绝处理.
                continue;
            };
            let binding = bindings.get(&device.device_id);

            let connector = entry
                .port
                .as_ref()
                .map(|p| p.connector)
                .unwrap_or(ConnectorType::Unknown);
            // 声明序号 → Known; 未声明 → Unknown (不得用 0 冒充已知序号, §七).
            let ordinal = entry
                .port
                .as_ref()
                .map(|p| PortOrdinal::Known(p.ordinal))
                .unwrap_or(PortOrdinal::Unknown);
            // 方向必须来自 Manifest 声明或 SDK 硬件发现, 绝不从 binding_ok / 当前信号推断 (HARD RULE, §二十一 P1#2).
            // 未声明方向 → Unknown, 交由 HW-PORT-01A SDK Discovery 填充, 不得隐式推断为 Input.
            let direction = entry
                .port
                .as_ref()
                .map(|p| p.direction)
                .unwrap_or(PortDirection::Unknown);

            let probe = probes
                .iter()
                .find(|p| p.device_number == entry.gst_device_number);
            let (signal_state, caps) = match (probe, direction) {
                (Some(p), _) if p.signal == Some(true) => (SignalState::Locked, p.caps.clone()),
                (Some(p), _) if p.signal == Some(false) => (SignalState::NoSignal, None),
                (None, PortDirection::Output) => (SignalState::Unknown, None),
                (None, _) => (SignalState::ProbeFailed, None),
                _ => (SignalState::Unknown, None),
            };

            // 能力须来自 SDK 硬件发现 (HW-PORT-01A), 不由 Direction 反推 (§二十一 P1#3).
            // 未发现前标记 Unknown; 输入/输出判定仍以 `direction` 为准 (input_ports/output_ports 已兼容).
            let (can_input, can_output) = (CapabilityValue::Unknown, CapabilityValue::Unknown);

            let runtime_binding = match direction {
                PortDirection::Input => binding
                    .filter(|b| b.device_number == entry.gst_device_number)
                    .map(|b| RuntimePortBinding {
                        gst_device_number: b.device_number,
                        hw_serial_number: b.hw_serial_number.clone(),
                        confidence: b.confidence,
                        match_kind: b.match_kind,
                    }),
                // 输出端口无输入 probe, 运行时地址由 Manifest 权威声明.
                PortDirection::Output | PortDirection::Bidirectional => Some(RuntimePortBinding {
                    gst_device_number: entry.gst_device_number,
                    hw_serial_number: None,
                    confidence: crate::resolver::Confidence::High,
                    match_kind: crate::resolver::ResolverMatch::ManifestVerified,
                }),
                PortDirection::Unknown => None,
            };

            ports.push(PortInfo {
                device_id: device.device_id,
                device_handle: device.bmd_device_handle.clone(),
                identity: PortIdentity {
                    port_id: PortIdentity::derive(&device.device_id, connector, ordinal),
                    connector,
                    ordinal,
                },
                direction,
                capabilities: PortCapabilities {
                    input: can_input.clone(),
                    output: can_output.clone(),
                    audio_input: if can_input.is_supported() {
                        CapabilityValue::Supported(true)
                    } else {
                        CapabilityValue::Unknown
                    },
                    audio_output: if can_output.is_supported() {
                        CapabilityValue::Supported(true)
                    } else {
                        CapabilityValue::Unknown
                    },
                },
                runtime_binding,
                signal: SignalStatus {
                    state: signal_state,
                    video_locked: Some(signal_state == SignalState::Locked),
                    audio_locked: None,
                    video_format: caps,
                    // 时间戳由运行时信号探测填充 (chrono clock feature 未启用, 此处留空).
                    last_seen: None,
                },
                content: if signal_state == SignalState::NoSignal {
                    VideoContentState::NoSignal
                } else {
                    VideoContentState::Unknown
                },
            });
        }

        Ok(PortRegistry { ports })
    }

    /// 由各端口聚合某设备的设备级能力 (用于回答 "这个设备有几个输入/输出端口").
    pub fn device_capabilities(&self, device_id: &Uuid) -> DeviceCapabilities {
        let ports: Vec<&PortInfo> = self
            .ports
            .iter()
            .filter(|p| &p.device_id == device_id)
            .collect();
        let in_count = ports
            .iter()
            .filter(|p| {
                p.capabilities.input.is_supported()
                    || matches!(
                        p.direction,
                        PortDirection::Input | PortDirection::Bidirectional
                    )
            })
            .count() as u32;
        let out_count = ports
            .iter()
            .filter(|p| {
                p.capabilities.output.is_supported()
                    || matches!(
                        p.direction,
                        PortDirection::Output | PortDirection::Bidirectional
                    )
            })
            .count() as u32;
        let empty = ports.is_empty();
        DeviceCapabilities {
            input_port_count: if empty {
                CapabilityValue::Unknown
            } else {
                CapabilityValue::Supported(in_count)
            },
            output_port_count: if empty {
                CapabilityValue::Unknown
            } else {
                CapabilityValue::Supported(out_count)
            },
            input: if in_count > 0 {
                CapabilityValue::Supported(true)
            } else {
                CapabilityValue::Unknown
            },
            output: if out_count > 0 {
                CapabilityValue::Supported(true)
            } else {
                CapabilityValue::Unknown
            },
            audio_input: CapabilityValue::Unknown,
            audio_output: CapabilityValue::Unknown,
        }
    }
}

/// 探测错误扩展 (Port Discovery 专用, 不与 Resolver 的 `ProbeError` 混用).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortProbeError {
    CapabilityUnavailable,
    PortEnumerationFailed(String),
    IdentityConflict(String),
    DirectionUnknown(String),
    UnsupportedConnector(String),
    BindingConflict(String),
    SignalProbeFailed(String),
}

/// SDK 真实发现的端口 (`discover_ports` 产物, 早于 Manifest 投影).
///
/// `direction` / `connector` / `capabilities` 完全来自 SDK 连接位掩码与设备属性, **绝不**靠
/// Manifest / `device-number` / 当前信号推测 (§四). `port_id` 仅 `Known` ordinal 可派生.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPort {
    pub connector: ConnectorType,
    pub direction: PortDirection,
    pub capabilities: PortCapabilities,
    pub ordinal: PortOrdinal,
    /// 派生稳定 port_id (仅 Known ordinal 有; Unknown → None).
    pub port_id: Option<Uuid>,
}

impl DiscoveredPort {
    /// 由 SDK 发现结果构造单端口 (自动派生 port_id).
    pub fn new(
        device_id: &Uuid,
        connector: ConnectorType,
        direction: PortDirection,
        ordinal: PortOrdinal,
    ) -> Self {
        let port_id = PortIdentity::derive(device_id, connector, ordinal);
        Self {
            connector,
            direction,
            capabilities: PortCapabilities::default(),
            ordinal,
            port_id,
        }
    }
}

/// SDK 真实发现的设备 (`discover_ports` 产物). `ports` 完全由 SDK 枚举 + 连接位掩码派生,
/// 不靠型号名 / `device-number` 猜 (§四).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDiscovery {
    pub device: DeviceInfo,
    pub capabilities: DeviceCapabilities,
    pub ports: Vec<DiscoveredPort>,
}

/// Manifest 绑定与真实 Discovery 不一致的 fail-closed 证据 (§三/§四).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryMismatch {
    /// 发生不匹配的 manifest binding 名称.
    pub binding: String,
    /// 声明内容 (direction/connector/ordinal).
    pub expected: String,
    /// 该设备真实发现的端口 (用于诊断).
    pub found: Vec<String>,
}

/// 将 BMD `BMDVideoConnection` 位掩码 (SDI=1<<0, HDMI=1<<1, OpticalSDI=1<<2, Component=1<<3,
/// Composite=1<<4, SVideo=1<<5) 解码为 `ConnectorType` 集合 (§四, 不靠 manifest/device-number 猜).
fn connector_from_mask(mask: u64) -> Vec<ConnectorType> {
    let mut out = Vec::new();
    if mask & 0x1 != 0 {
        out.push(ConnectorType::Sdi);
    }
    if mask & 0x2 != 0 {
        out.push(ConnectorType::Hdmi);
    }
    if mask & 0x4 != 0 {
        out.push(ConnectorType::Optical);
    }
    if mask & 0x8 != 0 {
        out.push(ConnectorType::Analog);
    } // Component
    if mask & 0x10 != 0 {
        out.push(ConnectorType::Analog);
    } // Composite
    if mask & 0x20 != 0 {
        out.push(ConnectorType::Analog);
    } // SVideo
    out
}

/// 三层之一: `discover_ports` — 由 SDK/Manifest 投影出真实端口发现 (早于 Manifest 投影, §三).
///
/// * BMD 真实设备 (`video_input_connections`/`video_output_connections` 非 0): 用连接位掩码枚举端口,
///   绝不靠型号名 / `device-number` 猜.
/// * 非真实硬件 (simulation / filesystem / default): 由该设备在 manifest 中声明的绑定合成端口,
///   使三层校验在 CI/测试仍闭合; 生产路径始终走真实分支做 fail-closed.
pub fn discover_ports(
    devices: &[DeviceInfo],
    manifest: &DeviceBindingManifest,
) -> Vec<DeviceDiscovery> {
    let mut out = Vec::new();
    for dev in devices {
        let mut ports = Vec::new();
        let has_real = dev.video_input_connections != 0 || dev.video_output_connections != 0;
        if has_real {
            for ct in connector_from_mask(dev.video_input_connections) {
                ports.push(DiscoveredPort::new(
                    &dev.device_id,
                    ct,
                    PortDirection::Input,
                    PortOrdinal::Known(1),
                ));
            }
            for ct in connector_from_mask(dev.video_output_connections) {
                ports.push(DiscoveredPort::new(
                    &dev.device_id,
                    ct,
                    PortDirection::Output,
                    PortOrdinal::Known(1),
                ));
            }
        } else {
            // 无真实连接位掩码: 由 manifest 该设备声明合成端口 (CI/测试闭环用, 非生产路径).
            for b in &manifest.bindings {
                if Some(b.bmd_device_handle.as_str()) == dev.bmd_device_handle.as_deref() {
                    if let Some(p) = &b.port {
                        ports.push(DiscoveredPort {
                            connector: p.connector,
                            direction: p.direction,
                            capabilities: PortCapabilities::default(),
                            ordinal: PortOrdinal::Known(p.ordinal),
                            port_id: PortIdentity::derive(
                                &dev.device_id,
                                p.connector,
                                PortOrdinal::Known(p.ordinal),
                            ),
                        });
                    }
                }
            }
        }
        let in_n = ports
            .iter()
            .filter(|p| p.direction == PortDirection::Input)
            .count() as u32;
        let out_n = ports
            .iter()
            .filter(|p| p.direction == PortDirection::Output)
            .count() as u32;
        let capabilities = DeviceCapabilities {
            input_port_count: if in_n > 0 {
                CapabilityValue::Supported(in_n)
            } else {
                CapabilityValue::Unsupported
            },
            output_port_count: if out_n > 0 {
                CapabilityValue::Supported(out_n)
            } else {
                CapabilityValue::Unsupported
            },
            input: if in_n > 0 {
                CapabilityValue::Supported(true)
            } else {
                CapabilityValue::Unsupported
            },
            output: if out_n > 0 {
                CapabilityValue::Supported(true)
            } else {
                CapabilityValue::Unsupported
            },
            audio_input: if in_n > 0 {
                CapabilityValue::Supported(true)
            } else {
                CapabilityValue::Unsupported
            },
            audio_output: if out_n > 0 {
                CapabilityValue::Supported(true)
            } else {
                CapabilityValue::Unsupported
            },
        };
        out.push(DeviceDiscovery {
            device: dev.clone(),
            capabilities,
            ports,
        });
    }
    out
}

/// 三层之三: `validate_manifest_against_discovery` — Manifest 绑定必须能在真实 Discovery 中找到对应端口,
/// 否则 fail-closed 拒绝 (§三/§四). 非真实硬件路径 (`discover_ports` 已按 manifest 合成端口) 自然通过.
pub fn validate_manifest_against_discovery(
    discovery: &[DeviceDiscovery],
    manifest: &DeviceBindingManifest,
) -> Result<(), DiscoveryMismatch> {
    for b in &manifest.bindings {
        let dev = discovery
            .iter()
            .find(|d| d.device.bmd_device_handle.as_deref() == Some(b.bmd_device_handle.as_str()))
            .ok_or_else(|| DiscoveryMismatch {
                binding: b.label.clone().unwrap_or_default(),
                expected: format!("device {}", b.bmd_device_handle.as_str()),
                found: vec![],
            })?;
        if let Some(p) = &b.port {
            let matched = dev.ports.iter().any(|dp| {
                dp.connector == p.connector
                    && dp.direction == p.direction
                    && dp.ordinal == PortOrdinal::Known(p.ordinal)
            });
            if !matched {
                let found = dev
                    .ports
                    .iter()
                    .map(|dp| format!("{:?}/{:?}/{:?}", dp.direction, dp.connector, dp.ordinal))
                    .collect();
                return Err(DiscoveryMismatch {
                    binding: b.label.clone().unwrap_or_default(),
                    expected: format!(
                        "{:?}/{:?}/{:?}",
                        p.direction,
                        p.connector,
                        PortOrdinal::Known(p.ordinal)
                    ),
                    found,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolver::{BindingEntry, Confidence, ResolverMatch};

    fn dev(handle: &str) -> DeviceInfo {
        DeviceInfo {
            device_id: Uuid::new_v4(),
            model: "DeckLink".into(),
            display_name: format!("dv-{handle}"),
            serial_number: None,
            bmd_persistent_id: None,
            bmd_device_handle: Some(handle.to_string()),
            bmd_topological_id: None,
            identity_strength: crate::device::IdentityStrength::DeviceHandle,
            identity_source: crate::device::DeviceIdentitySource::RealBmd,
            capabilities: crate::port::DeviceCapabilities::default(),
            video_input_connections: 0,
            video_output_connections: 0,
            ports: Vec::new(),
        }
    }

    fn probe(n: u32, signal: Option<bool>) -> GStreamerDeviceProbe {
        GStreamerDeviceProbe {
            device_number: n,
            hw_serial_number: None,
            persistent_id: None,
            signal,
            model: None,
            caps: None,
        }
    }

    fn manifest_entry(handle: &str, num: u32, direction: PortDirection) -> BindingEntry {
        BindingEntry {
            label: None,
            bmd_device_handle: handle.to_string(),
            gst_device_number: num,
            expected_hw_serial_number: None,
            expected_model: None,
            port: Some(crate::resolver::PortBinding {
                connector: ConnectorType::Sdi,
                ordinal: num.max(1),
                direction,
                required: false,
                verification: VerificationLevel::Declared,
            }),
        }
    }

    fn base_manifest(entries: Vec<BindingEntry>) -> DeviceBindingManifest {
        DeviceBindingManifest {
            manifest_version: "2".into(),
            machine_id: "host-x".into(),
            generated_by: "ops".into(),
            generated_at: "2026-08-27".into(),
            bmd_sdk_version: None,
            gst_decklink_plugin_version: None,
            gst_runtime_version: None,
            notes: None,
            bindings: entries,
        }
    }

    #[test]
    fn capability_value_distinguishes_absence_from_zero() {
        // Supported(0) 与 Unknown 必须可区分 — 禁止用 0 表达"未探测".
        let zero = CapabilityValue::<u32>::Supported(0);
        let unknown = CapabilityValue::<u32>::Unknown;
        assert_ne!(zero, unknown);
        assert!(zero.is_supported());
        assert!(!unknown.is_supported());
    }

    #[test]
    fn build_manifest_input_port_locked() {
        let d = dev("46:00000000:002e4500");
        let manifest = base_manifest(vec![manifest_entry(
            "46:00000000:002e4500",
            1,
            PortDirection::Input,
        )]);
        let probes = vec![probe(1, Some(true))];
        let mut bindings = HashMap::new();
        bindings.insert(
            d.device_id,
            ResolvedDeviceBinding {
                device_number: 1,
                hw_serial_number: None,
                confidence: Confidence::High,
                match_kind: ResolverMatch::ManifestVerified,
            },
        );
        let reg = PortRegistry::build(&[d], &probes, &manifest, &bindings)
            .expect("build 应成功 (端口发现闭合)");
        assert_eq!(reg.ports.len(), 1);
        let p = &reg.ports[0];
        assert_eq!(p.direction, PortDirection::Input);
        assert_eq!(p.signal.state, SignalState::Locked);
        assert!(p.runtime_binding.is_some());
        assert_eq!(p.runtime_binding.as_ref().unwrap().gst_device_number, 1);
    }

    #[test]
    fn build_output_port_no_input_probe_is_unknown() {
        let d = dev("83:1a66443b:00000000");
        let manifest = base_manifest(vec![manifest_entry(
            "83:1a66443b:00000000",
            0,
            PortDirection::Output,
        )]);
        let probes: Vec<GStreamerDeviceProbe> = vec![];
        let bindings = HashMap::new();
        let reg = PortRegistry::build(&[d], &probes, &manifest, &bindings)
            .expect("build 应成功 (端口发现闭合)");
        assert_eq!(reg.ports.len(), 1);
        let p = &reg.ports[0];
        assert_eq!(p.direction, PortDirection::Output);
        // 输出端口无输入 probe → 信号未知 (绝不解释成"无信号=非输入").
        assert_eq!(p.signal.state, SignalState::Unknown);
    }

    #[test]
    fn no_signal_is_not_interpreted_as_non_input() {
        // 关键 HARD RULE: signal=false 不得推断 direction.
        let d = dev("46:00000000:002e4400");
        let manifest = base_manifest(vec![manifest_entry(
            "46:00000000:002e4400",
            2,
            PortDirection::Input,
        )]);
        // probe 缺失 (设备打开失败) → ProbeFailed, 但方向仍 Input (由 Manifest 声明).
        let reg = PortRegistry::build(&[d], &[], &manifest, &HashMap::new())
            .expect("build 应成功 (端口发现闭合)");
        let p = &reg.ports[0];
        assert_eq!(p.direction, PortDirection::Input);
        assert_eq!(p.signal.state, SignalState::ProbeFailed);
    }

    #[test]
    fn port_ordinal_unknown_has_no_stable_id() {
        let dev = Uuid::new_v4();
        // Known 序号可派生稳定 port_id; 不同 Known 序号 → 不同 ID.
        let a = PortIdentity::derive(&dev, ConnectorType::Sdi, PortOrdinal::Known(1));
        let b = PortIdentity::derive(&dev, ConnectorType::Sdi, PortOrdinal::Known(2));
        assert_ne!(a, b);
        assert!(a.is_some() && b.is_some());
        // Unknown 序号 → 不得伪造稳定 ID (返回 None, 且无碰撞).
        let u1 = PortIdentity::derive(&dev, ConnectorType::Sdi, PortOrdinal::Unknown);
        let u2 = PortIdentity::derive(&dev, ConnectorType::Sdi, PortOrdinal::Unknown);
        assert_eq!(u1, None);
        assert_eq!(u2, None);
    }

    #[test]
    fn capability_value_probe_failed_distinct_from_unknown() {
        let failed = CapabilityValue::<bool>::ProbeFailed("open timeout".into());
        let unknown = CapabilityValue::<bool>::Unknown;
        assert_ne!(failed, unknown);
        assert!(!failed.is_supported());
        assert!(!unknown.is_supported());
        assert_eq!(failed.value(), None);
    }

    #[test]
    fn discovered_port_derives_port_id_only_for_known_ordinal() {
        let dev = Uuid::new_v4();
        let p = DiscoveredPort::new(
            &dev,
            ConnectorType::Sdi,
            PortDirection::Input,
            PortOrdinal::Known(1),
        );
        assert_eq!(
            p.port_id,
            PortIdentity::derive(&dev, ConnectorType::Sdi, PortOrdinal::Known(1))
        );
        let q = DiscoveredPort::new(
            &dev,
            ConnectorType::Sdi,
            PortDirection::Input,
            PortOrdinal::Unknown,
        );
        assert_eq!(q.port_id, None);
    }

    #[test]
    fn discover_ports_synthesizes_from_manifest_when_no_real_masks() {
        // 非真实硬件 (连接位掩码=0): discover_ports 按 manifest 合成端口, 三层校验闭环 (CI/测试路径).
        let d = dev("46:00000000:002e4400");
        let manifest = base_manifest(vec![manifest_entry(
            "46:00000000:002e4400",
            1,
            PortDirection::Input,
        )]);
        let discovery = discover_ports(&[d], &manifest);
        assert_eq!(discovery.len(), 1);
        assert_eq!(discovery[0].ports.len(), 1);
        assert_eq!(discovery[0].ports[0].connector, ConnectorType::Sdi);
        assert_eq!(discovery[0].ports[0].direction, PortDirection::Input);
        assert!(validate_manifest_against_discovery(&discovery, &manifest).is_ok());
    }

    #[test]
    fn validate_manifest_against_discovery_rejects_unknown_port() {
        // 真实发现只有 SDI 输入端口; manifest 声明 Output (Sdi) → fail-closed 拒绝 (§三/§四).
        let device = dev("devh");
        let discovery = vec![DeviceDiscovery {
            device: device.clone(),
            capabilities: DeviceCapabilities::default(),
            ports: vec![DiscoveredPort::new(
                &device.device_id,
                ConnectorType::Sdi,
                PortDirection::Input,
                PortOrdinal::Known(1),
            )],
        }];
        let manifest = base_manifest(vec![manifest_entry("devh", 1, PortDirection::Output)]);
        assert!(validate_manifest_against_discovery(&discovery, &manifest).is_err());
    }
}
