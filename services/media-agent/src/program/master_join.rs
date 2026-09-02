//! A2-5-03: Master Join —— Program Domain 第五块（V0.2 §1.20 联合判定）。
//!
//! **定位（A2-5-00..02 终裁链）**: Join 是**联合判定声明者**——出 Program-level
//! Join 语义结果 + 伴随分类输入；**不做 Recovery、零 action**（`SupervisorAction`
//! 是唯一 action 家）、不读 Runtime/Health/Event（failed 事实由参数注入）、
//! 零时间字段（D14 禁入）、不接 transport（A2-6 投影阶段才接线）。
//!
//! **三件分离（A2-5-01/02 终裁）**: Eligibility（能否作为有效参与者）≠
//! Readiness（联合是否可判定）≠ Result（联合语义结果）。五步优先序真值表
//! 见 `join()` doc——**failure/C′ 矛盾不受 readiness gate**（红线 12: failed
//! 事实必须在 Master 未 Ready 时仍能产生 DEGRADED/FAILED，否则 §1.20 单路
//! failed→DEGRADED 被逻辑屏蔽）。
//!
//! **语义三不等式（测试写死）**: `MasterJoinResult::Failed` = Program Join
//! semantic failure ≠ Runtime health 态 ≠ `CommandStatus::Failed` ≠
//! `SupervisorAction`。**投影边界**: DEGRADED/FAILED = §8.9 Master 域输入
//! 信号（Runtime/Safety 再决定动作）；禁 Channel Health 直推（Health Tree
//! 独立聚合——Primary failed+Backup 接管→Channel HEALTHY）。
//!
//! **AVSync 概念隔离（红线 3/5/6）**: `AVSyncClassification` 是已分类的
//! 伴随输入——不复用 `ClockObservationState`（offset/drift=时钟基准 SoT）、
//! 不复制 `avsync_measurements` DB schema、零测量字段零阈值（40/100/250ms
//! 归执行侧）；**`AVSyncClassification::Failed` 不快捷转 Join Degraded**
//! （§8.10: red 后先 classify_failure_domain，PLAYER 绝不切源）。
//!
//! 禁 `Master` trait（三 Master 非对称是 A2-4 终裁资产——组合参数非接口抽象）。

use crate::program::audio_master::AudioMaster;
use crate::program::metadata_master::{MetadataJoinDeclaration, MetadataMaster, MetadataPresence};
use crate::program::video_master::VideoMaster;
use serde::{Deserialize, Serialize};

/// Join Result 受纳词表快照（A2-5-02 终裁 §1.1；错误信息与测试共用）。
pub const JOIN_RESULTS: &[&str] = &["ACCEPTABLE", "DEGRADED", "FAILED"];

/// AVSync 分类受纳词表快照（A2-5-02 终裁 §1.3）。
pub const AVSYNC_CLASSIFICATIONS: &[&str] = &["ACCEPTABLE", "DEGRADED", "FAILED", "UNKNOWN"];

/// Master Join 联合语义结果 —— **Program Join semantic plane**（A2-5-02 §1.1）。
///
/// 三不等式（doc+测试写死）: `Failed` ≠ Runtime health 态 ≠ `CommandStatus::
/// Failed` ≠ `SupervisorAction`。无 `Ready` 成员（Readiness 是独立层）;
/// `None`（Option 外层）表达"尚不可判定"，**不是**第四枚举值。
/// 无 `Default`（无天然默认——默认 Acceptable/Degraded/Failed 均无依据）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MasterJoinResult {
    /// 三路声明有效、无 failure/inconsistency 且 Ready 的联合结果。
    Acceptable,
    /// 任一媒体路 failed（§1.20 L155 逐字: "任一路 failed → Program Master
    /// 进入 DEGRADED 或触发 FAILOVER"——FAILOVER 是 Runtime 动作非 Join 状态）。
    Degraded,
    /// Program Join semantic failure（C′ 矛盾快照 / 双媒体路 failed）。
    Failed,
}

/// AVSync 联合判定**伴随分类输入**（A2-5-02 §1.3——分级由上游执行侧给出）。
///
/// 消歧三不（红线 5/6）: 不复用 `ClockObservationState`、不复制
/// `avsync_measurements`、不携带 offset/drift 测量字段。Join 零阈值
/// （40/100/250ms、5ms/min 归 AVSync Measurement/Correction 执行侧）。
/// `Failed` = red 级（需 failure domain 分类——本身不是 action）;
/// `Unknown` = 未测量（不阻断不降级——§8.10 无"未测量"动作）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AVSyncClassification {
    Acceptable,
    Degraded,
    Failed,
    /// 未测量（`Default`——冷启动前态; 观测不足 ≠ 故障）。
    #[default]
    Unknown,
}

