//! Media Pipeline Lifecycle Orchestrator (PipelineController).
//!
//! Frozen interface per SoT §15.2 / Phase 0.6 canonical-ingest contract.
//! 当前为 Gate 2.1 骨架: 接口冻结, 未真正 launch GStreamer.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::graph_intent::GraphRuntimeIntent;

/// VBMF canonical 设备身份 = DeckLink `DeviceHandle` 派生 UUID
/// (见 `device.rs` `DeckLinkDeviceManager`, UUIDv5(serial)).
/// 跨进程/重启/拓扑变化稳定; 与 GStreamer `device-number` 索引解耦.
pub type CanonicalDeviceId = String;

/// Media Agent 在运行时对 `GraphRuntimeIntent` 的物化执行计划.
///
/// 关键边界 (Phase 0.6 锁死):
///   * `PipelinePlan` **不是** 第二套 Graph Model; 它是 `GraphRuntimeIntent`
///     的物化 (materialization), 仅描述 "如何启动一条真实 GStreamer 管线".
///   * 唯一 canonical 媒体采集通道 = GStreamer `decklinkvideosrc` +
///     `decklinkaudiosrc`. `IDeckLinkInput` (Rust FFI) 仅用于 Device
///     Capability / 模式探测 / 诊断, **不得**作为生产视频数据通道 (否则双采 /
///     设备争用).
///   * GStreamer decklink 插件**没有 `persistent-id` 属性**; 设备选择只接受
///     `device-number` (官方 decklinkvideosrc 文档). 因此 canonical identity
///     必须在物化阶段**解析为 GStreamer `device-number`** (见 `materialize`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelinePlan {
    pub source: SourcePlan,
    pub video: VideoPlan,
    pub audio: AudioPlan,
    pub switch: SwitchPlan,
    pub output: OutputPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcePlan {
    /// VBMF canonical identity (DeviceHandle 派生 UUID). 主键.
    pub persistent_id: CanonicalDeviceId,
    /// GStreamer `decklinkvideosrc`/`decklinkaudiosrc` 的 `device-number`
    /// 属性; 由 `materialize` 把 canonical identity 映射而来.
    /// GStreamer 唯一接受的硬件选择方式 (无 persistent-id 属性).
    pub device_number: u32,
    /// 模式协商后的输入契约 (Capability Match 产物).
    pub resolved_input: ResolvedInputContract,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedInputContract {
    pub mode: String,         // e.g. "1080i50"
    pub pixel_format: String, // e.g. "UYVY" (8-bit YUV)
    pub fps: f64,
    pub interlace: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoPlan {
    /// RAW → RAW (Signal Contract / clock / PTS). 在 Encode 之前, 切换之内.
    pub normalize: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioPlan {
    pub enabled: bool,
    pub channels: u32,    // 2 / 8 / 16
    pub sample_rate: u32, // 48000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchPlan {
    /// FRAME_SWITCH | MASTER_SWITCH. 决策由 Graph Compiler 给出, Media Agent 执行.
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPlan {
    /// 媒体出口 (SRS 负责 RTMP/HLS/WHEP 分发; 非 Encoder).
    pub sink: String,
}

/// 管线句柄 (物化后由 `prepare` 返回, 供 start/stop/recover 引用).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineHandle(pub Uuid);

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("device lease invalid")]
    LeaseInvalid,
    #[error("identity resolution failed: {0}")]
    IdentityUnresolved(String),
    #[error("gstreamer launch not implemented (Gate 2.1 skeleton)")]
    NotImplemented,
}

/// Media Agent = 媒体运行时生命周期 owner: 创建/配置/启动/停止/恢复 GStreamer.
/// **不**重新实现 `IDeckLinkInput` 帧搬运.
pub trait PipelineController {
    /// 物化 GraphRuntimeIntent → PipelinePlan, 校验 Device Lease, 解析
    /// canonical identity → GStreamer device-number, 构造管线 (尚未 launch).
    fn prepare(&self, plan: &PipelinePlan) -> Result<PipelineHandle, PipelineError>;
    /// 启动真实 GStreamer 采集 (decklinkvideosrc + decklinkaudiosrc → RAW).
    fn start(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    fn stop(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
    /// MEDIA-03: 崩溃/挂起后的恢复 (revalidate lease → restart GStreamer).
    fn recover(&self, handle: &PipelineHandle) -> Result<(), PipelineError>;
}

/// 把控制面 `GraphRuntimeIntent` 物化为 Media Agent 执行计划.
///
/// 关键步骤: canonical identity (DeviceIntent.device_id) → GStreamer
/// `device-number`. 当前为骨架: `device_number` 直接取自 `SourceIntent`
/// (若提供), 否则回退 0; 真实环境应由 discovery 层做 identity→index 映射.
pub fn materialize(intent: &GraphRuntimeIntent) -> Vec<PipelinePlan> {
    intent
        .devices
        .iter()
        .map(|d| {
            let device_number = d.pipeline.source.device_number.unwrap_or(0);
            PipelinePlan {
                source: SourcePlan {
                    persistent_id: d.device_id.clone(),
                    device_number,
                    resolved_input: ResolvedInputContract {
                        mode: "auto".into(), // 由 DoesSupportVideoMode 协商, 此处占位
                        pixel_format: "UYVY".into(),
                        fps: 0.0,
                        interlace: true,
                    },
                },
                video: VideoPlan { normalize: true },
                audio: AudioPlan {
                    enabled: d.pipeline.source.kind == "decklink",
                    channels: 2,
                    sample_rate: 48_000,
                },
                switch: SwitchPlan {
                    mode: "FRAME_SWITCH".into(),
                },
                output: OutputPlan {
                    sink: d.pipeline.sink.kind.clone(),
                },
            }
        })
        .collect()
}
