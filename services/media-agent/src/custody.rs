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
    /// 关联身份——**legacy event-field correlated identity = 设备 canonical
    /// 身份（DeviceId）in current RuntimeEvent implementation**（A2-7-03 终裁
    /// 标记: `PipelineFault.pipeline` 当前实现承载 device identity 属
    /// legacy/misnamed——同 enum 内 `SourceMaterialized.pipeline` 是 Pipeline
    /// identity, 同名双语义=Event Contract ambiguity, 类型级修正留 V0.3
    /// cleanup; 本字段名承事件字段名避免 churn, **勿误读为 PipelineHandle/
    /// PipelineId**）。归因只消费与本 Custody 周期匹配的 observation——
    /// 跨实例污染防线。
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

/// 生产桥（A2-7-03）—— `RuntimeEvent` 流 → `CustodyObservations` 的唯一
/// 转换点（Runtime failure fact → Custody 归因输入; **纯函数**, 消费点
/// 自行 drain 事件后调用——Custody 不订阅不持 Runtime 引用）。
///
/// **接线状态（03 终裁三段式: Production Runtime Connection DEFERRED TO
/// 04）**: 本桥已实现但**尚无生产调用者**——真实生产故障链现状为 mapper
/// 产 `PipelineFault(nil)`（本桥拒收）+ Supervisor echo（本桥再拒收）,
/// 即真实故障尚未经本桥进入 Custody; 闭合（真实 drain→桥→custody 周期）
/// 属 A2-7-04 mock lifecycle 验证。
///
/// 提取规则（身份 SoT 反推结论, 03 报告 §1）:
/// - 只提取 `PipelineFault{pipeline, summary, ..}`——`pipeline` 的真实语义
///   = **设备 canonical 身份**（Supervisor 决策句柄: register/report_failure
///   均按 device_id; supervisor.rs L38 注释原文）;
/// - **回声排除**: `summary == RESTART_ECHO_SUMMARY` 的 PipelineFault 是
///   Supervisor 决策回声非新故障事实（与 fault_trigger_from_events 同律）;
/// - **`Uuid::nil()` 不吸收**（mapper 产的上游故障未归属）——无身份证据
///   不归因（fail-closed; 显式跳过表达意图, 防误归因到任何真实设备）;
/// - HardwareFault/SessionFailed/HealthChanged/ClockLost 不提取（02 终裁
///   维持——等 attribution contract）;
/// - `avsync` 恒 `Unknown`（OQ-4 deferred 维持——Join 零阈值零测量）。
pub fn observations_from_events(events: &[crate::events::RuntimeEvent]) -> CustodyObservations {
    let failures = events
        .iter()
        .filter_map(|ev| match ev {
            crate::events::RuntimeEvent::PipelineFault {
                pipeline, summary, ..
            } if *pipeline != Uuid::nil()
                && summary.as_str() != crate::supervisor::RESTART_ECHO_SUMMARY =>
            {
                Some(FailureObservation {
                    pipeline_id: *pipeline,
                    source: FailureSource::PipelineFault,
                    scope: FailureScope::SharedPipeline,
                })
            }
            _ => None,
        })
        .collect();
    CustodyObservations {
        failures,
        avsync: AVSyncClassification::Unknown,
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

    /// 生产桥（A2-7-03）: RuntimeEvent 流 → observations 提取规则全锁——
    /// 真实 device_id PipelineFault 提取 / 回声排除 / nil 不吸收 / 其他
    /// kind（HardwareFault/SessionFailed）零提取 / avsync 恒 Unknown。
    #[test]
    fn custody_07_observations_from_events_extraction_rules() {
        use crate::events::RuntimeEvent;
        let device = Uuid::new_v4();
        let events = vec![
            RuntimeEvent::PipelineFault {
                pipeline: device,
                summary: "upstream decode error".into(),
                retryable: true,
            },
            // Supervisor 决策回声——非新故障事实, 不提取。
            RuntimeEvent::PipelineFault {
                pipeline: device,
                summary: crate::supervisor::RESTART_ECHO_SUMMARY.into(),
                retryable: true,
            },
            // mapper 未归属的上游故障——无身份证据不吸收。
            RuntimeEvent::PipelineFault {
                pipeline: Uuid::nil(),
                summary: "pipeline error: unknown source".into(),
                retryable: true,
            },
            // 其他 kind 零提取（02 终裁维持）。
            RuntimeEvent::HardwareFault {
                device_id: device,
                summary: "device lost".into(),
            },
            RuntimeEvent::SessionFailed {
                session_id: Uuid::new_v4(),
                reason: "preflight".into(),
            },
        ];
        let obs = observations_from_events(&events);
        assert_eq!(obs.failures.len(), 1, "恰提取 1 条真实故障");
        assert_eq!(obs.failures[0].pipeline_id, device);
        assert_eq!(obs.failures[0].source, FailureSource::PipelineFault);
        assert_eq!(obs.failures[0].scope, FailureScope::SharedPipeline);
        assert_eq!(obs.avsync, AVSyncClassification::Unknown);

        // 生产链端到端: 事件流 → 桥 → 该设备 custody → FAILED;
        // 他设备 custody → None（identity correlation 经生产桥仍成立）。
        let (video, audio) = initial_masters();
        let other = Uuid::new_v4();
        let (_, r_self) = custody_snapshot(&video, &audio, device, &obs);
        assert_eq!(r_self, Some(MasterJoinResult::Failed));
        let (_, r_other) = custody_snapshot(&video, &audio, other, &obs);
        assert_eq!(r_other, None, "他设备零污染（生产桥+归因双层防线）");

        // 空流/纯回声流 → 零提取 → 双 false。
        let empty = observations_from_events(&[]);
        assert!(empty.failures.is_empty());
        let echo_only = observations_from_events(&[RuntimeEvent::PipelineFault {
            pipeline: device,
            summary: crate::supervisor::RESTART_ECHO_SUMMARY.into(),
            retryable: true,
        }]);
        assert!(echo_only.failures.is_empty(), "纯回声流零提取");
    }
}

/// A2-7-04 mock lifecycle 测试族（`adapters::mock` 为 feature-gated, 独立
/// 子模块——与 session.rs `#[cfg(all(test, feature = "mock"))]` 同律）。
#[cfg(all(test, feature = "mock"))]
mod lifecycle {
    use super::*;
    use crate::custody::observations_from_events;
    use crate::program::{AudioMaster, MasterJoinResult, VideoMaster};
    use uuid::Uuid;

    fn initial_masters() -> (VideoMaster, AudioMaster) {
        (VideoMaster::new(), AudioMaster::new())
    }

    // ═══ A2-7-04: mock lifecycle 全链闭环（六验收 + A/B 反证, 终裁 §0''）═══
    // 验收链: SessionManager.create/start → MockBackend → SessionInput{device_id,
    // handle} → canonical RuntimeEvent(经现有 FanoutSink) → drain →
    // observations_from_events → attribute_failures → MasterJoinInput → join →
    // ProgramMaster。**零新基础设施**: 复用 session.rs 同款测试装配
    // （MockBackend/MockProviderB/FanoutSink 双日志/生产级绑定）, 不建第二套
    // Runtime、不造身份映射（Device↔Handle 关联仅经已有 SessionInput）。

    use crate::adapters::mock::MockBackend;
    use crate::contracts::provider::HardwareProvider;
    use crate::device::DeviceInfo;
    use crate::events::RuntimeEventSink;
    use crate::graph_intent::GraphRuntimeIntent;
    use crate::lease::InMemoryLeaseManager;
    use crate::pipeline::MaterializeMode;
    use crate::port::{PortCapabilities, PortDirection, PortInfo, PortOrdinal, PortRegistry};
    use crate::resolver::{Confidence, ResolverMatch};
    use crate::resource::{ResourceRegistry, SharedResourceRegistry};
    use crate::session::{SessionId, SessionManager, SessionTuning};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    type InMemoryLm = InMemoryLeaseManager;

    fn a27_devices() -> Vec<DeviceInfo> {
        crate::adapters::mock::MockProviderB
            .discover()
            .expect("mock-b discover")
            .into_iter()
            .map(|d| d.device)
            .collect()
    }

    fn a27_registry(devs: &[DeviceInfo]) -> PortRegistry {
        let mut ports = Vec::new();
        for dev in devs {
            ports.push(PortInfo {
                device_id: dev.device_id,
                provider_binding_ref: None,
                identity: crate::port::PortIdentity {
                    port_id: crate::port::PortIdentity::derive(
                        &dev.device_id,
                        crate::port::ConnectorType::Sdi,
                        PortOrdinal::Known(1),
                    ),
                    connector: crate::port::ConnectorType::Sdi,
                    ordinal: PortOrdinal::Known(1),
                },
                direction: PortDirection::Input,
                capabilities: PortCapabilities::default(),
                runtime_binding: None,
                signal: crate::port::SignalStatus::default(),
                content: crate::port::VideoContentState::Unknown,
            });
        }
        PortRegistry { ports }
    }

    fn a27_intent(dev: &DeviceInfo) -> GraphRuntimeIntent {
        GraphRuntimeIntent {
            version: "1.0".into(),
            devices: vec![crate::graph_intent::DeviceIntent {
                device_id: dev.device_id.to_string(),
                role: "CAPTURE".into(),
                pipeline: crate::graph_intent::PipelineIntent {
                    source: crate::graph_intent::SourceIntent {
                        kind: "decklink".into(),
                        device_id: dev.device_id.to_string(),
                        port_id: None,
                    },
                    sink: crate::graph_intent::SinkIntent {
                        kind: "appsink".into(),
                    },
                },
            }],
        }
    }

    /// 真实 SessionManager + MockBackend + FanoutSink 双日志（与 session.rs
    /// evt_int_rt_01 同款装配）。返回 (mgr, projection_log)。
    fn a27_manager() -> (Arc<SessionManager>, Arc<crate::events::RuntimeEventLog>) {
        let devices = a27_devices();
        let lm: Arc<InMemoryLm> = Arc::new(InMemoryLm::new());
        let resources = SharedResourceRegistry::new(ResourceRegistry::derive_from_discovery(
            &a27_registry(&devices),
        ));
        let projection = Arc::new(crate::events::RuntimeEventLog::new());
        let internal = Arc::new(crate::events::RuntimeEventLog::new());
        let sink: Arc<dyn crate::events::RuntimeEventSink> = Arc::new(
            crate::events::FanoutSink::new(projection.clone(), internal.clone()),
        );
        let sup = Arc::new(Mutex::new(crate::supervisor::Supervisor::new(
            crate::supervisor::RestartPolicy::default(),
            sink.clone(),
        )));
        let bindings: HashMap<Uuid, crate::resolver::ResolvedDeviceBinding> = devices
            .iter()
            .map(|d| {
                (
                    d.device_id,
                    crate::resolver::ResolvedDeviceBinding {
                        device_number: 0,
                        hw_serial_number: None,
                        persistent_id: None,
                        confidence: Confidence::High,
                        match_kind: ResolverMatch::DeviceHandleExact,
                    },
                )
            })
            .collect();
        let mgr = Arc::new(SessionManager::new(
            resources,
            lm,
            sup,
            Arc::new(MockBackend),
            Arc::new(devices),
            Arc::new(bindings),
            None,
            MaterializeMode::Diagnostic,
            SessionTuning::default(),
            sink,
        ));
        (mgr, projection)
    }

    /// A2-7-04 六验收闭环（终裁 §0'' 冻结版）:
    /// 1 Session lifecycle 真实走 SessionManager; 2 Execution identity 真实
    /// SessionInput{device_id, handle} 零新 mapping; 3 故障进**现有**
    /// FanoutSink 非旁路数组; 4 Bridge 恰提取目标 Device 真实 PipelineFault
    /// （echo=0/nil=0）; 5 Isolation A FAILED·B None·echo 零 observation;
    /// 6 Join 双路注入→Failed **不依赖 readiness**（三 Master 初始态）。
    #[test]
    fn custody_08_lifecycle_closed_loop_six_acceptances() {
        let (mgr, projection) = a27_manager();
        let dev_a = a27_devices()[0].device_id;

        // 验收 1+2: create→start 真实链; 真实 SessionInput 身份。
        let sid: SessionId = mgr.create(a27_intent(&a27_devices()[0])).expect("create");
        mgr.start(&sid).expect("start");
        let session = mgr.status(&sid).expect("status");
        let input = *session.inputs.first().expect("SessionInput");
        assert_eq!(input.device_id, dev_a, "真实 DeviceId（非 mock 编造）");
        assert_ne!(
            input.handle.0, 0,
            "真实 Handle（NEXT_PIPELINE_ID 同源, 非 0 哨兵）"
        );

        // 验收 3: 故障经**现有** RuntimeEventSink/FanoutSink 组件（非旁路数组）
        // ——测试消费者按终裁裁决③ emit canonical 事件（device 身份来自真实
        // SessionInput）+ 一条 Supervisor echo + 一条 nil（桥提取规则的实战流）。
        // 注: mgr 的 FanoutSink 为私有装配; 此处用同型 FanoutSink 写同一份
        // projection 日志——验证的是现有组件间数据契约（桥只消费 &[RuntimeEvent]）。
        let emit_sink: Arc<dyn RuntimeEventSink> = Arc::new(crate::events::FanoutSink::new(
            projection.clone(),
            Arc::new(crate::events::RuntimeEventLog::new()),
        ));
        emit_sink.emit(crate::events::RuntimeEvent::PipelineFault {
            pipeline: dev_a,
            summary: "decode error: upstream".into(),
            retryable: true,
        });
        emit_sink.emit(crate::events::RuntimeEvent::PipelineFault {
            pipeline: dev_a,
            summary: crate::supervisor::RESTART_ECHO_SUMMARY.into(),
            retryable: true,
        });
        emit_sink.emit(crate::events::RuntimeEvent::PipelineFault {
            pipeline: Uuid::nil(),
            summary: "pipeline error: unattributed".into(),
            retryable: true,
        });

        // 验收 3+4: 现有事件流 drain（破坏性单次, 与生产消费同律）→ 桥
        // 恰提取 1 条（echo=0, nil=0）。
        let drained = projection.drain();
        assert!(
            drained
                .iter()
                .any(|e| matches!(e, crate::events::RuntimeEvent::IdentityResolved { .. })),
            "start 真实链应已产生 IdentityResolved（验收 1 的链路证据）"
        );
        let obs = observations_from_events(&drained);
        assert_eq!(
            obs.failures.len(),
            1,
            "恰一条 FailureObservation（echo/nil 零提取）"
        );
        assert_eq!(
            obs.failures[0].pipeline_id, dev_a,
            "正确 Device correlation"
        );

        // 验收 5+6: A custody → FAILED（双路注入, 不依赖 readiness——三
        // Master 初始态）; B（另一设备）→ None 零污染。
        let (video, audio) = initial_masters();
        let dev_b = a27_devices()[1].device_id;
        let (_, r_a) = custody_snapshot(&video, &audio, dev_a, &obs);
        assert_eq!(
            r_a,
            Some(MasterJoinResult::Failed),
            "A: 双路 failed → FAILED（穿透未 Ready）"
        );
        let (_, r_b) = custody_snapshot(&video, &audio, dev_b, &obs);
        assert_eq!(r_b, None, "B: 零污染");

        mgr.stop(&sid).expect("cleanup stop");
        mgr.close(&sid).expect("cleanup close");
    }

    /// A/B 双实例反证（终裁附加必过项）: Session A{device A, handle H1} +
    /// Session B{device B, handle H2}, H1≠H2·A≠B·**零额外 registry**——
    /// A failure → Custody(A)=FAILED·Custody(B)=None; 反向 B failure 同理。
    /// 身份仅经各自 SessionInput（无任何 Handle→Device 推断）。
    #[test]
    fn custody_09_ab_dual_session_isolation_no_hidden_mapping() {
        let (mgr, projection) = a27_manager();
        let devices = a27_devices();
        let (dev_a, dev_b) = (devices[0].device_id, devices[1].device_id);

        let sid_a: SessionId = mgr.create(a27_intent(&devices[0])).expect("create A");
        mgr.start(&sid_a).expect("start A");
        let sid_b: SessionId = mgr.create(a27_intent(&devices[1])).expect("create B");
        mgr.start(&sid_b).expect("start B");

        let in_a = mgr.status(&sid_a).unwrap().inputs[0];
        let in_b = mgr.status(&sid_b).unwrap().inputs[0];
        assert_ne!(in_a.device_id, in_b.device_id, "A≠B");
        assert_ne!(
            in_a.handle, in_b.handle,
            "H1≠H2（NEXT_PIPELINE_ID 同源递增）"
        );
        // 零隐藏 mapping: Custody 输入只有 device_id（来自 SessionInput）,
        // handle 不参与归因（编译期事实——attribute_failures 签名无 Handle）。

        // A 管线故障 → 经现有事件流（同 custody_08 的 emit 路径）。
        let emit: Arc<dyn RuntimeEventSink> = Arc::new(crate::events::FanoutSink::new(
            projection.clone(),
            Arc::new(crate::events::RuntimeEventLog::new()),
        ));
        emit.emit(crate::events::RuntimeEvent::PipelineFault {
            pipeline: dev_a,
            summary: "signal lost".into(),
            retryable: true,
        });
        let obs = observations_from_events(&projection.drain());
        assert_eq!(obs.failures.len(), 1);

        let (video, audio) = initial_masters();
        let (_, r_a) = custody_snapshot(&video, &audio, in_a.device_id, &obs);
        let (_, r_b) = custody_snapshot(&video, &audio, in_b.device_id, &obs);
        assert_eq!(r_a, Some(MasterJoinResult::Failed), "A=FAILED");
        assert_eq!(r_b, None, "B=None（零污染, 无隐藏 mapping）");

        // 反向: B 故障 → B=FAILED, A 不受影响。
        emit.emit(crate::events::RuntimeEvent::PipelineFault {
            pipeline: dev_b,
            summary: "clock provider lost".into(),
            retryable: true,
        });
        let obs_b = observations_from_events(&projection.drain());
        assert_eq!(obs_b.failures.len(), 1);
        assert_eq!(obs_b.failures[0].pipeline_id, dev_b);
        let (_, r_b2) = custody_snapshot(&video, &audio, in_b.device_id, &obs_b);
        assert_eq!(r_b2, Some(MasterJoinResult::Failed), "B=FAILED（反向）");

        mgr.stop(&sid_a).expect("cleanup stop A");
        mgr.stop(&sid_b).expect("cleanup stop B");
        mgr.close(&sid_a).expect("cleanup close A");
        mgr.close(&sid_b).expect("cleanup close B");
    }
}
