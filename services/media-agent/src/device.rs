//! Device Registry — canonical 硬件身份源 (Gate 2.2 + Phase 0.5 + Final Merge Hardening P0-1).
//!
//! Boundary: this module owns the **Device Registry** (hardware identity source).
//! It does NOT perform media capture — that is owned by `pipeline.rs` (canonical
//! pipeline plan) per Phase 0.6. The Registry only enumerates device identity/metadata.
//!
//! **hardening (P0-1)**: `DeviceInfo` 是 Canonical Domain schema —— 依
//! `CANONICAL_IDENTITY.md` §4 (Provider Identity ≠ Canonical Identity)，vendor 身份机制字段
//! (DeviceHandle / PersistentID / TopologicalID) **不再出现在本结构**；
//! 证据随 `DiscoveredDevice.identity`（SPI 层 `ProviderIdentity`）配对输出，
//! 由 Provider Identity Adapter（resolver/绑定路径）消费。各 Provider 自行定义
//! 其局部身份证据优先级并收敛为 canonical `device_id` (UUIDv5)。
//!
//! 身份强度语义 (provider 自证, discovery 时由证据推导; Domain 不再交叉核验字段):
//!   PersistentId > DeviceHandle > TopologicalId > Enumeration
//! - 当前硬件 (10.30.15.10, SDK 16.0) 三台设备 PersistentID/TopologicalID 均不支持
//!   → canonical 硬件身份 = DeviceHandle 派生。详见 `evidence/bmd-10.30.15.10/`。

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

use crate::contracts::provider::{
    CapabilityReport, ConnectorConfig, DiscoveredDevice, HardwareProvider, ProviderError,
    ProviderIdentity,
};
use crate::port::{DeviceCapabilities, PortInfo};

/// 设备身份强度 (provider 在 discovery 时按自身证据自证; 决定 materialize 选卡路径).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentityStrength {
    /// Provider 本地持久标识 (跨机器/重启永久稳定; 当前硬件不支持).
    PersistentId,
    /// Provider 本地绑定引用 (当前主机 best-available identity, 非永久稳定).
    DeviceHandle,
    /// Provider 本地拓扑标识 (拓扑敏感, 重启/拓扑变化会漂移).
    TopologicalId,
    /// 纯 SDK/文件系统枚举 (无稳定硬件身份; 仅 CI/诊断).
    Enumeration,
}

/// 身份来源 (防 synthetic UUID 与真实硬件 UUID 混淆; 用户复核 §十六).
/// 控制面/Lease/GraphRuntime 应据此外区分 "真实硬件" 与 "合成身份".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceIdentitySource {
    /// 真实硬件 (canonical 身份由 Provider 本地绑定引用派生).
    RealBmd,
    /// 文件系统节点合成的占位身份 (CI/无硬件; 不可用于生产 materialize).
    FilesystemSynthetic,
    /// 显式模拟世界 (测试; 允许伪造身份证据因本身是测试世界).
    Simulation,
}

/// 设备注册表条目 (Device Registry — canonical 硬件身份源).
///
/// **不含 vendor 身份字段** (P0-1): 身份证据见 `contracts::provider::ProviderIdentity`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// VBMF 确定性设备 ID (UUIDv5 over Provider 稳定身份, 非随机).
    pub device_id: Uuid,
    /// 设备型号 (如 "DeckLink Mini Monitor 4K").
    pub model: String,
    /// 显示名 (如 "dv0" / "DeckLink SDI").
    pub display_name: String,
    /// 厂商序列号 (若有; 通用概念, 非 vendor 机制字段).
    pub serial_number: Option<String>,
    /// 真实输入连接位掩码 (provider 报告), 0 = 未探测/无输入.
    /// HW-PORT-01A `discover_ports` 据此枚举真实端口 (§四).
    pub video_input_connections: u64,
    /// 真实输出连接位掩码.
    pub video_output_connections: u64,
    /// 身份强度 (provider 自证; 决定 materialize 选卡路径).
    pub identity_strength: IdentityStrength,
    /// 身份来源 (RealBmd / FilesystemSynthetic / Simulation).
    pub identity_source: DeviceIdentitySource,
    /// 设备级能力 (由 Port Discovery 归并; 当前 Runtime 多为 Unknown — 见 `PortRegistry`).
    /// HARD RULE: 绝不得写死当前拓扑 (禁止硬编码 dn0/dn1/dn2 语义).
    pub capabilities: DeviceCapabilities,
    /// 该设备的物理端口注册表 (Port Discovery 结果; 属于 Runtime Discovery Evidence).
    pub ports: Vec<PortInfo>,
}

