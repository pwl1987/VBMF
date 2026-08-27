//! BMD / filesystem device discovery (Gate 2.2 + Phase 0.5 device registry).
//!
//! Boundary: this module owns the **Device Registry** (hardware identity source).
//! It does NOT perform media capture — that is owned by `pipeline.rs` (canonical
//! GStreamer) per Phase 0.6. The Registry only enumerates device identity/metadata.
//!
//! Identity hierarchy (Blackmagic + A0 实测, 10.30.15.10, SDK 16.0):
//!   PersistentID → TopologicalID → DeviceHandle → Enumeration
//! - 本硬件三台设备 `GetInt(PersistentID)`/`GetInt(TopologicalID)` 均 `0x80000003`
//!   (BMD 属性不支持) → canonical 硬件身份 = **DeviceHandle** (`GetString('devh')`).
//! - `DeviceHandle` 是 "当前主机的 best-available identity", **非**跨机器/重启永久稳定
//!   身份 (后者仅 PersistentID 提供). 详见 `evidence/bmd-10.30.15.10/`.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

use crate::decklink::BmdDeviceIdentity;
use crate::port::{DeviceCapabilities, PortInfo};

/// 设备身份强度 (A0 实测: 本硬件仅支持 DeviceHandle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentityStrength {
    /// BMD `PersistentID` (官方最高优先级, 跨机器/重启永久稳定). 当前硬件 `GetInt=0x80000003` 不支持.
    PersistentId,
    /// BMD `DeviceHandle` (4CC `devh`, 当前主机 best-available identity, 非永久稳定).
    DeviceHandle,
    /// BMD `TopologicalID` (拓扑敏感, 重启/拓扑变化会漂移).
    TopologicalId,
    /// 纯 SDK/文件系统枚举 (无稳定硬件身份; 仅 CI/诊断).
    Enumeration,
}

/// 身份来源 (防 synthetic UUID 与真实 BMD UUID 混淆; 用户复核 §十六).
/// 控制面/Lease/GraphRuntime 应据此外区分 "真实硬件" 与 "合成身份".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceIdentitySource {
    /// 真实 BMD DeckLink (DeviceHandle 派生 canonical 身份).
    RealBmd,
    /// 文件系统节点合成的占位身份 (CI/无硬件; 不可用于生产 materialize).
    FilesystemSynthetic,
    /// 显式模拟世界 (测试; 允许伪造 PersistentId 因本身是测试世界).
    Simulation,
}

/// 设备注册表条目 (Device Registry — 硬件身份源).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// VBMF 确定性设备 ID (UUIDv5 over 稳定身份, 非随机).
    pub device_id: Uuid,
    /// 设备型号 (如 "DeckLink Mini Monitor 4K").
    pub model: String,
    /// 显示名 (如 "dv0" / "DeckLink SDI").
    pub display_name: String,
    /// 厂商序列号 (若有).
    pub serial_number: Option<String>,
    /// BMD `PersistentID` (仅真实 BMD 且支持时 `Some`; 否则 `None`).
    /// **注意**: 不得用 hash/合成值伪造 (用户复核 §十八 P0 安全边界).
    pub bmd_persistent_id: Option<i64>,
    /// BMD `DeviceHandle` (`GetString('devh')`, 当前主机 best-available identity).
    pub bmd_device_handle: Option<String>,
    /// BMD `TopologicalID` (`GetInt('topl')`, 拓扑敏感, 重启/拓扑变化会漂移; SDK 不支持时 `None`).
    pub bmd_topological_id: Option<i64>,
    /// BMD 真实输入连接位掩码 (来自 `BmdDeviceIdentity.video_input_connections`), 0 = 未探测/非 BMD.
    /// HW-PORT-01A `discover_ports` 据此枚举真实端口 (§四).
    pub video_input_connections: u64,
    /// BMD 真实输出连接位掩码.
    pub video_output_connections: u64,
    /// 身份强度 (决定 materialize 选卡路径).
    pub identity_strength: IdentityStrength,
    /// 身份来源 (RealBmd / FilesystemSynthetic / Simulation).
    pub identity_source: DeviceIdentitySource,
    /// 设备级能力 (由 Port Discovery 归并; 当前 Runtime 多为 Unknown — 见 `PortRegistry`).
    /// HARD RULE: 绝不得写死当前拓扑 (禁止硬编码 dn0/dn1/dn2 语义).
    pub capabilities: DeviceCapabilities,
    /// 该设备的物理端口注册表 (Port Discovery 结果; 属于 Runtime Discovery Evidence).
    pub ports: Vec<PortInfo>,
}

