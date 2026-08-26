//! Device Manager — DeckLink 硬件发现与状态。
//! 接口形状按 SoT §15.2 (MEDIA-02/04) 冻结。Gate 2.2 落地文件系统枚举实现。
#![allow(dead_code)] // Gate 2.x: 部分接口尚未被上层调用, 编译期静音。

use serde::{Deserialize, Serialize};
use std::os::unix::fs::FileTypeExt;
use tracing::warn;
use uuid::Uuid;

/// Blackmagic 设备节点目录(宿主机由 DesktopVideoHelper 创建 dv0/dv1/io0)。
pub const BLACKMAGIC_DEV_DIR: &str = "/dev/blackmagic";

/// **诊断交叉校验用** (非 canonical 身份来源): 从 BMD `DeviceHandle` 字符串
/// (`RevisionID:PersistentID:TopologicalID`) 提取 `PersistentID` 中段 (hex).
/// 解析失败返回 `None`. 仅用于与 SDK 权威 `GetInt(BMDDeckLinkPersistentID)` 做一致性比对,
/// **不得**作为 `bmd_persistent_id` 的唯一来源 (否则会重蹈"中段猜 PersistentID"的脆弱解析).
pub fn parse_persistent_id(handle: &str) -> Option<u32> {
    let seg: Vec<&str> = handle.split(':').collect();
    if seg.len() == 3 {
        u32::from_str_radix(seg[1], 16).ok()
    } else {
        None
    }
}

/// 一个被发现的 DeckLink 设备(对应 /dev/blackmagic/dv0, dv1, io0)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: Uuid,
    pub node: String,        // e.g. "/dev/blackmagic/dv0"
    pub model: String,       // e.g. "DeckLink Quad HDMI Recorder"
    pub serial: String,      // BMD DeviceHandle 字符串 (RevisionID:PersistentID:TopologicalID)
    pub state: DeviceState,
    /// BMD 硬件持久身份 (BMDDeckLinkPersistentID), **权威来源 = SDK `GetInt(BMDDeckLinkPersistentID)`**.
    /// `None` = SDK 未提供 / 解析失败 ⇒ 身份未解析 (生产路径 `IdentityUnresolved`, 绝不盲开).
    /// `Some(0)` 是**合法零值**, 与 `None` 严格区分, 不应被误判为"解析失败".
    /// 对应 GStreamer `decklinkvideosrc`/`decklinkaudiosrc` 的 `persistent-id` (gint64) 属性;
    /// 1.22+ 官方支持, 优先级高于 `device-number`.
    pub bmd_persistent_id: Option<u32>,
    /// 完整硬件身份字符串 (DeviceHandle: RevisionID:PersistentID:TopologicalID), 用于诊断 / inventory /
    /// 拓扑变化记录. `None` 表示无 SDK 来源 (如 filesystem / simulation 占位).
    pub bmd_device_handle: Option<String>,
    /// **诊断交叉校验**: 由 DeviceHandle 中段解析得到的 PersistentID. 与 `bmd_persistent_id` (SDK 权威)
    /// 独立; 二者均 `Some` 且不相等 ⇒ 身份来源冲突, 需人工核查 (不静默择一).
    pub handle_persistent_id: Option<u32>,
    /// GStreamer `device-number` 属性 (回退 / 诊断). 枚举索引, 与 bmd_persistent_id
    /// 指向同一块已解析设备.
    pub device_number: u32,
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
        for (idx, entry) in dir.flatten().enumerate() {
            let path = entry.path();
            // DeckLink 节点是字符设备(crw, e.g. dv0/dv1/io0, 主设备号 10),
            // 不是目录 —— 实测 BMD 上 /dev/blackmagic/{dv0,dv1,io0} 均为 crw。
            // 只认字符设备, 排除可能的普通文件/目录噪音。
            match entry.file_type() {
                Ok(ft) if ft.is_char_device() => {}
                _ => continue,
            }
            let node = path.to_string_lossy().to_string();
            // 文件系统枚举无法读取真实 BMD 身份, 用节点名派生稳定的占位身份
            // (仅 CI / 无 SDK 环境; 真实 persistent-id 来自 SDK 深度枚举).
            let ph: u32 = node
                .bytes()
                .fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
            devices.push(DeviceInfo {
                device_id: Self::node_id(&node),
                node: node.clone(),
                model: "filesystem-probe".to_string(),
                serial: String::new(),
                state: DeviceState::Available,
                // 文件系统枚举无 SDK, 用节点名派生占位身份以保非硬件构建可运行;
                // 真实 BMD PersistentID 仅来自 DeckLinkDeviceManager (SDK 权威读取).
                bmd_persistent_id: Some(ph),
                bmd_device_handle: None,
                handle_persistent_id: None,
                device_number: idx as u32,
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
                bmd_persistent_id: Some((i as u32) + 1),
                bmd_device_handle: Some(format!("sim-{i}")),
                handle_persistent_id: None,
                device_number: i as u32,
            })
            .collect()
    }

    fn status(&self, _device_id: &Uuid) -> DeviceState {
        DeviceState::Available
    }
}

