//! GStreamer Reference Backend.
//!
//! 仅此模块引用 vendor `gstreamer` crate 顶层。
#[cfg(feature = "gstreamer-backend")]
pub use crate::pipeline::GStreamerPipelineController;

/// 运行时 GStreamer 版本 (证据归档用). 仅在 gstreamer 构建可用.
#[cfg(feature = "gstreamer-backend")]
pub fn gstreamer_runtime_version() -> (u32, u32, u32, u32) {
    gstreamer::version()
}
