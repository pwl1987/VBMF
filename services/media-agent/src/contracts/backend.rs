//! Phase 0.6 C2 (0.6C): MediaBackend SPI — canonical Media Runtime contract.
//!
//! **C2 状态：独立的 `trait MediaBackend`**（替代 C1 的 transitional alias `PipelineController`）。
//! 由 `pipeline::GStreamerPipelineController` 实现（物理迁移到 `adapters/gstreamer` 留 C6/C7）。
//!
//! **解耦修复（A 批，2026-08-28）**：Backend 与 Provider 是正交替换轴，Backend SPI 不耦合 BMD Provider。
//! `MediaBackend` 仅在 `gstreamer-backend` 下定义（与 C1 A 批别名门控一致）：无后端时该契约本就不存在。
#[cfg(feature = "gstreamer-backend")]
use crate::pipeline::{PipelineBusEvent, PipelineError, PipelineHandle, PipelinePlan};

/// Media Runtime 契约：从同一 `PipelinePlan` 物化并管理管线生命周期。
///
/// `Send + Sync` 以便跨运行时线程（Supervisor / watchdog）持有。
#[cfg(feature = "gstreamer-backend")]
pub trait MediaBackend: Send + Sync {
    fn prepare(&self, plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError>;
    fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    // `poll_bus` 当前经 `GStreamerPipelineController` 固有方法被 main 调用（C2c 迁移后改走 trait）；
    // trait 方法在 C3 Mock / C2c 才被消费，属冻结 SPI 形状，允许 dead_code。
    #[allow(dead_code)]
    fn poll_bus(&self, handle: &PipelineHandle) -> Vec<PipelineBusEvent>;
}
