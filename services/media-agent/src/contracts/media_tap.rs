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

/// A2-8-02-G/H（第十四轮终裁）: **Bridge Observation**——tap→inter source
/// 段实际经过数据的**运行观测事实**（runtime observation fact）。
///
/// **与 `MediaTapAttachment` 严格分层**（第十四轮 §5）: attachment=静态
/// 资源/配置事实（recover 重放唯一依据）; 本结构=动态观测事实——
/// 二者混装会破坏 Health Tree/Incident/Timeline 所需事实分层。
///
/// 实测语义: 值来自分流分支上的真实缓冲（pad probe）, **不是**
/// InputObservation/ProgramObservation 的复制——三列各自独立测量。
/// `channel` 为桥接地址（adapter 侧不透明; device 映射归消费方经
/// `tap_channel` join）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BridgeObservation {
    pub channel: String,
    pub video_last_pts: Option<u64>,
    pub audio_last_pts: Option<u64>,
    pub video_pts_state: crate::pipeline::PtsMonotonicity,
    pub audio_pts_state: crate::pipeline::PtsMonotonicity,
    pub video_frames: u64,
    pub audio_frames: u64,
}

/// Bridge 观测查询面——与 `MediaTapPort` 平行（MediaTapPort 契约零改动;
/// 本 trait 为 G/H 新增观测原语, 仍属执行层内部能力, 不进 Session/Domain）。
///
/// SPI 方法在无调用点的 feature 组合下可能未消费; 与既有 SPI 一致在
/// trait 级允许 dead_code。
#[allow(dead_code)]
pub trait BridgeObservationPort: Send + Sync {
    /// 按管线列桥观测（每 channel 一行; 分支已摘除的 channel 不出现=
    /// 无证据——absence≠evidence, 非零值非伪造）。
    fn bridge_observations(&self, handle: &PipelineHandle) -> Vec<BridgeObservation>;
}