/// CI / 非硬件构建: 从 `/dev/blackmagic/*` 节点发现 (无 SDK/硬件假定).
pub struct FilesystemDeviceManager;

impl FilesystemDeviceManager {
    // A2-0: lib 化使本项进入 lib 公开面, clippy new_without_default 随之触发——
    // 结构性编译后果的最小处置（allow 而非补 Default 语义; 不借 A2-0 改 Runtime）。
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }
}

impl HardwareProvider for FilesystemDeviceManager {
    fn discover(&self) -> Result<Vec<DiscoveredDevice>, ProviderError> {
        let base = Path::new("/dev/blackmagic");
        let mut out = Vec::new();
        let entries = match std::fs::read_dir(base) {
            Ok(e) => e,
            // 节点目录不存在 = 无设备 (正常空结果), 非 Provider 故障.
            Err(_) => return Ok(Vec::new()),
        };
        for e in entries.flatten() {
            let dv = e.file_name().to_string_lossy().to_string();
            if !dv.starts_with("dv") && !dv.starts_with("io") {
                continue;
            }
            // 仅用节点名派生确定性 UUID (CI 可复现); 无真实硬件身份.
            // 注: 真实硬件持久标识仅来自真实 Provider 枚举. 节点名作为 provider 本地绑定引用
            // 进入 identity 证据 (provider="filesystem"), 强度恒为 `Enumeration`;
            // 生产 materialize 据 `identity_strength` 拒绝 (绝不接受合成身份).
            out.push(DiscoveredDevice {
                device: DeviceInfo {
                    device_id: Uuid::new_v5(&VBMF_FS_NS, format!("vbmf:fs:{dv}").as_bytes()),
                    model: "blackmagic-filesystem-node".into(),
                    display_name: dv.clone(),
                    serial_number: None,
                    video_input_connections: 0,
                    video_output_connections: 0,
                    identity_strength: IdentityStrength::Enumeration,
                    identity_source: DeviceIdentitySource::FilesystemSynthetic,
                    capabilities: DeviceCapabilities::default(),
                    ports: Vec::new(),
                },
                identity: Some(ProviderIdentity {
                    provider: "filesystem",
                    persistent_id: None,
                    device_handle: Some(dv),
                    topological_id: None,
                }),
            });
        }
        Ok(out)
    }
    fn probe_capabilities(&self) -> Vec<CapabilityReport> {
        Vec::new()
    }
    fn probe_connector_config(&self) -> ConnectorConfig {
        ConnectorConfig::default()
    }
}

/// 模拟设备 (CI / 单元测试; 无硬件/SDK). 模拟世界允许伪造身份证据 (本身是测试世界).
pub struct SimulatedDeviceManager;

impl SimulatedDeviceManager {
    // A2-0: 同 FilesystemDeviceManager——lib 公开面触发 clippy 的最小 allow 处置。
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self
    }
}

impl HardwareProvider for SimulatedDeviceManager {
    fn discover(&self) -> Result<Vec<DiscoveredDevice>, ProviderError> {
        Ok((0..2)
            .map(|i| DiscoveredDevice {
                device: DeviceInfo {
                    device_id: Uuid::new_v5(&VBMF_SIM_NS, format!("vbmf:sim:{i}").as_bytes()),
                    model: "DeckLink Mini Monitor 4K (sim)".into(),
                    display_name: format!("sim-{i}"),
                    serial_number: Some(format!("SIM-SERIAL-{i}")),
                    video_input_connections: 0,
                    video_output_connections: 0,
                    identity_strength: IdentityStrength::PersistentId,
                    identity_source: DeviceIdentitySource::Simulation,
                    capabilities: DeviceCapabilities::default(),
                    ports: Vec::new(),
                },
                identity: Some(ProviderIdentity {
                    provider: "simulation",
                    persistent_id: Some(9000 + i as i64),
                    device_handle: Some(format!("sim-handle-{i}")),
                    topological_id: None,
                }),
            })
            .collect())
    }
    fn probe_capabilities(&self) -> Vec<CapabilityReport> {
        Vec::new()
    }
    fn probe_connector_config(&self) -> ConnectorConfig {
        ConnectorConfig::default()
    }
}

/// 确定性 UUID 命名空间 (避免随机 UUID 导致设备 ID 漂移).
const VBMF_FS_NS: Uuid = Uuid::from_u128(0x9f3b2c1d_4e5a_4b6c_8d7e_0f1a2b3c4d5e);
const VBMF_SIM_NS: Uuid = Uuid::from_u128(0x1a2b3c4d_5e6f_4078_9a0b_1c2d3e4f5a6b);
