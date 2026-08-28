//! Phase 0.6 C1 (0.6B): HardwareProvider SPI — canonical Hardware Plane contract.
//!
//! **C1 状态：transitional alias（Strangler 过渡层），不是最终 HardwareProvider SPI。**
//! 当前等价于既有 `device::DeviceManager`；作为冻结的 canonical 名称对外暴露。
//! `discover()` 返回 canonical `DeviceInfo`（vendor 身份细节由 Adapter 在内部消化，不外泄）。
//!
//! **C2 目标契约**（见 design.md §3）：独立的 `trait HardwareProvider`。
//! 它应提供 `discover() -> Result<Vec<DeviceInfo>, ProviderError>`、`probe_capabilities()`、`probe_connector_config()`，
//! 并由 `BlackmagicHardwareProvider` / `MockHardwareProvider` / `FilesystemHardwareProvider` 分别实现。
//! C1 暂不引入 `ProviderError`/`CapabilityReport`/`ConnectorConfig`（统一到 RuntimeEvent 留 0.6D）。
pub use crate::device::DeviceManager as HardwareProvider;