/// Device Manager trait — 不同环境 (simulation / filesystem / real BMD) 实现不同发现逻辑.
pub trait DeviceManager {
    fn discover(&self) -> Vec<DeviceInfo>;
}

/// CI / 非硬件构建: 从 `/dev/blackmagic/*` 节点发现 (无 SDK/硬件假定).
pub struct FilesystemDeviceManager;

impl FilesystemDeviceManager {
    pub fn new() -> Self {
        Self
    }
}

impl DeviceManager for FilesystemDeviceManager {
    fn discover(&self) -> Vec<DeviceInfo> {
        let base = Path::new("/dev/blackmagic");
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(base) {
            for e in entries.flatten() {
                let dv = e.file_name().to_string_lossy().to_string();
                if !dv.starts_with("dv") && !dv.starts_with("io") {
                    continue;
                }
                // 仅用节点名派生确定性 UUID (CI 可复现); 无真实硬件身份.
                // 注: 真实 BMD `PersistentID` 仅来自 `DeckLinkDeviceManager` (A0 实测). 这里用节点名
                // 派生合成占位身份以保非硬件构建可运行, 但**绝不伪造 bmd_persistent_id**
                // (旧实现 `Some(hash)` 会让 `materialize` 把它当成真实 PersistentID → 静默越权,
                // 用户复核 §十八 列为 P0 安全边界问题, 已修). filesystem 身份来源 = FilesystemSynthetic,
                // 强度恒为 `Enumeration`; 生产 materialize 据 `identity_strength` 拒绝 (绝不接受
                // 合成持久身份). bmd_persistent_id = None 是正确语义 (此处无真实持久身份).
                out.push(DeviceInfo {
                    device_id: Uuid::new_v5(&VBMF_FS_NS, format!("vbmf:fs:{dv}").as_bytes()),
                    model: "blackmagic-filesystem-node".into(),
                    display_name: dv.clone(),
                    serial_number: None,
                    bmd_persistent_id: None,
                    bmd_device_handle: None,
                    bmd_topological_id: None,
                    video_input_connections: 0,
                    video_output_connections: 0,
                    identity_strength: IdentityStrength::Enumeration,
                    identity_source: DeviceIdentitySource::FilesystemSynthetic,
                    capabilities: DeviceCapabilities::default(),
                    ports: Vec::new(),
                });
            }
        }
        out
    }
}

/// 模拟设备 (CI / 单元测试; 无硬件/SDK). 模拟世界允许伪造 PersistentId (本身是测试世界).
pub struct SimulatedDeviceManager;

impl SimulatedDeviceManager {
    pub fn new() -> Self {
        Self
    }
}

impl DeviceManager for SimulatedDeviceManager {
    fn discover(&self) -> Vec<DeviceInfo> {
        (0..2)
            .map(|i| DeviceInfo {
                device_id: Uuid::new_v5(&VBMF_SIM_NS, format!("vbmf:sim:{i}").as_bytes()),
                model: "DeckLink Mini Monitor 4K (sim)".into(),
                display_name: format!("sim-{i}"),
                serial_number: Some(format!("SIM-SERIAL-{i}")),
                bmd_persistent_id: Some(9000 + i as i64),
                bmd_device_handle: Some(format!("sim-handle-{i}")),
                bmd_topological_id: None,
                video_input_connections: 0,
                video_output_connections: 0,
                identity_strength: IdentityStrength::PersistentId,
                identity_source: DeviceIdentitySource::Simulation,
                capabilities: DeviceCapabilities::default(),
                ports: Vec::new(),
            })
            .collect()
    }
}

