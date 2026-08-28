//! GStreamer Reference Backend.
//!
//! 仅此模块引用 vendor `gstreamer` crate 顶层。
// 模块门面 re-export. 在 `bmd-provider,gstreamer-backend,mock` 组合下, main 经
// `dyn MediaBackend` 接线会优先选用 MockBackend, 导致 GStreamerPipelineController 不被引用;
// 此 re-export 在该组合下看似未用, 但属架构门面 (Concrete Adapters 公共 API), 故允许.
#[cfg(feature = "gstreamer-backend")]
#[allow(unused_imports)]
pub use crate::pipeline::GStreamerPipelineController;

/// 运行时 GStreamer 版本 (证据归档用). 仅在 gstreamer 构建可用.
#[cfg(feature = "gstreamer-backend")]
pub fn gstreamer_runtime_version() -> (u32, u32, u32, u32) {
    gstreamer::version()
}