/// Master Join 组合输入（**零 trait**——三 Master 按值组合, 非引用非展平）。
///
/// `video_failed`/`audio_failed` = **Runtime failure facts 的显式参数注入**
/// （红线 4: Join 不读 Runtime Snapshot/Event Projection/Health Tree, 不自行
/// 探测故障——单向依赖: Runtime 注入, Program 消费）。非 serde 对象（无 wire
/// 契约——A2-6 投影需要时另裁）。PartialEq-only（含 AudioMaster 的 f32 字段,
/// 与其同律——A2-3 先例）。
#[derive(Debug, Clone, PartialEq)]
pub struct MasterJoinInput {
    pub video: VideoMaster,
    pub audio: AudioMaster,
    pub metadata: MetadataMaster,
    /// 已分类的 AVSync 伴随输入（非 Option——`Unknown` 变体即"未测",
    /// None/Unknown 双表达 = 双 SoT）。
    pub avsync: AVSyncClassification,
    pub video_failed: bool,
    pub audio_failed: bool,
}

/// 三域 Eligibility + 联合 Readiness（A2-5-02 终裁 §2）。
///
/// Eligibility = "能否作为 Join 的有效参与者"; Readiness = 三者合取
/// （**中间 decision, 不入 `MasterJoinResult`**——R-C/R-D）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JoinEligibility {
    /// 复用 `VideoMaster::is_program_scope_master()`（不重定义第二判定）。
    pub video: bool,
    /// 复用 `AudioMaster::is_program_scope_master()`。
    pub audio: bool,
    /// `Participating | NotPresent` 均有效; `Unknown` = 声明未成（非非法非 failed）。
    pub metadata: bool,
    /// `video ∧ audio ∧ metadata`。
    pub ready: bool,
}

/// 伴随分类输入（OQ-D: Join 可产生/暴露——**零 action 零 recovery**）。
///
/// 消费面 = Runtime/Safety 的 `classify_failure_domain`（§8.10/§8.9）;
/// 非分类执行者（分类不归 Join）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JoinClassificationInput {
    /// AVSync 分级透传（不改变 Result——红线 3 测试锁定）。
    pub avsync: AVSyncClassification,
    /// C′ 矛盾快照存在（`NotPresent ∧ ∃ Present fact`）。
    pub inconsistency: bool,
}

/// Master Join 输出 —— 三件分离（A2-5-02 终裁 §1.4）。
#[derive(Debug, Clone, PartialEq)]
pub struct MasterJoinOutput {
    pub eligibility: JoinEligibility,
    /// **Option 语义**: `None` = 当前尚不能形成联合 Result（无 failure/
    /// inconsistency 且未 Ready）——绝不伪造 DEGRADED/FAILED（终裁 §1.1 第 4 行）。
    pub result: Option<MasterJoinResult>,
    pub classification_input: JoinClassificationInput,
}

