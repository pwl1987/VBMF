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
//!
//! **identity correlation（A2-7-02 二轮复核终裁）**: FailureObservation 携带
//! `pipeline_id: Uuid`（沿用 `RuntimeEvent::PipelineFault.pipeline` 真实
//! 身份; **禁**强行统一 PipelineHandle(u64)↔Uuid——两级身份映射留 A2-7-03
//! 接线时确认 SoT）。归因只消费 **pipeline_id 匹配 ∧ PipelineFault ∧
//! SharedPipeline** 联合证据——防跨实例污染（Pipeline A fault 不得污染
//! Pipeline B snapshot）。

use crate::program::{
    join, AVSyncClassification, MasterJoinInput, MasterJoinResult, MetadataMaster, ProgramMaster,
};
use uuid::Uuid;

/// 归因后的媒体路失败事实 —— Custody attribution 产物（注入 MasterJoinInput;
/// **A2-7-02 复核终裁**: SharedPipeline 执行故障 → 双路 failed）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttributedFailures {
    pub video_failed: bool,
    pub audio_failed: bool,
}

/// Runtime failure observation —— Custody 的归因输入（**A2-7-02 二轮终裁**:
/// 携带 `pipeline_id` 关联身份 = 沿用 `RuntimeEvent::PipelineFault.pipeline`;
/// 输入是真实故障 scope **+身份**证据, 非调用方预归因结论。非持久实体,
/// 消费时装配的参数包——与 `MasterJoinInput` 同律, 零第二 SoT）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureObservation {
    /// 关联执行管线身份（= `RuntimeEvent::PipelineFault.pipeline`; 归因
    /// 只消费与本 Custody 周期匹配的 observation——跨实例污染防线）。
    pub pipeline_id: Uuid,
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