/// 真实 BMD DeckLink (feature `bmd`): 基于 DeviceHandle 派生 canonical 身份.
/// 身份来源 = RealBmd; 强度由 SDK 属性可用性决定 (本硬件 → DeviceHandle).
pub struct DeckLinkDeviceManager;

impl DeckLinkDeviceManager {
    pub fn new() -> Self {
        Self
    }
}

impl DeviceManager for DeckLinkDeviceManager {
    fn discover(&self) -> Vec<DeviceInfo> {
        // Identity Closure Patch: enumerate() 已返回独立、互不污染的
        // model/display/serial/persistent_id/device_handle/topological_id. 这里严格按官方身份层级
        // (PersistentID → DeviceHandle → TopologicalID → 枚举序号) 构造 canonical 派生键,
        // 不再用 display/serial 伪装 DeviceHandle. 各身份维度独立写入 DeviceInfo,
        // 使 `device_id = UUIDv5(DeviceHandle)`、`bmd_device_handle`=真实 devh 真正闭合.
        let discovered = match crate::decklink::enumerate() {
            Ok(d) => d,
            // 非 bmd 构建: enumerate 恒 Err, 无设备可派生身份 (绝不伪造).
            Err(_) => return Vec::new(),
        };
        discovered
            .into_iter()
            .map(|d: BmdDeviceIdentity| {
                // canonical 派生键 = 真实 SDK 身份; 优先级 PersistentID > DeviceHandle > Serial > 枚举.
                // 当前硬件 (10.30.15.10, SDK 16.0) 三台均无 PersistentID → canonical = DeviceHandle.
                let canonical = if let Some(pid) = d.persistent_id {
                    format!("pid:{pid}")
                } else if !d.device_handle.is_empty() {
                    d.device_handle.clone()
                } else if !d.serial.is_empty() {
                    d.serial.clone()
                } else {
                    "unknown".to_string()
                };
                let identity_strength = if d.persistent_id.is_some() {
                    IdentityStrength::PersistentId
                } else if !d.device_handle.is_empty() || !d.serial.is_empty() {
                    IdentityStrength::DeviceHandle
                } else {
                    IdentityStrength::Enumeration
                };
                DeviceInfo {
                    device_id: Uuid::new_v5(
                        &VBMF_BMD_NS,
                        format!("vbmf:bmd:{canonical}").as_bytes(),
                    ),
                    model: d.model.clone(),
                    display_name: d.display.clone(),
                    serial_number: if d.serial.is_empty() {
                        None
                    } else {
                        Some(d.serial.clone())
                    },
                    bmd_persistent_id: d.persistent_id.map(|v| v as i64),
                    bmd_device_handle: if d.device_handle.is_empty() {
                        None
                    } else {
                        Some(d.device_handle.clone())
                    },
                    bmd_topological_id: d.topological_id.map(|v| v as i64),
                    video_input_connections: d.video_input_connections,
                    video_output_connections: d.video_output_connections,
                    identity_strength,
                    identity_source: DeviceIdentitySource::RealBmd,
                    capabilities: DeviceCapabilities::default(),
                    ports: Vec::new(),
                }
            })
            .collect()
    }
}

/// 确定性 UUID 命名空间 (避免随机 UUID 导致设备 ID 漂移).
const VBMF_FS_NS: Uuid = Uuid::from_u128(0x9f3b2c1d_4e5a_4b6c_8d7e_0f1a2b3c4d5e);
const VBMF_SIM_NS: Uuid = Uuid::from_u128(0x1a2b3c4d_5e6f_4078_9a0b_1c2d3e4f5a6b);
const VBMF_BMD_NS: Uuid = Uuid::from_u128(0x2b3c4d5e_6f70_4180_ab0c_2d3e4f5a6b7c);
