//! Phase 0.6 C1 (0.6B): HardwareProvider SPI — canonical Hardware Plane contract.
//!
//! 当前等价于既有 `device::DeviceManager`；作为冻结的 canonical 名称对外暴露。
//! `discover()` 返回 canonical `DeviceInfo`（vendor 身份细节由 Adapter 在内部消化，不外泄）。
pub use crate::device::DeviceManager as HardwareProvider;
