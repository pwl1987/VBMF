//! Five-layer Runtime Discovery model: Device → Port → Capability → Runtime Binding → Signal.
//!
//! Boundary (PORT-COLLISION-01 closure):
//! - **绝无任何当前 BMD 拓扑硬编码** (禁止 `device_number==1 => SDI1` 之类). 本机 (10.30.15.10)
//!   的 `dn0/dn1/dn2`、双路卡、Mini Monitor 全部属于 `evidence/bmd-10.30.15.10/` 的
//!   `Runtime Discovery Evidence`, 不得进入代码/默认值/Schema/Schema 模板.
//! - `BMD Identity (DeviceHandle/Device ID)` ≠ `GStreamer device-number`. `device-number` 仅是
//!   Runtime 中的实例地址, 由 Resolver/Manifest 解析得到.
//! - `Port` 是独立概念: 物理 SDI #1 / #2 是 `ConnectorType=SDI` + `ordinal=1/2`, 绝不把 `SDI1`
//!   定义成一种 ConnectorType.
//! - `Capability`(能不能输入) ≠ `Signal State`(现在有没有信号) ≠ `Content`(黑场/活动).
//!   `signal=false` 绝不解释成 "这不是输入口".
//! - **端口身份 = device_id + connector + 物理槽位 ordinal**; `direction` 与 `RuntimeBinding`
//!   均为属性, **绝不**进入身份键(冻结于 CANONICAL_IDENTITY.md §7/§5.1).
//! - **物理 jack 真实存在(BMD 实证)**: 固定全双工卡的 in-jack 与 out-jack 是两个独立 BNC,
//!   SDK `video_input_connections` / `video_output_connections` 掩码按方向分槽,
//!   `discover_ports` 按"in 掩码位 1..N (Input)" + "out 掩码位 1..M (Output)"分配槽位,
//!   保证每个物理 jack 唯一 PortId、跨重启稳定。
//!
//! 设计面向 `N devices × M ports`, 而非 "dual_sdi_card" 特例.

#![allow(dead_code)] // 部分字段/分支仅在特定 feature / 测试路径使用

