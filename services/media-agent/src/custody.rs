//! A2-7-02: Program Runtime Custody —— Program Semantic Lifecycle Owner。
//!
//! **定位（A2-6-00/A2-7-01 终裁）**: Runtime/Orchestration 边界上的独立
//! 角色（已批角色的首版实现），与 SessionManager（Session lifecycle owner）
//! **协作不取代**，与 Supervisor（Recovery decision owner）**分线**。
//!
//! **可以做（终裁 §Custody）**: consume execution facts / attribute facts
//! to 三域 / **advance only when transition evidence exists** / build
//! `MasterJoinInput` / call `join()` / compose ProgramMaster snapshot。
//!
//! **不能做（七不, 终裁原文）**: 猜测执行完成 / 创建新 Runtime Health /
//! 修改 Supervisor / 读取 GStreamer 对象 / 修改 PipelinePlan / 执行
//! recovery / 生成 metadata truth。
//!
//! **全局修正记档（A2-7-01 终裁）**: A2-7 不追求 "ProgramMaster 一定形成"
//! ——当前事实下唯一合法快照 = `join_result: None`（Metadata 无 producer
//! 恒 Unknown → 三 Master 永不全 eligible → Join.result=None; A2-6 的
//! None→null 语义在此成为完整上游约束）。
//!
//! **最小 Fact boundary（OQ-8 终裁: fact absent 而非 fact=false）**: 本
//! 首版只建模**当前已有证据**的 observations——ingest acceptance（b1/b2/
//! b3/b4, 归类 = Ingest Observation/Acceptance Evidence 非 Normalize
//! Execution Fact）+ attributed runtime failures。SWITCHED/COMPOSED/MIXED/
//! LOUDNESS/DELAY completion facts **不存在**（无执行节点）→ 不建任何
//! false 字段（防五态压成一 bool）。advance 当前零触发（无 transition
//! evidence）——三 Master 停留声明初始态是**诚实状态**。
//!
//! **attribution 规则（A2-7-02 复核终裁修正）**: 输入 = 真实故障 **scope
//! 证据**（非调用方预归因的 path 结论——`PipelineFault{pipeline}` 无
//! video/audio path, caller 无从得知）。首版仅 `SharedPipeline`: 一个
//! PipelineHandle 同载 video+audio 两路 → 归因**双路 failed**; 无 path
//! 证据不凭空生成单路归因（scope 无 VideoPath/AudioPath 变体, 编译期即
//! 证）。element 级 attribution（BusEvent.source）演进 deferred。

use crate::program::{
    join, AVSyncClassification, MasterJoinInput, MasterJoinResult, MetadataMaster, ProgramMaster,
};

/// 归因后的媒体路失败事实 —— Runtime failure fact 经 Custody attribution
/// 的产物（OQ-3 终裁: Runtime 产生 failure fact, Custody 负责来源+identity
/// +media path 映射, **Join 不读 Runtime**）。
/// 归因后的媒体路失败事实 —— Custody attribution 产物（注入 MasterJoinInput;
/// **A2-7-02 复核终裁**: SharedPipeline 执行故障 → 双路 failed）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttributedFailures {
    pub video_failed: bool,
    pub audio_failed: bool,
}

/// Runtime failure observation —— Custody 的归因输入（**A2-7-02 复核终裁
/// 修正**: 输入是**真实故障 scope 证据**非调用方预归因的 path 结论——
/// `PipelineFault{pipeline: Uuid}` 无 video/audio path, caller 无从得知;
/// attribution 由 Custody 承担。非持久实体, 消费时装配的参数包——与
/// `MasterJoinInput` 同律, 零第二 SoT）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureObservation {
    /// 故障来源（首版单值: PipelineFault = 唯一能归属执行管线的来源;
    /// SessionFailed/HardwareFault/HealthChanged/ClockLost **不机械映射**——
    /// 等 attribution contract 明确, 加法演进）。
    pub source: FailureSource,
    /// 故障作用域证据（首版仅 SharedPipeline: 一个 PipelineHandle 同载
    /// video+audio 两路, 无 media path 标注）。
    pub scope: FailureScope,
}

