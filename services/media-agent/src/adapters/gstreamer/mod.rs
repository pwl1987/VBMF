//! GStreamer Reference Backend.
//!
//! 仅此模块引用 vendor `gstreamer` crate 顶层。
// 模块门面 re-export. 在 `bmd-provider,gstreamer-backend,mock` 组合下, main 经
// `dyn MediaBackend` 接线会优先选用 MockBackend, 导致 GStreamerPipelineController 不被引用;
// 此 re-export 在该组合下看似未用, 但属架构门面 (Concrete Adapters 公共 API), 故允许.
// `controller` 子模块无条件声明: 其中仅保留 GStreamer 后端实现 (`GStreamerPipelineController` 及
// gstreamer 相关 import, 均为 gated). 共享事件/健康类型 (`PipelineBusEvent`/`HEALTH_ARCS` 等) 与
// 全局健康表已迁至中性模块 `pipeline_events.rs` (不依赖 vendor `gstreamer` crate), 在
// default/simulation/mock 等无 gstreamer 构建下也须编译, 以保证 `crate::pipeline::*` 契约
// (contracts/backend.rs、mock.rs、main.rs 经其引用) 可用.
#[allow(unused_imports)]
pub(crate) mod controller;
#[cfg(feature = "gstreamer-backend")]
#[allow(unused_imports)]
pub use controller::GStreamerPipelineController;

/// 运行时 GStreamer 版本 (证据归档用). 仅在其唯一消费者 `main` 的 evidence 日志
/// (`#[cfg(feature = "bmd-provider")]` 块内) 编译时才被引用, 故收紧到
/// `all(gstreamer-backend, bmd-provider)`, 避免 `gstreamer-backend` 单独组合下的 dead-code。
#[cfg(all(feature = "gstreamer-backend", feature = "bmd-provider"))]
pub fn gstreamer_runtime_version() -> (u32, u32, u32, u32) {
    gstreamer::version()
}
