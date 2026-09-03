//! A2-8-02: Generic MediaTap —— 管线**通用媒体分流能力**契约（第五轮终裁
//! 方向 A, probe §10.2）。
//!
//! **语义边界（硬约束）**: tap 只知"要该管线的 video/audio 媒体输出",
//! **不知 Program/Switch/ExecutionGroup/active source**——契约面零
//! Program 词汇。`channel` 为调用方（Program Execution 层）从设备标识
//! 派生的不透明字符串, 本契约不理解其构成。
//!
//! **与 identity 的关系**: `MediaTapAttachment` = **execution resource
//! attachment bookkeeping**（哪条管线挂着哪些 tap——供 recover 重建后
//! 重放 attach 的唯一事实源）, **不是新 Device Identity Registry**
//! （第五轮终裁 §10.3 明确区分）。
//!
//! **C2 裁定的实现形式**: 禁止在 recover 内"裸调 attach"——恢复以
//! `tap_attachments()` 簿记为唯一事实源重放。

use crate::pipeline::PipelineHandle;

/// 分流平面（video/audio/both——成对语义由调用方表达, 契约不解释）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TapPlanes {
    Video,
    Audio,
    Both,
}

/// tap 请求（attach 入参）。`channel` = 不透明通道标识（调用方从设备
/// 身份派生; inter 物化按 channel 值桥接 sink/src）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MediaTapRequest {
    pub channel: String,
    pub planes: TapPlanes,
}

/// 已挂 tap 的簿记行（execution resource bookkeeping——recover 重放
/// attach 的唯一事实源; 非第二 identity registry）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MediaTapAttachment {
    pub channel: String,
    pub planes: TapPlanes,
}

/// MediaTap 封闭错误词表（fail-closed 全可观测）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TapError {
    #[error("unknown pipeline handle {0:?}")]
    UnknownPipeline(PipelineHandle),
    #[error("media tap point unavailable on this pipeline form: {0}")]
    TapPointUnavailable(String),
    #[error("tap channel {0} already attached")]
    AlreadyAttached(String),
    #[error("tap channel {0} not attached")]
    NotAttached(String),
    #[error("media tap backend error: {0}")]
    Backend(String),
}

/// Generic MediaTap 能力——与 `MediaBackend` 生命周期五方法、
/// `SwitchExecutionAdapter` 平行的执行面 SPI（第五轮终裁批准）。
///
/// SPI 方法在无调用点的 feature 组合下可能未消费; 与既有 SPI 一致在
/// trait 级允许 dead_code。
#[allow(dead_code)]
pub trait MediaTapPort: Send + Sync {
    /// 为已运行（或已构造）的管线挂通用媒体分流。幂等约束: 同 channel
    /// 重复 attach → `AlreadyAttached`（fail-closed, 不静默重定义）。
    fn attach_media_tap(
        &self,
        handle: &PipelineHandle,
        req: &MediaTapRequest,
    ) -> Result<(), TapError>;

    /// 摘除指定 channel 的分流。
    fn detach_media_tap(&self, handle: &PipelineHandle, channel: &str) -> Result<(), TapError>;

    /// 簿记查询（恢复重放/观测的唯一事实源——按管线列已挂 tap）。
    fn tap_attachments(&self, handle: &PipelineHandle) -> Vec<MediaTapAttachment>;
}