/// 失败来源封闭词表（首版单值; 新来源加法演进）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureSource {
    /// Backend 共享管线执行故障（`PipelineFault`——唯一能归属执行管线的来源）。
    PipelineFault,
}

/// 故障作用域（首版单值; **无 VideoPath/AudioPath 变体**——无 path 证据
/// 不凭空生成单路归因, 编译期即证; additive 演进留口）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureScope {
    /// 共享管线执行故障: 一个 PipelineHandle 同载 video+audio 两路。
    SharedPipeline,
}

/// Custody 归因（**真 attribution, 非调用方结论搬运**）: SharedPipeline
/// 执行故障 → 该管线双路执行不可用（video+audio 同 Handle, conservative
/// 双路）→ `video_failed=true ∧ audio_failed=true`。空切片 → 双路 false。
pub fn attribute_failures(observations: &[FailureObservation]) -> AttributedFailures {
    let shared_failed = observations
        .iter()
        .any(|o| o.scope == FailureScope::SharedPipeline);
    AttributedFailures {
        video_failed: shared_failed,
        audio_failed: shared_failed,
    }
}

/// Custody 快照输入 —— 一次 custody 周期的全部已证事实装配（**零第二
/// SoT**: 观测值由调用点从 watch 现场/事件流装配传入, Custody 不自取）。
///
/// `avsync` 当前恒 `Unknown`（OQ-4: measurement/classification 通路 deferred
/// ——Join 零阈值; 无分类器产出前注入 Unknown 是唯一诚实值）。
#[derive(Debug, Clone, PartialEq)]
pub struct CustodyObservations {
    pub failures: Vec<FailureObservation>,
    pub avsync: AVSyncClassification,
}

