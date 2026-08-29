//! Phase 0.6 C2 (0.6C) + Final Merge Hardening (P0-2/P0-3): MediaBackend SPI — canonical Media Runtime contract.
//!
//! **hardening 变更（BREAKING，一次付清）**：
//! - **P0-3**：trait 无条件编译（不再被 `gstreamer-backend`/`mock` feature 门控）——
//!   契约属于 `contracts` 层，default（无任何 backend 实现）下依然存在；
//!   只有 concrete `impl` 块保留各自 feature 门控（对齐 `MEDIA_BACKEND_CONTRACT.md` §4 精神：
//!   Domain/Runtime Contract 在无后端构建下仍可编译）。
//! - **P0-2**：方法形状对齐冻结契约 `MEDIA_BACKEND_CONTRACT.md` §1：
//!   `prepare→instantiate`、`poll_bus→observe`（载荷已是 vendor-neutral `PipelineBusEvent`，
//!   非 GStreamer Bus Message 语义）、补 `stop`（此前冻结契约有而实现缺失）。
//!   错误类型复用 `PipelineError`（不引入 `BackendError`，避免二次 breaking；契约文档已加对齐注记）。
//!
//! **解耦修复（A 批，2026-08-28）**：Backend 与 Provider 是正交替换轴，Backend SPI 不耦合 BMD Provider。
use crate::pipeline::{PipelineError, PipelineHandle, PipelinePlan};
use crate::pipeline_events::PipelineBusEvent;

/// Media Runtime 契约：从同一 `PipelinePlan` 物化并管理管线生命周期。
///
/// `Send + Sync` 以便跨运行时线程（Supervisor / watchdog）持有。
// SPI 方法在无调用点的组合下可能未消费; 与 HardwareProvider 一致在 trait 级允许 dead_code.
#[allow(dead_code)]
pub trait MediaBackend: Send + Sync {
    /// 从 canonical `PipelinePlan` 物化管线实例（契约: `instantiate`）。
    fn instantiate(&self, plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError>;
    fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    /// 停止并释放管线实例（契约: `stop`；hardening P0-2 补齐）。
    fn stop(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    /// 观测运行时事件（契约: `observe`；vendor-neutral 统一事件载荷, 非 Bus Message 语义）。
    fn observe(&self, handle: &PipelineHandle) -> Vec<PipelineBusEvent>;
}