/// 联合判定纯函数 —— 五步优先序（A2-5-02 终裁 §2 真值表, 短路求值）。
///
/// ```text
/// 1. C′ semantic inconsistency（NotPresent ∧ ∃ Present fact）→ FAILED
///    —— 不受 readiness 限制（红线 11: 不得被 readiness gate 吞掉）
/// 2. video_failed ∧ audio_failed                                → FAILED
/// 3. video_failed XOR audio_failed                              → DEGRADED
///    —— 行 2/3 同样不受 readiness 限制（红线 12: failed 事实必须在
///       Master 未 Ready 时仍产生 DEGRADED/FAILED, 否则 §1.20 被屏蔽）
/// 4. 无 failure/inconsistency 且 !Ready                         → None
/// 5. 无 failure/inconsistency 且 Ready                          → ACCEPTABLE
/// ```
///
/// `NotPresent`（无 Present 证据）本身**绝不触发降级**; `Participating`
/// 本身**绝不提升为 Ready**（A2-4 遗产: declaration ≠ readiness）。
pub fn join(input: &MasterJoinInput) -> MasterJoinOutput {
    let metadata_eligible = matches!(
        input.metadata.join_declaration,
        MetadataJoinDeclaration::Participating | MetadataJoinDeclaration::NotPresent
    );
    let eligibility = JoinEligibility {
        video: input.video.is_program_scope_master(),
        audio: input.audio.is_program_scope_master(),
        metadata: metadata_eligible,
        ready: false, // 下方合取
    };
    let ready = eligibility.video && eligibility.audio && eligibility.metadata;
    let eligibility = JoinEligibility {
        ready,
        ..eligibility
    };

    // C′ 矛盾快照（A2-4 终裁: Join 必须 fail-closed, 不得按有效 NotPresent 消费）。
    let inconsistency = input.metadata.join_declaration == MetadataJoinDeclaration::NotPresent
        && input
            .metadata
            .facts
            .iter()
            .any(|f| f.presence == MetadataPresence::Present);

    // 终裁真值表行 1/2 同产 FAILED（C′ 矛盾与双媒体路 failed——短路序保持
    // inconsistency 语义优先计入 classification_input, 行为由优先序测试锁定）。
    let result = if inconsistency || (input.video_failed && input.audio_failed) {
        Some(MasterJoinResult::Failed)
    } else if input.video_failed || input.audio_failed {
        Some(MasterJoinResult::Degraded)
    } else if !ready {
        None
    } else {
        Some(MasterJoinResult::Acceptable)
    };

    MasterJoinOutput {
        eligibility,
        result,
        classification_input: JoinClassificationInput {
            avsync: input.avsync,
            inconsistency,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::CanonicalSourceRef;
    use crate::program::audio_master::AudioMasterStage;
    use crate::program::metadata_master::{MetadataFact, MetadataType};
    use crate::program::video_master::VideoMasterStage;
    use uuid::Uuid;

    fn src() -> CanonicalSourceRef {
        CanonicalSourceRef {
            device_id: Uuid::nil(),
            port_id: None,
        }
    }

    fn fact(presence: MetadataPresence) -> MetadataFact {
        MetadataFact {
            kind: MetadataType::Caption,
            source: src(),
            presence,
        }
    }

    /// 三 Master 全终态 Ready 的输入底座（单测各字段覆写）。
    fn ready_base() -> MasterJoinInput {
        MasterJoinInput {
            video: VideoMaster {
                stage: VideoMasterStage::MasterJoined,
                ..VideoMaster::new()
            },
            audio: AudioMaster {
                stage: AudioMasterStage::MasterJoined,
                ..AudioMaster::new()
            },
            metadata: MetadataMaster {
                join_declaration: MetadataJoinDeclaration::Participating,
                ..MetadataMaster::new()
            },
            avsync: AVSyncClassification::Unknown,
            video_failed: false,
            audio_failed: false,
        }
    }

    /// 词表快照锁 + wire 名 + **跨平面污染拒收**（Ready/NotReady/Restart/
    /// Unknown 不得进入 Result; Unknown 不得进 Result 但属于 AVSync）。
    #[test]
    fn program_rt_05_join_result_vocabulary_lock() {
        assert_eq!(JOIN_RESULTS, &["ACCEPTABLE", "DEGRADED", "FAILED"]);
        for (v, wire) in [
            (MasterJoinResult::Acceptable, "ACCEPTABLE"),
            (MasterJoinResult::Degraded, "DEGRADED"),
            (MasterJoinResult::Failed, "FAILED"),
        ] {
            assert_eq!(serde_json::to_string(&v).unwrap(), format!("\"{wire}\""));
            assert_eq!(
                serde_json::from_str::<MasterJoinResult>(&serde_json::to_string(&v).unwrap())
                    .unwrap(),
                v
            );
        }
        // Result 拒跨平面值（Readiness 词/健康词/未测词均禁入 Result）。
        for bad in [
            "READY",
            "NOT_READY",
            "RESTART",
            "ESCALATE",
            "UNKNOWN",
            "NONE",
        ] {
            assert!(serde_json::from_str::<MasterJoinResult>(&format!("\"{bad}\"")).is_err());
        }
        assert_eq!(
            AVSYNC_CLASSIFICATIONS,
            &["ACCEPTABLE", "DEGRADED", "FAILED", "UNKNOWN"]
        );
        assert_eq!(
            serde_json::to_string(&AVSyncClassification::default()).unwrap(),
            "\"UNKNOWN\""
        );
    }

    /// Eligibility: Video/Audio 复用终态判定（中间态不可参与）; Metadata
    /// Participating/NotPresent 有效、Unknown 声明未成; ready 三合取（8 组合表驱动）。
    #[test]
    fn program_rt_05_join_eligibility_matrix() {
        let mut m = ready_base();
        assert!(join(&m).eligibility.ready, "终态+Participating = Ready");
        m.video.stage = VideoMasterStage::Switched;
        let o = join(&m);
        assert!(!o.eligibility.video && !o.eligibility.ready && o.result.is_none());
        m.video.stage = VideoMasterStage::MasterJoined;
        m.audio.stage = AudioMasterStage::LoudnessNormalized;
        assert!(!join(&m).eligibility.audio && !join(&m).eligibility.ready);
        m.audio.stage = AudioMasterStage::MasterJoined;
        m.metadata.join_declaration = MetadataJoinDeclaration::NotPresent;
        assert!(join(&m).eligibility.metadata, "NotPresent 是有效声明");
        m.metadata.join_declaration = MetadataJoinDeclaration::Unknown;
        let o = join(&m);
        assert!(!o.eligibility.metadata && !o.eligibility.ready && o.result.is_none());
        // ready 合取全 8 组合。
        for (v, a, md, want) in [
            (true, true, true, true),
            (true, true, false, false),
            (true, false, true, false),
            (false, true, true, false),
            (true, false, false, false),
            (false, true, false, false),
            (false, false, true, false),
            (false, false, false, false),
        ] {
            let mut m = ready_base();
            if !v {
                m.video.stage = VideoMasterStage::SourceRaw;
            }
            if !a {
                m.audio.stage = AudioMasterStage::SourceRaw;
            }
            if !md {
                m.metadata.join_declaration = MetadataJoinDeclaration::Unknown;
            }
            assert_eq!(join(&m).eligibility.ready, want, "ready({v},{a},{md})");
        }
    }

    /// Result 五步优先序（终裁 §2 真值表逐行; **failure/C′ 不受 readiness gate**）。
    #[test]
    fn program_rt_05_join_result_priority_matrix() {
        // 行 1: C′ 矛盾 → FAILED——Video 未终态（非 Ready）仍不得被吞（红线 11）。
        let mut m = ready_base();
        m.video.stage = VideoMasterStage::SourceRaw;
        m.metadata.join_declaration = MetadataJoinDeclaration::NotPresent;
        m.metadata.facts = vec![fact(MetadataPresence::Present)];
        let o = join(&m);
        assert_eq!(o.result, Some(MasterJoinResult::Failed));
        assert!(o.classification_input.inconsistency);
        assert!(!o.eligibility.ready, "矛盾判定无需 Ready");

        // 行 2: 双媒体 failed → FAILED（非 Ready 场景——红线 12）。
        let mut m = ready_base();
        m.video.stage = VideoMasterStage::Normalized;
        m.audio.stage = AudioMasterStage::Mixed;
        m.video_failed = true;
        m.audio_failed = true;
        assert_eq!(join(&m).result, Some(MasterJoinResult::Failed));

        // 行 3: 单媒体 failed → DEGRADED（§1.20 逐字; 非 Ready 场景）。
        let mut m = ready_base();
        m.audio.stage = AudioMasterStage::SourceRaw;
        m.audio_failed = true;
        assert_eq!(join(&m).result, Some(MasterJoinResult::Degraded));

        // 行 4: 无 failure/inconsistency 且 !Ready → None（不伪造）。
        let mut m = ready_base();
        m.metadata.join_declaration = MetadataJoinDeclaration::Unknown;
        assert_eq!(join(&m).result, None);

        // 行 5: Ready 无 failure → ACCEPTABLE（含 NotPresent 负声明——不降级）。
        let mut m = ready_base();
        m.metadata.join_declaration = MetadataJoinDeclaration::NotPresent;
        m.metadata.facts = vec![fact(MetadataPresence::NotPresent)];
        assert_eq!(join(&m).result, Some(MasterJoinResult::Acceptable));

        // 优先级: C′(1) 压过单路 failed(3)。
        let mut m = ready_base();
        m.video_failed = true;
        m.metadata.join_declaration = MetadataJoinDeclaration::NotPresent;
        m.metadata.facts = vec![fact(MetadataPresence::Present)];
        assert_eq!(join(&m).result, Some(MasterJoinResult::Failed));
    }

    /// 红线 3: `AVSyncClassification::Failed` 不快捷转 Join Degraded——
    /// red 后须 Runtime classify_failure_domain（§8.10）; Join 只透传。
    #[test]
    fn program_rt_05_join_avsync_does_not_mutate_result() {
        let mut m = ready_base();
        m.avsync = AVSyncClassification::Failed;
        let o = join(&m);
        assert_eq!(
            o.result,
            Some(MasterJoinResult::Acceptable),
            "AVSync red 不改 Result"
        );
        assert_eq!(
            o.classification_input.avsync,
            AVSyncClassification::Failed,
            "伴随透传"
        );
        let mut m2 = ready_base();
        m2.metadata.join_declaration = MetadataJoinDeclaration::Unknown;
        m2.avsync = AVSyncClassification::Degraded;
        let o2 = join(&m2);
        assert_eq!(o2.result, None, "AVSync 也不改变 None（未 Ready 不伪造）");
        assert_eq!(
            o2.classification_input.avsync,
            AVSyncClassification::Degraded
        );
    }

    /// classification_input 联动: inconsistency 恰在 C′ 矛盾时为真;
    /// 三不等式 doc 锚（Failed 无 Default——枚举层防"默认故障"）。
    #[test]
    fn program_rt_05_join_classification_input_and_invariants() {
        let o = join(&ready_base());
        assert!(!o.classification_input.inconsistency);
        assert_eq!(o.classification_input.avsync, AVSyncClassification::Unknown);
        // MasterJoinResult 无 Default（无天然默认值——默认任何结果均无依据）。
        // （编译期性质: 类型不 derive Default; 此处以词表恰三值作运行时锚。）
        assert_eq!(JOIN_RESULTS.len(), 3);
    }
}
