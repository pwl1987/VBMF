//! BMD DeckLink 真实设备发现 (Reference Adapter, feature `bmd-provider`).
//!
//! 本模块属于 Concrete Adapters 层; Domain (`device.rs`) 不引用此处,
//! 满足 ARCH-PORTABILITY-01 Test A (删 BMD Provider 后 Domain 仍可编译).

use uuid::Uuid;

use crate::adapters::blackmagic::decklink::{enumerate, BmdDeviceIdentity};
use crate::contracts::provider::{CapabilityReport, ConnectorConfig, HardwareProvider};
use crate::device::{DeviceIdentitySource, DeviceInfo, DeviceManager, IdentityStrength};
use crate::port::DeviceCapabilities;

/// 真实 BMD DeckLink (feature `bmd`): 基于 DeviceHandle 派生 canonical 身份.
/// 身份来源 = RealBmd; 强度由 SDK 属性可用性决定 (本硬件 → DeviceHandle).
pub struct DeckLinkDeviceManager;

impl DeckLinkDeviceManager {
    // 在 `bmd-provider,gstreamer-backend,mock` 组合下, main 优先选用 MockProvider,
    // 本构造函数不被引用; 属 mock 优先接线的预期副作用, 故允许.
    #[allow(dead_code)]
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
        let discovered = match enumerate() {
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
const VBMF_BMD_NS: Uuid = Uuid::from_u128(0x2b3c4d5e_6f70_4180_ab0c_2d3e4f5a6b7c);

/// C2: HardwareProvider SPI 实现（复用既有 `DeviceManager` 发现逻辑）.
impl HardwareProvider for DeckLinkDeviceManager {
    fn discover(&self) -> Vec<DeviceInfo> {
        <Self as DeviceManager>::discover(self)
    }
    fn probe_capabilities(&self) -> Vec<CapabilityReport> {
        Vec::new()
    }
    fn probe_connector_config(&self) -> ConnectorConfig {
        ConnectorConfig::default()
    }
}
