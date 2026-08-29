//! Phase 0.6 C2 (0.6B) + Final Merge Hardening (P0-1/P1-2): HardwareProvider SPI.
//!
//! **hardening 变更（BREAKING，一次付清）**：
//! - `discover -> Result<Vec<DiscoveredDevice>, ProviderError>`（P1-2，闭环 frozen 契约
//!   `HARDWARE_PROVIDER_CONTRACT.md` 的 fail-closed 语义；SDK/驱动失败不再与"无硬件=Ok(空)"混淆；
//!   旧口径"返回 Vec 的偏差（C2 审计注记）"就此解除）。
//! - Provider 身份证据离开 Domain：`DeviceInfo` 不再携带 vendor 身份字段；
//!   证据以 `ProviderIdentity`（provider 标签 + 机制中立字段）随 `DiscoveredDevice` 配对输出，
//!   仅由 Provider Identity Adapter（resolver/绑定路径）消费（CANONICAL_IDENTITY §4：
//!   Provider Identity ≠ Canonical Identity；`(provider, provider_identity)` 二元组收敛为 DeviceId）。
//!
//! 由 Blackmagic / Filesystem / Simulation / Mock 四套 Reference Adapter 实现。
use crate::device::DeviceInfo;

/// Provider 侧身份证据（SPI 层；机制中立承载，字段名不冠 vendor 专名）。
///
/// Domain/Graph/UI schema 不得出现此类型（CANONICAL_IDENTITY §4）；
/// 仅 resolver（Provider Identity Adapter）/ 绑定 / 诊断路径消费。
/// 各 Provider **自行定义**其内部证据优先级，本结构只做承载，不做跨 Provider 解释。
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ProviderIdentity {
    /// Provider 标识（如 `"blackmagic"` / `"filesystem"` / `"simulation"` / `"mock"`）。
    pub provider: &'static str,
    /// Provider 本地持久标识（机制中立；当前硬件 BMD 不支持 → 多为 `None`）。
    pub persistent_id: Option<i64>,
    /// Provider 本地绑定引用（当前硬件 = BMD `GetString('devh')`；清单交叉核验键）。
    pub device_handle: Option<String>,
    /// Provider 本地拓扑标识（拓扑敏感，重启/拓扑变化漂移）。
    pub topological_id: Option<i64>,
}

/// 单个发现结果：canonical `DeviceInfo`（身份已由 Provider 内部收敛为 `device_id`）
/// + Provider 侧身份证据（供 resolver/绑定/诊断消费；Domain 不解释）。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiscoveredDevice {
    pub device: DeviceInfo,
    pub identity: Option<ProviderIdentity>,
}

/// Provider 发现失败分类（fail-closed：与"无硬件 = `Ok(vec![])`"严格区分）。
/// `#[allow(dead_code)]`: 冻结 SPI 分类形状 — 全部分类由契约定义, 当前硬件仅触发部分。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ProviderErrorKind {
    /// SDK / 运行库不可用（未安装、dlopen 失败等）。
    SdkUnavailable,
    /// SDK 可用但枚举/驱动调用失败。
    DriverFailure,
    /// 权限不足（设备节点不可读等）。
    PermissionDenied,
    /// Provider 初始化失败（其余未分类错误）。
    InitFailed,
}

/// Provider 发现/探针错误。
#[derive(Debug, Clone)]
pub struct ProviderError {
    pub kind: ProviderErrorKind,
    pub detail: String,
}

impl ProviderError {
    #[allow(dead_code)] // 冻结 SPI 构造形状; 部分组合暂无触发点 (同 ProviderErrorKind)
    pub fn new(kind: ProviderErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.detail)
    }
}

impl std::error::Error for ProviderError {}

/// C2 引入的 canonical 能力报告（占位形状；真实 SDK 能力探针归并留 0.7）。
///
/// 当前实现返回空，仅用于冻结 SPI 形状，避免后续 Mock / 真实 Adapter 反复改签名。
/// 字段暂未被消费，故允许 dead_code。
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct CapabilityReport {
    /// 能力来源标识（如 `"blackmagic-simulation"` / `"filesystem"`）。
    pub source: String,
    /// 已探明的具名能力（占位，当前恒空）。
    pub items: Vec<String>,
}

/// C2 引入的 connector 配置探针结果（占位形状；真实端口闭环接 `hw_port_01`）。
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ConnectorConfig {
    /// 已探明的 connector 名称列表（占位，当前恒空）。
    pub connectors: Vec<String>,
}

/// Hardware Plane 契约：枚举硬件并暴露能力/连接配置探针。
///
/// `Send + Sync` 以便跨运行时线程（Supervisor / watchdog）持有。
///
/// `probe_capabilities` / `probe_connector_config` 当前无调用方，属冻结 SPI 形状，允许 dead_code。
#[allow(dead_code)]
pub trait HardwareProvider: Send + Sync {
    /// 枚举硬件：canonical `DeviceInfo`（`device_id` 已由 Provider 内部经自身身份证据收敛）
    /// + Provider 侧身份证据配对输出。
    ///
    /// **fail-closed**：SDK/驱动/权限失败必须 `Err(ProviderError)`，绝不与
    /// "无硬件 = `Ok(vec![])`" 混淆（P1-2，修正 frozen 契约的 discover 语义）。
    fn discover(&self) -> Result<Vec<DiscoveredDevice>, ProviderError>;
    /// SDK 能力探针（仅 Reference Adapter 实现；返回 canonical 能力报告）。
    fn probe_capabilities(&self) -> Vec<CapabilityReport>;
    /// 连接配置探针（diagnostic / 端口闭环）。
    fn probe_connector_config(&self) -> ConnectorConfig;
}