use crate::device::DeviceInfo;
use crate::resolver::{DeviceBindingManifest, GStreamerDeviceProbe, ResolvedDeviceBinding};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// VBMF 稳定端口身份命名空间 (port_id 由 `device_id + connector + 物理槽位 ordinal`
/// 派生, 跨重启稳定; `direction` 与 RuntimeBinding 均为属性, 绝不进身份键 —
/// 详见 CANONICAL_IDENTITY.md §5.1/§7 与模块级注释)。
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
        let actual_field_rate = self
            .frame_rate
            .as_ref()
            .and_then(|fr| fr.split('/').next().and_then(|n| n.parse::<f64>().ok()))
            .unwrap_or(0.0)
            * if interlaced { 2.0 } else { 1.0 };
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
    /// Provider 本地绑定引用 (P0-1 中立化: 由 Provider Identity Adapter 从清单交叉核验解析;
    /// 验收证据关联用, 非身份主键. 字段名不冠 vendor 专名).
    pub provider_binding_ref: Option<String>,
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
    /// RF-FF-01D: 仅由 canonical Provider discovery + provisioning manifest 构建 Session/Resource
    /// 所需的端口授权视图。concrete GStreamer RuntimeBinding 与 signal probe 明确缺席。
    pub fn build_authorized(
        devices: &[crate::contracts::provider::DiscoveredDevice],
        manifest: &DeviceBindingManifest,
    ) -> Result<PortRegistry, DiscoveryMismatch> {
        let empty_probes: Vec<GStreamerDeviceProbe> = Vec::new();
        let empty_bindings: HashMap<Uuid, ResolvedDeviceBinding> = HashMap::new();
        let mut registry = Self::build(devices, &empty_probes, manifest, &empty_bindings)?;
        for port in &mut registry.ports {
            port.runtime_binding = None;
            // 未执行 concrete backend signal probe = Unknown，不得误报 ProbeFailed。
            if port.signal.state == SignalState::ProbeFailed {
                port.signal.state = SignalState::Unknown;
                port.signal.video_locked = None;
                port.signal.audio_locked = None;
                port.signal.video_format = None;
                port.content = VideoContentState::Unknown;
            }
        }
        Ok(registry)
    }

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
        devices: &[crate::contracts::provider::DiscoveredDevice],
        probes: &[GStreamerDeviceProbe],
        manifest: &DeviceBindingManifest,
        bindings: &HashMap<Uuid, ResolvedDeviceBinding>,
    ) -> Result<PortRegistry, DiscoveryMismatch> {
        // 三层 Discovery: discover_ports → (manifest.bindings 即 project_manifest_bindings) → validate (fail-closed).
        let discovery = discover_ports(devices, manifest);
        // PORT-COLLISION-01 closure: discovery 层 PortId 不变量 guard(test/debug panic, release log+continue;
        // registry 装配层最后一道 fail-closed 保证别名永远进不了寻址 SoT)。
        guard_discovery_port_id_invariant(&discovery);
        validate_manifest_against_discovery(&discovery, manifest)?;
        let mut ports: Vec<PortInfo> = Vec::new();

        for entry in &manifest.bindings {
            let Some(dev) = devices.iter().find(|d| {
                crate::resolver::identity_handle(d).as_deref()
                    == Some(entry.bmd_device_handle.as_str())
            }) else {
                // 设备不在 Discovery 结果中 (硬件变更) — 跳过, 由 Manifest 校验拒绝处理.
                continue;
            };
            let device = &dev.device;
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

            // 02-I P0-2 + PORT-COLLISION-01 closure：能力=SDK **该 jack 方向**位掩码的
            // jack 级证据。固定全双工卡(BMD 实证): in-mask 与 out-mask 都含 SDI →
            // in-jack 仅在 in-mask 含 SDI 时得 `input=Supported`, out-jack 仅在
            // out-mask 含 SDI 时得 `output=Supported`; **绝不**把设备级另一方向掩码
            // 摊平到当前 jack(否则 in-jack 误报 output, 触发 Resource 别名)。
            // 仿真/合成(双掩码=0)或 connector 未声明: 无 SDK 能力证据 → Unknown
            // (manifest 方向声明是 direction 证据, 不是能力证据)。Bidirectional
            // jack 视方向对偶: 仅当 in+out 两侧均含此 connector 才报双向 Supported,
            // 缺任一侧则仅在该侧报 Supported(混合实机常态)。
            let (can_input, can_output) = if connector == ConnectorType::Unknown
                || (device.video_input_connections == 0 && device.video_output_connections == 0)
            {
                (CapabilityValue::Unknown, CapabilityValue::Unknown)
            } else {
                let in_supports_connector =
                    connector_in_mask(connector, device.video_input_connections);
                let out_supports_connector =
                    connector_in_mask(connector, device.video_output_connections);
                (
                    match direction {
                        PortDirection::Input => {
                            if in_supports_connector {
                                CapabilityValue::Supported(true)
                            } else {
                                CapabilityValue::Unsupported
                            }
                        }
                        // Output jack: 物理上无法采集, output 能力无意义 → Unsupported
                        // (不是 Unknown: 方向语义决定该能力"确定不支持")。
                        PortDirection::Output => CapabilityValue::Unsupported,
                        PortDirection::Bidirectional => {
                            if in_supports_connector {
                                CapabilityValue::Supported(true)
                            } else {
                                CapabilityValue::Unsupported
                            }
                        }
                        PortDirection::Unknown => CapabilityValue::Unknown,
                    },
                    match direction {
                        // Input jack: 物理上无法输出, input 能力无意义 → Unsupported。
                        PortDirection::Input => CapabilityValue::Unsupported,
                        PortDirection::Output => {
                            if out_supports_connector {
                                CapabilityValue::Supported(true)
                            } else {
                                CapabilityValue::Unsupported
                            }
                        }
                        PortDirection::Bidirectional => {
                            if out_supports_connector {
                                CapabilityValue::Supported(true)
                            } else {
                                CapabilityValue::Unsupported
                            }
                        }
                        PortDirection::Unknown => CapabilityValue::Unknown,
                    },
                )
            };

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
                provider_binding_ref: dev.identity.as_ref().and_then(|i| i.device_handle.clone()),
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

        // 02-I 前置（第十七轮 §七②）belt: manifest 侧多条声明共享同一 port_id
        // 同样拒绝（discovery 校验之后的 registry 装配层最后一道）。
        {
            let mut seen: HashMap<Uuid, String> = HashMap::new();
            for p in &ports {
                let Some(pid) = p.identity.port_id else {
                    continue;
                };
                let desc = format!(
                    "{:?}/{:?}/{:?}",
                    p.direction, p.identity.connector, p.identity.ordinal
                );
                if let Some(prev) = seen.insert(pid, desc.clone()) {
                    return Err(duplicate_port_id_mismatch(pid, &prev, &desc));
                }
            }
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
            // 02-I P0-2: 设备级 audio 能力由端口级 SDK 证据聚合（任一端口
            // Supported 即 Supported; 无证据保持 Unknown——不做方向反推）。
            audio_input: if ports
                .iter()
                .any(|p| p.capabilities.audio_input.is_supported())
            {
                CapabilityValue::Supported(true)
            } else {
                CapabilityValue::Unknown
            },
            audio_output: if ports
                .iter()
                .any(|p| p.capabilities.audio_output.is_supported())
            {
                CapabilityValue::Supported(true)
            } else {
                CapabilityValue::Unknown
            },
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
    /// Provider 侧身份证据 (P0-1: 清单交叉核验键来源; Domain 结构不携带).
    /// serde skip: 证据为运行期配对物 (ProviderIdentity 含 &static provider 标签), 不进序列化证据.
    #[serde(skip)]
    pub identity: Option<crate::contracts::provider::ProviderIdentity>,
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

/// BMD `BMDVideoConnection` 连接器掩码 (SDI=1<<0, HDMI=1<<1, OpticalSDI=1<<2, Component=1<<3,
/// Composite=1<<4, SVideo=1<<5) 中每位代表的 `ConnectorType`. 顺序与 SDK 头文件
/// `DeckLinkAPIVideoOutputConversion.h::BMDVideoConnection` 一致(低→高位), 跨重启稳定。
const CONNECTOR_MASK_TABLE: &[(u64, ConnectorType)] = &[
    (0x1, ConnectorType::Sdi),
    (0x2, ConnectorType::Hdmi),
    (0x4, ConnectorType::Optical),
    (0x8, ConnectorType::Analog),  // Component
    (0x10, ConnectorType::Analog), // Composite
    (0x20, ConnectorType::Analog), // SVideo
];

/// 将连接器掩码解码为 `(ConnectorType, mask-bit)` 列表(位低位高位序, 稳定)。
/// 多个模拟位(Analog)折叠为同名 `ConnectorType` 但每个位独立表达一个物理 jack;
/// 槽位由 `discover_ports` 按"同方向内的位序"分配, 不得合并(§四/§七)。
fn connectors_with_bits(mask: u64) -> Vec<(u64, ConnectorType)> {
    CONNECTOR_MASK_TABLE
        .iter()
        .filter(|(bit, _)| mask & bit != 0)
        .map(|(bit, ct)| (*bit, *ct))
        .collect()
}

/// 判断 connector 是否被某掩码覆盖(任一位匹配即 true, 含 Analog 位折叠)。
fn connector_in_mask(connector: ConnectorType, mask: u64) -> bool {
    connectors_with_bits(mask)
        .into_iter()
        .any(|(_, ct)| ct == connector)
}

/// 将掩码解码为 `ConnectorType` 集合(用于能力检测, 不用于槽位分配;
/// 同类型重复位折叠为首次出现, 表达"该类型至少一个 jack 存在"; `connectors_with_bits`
/// 已按掩码位序遍历, dedup 即可)。
fn connector_from_mask(mask: u64) -> Vec<ConnectorType> {
    let mut out = Vec::new();
    for (_bit, ct) in connectors_with_bits(mask) {
        if !out.contains(&ct) {
            out.push(ct);
        }
    }
    out
}

/// 三层之一: `discover_ports` — 由 SDK/Manifest 投影出真实端口发现 (早于 Manifest 投影, §三)。
///
/// 端口身份 `port_id = uuid5(NS, device_id:connector:ordinal)`, `ordinal` 表达
/// "该 jack 的物理槽位(从 1 起按 SDK 掩码位序); `direction` 与 RuntimeBinding
/// 均为属性, 绝不进身份键"(冻结于 CANONICAL_IDENTITY.md §5.1/§7; PORT-COLLISION-01
/// closure)。槽位空间正交划分: 输入 jack 占用 1..N, 输出 jack 占用 N+1..N+M,
/// 保证两个物理 jack 即使 connector 相同也得到唯一 PortId, 且跨重启稳定(SDK
/// 掩码位序稳定, ordinal 序列完全派生, 无任何运行时状态依赖)。
///
/// * BMD 真实设备 (`video_input_connections`/`video_output_connections` 非 0):
///   按位枚举 in-jack 与 out-jack, 分别分配 1..N 与 N+1..N+M 槽位。
/// * 非真实硬件 (simulation / filesystem / default): 由该设备在 manifest 中声明
///   的绑定合成端口, 使三层校验在 CI/测试仍闭合; 生产路径始终走真实分支做 fail-closed。
pub fn discover_ports(
    devices: &[crate::contracts::provider::DiscoveredDevice],
    manifest: &DeviceBindingManifest,
) -> Vec<DeviceDiscovery> {
    let mut out = Vec::new();
    for discovered in devices {
        let dev = &discovered.device;
        let mut ports = Vec::new();
        let has_real = dev.video_input_connections != 0 || dev.video_output_connections != 0;
        if has_real {
            // 输入方向 jack: 按掩码位序 1..N 分配槽位。
            for (idx, (_bit, ct)) in connectors_with_bits(dev.video_input_connections)
                .into_iter()
                .enumerate()
            {
                ports.push(DiscoveredPort::new(
                    &dev.device_id,
                    ct,
                    PortDirection::Input,
                    PortOrdinal::Known(idx as u32 + 1),
                ));
            }
            // 输出方向 jack: 槽位起点 = in-jack 数 + 1, 跨重启由 SDK 掩码
            // 决定起始偏移, 稳定。in/out 两组槽位空间不相交 → 即便 connector
            // 相同, PortId 也唯一。
            let out_offset = connectors_with_bits(dev.video_input_connections).len() as u32;
            for (idx, (_bit, ct)) in connectors_with_bits(dev.video_output_connections)
                .into_iter()
                .enumerate()
            {
                ports.push(DiscoveredPort::new(
                    &dev.device_id,
                    ct,
                    PortDirection::Output,
                    PortOrdinal::Known(out_offset + idx as u32 + 1),
                ));
            }
        } else {
            // 无真实连接位掩码: 由 manifest 该设备声明合成端口 (CI/测试闭环用, 非生产路径).
            for b in &manifest.bindings {
                if Some(b.bmd_device_handle.as_str())
                    == crate::resolver::identity_handle(discovered).as_deref()
                {
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
            identity: discovered.identity.clone(),
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
            .find(|d| {
                d.identity.as_ref().and_then(|i| i.device_handle.as_deref())
                    == Some(b.bmd_device_handle.as_str())
            })
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

/// PORT-COLLISION-01 closure: Discovery 层 PortId 不变量 guard。
///
/// 修复后槽位忠实枚举 + direction 不入键已保证 discovery 不产出重复 port_id;
/// 此函数是真正的 invariant violation 检测(此前为已知缺口告警), 命中即 panic
/// 让回归证据无法藏匿。生产路径下命中 = 修复退化 → fail-fast 让运维立即知晓。
#[cfg(any(test, debug_assertions))]
fn guard_discovery_port_id_invariant(discovery: &[DeviceDiscovery]) {
    let mut seen: HashMap<Uuid, String> = HashMap::new();
    for d in discovery {
        for p in &d.ports {
            let Some(pid) = p.port_id else { continue };
            let desc = format!(
                "{:?}/{:?}/{:?}@{}",
                p.direction, p.connector, p.ordinal, d.device.display_name
            );
            if let Some(prev) = seen.insert(pid, desc.clone()) {
                panic!(
                    "PortId 不变量被破坏 (PORT-COLLISION-01 regression): {pid} 先={prev} 再={desc}"
                );
            }
        }
    }
}

#[cfg(not(any(test, debug_assertions)))]
fn guard_discovery_port_id_invariant(discovery: &[DeviceDiscovery]) {
    let mut seen: HashMap<Uuid, String> = HashMap::new();
    for d in discovery {
        for p in &d.ports {
            let Some(pid) = p.port_id else { continue };
            let desc = format!(
                "{:?}/{:?}/{:?}@{}",
                p.direction, p.connector, p.ordinal, d.device.display_name
            );
            if let Some(prev) = seen.insert(pid, desc.clone()) {
                tracing::error!(
                    port_id = %pid,
                    first = %prev,
                    second = %desc,
                    "PortId 不变量被破坏 (PORT-COLLISION-01 regression): \
                     槽位忠实枚举失效, 立即检查 connector_from_mask 位序"
                );
            }
        }
    }
}

fn duplicate_port_id_mismatch(pid: Uuid, first: &str, second: &str) -> DiscoveryMismatch {
    DiscoveryMismatch {
        binding: format!("duplicate-port-id:{pid}"),
        expected: format!("唯一 PortIdentity: {first}"),
        found: vec![second.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::provider::DiscoveredDevice;
    use crate::resolver::{BindingEntry, Confidence, ResolverMatch};

    fn dev(handle: &str) -> crate::contracts::provider::DiscoveredDevice {
        crate::contracts::provider::DiscoveredDevice {
            device: DeviceInfo {
                device_id: Uuid::new_v4(),
                model: "DeckLink".into(),
                display_name: format!("dv-{handle}"),
                serial_number: None,
                identity_strength: crate::device::IdentityStrength::DeviceHandle,
                identity_source: crate::device::DeviceIdentitySource::RealBmd,
                capabilities: crate::port::DeviceCapabilities::default(),
                video_input_connections: 0,
                video_output_connections: 0,
                ports: Vec::new(),
            },
            identity: Some(crate::contracts::provider::ProviderIdentity {
                provider: "blackmagic",
                persistent_id: None,
                device_handle: Some(handle.to_string()),
                topological_id: None,
            }),
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

    /// PORT-COLLISION-01 closure: 显式传 ordinal + connector, 解耦于
    /// gst_device_number, 反映物理 jack 槽位语义(in 1..N, out N+1..N+M)。
    fn manifest_port(
        handle: &str,
        gst_num: u32,
        direction: PortDirection,
        connector: ConnectorType,
        ordinal: u32,
    ) -> BindingEntry {
        BindingEntry {
            label: None,
            bmd_device_handle: handle.to_string(),
            gst_device_number: gst_num,
            expected_hw_serial_number: None,
            expected_model: None,
            port: Some(crate::resolver::PortBinding {
                connector,
                ordinal,
                direction,
                required: false,
                verification: VerificationLevel::Declared,
            }),
        }
    }

    /// 02-I 前置（第十七轮 §七②）测试夹具: 指定 connector 的 manifest 端口声明。
    fn manifest_entry_conn(
        handle: &str,
        direction: PortDirection,
        connector: ConnectorType,
    ) -> BindingEntry {
        BindingEntry {
            label: None,
            bmd_device_handle: handle.to_string(),
            gst_device_number: 1,
            expected_hw_serial_number: None,
            expected_model: None,
            port: Some(crate::resolver::PortBinding {
                connector,
                ordinal: 1,
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
    fn rf_ff_01d_authorized_registry_has_no_backend_runtime_address() {
        let d = dev("46:00000000:002e4500");
        let manifest = base_manifest(vec![manifest_entry(
            "46:00000000:002e4500",
            7,
            PortDirection::Input,
        )]);
        let reg = PortRegistry::build_authorized(&[d], &manifest).expect("authorized registry");
        assert_eq!(reg.ports.len(), 1);
        let p = &reg.ports[0];
        assert!(
            p.runtime_binding.is_none(),
            "neutral registry must not carry gst address"
        );
        assert_eq!(p.signal.state, SignalState::Unknown);
        assert_eq!(p.signal.video_locked, None);
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
            d.device.device_id,
            ResolvedDeviceBinding {
                device_number: 1,
                hw_serial_number: None,
                persistent_id: None,
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
            device: device.device.clone(),
            identity: device.identity.clone(),
            capabilities: DeviceCapabilities::default(),
            ports: vec![DiscoveredPort::new(
                &device.device.device_id,
                ConnectorType::Sdi,
                PortDirection::Input,
                PortOrdinal::Known(1),
            )],
        }];
        let manifest = base_manifest(vec![manifest_entry("devh", 1, PortDirection::Output)]);
        assert!(validate_manifest_against_discovery(&discovery, &manifest).is_err());
    }

    #[test]
    fn build_real_mask_capabilities_from_sdk_evidence() {
        // 02-I P0-2: SDK 连接位掩码 = 能力证据。SDI 输入位 → input
        // Supported(true) + audio 嵌入 Supported(true); 输出掩码=0 →
        // output Unsupported（SDK 枚举权威, 非方向反推）。
        let mut d = dev("46:00000000:002e4500");
        d.device.video_input_connections = 0x1; // SDI in
        let manifest = base_manifest(vec![manifest_entry(
            "46:00000000:002e4500",
            1,
            PortDirection::Input,
        )]);
        let reg = PortRegistry::build(&[d], &[], &manifest, &HashMap::new()).expect("build 应成功");
        let p = &reg.ports[0];
        assert!(matches!(
            p.capabilities.input,
            CapabilityValue::Supported(true)
        ));
        assert!(matches!(
            p.capabilities.audio_input,
            CapabilityValue::Supported(true)
        ));
        assert_eq!(p.capabilities.output, CapabilityValue::Unsupported);
        // 设备级聚合: 端口证据传播。
        let dev_caps = reg.device_capabilities(&p.device_id);
        assert!(matches!(
            dev_caps.audio_input,
            CapabilityValue::Supported(true)
        ));
    }

    #[test]
    fn build_synthetic_device_capabilities_stay_unknown() {
        // 仿真/合成（双掩码=0）: 无 SDK 能力证据 → Unknown（manifest 方向
        // 声明不是能力证据, 禁反推）。
        let d = dev("46:00000000:002e4400");
        let manifest = base_manifest(vec![manifest_entry(
            "46:00000000:002e4400",
            1,
            PortDirection::Input,
        )]);
        let reg = PortRegistry::build(&[d], &[], &manifest, &HashMap::new()).expect("build 应成功");
        let p = &reg.ports[0];
        assert_eq!(p.capabilities.input, CapabilityValue::Unknown);
        assert_eq!(p.capabilities.audio_input, CapabilityValue::Unknown);
        let dev_caps = reg.device_capabilities(&p.device_id);
        assert_eq!(dev_caps.audio_input, CapabilityValue::Unknown);
    }

    #[test]
    fn build_unspecified_connector_capabilities_unknown() {
        // manifest 条目未声明 port（connector=Unknown）: 能力无锚点 → Unknown。
        let d = dev("46:00000000:002e4300");
        let entry = BindingEntry {
            label: None,
            bmd_device_handle: "46:00000000:002e4300".into(),
            gst_device_number: 1,
            expected_hw_serial_number: None,
            expected_model: None,
            port: None,
        };
        let manifest = base_manifest(vec![entry]);
        let reg = PortRegistry::build(&[d], &[], &manifest, &HashMap::new()).expect("build 应成功");
        assert_eq!(reg.ports[0].capabilities.input, CapabilityValue::Unknown);
    }

    #[test]
    fn build_duplex_card_in_out_jacks_have_distinct_port_ids() {
        // PORT-COLLISION-01 closure 正控制: 固定全双工卡(BMD 实证)。
        // in-jack 与 out-jack 是两个独立 BNC, discovery 按方向分槽 →
        // in slot=1, out slot=2(因 in 掩码 1 位, out_offset=1)。
        let mut d = dev("46:00000000:002e4700");
        d.device.video_input_connections = 0x1; // SDI in
        d.device.video_output_connections = 0x1; // SDI out
        let manifest = base_manifest(vec![
            manifest_port(
                "46:00000000:002e4700",
                1,
                PortDirection::Input,
                ConnectorType::Sdi,
                1,
            ),
            manifest_port(
                "46:00000000:002e4700",
                2,
                PortDirection::Output,
                ConnectorType::Sdi,
                2,
            ),
        ]);
        let reg = PortRegistry::build(&[d], &[], &manifest, &HashMap::new())
            .expect("双工卡两个独立 jack 必须可同时构建");
        assert_eq!(reg.ports.len(), 2);
        let (a, b) = (&reg.ports[0], &reg.ports[1]);
        assert_ne!(a.identity.port_id, b.identity.port_id);
        assert_eq!(a.direction, PortDirection::Input);
        assert_eq!(a.identity.ordinal, PortOrdinal::Known(1));
        assert_eq!(b.direction, PortDirection::Output);
        assert_eq!(b.identity.ordinal, PortOrdinal::Known(2));
    }

    #[test]
    fn build_duplex_mask_single_direction_manifest_ok() {
        // 正控制（盒上实测形态）: 双工掩码（SDI in+out）+ manifest 只声明输入侧 →
        // 证据面碰撞告警不 brick 流程, registry 无别名（单端口）。
        let mut d = dev("46:00000000:002e4500");
        d.device.video_input_connections = 0x1;
        d.device.video_output_connections = 0x1;
        let manifest = base_manifest(vec![manifest_entry(
            "46:00000000:002e4500",
            1,
            PortDirection::Input,
        )]);
        let reg = PortRegistry::build(&[d], &[], &manifest, &HashMap::new())
            .expect("双工卡只声明消费侧端口应成功");
        assert_eq!(reg.ports.len(), 1);
        let mut ids: Vec<Uuid> = reg
            .ports
            .iter()
            .filter_map(|p| p.identity.port_id)
            .collect();
        ids.dedup();
        assert_eq!(ids.len(), reg.ports.len(), "registry 内无别名 port_id");
    }

    #[test]
    fn build_analog_bit_folding_yields_distinct_slots() {
        // PORT-COLLISION-01 closure: Component(0x8) + Composite(0x10) 同属 Analog,
        // 但每位表达一个独立物理 jack; discovery 按位序分配槽位 1/2, PortId 不同。
        let mut d = dev("46:00000000:002e4600");
        d.device.video_input_connections = 0x8 | 0x10; // Component | Composite
        let discovery = discover_ports(&[d.clone()], &base_manifest(vec![]));
        assert_eq!(discovery[0].ports.len(), 2);
        assert_ne!(
            discovery[0].ports[0].port_id, discovery[0].ports[1].port_id,
            "Analog 位折叠必须按位序分槽"
        );
        // manifest 用 jack 级 ordinal(1, 2) 分别寻址两个独立 jack。
        let manifest = base_manifest(vec![
            manifest_port(
                "46:00000000:002e4600",
                1,
                PortDirection::Input,
                ConnectorType::Analog,
                1,
            ),
            manifest_port(
                "46:00000000:002e4600",
                2,
                PortDirection::Input,
                ConnectorType::Analog,
                2,
            ),
        ]);
        let reg = PortRegistry::build(&[d], &[], &manifest, &HashMap::new())
            .expect("按位序分槽后 Analog 多 jack 可同时声明");
        assert_eq!(reg.ports.len(), 2);
        let mut ids: Vec<Uuid> = reg
            .ports
            .iter()
            .filter_map(|p| p.identity.port_id)
            .collect();
        ids.dedup();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn build_duplicate_manifest_port_identity_fail_closed() {
        // 02-I 前置（第十七轮 §七②）belt: 真实掩码只有 SDI 输入时, 两条 manifest
        // 声明同 handle 同 SDI/1 → registry 装配出重复 port_id（discovery 层无碰撞,
        // 由 registry 侧防线捕获）。
        let mut d = dev("46:00000000:002e4800");
        d.device.video_input_connections = 0x1;
        let manifest = base_manifest(vec![
            manifest_entry("46:00000000:002e4800", 1, PortDirection::Input),
            manifest_entry("46:00000000:002e4800", 1, PortDirection::Input),
        ]);
        let err = PortRegistry::build(&[d], &[], &manifest, &HashMap::new())
            .expect_err("manifest 重复声明同一 PortIdentity 必须 fail-closed");
        assert!(err.binding.contains("duplicate-port-id"), "{err:?}");
    }

    #[test]
    fn build_distinct_connectors_do_not_collide() {
        // 正控制: 同设备不同 connector（SDI|HDMI 输入）不触发碰撞防线。
        let mut d = dev("46:00000000:002e4900");
        d.device.video_input_connections = 0x1 | 0x2; // SDI | HDMI
        let manifest = base_manifest(vec![manifest_entry(
            "46:00000000:002e4900",
            1,
            PortDirection::Input,
        )]);
        let reg = PortRegistry::build(&[d], &[], &manifest, &HashMap::new())
            .expect("不同 connector 不碰撞");
        assert_eq!(reg.ports.len(), 1);
    }

    // ── PORT-COLLISION-01: 物理端口忠实发现（failure-first 测试先行） ──
    //
    // BMD 真机证据（10.30.15.10, 2026-09-24 `VBMF_CONFIG_PROBE`, 已存 evidence）:
    // "DeckLink SDI" ×2 每子设备 cap_video_in=1 且 cap_video_out=1, cfg_video_in=1
    // **同时** cfg_video_out=1, 无 profile/ConnectorMode 切换证据 → 固定全双工卡:
    // SDI-IN 与 SDI-OUT 是两个物理独立 jack。旧 discover_ports 按"每 connector
    // 类型 × 每方向"伪造端口且序号恒 Known(1) → 两个物理 jack 共享同一 PortId
    // (碰撞 ×2 实证)。修复 = 槽位忠实枚举 + jack 级能力, derive 键不变。

    fn dev_with_masks(handle: &str, in_mask: u64, out_mask: u64) -> DiscoveredDevice {
        let mut d = dev(handle);
        d.device.video_input_connections = in_mask;
        d.device.video_output_connections = out_mask;
        d
    }

    #[test]
    fn pc01_discovery_full_duplex_card_yields_two_distinct_physical_ports() {
        // 同设备 in/out 掩码都含 SDI（BMD 实证形态）→ 两个物理端口:
        // in-jack = Sdi/1/Input, out-jack = Sdi/2/Output(因 in 掩码 1 位,
        // out_offset=1), PortId 必不同。
        let d = dev_with_masks("46:00000000:002e4500", 0x1, 0x1);
        let manifest = base_manifest(vec![]);
        let discovery = discover_ports(&[d], &manifest);
        let ports = &discovery[0].ports;
        assert_eq!(ports.len(), 2, "全双工卡必须发现两个物理端口: {ports:?}");
        assert_eq!(ports[0].direction, PortDirection::Input);
        assert_eq!(ports[0].ordinal, PortOrdinal::Known(1));
        assert_eq!(ports[1].direction, PortDirection::Output);
        assert_eq!(ports[1].ordinal, PortOrdinal::Known(2));
        let (a, b) = (ports[0].port_id.unwrap(), ports[1].port_id.unwrap());
        assert_ne!(a, b, "两个物理 jack 不得共享 PortId（BMD 碰撞 ×2 根因）");
    }

    #[test]
    fn pc01_discovery_port_ids_stable_across_repeat_and_device_order() {
        // 同一物理端口跨发现次序/重复运行 PortId 不变（身份稳定性）。
        let d1 = dev_with_masks("46:00000000:002e4500", 0x1, 0x1);
        let d2 = dev_with_masks("46:00000000:002e4400", 0x1, 0x0);
        let manifest = base_manifest(vec![]);
        let run1 = discover_ports(&[d1.clone(), d2.clone()], &manifest);
        let run2 = discover_ports(&[d2.clone(), d1.clone()], &manifest);
        let ids = |run: &[DeviceDiscovery], handle: &str| -> Vec<Option<Uuid>> {
            run.iter()
                .find(|r| r.device.display_name == format!("dv-{handle}"))
                .map(|r| r.ports.iter().map(|p| p.port_id).collect())
                .unwrap()
        };
        assert_eq!(
            ids(&run1, "46:00000000:002e4500"),
            ids(&run2, "46:00000000:002e4500")
        );
        assert_eq!(
            ids(&run1, "46:00000000:002e4400"),
            ids(&run2, "46:00000000:002e4400")
        );
    }

    #[test]
    fn pc01_discovery_output_only_card_gets_slot_per_type() {
        // Mini Monitor 4K（真机 cap_out=0x3, 无输入）: Sdi 槽位 1 + Hdmi 槽位 2
        // (按 SDK 掩码位序 0x1,0x2; in 掩码=0 → out_offset=0)。
        let d = dev_with_masks("83:1a66443b:00000000", 0x0, 0x3);
        let discovery = discover_ports(&[d], &base_manifest(vec![]));
        let ports = &discovery[0].ports;
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].connector, ConnectorType::Sdi);
        assert_eq!(ports[0].ordinal, PortOrdinal::Known(1));
        assert_eq!(ports[1].connector, ConnectorType::Hdmi);
        assert_eq!(ports[1].ordinal, PortOrdinal::Known(2));
        assert_ne!(ports[0].port_id, ports[1].port_id);
    }

    #[test]
    fn pc01_discovery_analog_mask_folding_gets_distinct_slots() {
        // Component(0x8)+Composite(0x10) 折叠为 Analog 时仍不得共享身份:
        // 按掩码位序分槽（Analog/1, Analog/2），位序由 SDK 头定义、跨重启稳定。
        let d = dev_with_masks("46:00000000:002e4600", 0x8 | 0x10, 0x0);
        let discovery = discover_ports(&[d], &base_manifest(vec![]));
        let ports = &discovery[0].ports;
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].connector, ConnectorType::Analog);
        assert_eq!(ports[0].ordinal, PortOrdinal::Known(1));
        assert_eq!(ports[1].connector, ConnectorType::Analog);
        assert_eq!(ports[1].ordinal, PortOrdinal::Known(2));
        assert_ne!(ports[0].port_id, ports[1].port_id);
    }

    #[test]
    fn pc01_discovery_never_produces_duplicate_port_ids() {
        // invariant: 任何真实掩码组合下 discovery 不得产出重复 port_id。
        let fixtures = vec![
            dev_with_masks("h-duplex", 0x1, 0x1),
            dev_with_masks("h-out-only", 0x0, 0x3),
            dev_with_masks("h-analog", 0x8 | 0x10, 0x0),
            dev_with_masks("h-in-multi", 0x1 | 0x2, 0x0),
            dev_with_masks("h-full", 0x1 | 0x2, 0x1 | 0x2),
        ];
        let discovery = discover_ports(&fixtures, &base_manifest(vec![]));
        let mut ids: Vec<Uuid> = discovery
            .iter()
            .flat_map(|d| d.ports.iter().filter_map(|p| p.port_id))
            .collect();
        let total = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), total, "discovery 层 PortId 不变量被破坏");
    }

    #[test]
    fn pc01_registry_full_duplex_manifest_both_jacks_builds_two_ports() {
        // 全双工卡的正确 manifest 表达: in-jack = sdi/1, out-jack = sdi/2
        // (ordinal = 物理槽位语义)。两端口必须可同时唯一寻址, 能力按 jack 级
        // 证据（不把设备级输出掩码摊平到 in-jack）。
        let d = dev_with_masks("46:00000000:002e4500", 0x1, 0x1);
        let manifest = base_manifest(vec![
            manifest_entry("46:00000000:002e4500", 1, PortDirection::Input),
            manifest_entry("46:00000000:002e4500", 2, PortDirection::Output),
        ]);
        let reg = PortRegistry::build(&[d], &[], &manifest, &HashMap::new())
            .expect("全双工卡双 jack 声明必须可构建");
        assert_eq!(reg.ports.len(), 2);
        let (a, b) = (&reg.ports[0], &reg.ports[1]);
        assert_ne!(
            a.identity.port_id, b.identity.port_id,
            "in-jack 与 out-jack 必须是两个可寻址身份"
        );
        assert_eq!(a.direction, PortDirection::Input);
        assert_eq!(b.direction, PortDirection::Output);
        // jack 级能力: in-jack 不能输出, out-jack 不能采集。
        assert!(matches!(
            a.capabilities.input,
            CapabilityValue::Supported(true)
        ));
        assert_eq!(a.capabilities.output, CapabilityValue::Unsupported);
        assert!(matches!(
            b.capabilities.output,
            CapabilityValue::Supported(true)
        ));
        assert_eq!(b.capabilities.input, CapabilityValue::Unsupported);
        // 设备级聚合按物理 jack 计数。
        let dc = reg.device_capabilities(&a.device_id);
        assert!(matches!(dc.input_port_count, CapabilityValue::Supported(1)));
        assert!(matches!(
            dc.output_port_count,
            CapabilityValue::Supported(1)
        ));
    }

    #[test]
    fn pc01_registry_in_jack_output_cap_not_flattened_from_device_mask() {
        // 只声明 in-jack 时: 设备级 out 掩码存在也不得让 in-jack 获得 output 能力
        // （旧实现摊平设备掩码 → in-jack 误报 output Supported → Resource 别名）。
        let d = dev_with_masks("46:00000000:002e4500", 0x1, 0x1);
        let manifest = base_manifest(vec![manifest_entry(
            "46:00000000:002e4500",
            1,
            PortDirection::Input,
        )]);
        let reg = PortRegistry::build(&[d], &[], &manifest, &HashMap::new()).expect("build");
        assert_eq!(reg.ports.len(), 1);
        let p = &reg.ports[0];
        assert!(matches!(
            p.capabilities.input,
            CapabilityValue::Supported(true)
        ));
        assert_eq!(p.capabilities.output, CapabilityValue::Unsupported);
        // Resource 映射: 该端口只派生 input 资源, 无 output 别名资源。
        let pid = p.identity.port_id.unwrap();
        assert!(matches!(
            reg.device_capabilities(&p.device_id).output_port_count,
            CapabilityValue::Supported(0)
        ));
        let _ = crate::resource::input_resource_id_for_port(pid);
    }

    #[test]
    fn pc01_registry_contradictory_same_slot_declaration_fail_closed() {
        // 旧词汇 sdi/1/input + sdi/1/output = 同槽位方向矛盾（对固定全双工卡
        // 这是物理错误声明）→ fail-closed 不变; 正确表达见
        // pc01_registry_full_duplex_manifest_both_jacks_builds_two_ports。
        let d = dev_with_masks("46:00000000:002e4700", 0x1, 0x1);
        let manifest = base_manifest(vec![
            manifest_entry("46:00000000:002e4700", 1, PortDirection::Input),
            manifest_entry("46:00000000:002e4700", 1, PortDirection::Output),
        ]);
        assert!(
            PortRegistry::build(&[d], &[], &manifest, &HashMap::new()).is_err(),
            "同槽位矛盾方向声明必须 fail-closed"
        );
    }

    #[test]
    fn pc01_registry_port_id_immune_to_provider_binding_change() {
        // Runtime binding（gst device-number）存在与否不得改变 canonical PortId。
        let d = dev_with_masks("46:00000000:002e4500", 0x1, 0x0);
        let manifest = base_manifest(vec![manifest_entry(
            "46:00000000:002e4500",
            1,
            PortDirection::Input,
        )]);
        let mut bindings = HashMap::new();
        bindings.insert(
            d.device.device_id,
            ResolvedDeviceBinding {
                device_number: 1,
                hw_serial_number: None,
                persistent_id: None,
                confidence: Confidence::High,
                match_kind: ResolverMatch::ManifestVerified,
            },
        );
        let r1 = PortRegistry::build(std::slice::from_ref(&d), &[], &manifest, &HashMap::new())
            .expect("r1");
        let r2 = PortRegistry::build(&[d], &[], &manifest, &bindings).expect("r2");
        assert_eq!(r1.ports[0].identity.port_id, r2.ports[0].identity.port_id);
        // binding 生效差异只体现在 runtime_binding, 不在身份。
        assert!(r1.ports[0].runtime_binding.is_none());
        assert!(r2.ports[0].runtime_binding.is_some());
    }

    #[test]
    fn pc01_bidirectional_declaration_no_identity_drift() {
        // direction 是可变声明属性: 同一物理槽位在 Input ↔ Bidirectional 声明
        // 切换时 PortId 绝不漂移（direction 不进身份键, 冻结约束）。
        let d = dev("46:00000000:002e4b00"); // 合成设备（掩码=0）
        let m_in = base_manifest(vec![manifest_entry(
            "46:00000000:002e4b00",
            1,
            PortDirection::Input,
        )]);
        let m_bidir = base_manifest(vec![manifest_entry(
            "46:00000000:002e4b00",
            1,
            PortDirection::Bidirectional,
        )]);
        let r1 =
            PortRegistry::build(std::slice::from_ref(&d), &[], &m_in, &HashMap::new()).expect("r1");
        let r2 = PortRegistry::build(&[d], &[], &m_bidir, &HashMap::new()).expect("r2");
        assert_eq!(r1.ports[0].identity.port_id, r2.ports[0].identity.port_id);
        assert_eq!(r2.ports[0].direction, PortDirection::Bidirectional);
    }
}
