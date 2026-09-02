//! A2-1: Program Domain —— VBMF 节目生产模型的 Canonical 层。
//!
//! 用户 2026-09-02 裁定链（A2-1..A2-8）: SwitchPolicy → Video Master → Audio Master →
//! Metadata Master → Master Join → ProgramMaster Runtime Projection → GStreamer
//! Materialization → 双输入真机切换。
//!
//! **纪律**: 本域对象是 Canonical 声明（"是什么"）; GStreamer 是其 Execution Adapter
//! （A2-7 materialize）——绝不反向让 pipeline 元素推导出 Program Domain。
//! Channel 完整模型属控制面线（A4）。

pub mod audio_master;
pub mod metadata_master;
pub mod switch_policy;
pub mod video_master;

pub use audio_master::{
    AudioDataPlane, AudioMaster, AudioMasterStage, MixLayout, DEFAULT_DELAY_MS,
};
pub use metadata_master::{MetadataDataPlane, MetadataType, METADATA_TYPES};
pub use switch_policy::{ProgramDomainError, SwitchIoPlane, SwitchPolicy, ACCEPTED_LIST};
pub use video_master::{ProgramComposition, VideoDataPlane, VideoMaster, VideoMasterStage};
