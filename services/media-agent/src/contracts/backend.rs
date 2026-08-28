//! Phase 0.6 C1 (0.6C): MediaBackend SPI — canonical Media Runtime contract.
//!
//! **C1 状态：transitional alias（Strangler 过渡层），不是最终 Backend SPI。**
//! 当前等价于既有 `pipeline::PipelineController`；作为冻结的 canonical 名称对外暴露，
//! 让调用方先改用 canonical 名。
//!
//! **解耦修复（A 批，2026-08-28）**：`MediaBackend` 仅由 `gstreamer-backend` 门控，
//! 不再依赖 `bmd-provider` —— Backend 与 Provider 是正交替换轴，Backend SPI 不应耦合 BMD Provider。
//!
//! **C2 目标契约**（见 design.md §3）：独立的 `trait MediaBackend`
//! (`prepare`/`start`/`recover`/`poll_bus`)，由 `GStreamerMediaBackend` / `MockMediaBackend`
//! 分别实现，从同一 `PipelinePlan` 物化。
#[cfg(feature = "gstreamer-backend")]
pub use crate::pipeline::PipelineController as MediaBackend;
