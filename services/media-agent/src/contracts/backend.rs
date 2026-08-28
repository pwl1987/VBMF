//! Phase 0.6 C1 (0.6C): MediaBackend SPI — canonical Media Runtime contract.
//!
//! 当前等价于既有 `pipeline::PipelineController`；作为冻结的 canonical 名称对外暴露。
//! GStreamer / Mock 等 Backend 实现同一 trait，从同一 `PipelinePlan` 物化。
#[cfg(all(feature = "bmd-provider", feature = "gstreamer-backend"))]
pub use crate::pipeline::PipelineController as MediaBackend;
