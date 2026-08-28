//! Phase 0.6 C2 (0.6B): HardwareProvider SPI — canonical Hardware Plane contract.
//!
//! **C2 状态：独立的 `trait HardwareProvider`**（替代 C1 的 transitional alias `DeviceManager`）。
//! 由 `device::*DeviceManager` 实现（Blackmagic / Filesystem / Simulation 三套发现逻辑）。
//!
//! **与 design.md §3 的偏差（已对齐审计）**：
//! `discover()` 当前返回 `Vec<DeviceInfo>`（**不是** `Result<Vec<DeviceInfo>, ProviderError>`），
//! 因为 `main.rs` 在 C2c 之前仍按 `Vec` 消费，且 vendor 错误类型统一到 `RuntimeEvent` 留 0.6D。
//! `probe_capabilities` / `probe_connector_config` 已就位；真实 SDK 能力/端口探针回填留 C5/C...。
use crate::device::DeviceInfo;

/// C2 引入的 canonical 能力报告（占位形状；真实 SDK 能力探针归并留 0.6D/C5）。
///
/// 当前实现返回空，仅用于冻结 SPI 形状，避免后续 Mock / 真实 Adapter 反复改签名。
/// 字段暂未被消费（真实 SDK 能力探针回填留 C5/C...），故允许 dead_code。
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct CapabilityReport {
    /// 能力来源标识（如 `"blackmagic-simulation"` / `"filesystem"`）。
    pub source: String,
    /// 已探明的具名能力（占位，当前恒空）。
    pub items: Vec<String>,
}

/// C2 引入的 connector 配置探针结果（占位形状；真实端口闭环接 `hw_port_01` 留 C...）。
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
/// `probe_capabilities` / `probe_connector_config` 当前无调用方，将在 C2c（main 迁移）/
/// C3（Mock）/ C5（真实 SDK 探针）才被消费；属冻结 SPI 形状，故允许 dead_code。
#[allow(dead_code)]
pub trait HardwareProvider: Send + Sync {
    /// 枚举硬件并解析为 canonical `DeviceInfo`（BMD 身份细节在 Adapter 内消化，不外泄）。
    fn discover(&self) -> Vec<DeviceInfo>;
    /// SDK 能力探针（仅 Reference Adapter 实现；返回 canonical 能力报告）。
    fn probe_capabilities(&self) -> Vec<CapabilityReport>;
    /// 连接配置探针（diagnostic / 端口闭环）。
    fn probe_connector_config(&self) -> ConnectorConfig;
}