/// Custody 归因（**identity correlation + 真 attribution**）: 只消费
/// **pipeline_id 匹配 ∧ (PipelineFault, SharedPipeline) 联合证据** 的
/// observation → 双路 failed。身份不匹配的故障（其他管线）**零污染**;
/// 空切片/全不匹配 → 双路 false。
pub fn attribute_failures(
    pipeline_id: Uuid,
    observations: &[FailureObservation],
) -> AttributedFailures {
    let shared_failed = observations.iter().any(|o| {
        o.pipeline_id == pipeline_id
            && matches!(
                (&o.source, &o.scope),
                (FailureSource::PipelineFault, FailureScope::SharedPipeline)
            )
    });
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

/// Custody 周期 —— 消费已证事实 → **identity correlation**（只归因本
/// `pipeline_id` 的故障——跨实例污染防线）→ （**仅在有 transition evidence
/// 时** advance——当前零触发, 三 Master 保持声明初始态）→ 装配 JoinInput →
/// `join()` → compose 快照。
///
/// `pipeline_id` = 本 Custody 周期归属的执行管线身份（沿用
/// `RuntimeEvent::PipelineFault.pipeline`）。
///
/// 返回 `(snapshot, join_result)`——join_result 透传自 [`join`]（当前事实下
/// 恒 None: Metadata Unknown → 不 eligible; **本管线** SharedPipeline
/// failure 注入 → 双路 failed → 五步优先序**行 2 FAILED**——**Degraded
/// （行 3 单路）首版不可达**, 等 VideoPath/AudioPath scope 演进; 均不受
/// readiness gate）。
pub fn custody_snapshot(
    video: &crate::program::VideoMaster,
    audio: &crate::program::AudioMaster,
    pipeline_id: Uuid,
    observations: &CustodyObservations,
) -> (ProgramMaster, Option<MasterJoinResult>) {
    let failures = attribute_failures(pipeline_id, &observations.failures);
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
    use uuid::Uuid;

    fn initial_masters() -> (crate::program::VideoMaster, crate::program::AudioMaster) {
        (
            crate::program::VideoMaster::new(),
            crate::program::AudioMaster::new(),
        )
    }

    /// 装配 observations: n 条故障, pipeline_id=`pipeline`（首条）; 其余为
    /// `other`（跨实例污染测试用他管线身份）。
    fn obs_for(pipeline: Uuid, n: usize, other: Uuid) -> CustodyObservations {
        let mut failures = Vec::new();
        for i in 0..n {
            failures.push(FailureObservation {
                pipeline_id: if i == 0 { pipeline } else { other },
                source: FailureSource::PipelineFault,
                scope: FailureScope::SharedPipeline,
            });
        }
        CustodyObservations {
            failures,
            avsync: AVSyncClassification::Unknown,
        }
    }

    fn obs_on(pipeline: Uuid, n: usize) -> CustodyObservations {
        let mut failures = Vec::new();
        for _ in 0..n {
            failures.push(FailureObservation {
                pipeline_id: pipeline,
                source: FailureSource::PipelineFault,
                scope: FailureScope::SharedPipeline,
            });
        }
        CustodyObservations {
            failures,
            avsync: AVSyncClassification::Unknown,
        }
    }

    /// Custody 真归因（**identity correlation**）: 只消费 pipeline_id 匹配
    /// ∧ (PipelineFault, SharedPipeline) 联合证据 → 双路 failed; 身份不匹配
    /// **零污染**; 空观察 = 双路 false。**单路归因不可构造**——FailureScope
    /// 无 VideoPath/AudioPath 变体（编译期即证）。
    #[test]
    fn custody_01_attribute_identity_correlation() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let none = attribute_failures(a, &[]);
        assert!(!none.video_failed && !none.audio_failed);
        let matched = attribute_failures(a, &obs_on(a, 1).failures);
        assert!(
            matched.video_failed && matched.audio_failed,
            "本管线 SharedPipeline 故障 → 双路"
        );
        // 跨实例: Pipeline A fault 对 B **零污染**（二轮终裁核心回归）。
        let cross = attribute_failures(b, &obs_on(a, 2).failures);
        assert!(
            !cross.video_failed && !cross.audio_failed,
            "A 管线故障不得污染 B 管线 snapshot"
        );
        // 混合流断言（修正）: 流中仅含 A 的故障——A 命中、B 零污染。
        let mut mixed = Vec::new();
        mixed.push(FailureObservation {
            pipeline_id: a,
            source: FailureSource::PipelineFault,
            scope: FailureScope::SharedPipeline,
        });
        let b_view = attribute_failures(b, &mixed);
        assert!(!b_view.video_failed && !b_view.audio_failed);
        let a_view = attribute_failures(a, &mixed);
        assert!(a_view.video_failed && a_view.audio_failed);
        // 各归各场景: 流中同时含 A 与 B 各自故障——双方各自命中, 互不串扰。
        mixed.push(FailureObservation {
            pipeline_id: b,
            source: FailureSource::PipelineFault,
            scope: FailureScope::SharedPipeline,
        });
        assert!(attribute_failures(a, &mixed).video_failed);
        assert!(attribute_failures(b, &mixed).video_failed);
    }

    /// 最小闭环（无 failure）: 三 Master 初始 + Metadata Unknown →
    /// join=None（诚实: 尚不能判定, 不伪造）; **advance 零触发**——
    /// Master 停留声明初始态（无 transition evidence 不推进, 红线 11/12
    /// 反面: 不虚推进）。
    #[test]
    fn custody_02_no_failure_snapshot_is_none_with_initial_masters() {
        let (video, audio) = initial_masters();
        let pipeline = Uuid::new_v4();
        let (snapshot, result) =
            custody_snapshot(&video, &audio, pipeline, &obs_for(pipeline, 0, pipeline));
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

    /// 本管线 SharedPipeline failure 注入穿透 readiness gate（红线 12 Custody
    /// 级实证）: Master 未 Ready + 双路 failed → **FAILED**（行 2）;
    /// **Degraded（行 3 单路）首版不可达**——scope 无单路变体（保守归因的
    /// 诚实后果, 记档待 VideoPath/AudioPath 演进）; AVSync FAILED 不改
    /// Result（仅透传——红线 3）。
    #[test]
    fn custody_03_shared_pipeline_failure_yields_failed() {
        let (video, audio) = initial_masters();
        let pipeline = Uuid::new_v4();
        let (snapshot, result) = custody_snapshot(&video, &audio, pipeline, &obs_on(pipeline, 1));
        assert_eq!(
            result,
            Some(MasterJoinResult::Failed),
            "本管线 SharedPipeline 故障 → 双路 failed → 行 2 FAILED（穿透未 Ready）"
        );
        assert_eq!(snapshot.join_result, Some(MasterJoinResult::Failed));

        let mut avsync_failed = obs_on(pipeline, 0);
        avsync_failed.avsync = AVSyncClassification::Failed;
        let (snapshot3, result3) = custody_snapshot(&video, &audio, pipeline, &avsync_failed);
        assert_eq!(
            result3, None,
            "AVSync FAILED 不改 Result（无 failure+未 Ready）"
        );
        assert_eq!(snapshot3.join_result, None);
    }

    /// **跨实例污染回归（二轮终裁核心）**: Pipeline A fault + Pipeline B
    /// snapshot → B **不被判 failed**（result=None）; 反之 A 自身 snapshot
    /// → FAILED。同一 observations 流, 仅 pipeline_id 区分——identity
    /// correlation 防线实证。
    #[test]
    fn custody_05_cross_pipeline_fault_does_not_pollute_other_snapshot() {
        let (video, audio) = initial_masters();
        let pipeline_a = Uuid::new_v4();
        let pipeline_b = Uuid::new_v4();
        // 故障流: 仅 A 管线故障（2 条）。
        let observations = obs_on(pipeline_a, 2);
        let (_snap_a, result_a) = custody_snapshot(&video, &audio, pipeline_a, &observations);
        let (snap_b, result_b) = custody_snapshot(&video, &audio, pipeline_b, &observations);
        assert_eq!(
            result_a,
            Some(MasterJoinResult::Failed),
            "A 自身故障 → A FAILED"
        );
        assert_eq!(result_b, None, "A 故障不污染 B——B snapshot 保持 None");
        assert_eq!(snap_b.join_result, None);
        // 反向: B 无故障流 → B None; A 收到 B 的"故障"（实为 B 身份）→ 亦
        // 不污染 A。
        let b_faults = obs_on(pipeline_b, 1);
        let (_, result_a2) = custody_snapshot(&video, &audio, pipeline_a, &b_faults);
        assert_eq!(result_a2, None, "B 管线故障不得污染 A snapshot");
    }

    /// Custody 确定性: 同输入两次快照恒等（零 cache 零随机性）; C′ 不可达
    /// （Metadata Unknown 无矛盾组合——declaration≠NotPresent）。
    #[test]
    fn custody_06_deterministic_and_c_prime_unreachable() {
        let (video, audio) = initial_masters();
        let pipeline = Uuid::new_v4();
        let observations = obs_on(pipeline, 1);
        let a = custody_snapshot(&video, &audio, pipeline, &observations);
        let b = custody_snapshot(&video, &audio, pipeline, &observations);
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