/// Custody 周期 —— 消费已证事实 → 归因 → （**仅在有 transition evidence 时**
/// advance——当前零触发, 三 Master 保持声明初始态）→ 装配 JoinInput →
/// `join()` → compose 快照。
///
/// 返回 `(snapshot, join_result)`——join_result 透传自 [`join`]（当前事实下
/// 恒 None: Metadata Unknown → 不 eligible; SharedPipeline failure 注入 →
/// 双路 failed → 五步优先序**行 2 FAILED**——**Degraded（行 3 单路）首版
/// 不可达**, 等 VideoPath/AudioPath scope 演进; 均不受 readiness gate）。
pub fn custody_snapshot(
    video: &crate::program::VideoMaster,
    audio: &crate::program::AudioMaster,
    observations: &CustodyObservations,
) -> (ProgramMaster, Option<MasterJoinResult>) {
    let failures = attribute_failures(&observations.failures);
    let metadata = MetadataMaster::default(); // 无 producer → Unknown（OQ-2 fail-closed）
    let input = MasterJoinInput {
        video: *video,
        audio: *audio,
        metadata: metadata.clone(),
        avsync: observations.avsync,
        video_failed: failures.video_failed,
        audio_failed: failures.audio_failed,
    };
    let output = join(&input);
    (
        ProgramMaster::compose(*video, *audio, metadata, output.result),
        output.result,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::program::video_master::VideoMasterStage;
    use crate::program::AudioMasterStage;

    fn initial_masters() -> (crate::program::VideoMaster, crate::program::AudioMaster) {
        (
            crate::program::VideoMaster::new(),
            crate::program::AudioMaster::new(),
        )
    }

    fn obs(count: usize) -> CustodyObservations {
        CustodyObservations {
            failures: (0..count)
                .map(|_| FailureObservation {
                    source: FailureSource::PipelineFault,
                    scope: FailureScope::SharedPipeline,
                })
                .collect(),
            avsync: AVSyncClassification::Unknown,
        }
    }

    /// Custody 真归因（复核终裁修正）: **SharedPipeline 执行故障 → 双路
    /// failed**; 空观察 = 双路 false。**单路归因不可构造**——FailureScope
    /// 无 VideoPath/AudioPath 变体（编译期即证, 无 path 证据不凭空生成
    /// 单路归因）。
    #[test]
    fn custody_01_attribute_shared_pipeline_both_paths() {
        let none = attribute_failures(&[]);
        assert!(!none.video_failed && !none.audio_failed);
        let one = attribute_failures(&obs(1).failures);
        assert!(
            one.video_failed && one.audio_failed,
            "一条 SharedPipeline 故障 → 双路"
        );
        let two = attribute_failures(&obs(2).failures);
        assert!(two.video_failed && two.audio_failed);
    }

    /// 最小闭环（无 failure）: 三 Master 初始 + Metadata Unknown →
    /// join=None（诚实: 尚不能判定, 不伪造）; **advance 零触发**——
    /// Master 停留声明初始态（无 transition evidence 不推进, 红线 11/12
    /// 反面: 不虚推进）。
    #[test]
    fn custody_02_no_failure_snapshot_is_none_with_initial_masters() {
        let (video, audio) = initial_masters();
        let (snapshot, result) = custody_snapshot(&video, &audio, &obs(0));
        assert_eq!(result, None);
        assert_eq!(snapshot.join_result, None);
        assert_eq!(
            snapshot.video.stage,
            VideoMasterStage::SourceRaw,
            "无证据不推进"
        );
        assert_eq!(snapshot.audio.stage, AudioMasterStage::SourceRaw);
        assert_eq!(
            snapshot.metadata.join_declaration,
            crate::program::MetadataJoinDeclaration::Unknown,
            "无 producer → Unknown fail-closed（OQ-2）"
        );
    }

    /// SharedPipeline failure 注入穿透 readiness gate（红线 12 Custody 级
    /// 实证）: Master 未 Ready + SharedPipeline 故障（双路 failed）→
    /// **FAILED**（行 2）; **Degraded（行 3 单路）首版不可达**——scope 无
    /// 单路变体（保守归因的诚实后果, 记档待 VideoPath/AudioPath 演进）;
    /// AVSync FAILED 不改 Result（仅透传——红线 3）。
    #[test]
    fn custody_03_shared_pipeline_failure_yields_failed() {
        let (video, audio) = initial_masters();
        let (snapshot, result) = custody_snapshot(&video, &audio, &obs(1));
        assert_eq!(
            result,
            Some(MasterJoinResult::Failed),
            "SharedPipeline 故障 → 双路 failed → 行 2 FAILED（穿透未 Ready）"
        );
        assert_eq!(snapshot.join_result, Some(MasterJoinResult::Failed));

        let mut avsync_failed = obs(0);
        avsync_failed.avsync = AVSyncClassification::Failed;
        let (snapshot3, result3) = custody_snapshot(&video, &audio, &avsync_failed);
        assert_eq!(
            result3, None,
            "AVSync FAILED 不改 Result（无 failure+未 Ready）"
        );
        assert_eq!(snapshot3.join_result, None);
    }

    /// Custody 确定性: 同输入两次快照恒等（零 cache 零随机性）; C′ 不可达
    /// （Metadata Unknown 无矛盾组合——declaration≠NotPresent）。
    #[test]
    fn custody_04_deterministic_and_c_prime_unreachable() {
        let (video, audio) = initial_masters();
        let observations = obs(1);
        let a = custody_snapshot(&video, &audio, &observations);
        let b = custody_snapshot(&video, &audio, &observations);
        assert_eq!(a, b);
        // C′ 矛盾（NotPresent+Present fact）在 Metadata=Unknown 下结构性不可达。
        assert_ne!(
            snapshot_metadata_declaration(&a.0),
            crate::program::MetadataJoinDeclaration::NotPresent
        );
    }

    fn snapshot_metadata_declaration(s: &ProgramMaster) -> crate::program::MetadataJoinDeclaration {
        s.metadata.join_declaration
    }
}
