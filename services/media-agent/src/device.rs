//! Device Manager — DeckLink 硬件发现与状态。
//! 接口形状按 SoT §15.2 (MEDIA-02/04) 冻结。Gate 2.2 落地文件系统枚举实现。
#![allow(dead_code)] // Gate 2.x: 部分接口尚未被上层调用, 编译期静音。

use serde::{Deserialize, Serialize};
use std::os::unix::fs::FileTypeExt;
use uuid::Uuid;

/// Blackmagic 设备节点目录(宿主机由 DesktopVideoHelper 创建 dv0/dv1/io0)。
pub const BLACKMAGIC_DEV_DIR: &str = "/dev/blackmagic";

/// 一个被发现的 DeckLink 设备(对应 /dev/blackmagic/dv0, dv1, io0)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: Uuid,
    pub node: String,        // e.g. "/dev/blackmagic/dv0"
    pub model: String,       // e.g. "DeckLink Quad HDMI Recorder"
    pub serial: String,
    pub state: DeviceState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceState {
    Available,
    Leased,
    Capturing,
    Error,
    /// MEDIA-04: 设备消失(线缆拔出 / 驱动重启)
    Lost,
}

/// 发现 + 状态接口(形状冻结, 见 MEDIA_AGENT_STATE_MACHINE.md)。
pub trait DeviceManager {
    /// 通过 DesktopVideoHelper / Blackmagic SDK 枚举 DeckLink 设备。
    /// 约定: 枚举失败(如无设备节点)返回空 vec, 不抛错 —— 对应状态机
    /// "0 设备 → DEGRADED"。
    fn discover(&self) -> Vec<DeviceInfo>;
    /// 设备实时状态(健康 / 信号存在性)。
    fn status(&self, device_id: &Uuid) -> DeviceState;
}

/// MEDIA-04: 热插拔事件通道(udev / DesktopVideoHelper IPC)。
#[derive(Debug, Clone)]
pub enum HotplugEvent {
    Attached(DeviceInfo),
    Detached(Uuid),
}

/// Gate 2.2 实现: 纯文件系统枚举 `/dev/blackmagic/*` 节点。
///
/// 这是发现的最小可用实现 —— 不依赖 DeckLink SDK, 因此在无 SDK 的 CI
/// 环境与无设备的 runner 上也能编译并安全返回空。真实 model/serial 的
/// 深度枚举(调用 libDeckLinkAPI)作为后续增量, 必须在 BMD runc 容器内实测。
pub struct FilesystemDeviceManager {
    pub dev_dir: String,
}

impl FilesystemDeviceManager {
    pub fn new() -> Self {
        Self { dev_dir: BLACKMAGIC_DEV_DIR.to_string() }
    }

    pub fn with_dir(dev_dir: impl Into<String>) -> Self {
        Self { dev_dir: dev_dir.into() }
    }

    /// 从节点路径派生确定性 device_id(同节点跨重启稳定, 便于租约关联)。
    fn node_id(node: &str) -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_OID, node.as_bytes())
    }
}

impl Default for FilesystemDeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceManager for FilesystemDeviceManager {
    fn discover(&self) -> Vec<DeviceInfo> {
        let mut devices = Vec::new();
        // 目录不存在(无 DeckLink / 非 BMD)→ 返回空, 对应状态机 0 设备分支。
        let Ok(dir) = std::fs::read_dir(&self.dev_dir) else {
            return devices;
        };
        for entry in dir.flatten() {
            let path = entry.path();
            // DeckLink 节点是字符设备(crw, e.g. dv0/dv1/io0, 主设备号 10),
            // 不是目录 —— 实测 BMD 上 /dev/blackmagic/{dv0,dv1,io0} 均为 crw。
            // 只认字符设备, 排除可能的普通文件/目录噪音。
            match entry.file_type() {
                Ok(ft) if ft.is_char_device() => {}
                _ => continue,
            }
            let node = path.to_string_lossy().to_string();
            devices.push(DeviceInfo {
                device_id: Self::node_id(&node),
                node,
                model: "filesystem-probe".to_string(),
                serial: String::new(),
                state: DeviceState::Available,
            });
        }
        devices
    }

    fn status(&self, _device_id: &Uuid) -> DeviceState {
        // Gate 2.2: 文件系统枚举不持有实时信号状态, 默认 Available。
        // 真实信号/健康探测在 SDK 深度枚举落地后补。
        DeviceState::Available
    }
}

/// Gate 5/7 无硬件单测用的模拟设备源 (`simulation` feature)。
/// 让 CI / `cargo test --features simulation` 在没有 BMD 与 SDK 的情况下也能跑通
/// discovery → lease → supervisor 全链路, 无需真实 DeckLink。
#[cfg(feature = "simulation")]
pub struct SimulatedDeviceManager;

#[cfg(feature = "simulation")]
impl SimulatedDeviceManager {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "simulation")]
impl Default for SimulatedDeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "simulation")]
impl DeviceManager for SimulatedDeviceManager {
    fn discover(&self) -> Vec<DeviceInfo> {
        (0..3)
            .map(|i| DeviceInfo {
                device_id: Uuid::new_v4(),
                node: format!("/dev/blackmagic/dv{i}"),
                model: format!("Simulated DeckLink {i}"),
                serial: format!("SIM{:06}", i),
                state: DeviceState::Available,
            })
            .collect()
    }

    fn status(&self, _device_id: &Uuid) -> DeviceState {
        DeviceState::Available
    }
}