/// Gate 2.6 (P1①): 真实 SDK 深度枚举 —— 以 DeckLink 设备唯一标识 (DeviceHandle) 作为
/// **canonical device identity**。SDK 枚举返回的 `serial` 即 `BMDDeckLinkDeviceHandle`
/// (官方手册 3.17 的 unique identifier), 由其派生确定性 `device_id`, 取代原先
/// "filesystem 节点 index 与 SDK 枚举 index 按位置合并" 的不稳定做法 (拓扑变化后 index 会变)。
/// 仅 `bmd`/`hardware-test` feature 下可用 (需要 libDeckLinkAPI.so + 真机)。
#[cfg(feature = "bmd")]
pub struct DeckLinkDeviceManager;

#[cfg(feature = "bmd")]
impl DeckLinkDeviceManager {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "bmd")]
impl Default for DeckLinkDeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "bmd")]
impl DeviceManager for DeckLinkDeviceManager {
    fn discover(&self) -> Vec<DeviceInfo> {
        match crate::decklink::enumerate() {
            Ok(list) => list
                .into_iter()
                .enumerate()
                .map(|(idx, (model, display, serial, pid))| {
                    // canonical identity: 由 DeckLink DeviceHandle (serial 字段) 派生,
                    // 跨进程/跨重启对同一个物理设备稳定, 且不受设备拓扑顺序影响。
                    let device_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, serial.as_bytes());
                    // 诊断交叉校验: 从 DeviceHandle 中段解析 PersistentID (仅用于一致性比对).
                    let handle_pid = parse_persistent_id(&serial);
                    // **权威** BMD PersistentID 来自 SDK `GetInt(BMDDeckLinkPersistentID)`
                    // (enumerate 已读取), 不再用 DeviceHandle 中段猜测。
                    // None = 未解析 ⇒ 生产路径将 IdentityUnresolved; Some(0) 是合法零值, 正常接受。
                    if let (Some(sdk), Some(handle)) = (pid, handle_pid) {
                        if sdk != handle {
                            warn!(
                                model = %model,
                                sdk_persistent_id = sdk,
                                handle_persistent_id = handle,
                                "BMD 身份来源冲突: SDK PersistentID 与 DeviceHandle 中段不一致, 需人工核查"
                            );
                        }
                    }
                    DeviceInfo {
                        device_id,
                        node: display.clone(), // 真实显示名作为可读节点描述
                        model,
                        serial: serial.clone(),
                        state: DeviceState::Available,
                        bmd_persistent_id: pid,
                        bmd_device_handle: Some(serial.clone()),
                        handle_persistent_id: handle_pid,
                        device_number: idx as u32,
                    }
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    fn status(&self, _device_id: &Uuid) -> DeviceState {
        DeviceState::Available
    }
}
