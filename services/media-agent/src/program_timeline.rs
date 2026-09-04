//! A2-8-C-TIMELINE-01: Program Timeline Authority —— Program Execution 层
//! 的**媒体时间线权威**（纯 Domain, 零 GStreamer）。
//!
//! 设计 SoT: `docs/superpowers/reports/2026-09-04-c-timeline-01-design-freeze.md`
//! §1-§15 + 第三十一轮 IMP 终裁（设计探针 §14）。八红线 R1-R8 全程有效:
//! R1 禁 wall-clock 修 PTS · R2 禁 `max(last+dur, incoming)` 假闭合 ·
//! R3/RR4/R5 Authority 不入 ExecutionGroup/Supervisor/MediaBackend ·
//! R6 Gst Segment/Event 是 Adapter 执行机制非 Authority · R7 格式归一化
//! 不由 Timeline 偷做 · R8 TimelineMapped ≠ TimelineHealthy。
//!
//! 分层（终裁纪律照录）: **TimelineAuthority 产生"应该怎样映射"的声明;
//! selector downstream Event/Buffer 产生"实际上发生了什么"的证据; 两者在
//! Runtime 中闭合成 TimelineMapped**——本模块只承载 Authority 与证据校验,
//! 不执行任何 GStreamer 行为（执行面=SwitchGraph Adapter, 第二批）。
//!
//! 微观序（IMP-5 ①-⑩ 冻结）: 取锚 → 声明 → install → active-pad 翻转 →
//! Segment(B) event → 下一枚 B 实际 buffer → mapping → TimelineMapped →
//! settle → Stable。**生效边界 = "事件确认 + 下一 Buffer"**; active-pad
//! readback 只能做辅助 observation, 不是 Timeline 生效边界（F6）。
//!
//! epoch 口径（终裁 §十一, 与 Freeze §3 字面差异已披露=探针 §14.5.2）:
//! Preserve=同世代延续（epoch 不变）/ NewEpoch=连续性不可证→epoch+1
//! （禁硬接 PTS, 禁 max 假闭合）/ FailClosed=终态非 Stable。

use crate::pipeline::PtsMonotonicity;
use uuid::Uuid;

// ── 身份 ──────────────────────────────────────────────────────────────

/// 媒体语义时间线世代（≠ switch_epoch 执行事件计数; Freeze §3）。
/// Video/Audio **共享同一 epoch**（同一次切换同一世代）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ProgramEpoch(pub u64);

/// 段世代 id（一次声明的 video/audio 两段共享同一 id——段世代对切换唯一）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SegmentId(pub u64);

/// 媒体平面（Video/Audio 各自独立 PTS/映射, 不共享数值序列; Freeze §9）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MediaPlane {
    Video,
    Audio,
}

impl std::fmt::Display for MediaPlane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MediaPlane::Video => "video",
            MediaPlane::Audio => "audio",
        })
    }
}

// ── 声明（Authority 产出 → Adapter 安装）──────────────────────────────

/// 声明用锚对（IMP-5 ①）: `mapping = program_anchor − source_anchor`
/// （Freeze §4 照录——保存 Segment 结构, 非 last_program_pts 单值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnchorPair {
    /// Program 时间线连续性锚（B 首帧应落位的 Program 位置——Adapter 观测）。
    pub program_anchor: u64,
    /// Source B 连续性锚（B 侧对应帧的源 PTS——Adapter 观测）。
    pub source_anchor: u64,
}

/// Source Segment——某源进入 Program 的段映射声明（Freeze §4 结构照录）。
///
/// 段内映射: `program_pts = source_pts + offset`; 永禁 R1/R2 假闭合。
/// offset 用 i64（ns）: 映射偏移=帧级相位差量级, 不承载 wall-clock 大偏移
/// （R1 语义下恒小）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceSegment {
    pub source_id: Uuid,
    pub program_epoch: ProgramEpoch,
    pub segment_id: SegmentId,
    pub source_start_pts: u64,
    pub program_start_pts: u64,
    /// `program_start_pts − source_start_pts`（declare 单点计算）。
    pub offset: i64,
}

impl SourceSegment {
    /// 初始恒等段（Program 时间线以初始源锚定: offset 0——program=source）。
    pub fn identity(source_id: Uuid, program_epoch: ProgramEpoch, segment_id: SegmentId) -> Self {
        Self {
            source_id,
            program_epoch,
            segment_id,
            source_start_pts: 0,
            program_start_pts: 0,
            offset: 0,
        }
    }

    /// 声明段（唯一生产路径）: offset = program_anchor − source_anchor。
    pub fn declare(
        source_id: Uuid,
        program_epoch: ProgramEpoch,
        segment_id: SegmentId,
        anchors: AnchorPair,
    ) -> Result<Self, TransitionFailure> {
        let offset = i64::try_from(anchors.program_anchor)
            .ok()
            .and_then(|p| p.checked_sub(i64::try_from(anchors.source_anchor).ok()?))
            .ok_or(TransitionFailure::AnchorOutOfRange {
                plane: None,
                program_anchor: anchors.program_anchor,
                source_anchor: anchors.source_anchor,
            })?;
        Ok(Self {
            source_id,
            program_epoch,
            segment_id,
            source_start_pts: anchors.source_anchor,
            program_start_pts: anchors.program_anchor,
            offset,
        })
    }

    /// 段内映射 `f(source_pts)`（越界=None, 调用方 fail-closed）。
    pub fn map_pts(&self, source_pts: u64) -> Option<u64> {
        i64::try_from(source_pts)
            .ok()
            .and_then(|s| s.checked_add(self.offset))
            .and_then(|v| u64::try_from(v).ok())
    }
}

/// ProgramTimelinePlan——Authority 声明 → Adapter 安装的执行计划
/// （IMP-2 纠偏链: Runtime→TimelineAuthority→Plan→Adapter; Freeze §11）。
/// video/audio 两段共享 target/switch_epoch/segment 世代 id, 数值序列独立。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramTimelinePlan {
    pub target: Uuid,
    /// 与 `SwitchExecutionPlan.epoch` 的身份联动（on_switch_executed 校验）。
    pub switch_epoch: u64,
    pub video: SourceSegment,
    pub audio: SourceSegment,
}

impl ProgramTimelinePlan {
    fn plane_segment(&self, plane: MediaPlane) -> &SourceSegment {
        match plane {
            MediaPlane::Video => &self.video,
            MediaPlane::Audio => &self.audio,
        }
    }
}

// ── 证据（evidence ≠ authority; IMP-4）────────────────────────────────

/// 平面连续性证据（禁裸 bool——词表纪律）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaneContinuity {
    /// 无证据（未观测到该平面映射后缓冲）——absence ≠ evidence。
    Unproven,
    /// 连续成立（映射后 PTS 单调且与声明一致）。
    Continuous,
    /// 声明的合法世代边界（NewEpoch/rebase——非违反, 亦非连续声称）。
    DeclaredDiscontinuity,
    /// 违反（未声明回退/与声明不符）。
    Violated,
}

/// 未声明回退事实（FailClosed 证据载荷）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackwardJumpFact {
    pub plane: MediaPlane,
    pub observed: u64,
    pub last_program_pts: u64,
}

/// Timeline Transition 证据（IMP-7 结构照录; L4 九项合取的输入）。
///
/// 单行字段（source_pts/mapped_program_pts/mapping_offset）以 **video 边界帧
/// 为规范载体**（L4 之问的"B 首帧"=video 边界帧; audio 独立性由
/// audio_continuity 与 PlaneTimeline 承载——§8 冻结单行形状内的实现约定）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineTransitionEvidence {
    pub declared_segment: SegmentId,
    pub observed_segment: SegmentId,
    pub program_epoch: ProgramEpoch,
    pub source_id: Uuid,
    pub source_pts: u64,
    pub mapped_program_pts: u64,
    pub mapping_offset: i64,
    pub video_continuity: PlaneContinuity,
    pub audio_continuity: PlaneContinuity,
    pub discontinuity_state: PtsMonotonicity,
    /// None = 无未声明回退（禁裸 bool）。
    pub undeclared_backward_jump: Option<BackwardJumpFact>,
}

/// TimelineMapped Execution Fact（Freeze §7 结构照录; ≠ TimelineHealthy R8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineMapped {
    pub program_epoch: ProgramEpoch,
    pub source_id: Uuid,
    pub segment_id: SegmentId,
    /// video 段映射（规范载体约定同证据单行字段）。
    pub mapping: SourceSegment,
    pub evidence: TimelineTransitionEvidence,
}

/// TimelineObservation——专门证据面（Freeze §8 结构照录, 键集恰十键锁）。
///
/// `observed_at_ms` = wall clock（**观察层, 绝不用于计算 program_pts**——R1）。
/// 单行字段以 video 平面为规范载体（实现约定, 见
/// TimelineTransitionEvidence 文档）; 全 None/Unknown 行=诚实缺席
/// （Adapter 侧执行层未实装/未观测——absence ≠ evidence）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimelineObservation {
    pub program_epoch: ProgramEpoch,
    pub source_id: Option<Uuid>,
    pub segment_id: Option<SegmentId>,
    pub input_pts: Option<u64>,
    pub mapped_program_pts: Option<u64>,
    pub mapping_offset: Option<i64>,
    pub discontinuity_state: PtsMonotonicity,
    pub video_continuity: PlaneContinuity,
    pub audio_continuity: PlaneContinuity,
    pub observed_at_ms: u64,
}

impl TimelineObservation {
    /// 无证据行（absence ≠ evidence——不伪造任何事实）。**program_epoch
    /// 携带当前已知 epoch**（第三十二轮前置②: 固定 0 会使"0"变成看起来
    /// 真实的值——缺席行=epoch 如实 + 其余字段缺席; 十键形状不改 Option）。
    pub fn no_evidence(program_epoch: ProgramEpoch, observed_at_ms: u64) -> Self {
        Self {
            program_epoch,
            source_id: None,
            segment_id: None,
            input_pts: None,
            mapped_program_pts: None,
            mapping_offset: None,
            discontinuity_state: PtsMonotonicity::Unknown,
            video_continuity: PlaneContinuity::Unproven,
            audio_continuity: PlaneContinuity::Unproven,
            observed_at_ms,
        }
    }
}

// ── 三结局（IMP-6; 不增第四种"猜测成功"）──────────────────────────────

/// 失败/拒收封闭词表（fail-closed, 全部可观测）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransitionFailure {
    #[error("timeline operation requires another phase (operation: {operation})")]
    InvalidPhase { operation: &'static str },
    #[error("timeline target {0} is the current active source")]
    TargetEqualsActive(Uuid),
    #[error("timeline anchor out of representable range (program={program_anchor} source={source_anchor} plane={plane:?})")]
    AnchorOutOfRange {
        plane: Option<MediaPlane>,
        program_anchor: u64,
        source_anchor: u64,
    },
    /// Buffers 已流入而无已安装声明（Runtime/Adapter 检出上报）。
    #[error("timeline mapping missing while buffers are flowing")]
    MappingMissing,
    /// 观测的 mapped ≠ f(source_pts)——证据与声明不符（IMP-4: 不自动接受）。
    #[error(
        "timeline mapping mismatch on {plane}: observed {observed:?} declared-mapped {expected:?}"
    )]
    MappingMismatch {
        plane: MediaPlane,
        observed: u64,
        expected: Option<u64>,
    },
    /// Event/Buffer 的 source ≠ 声明 target——身份三件闭合失败（终裁 §八）。
    #[error("timeline segment mismatch on {plane}: observed source {observed_source} declared {declared_target}")]
    SegmentMismatch {
        plane: MediaPlane,
        observed_source: Uuid,
        declared_target: Uuid,
    },
    #[error("timeline epoch inconsistent: got {got} expected {expected}")]
    EpochInconsistent { got: u64, expected: u64 },
    /// 稳态观测到回退且无任何声明覆盖（≠ DiscontinuityDeclared）。
    #[error("undeclared backward jump on {plane}: observed {observed} last {last}")]
    UndeclaredBackwardJump {
        plane: MediaPlane,
        observed: u64,
        last: u64,
    },
    /// 证据次序违反微观序（如 buffer 先于 Segment event——⑤ 先于 ⑥）。
    #[error("timeline evidence out of canonical order on {plane}")]
    EvidenceOutOfOrder { plane: MediaPlane },
    #[error("timeline evidence insufficient (pending planes: {pending:?})")]
    EvidenceInsufficient { pending: Vec<MediaPlane> },
}

/// 切换三结局（IMP-6 照录: Preserve/NewEpoch/FailClosed）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionOutcome {
    /// 双平面映射连续性成立——Program 时间线**同世代延续**（epoch 不变;
    /// 终裁 §十一）。TimelineMapped=Execution Fact（≠ TimelineHealthy R8）。
    Preserved {
        epoch: ProgramEpoch,
        mapped: TimelineMapped,
    },
    /// 执行成功但连续性不可证——**世代推进（epoch+1）**, 新段按观测实况
    /// re-base（offset=已观测值, 不改旧 PTS——R2 绝对禁区）。
    NewEpoch {
        epoch: ProgramEpoch,
        video: SourceSegment,
        audio: SourceSegment,
    },
    /// 拒收终态——不得继续把 Program 当作正常 Stable。
    Failed { reason: TransitionFailure },
}

// ── 状态机（OQ-5/Freeze §10 + IMP-5 ①-⑩）────────────────────────────

/// Plane 证据推进态（⑤→⑥⑦; 每 plane 独立）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaneTransitionState {
    /// 待 Segment(target) 事件证实（⑤）。
    AwaitSegmentEvent,
    /// Segment 已证实, 待首枚 B 映射缓冲（⑥⑦）。
    AwaitFirstMappedBuffer,
    /// 首枚映射缓冲已观测并经 Authority 校验（⑥⑦ 完成）。
    Mapped,
}

/// Program 级阶段（Stable→SwitchRequested→SwitchExecuted→
/// TimelineTransition→Stable; TransitionFailed=IMP-6 FailClosed 终态）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelinePhase {
    /// 稳态（无进行中 transition; source 独占）。
    Stable { source: Uuid },
    /// ② 已声明（段+映射已产出, 待 Adapter 安装/翻转）。
    SwitchRequested { from: Uuid, to: Uuid },
    /// ④ active-pad 已翻（adapter switch 返回）, 待 ⑤⑥ 证据。
    SwitchExecuted { from: Uuid, to: Uuid },
    /// ⑧ 双平面证据闭合——settle 段（⑨: 映射已生效, 待稳定确认）。
    TimelineTransition {
        to: Uuid,
        outcome: TransitionOutcome,
    },
    /// FailClosed 终态（recover/人工介入前停留——非 Stable）。
    TransitionFailed { reason: TransitionFailure },
}

/// 单平面时间线（current segment/映射/连续性——终裁 §七 形状）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaneTimeline {
    pub current_source: Uuid,
    pub current_segment: SourceSegment,
    pub transition: Option<PlaneTransitionState>,
    pub last_source_pts: Option<u64>,
    pub last_program_pts: Option<u64>,
    pub pts_state: PtsMonotonicity,
    pub continuity: PlaneContinuity,
    /// ⑥⑦ 观测到的边界帧（source/mapped 双记录）。
    pub boundary: Option<(u64, u64)>,
}

// ── TimelineAuthority ────────────────────────────────────────────────

/// Program Execution 层时间线权威（组合入 ProgramExecutionRuntime——第二批;
/// 不入 ExecutionGroup/Supervisor/MediaBackend, R3/R4/R5）。
///
/// 纯状态机: 一切输入是 Adapter/ Runtime 上报的**证据**, Authority 校验后
/// 推进（IMP-4: evidence 不是 authority, 禁自动接受）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineAuthority {
    epoch: ProgramEpoch,
    segment_counter: u64,
    video: PlaneTimeline,
    audio: PlaneTimeline,
    phase: TimelinePhase,
    plan: Option<ProgramTimelinePlan>,
    last_outcome: Option<TransitionOutcome>,
    /// 段历史（**只增不改**——Segment=immutable 历史事实; 当前运行映射=
    /// PlaneTimeline.current_segment。第三十二轮 §14.3 风险 3 锁: 禁把
    /// TimelineMapped.mapping 当 immutable history 唯一仓库）。
    video_history: Vec<SourceSegment>,
    audio_history: Vec<SourceSegment>,
}

impl TimelineAuthority {
    /// 初始权威: epoch 0 + 初始源恒等段（Program 时间线以初始源锚定）。
    pub fn new(initial_source: Uuid) -> Self {
        let epoch = ProgramEpoch(0);
        let seg = SegmentId(0);
        let identity = SourceSegment::identity(initial_source, epoch, seg);
        let plane = |src: Uuid| PlaneTimeline {
            current_source: src,
            current_segment: identity,
            transition: None,
            last_source_pts: None,
            last_program_pts: None,
            pts_state: PtsMonotonicity::Unknown,
            continuity: PlaneContinuity::Unproven,
            boundary: None,
        };
        Self {
            epoch,
            segment_counter: 0,
            video: plane(initial_source),
            audio: plane(initial_source),
            phase: TimelinePhase::Stable {
                source: initial_source,
            },
            plan: None,
            last_outcome: None,
            video_history: vec![identity],
            audio_history: vec![identity],
        }
    }

    /// 段历史（只增不改——per plane; 索引序=提交序）。
    pub fn segment_history(&self, plane: MediaPlane) -> &[SourceSegment] {
        match plane {
            MediaPlane::Video => &self.video_history,
            MediaPlane::Audio => &self.audio_history,
        }
    }

    pub fn phase(&self) -> &TimelinePhase {
        &self.phase
    }

    pub fn epoch(&self) -> ProgramEpoch {
        self.epoch
    }

    pub fn last_outcome(&self) -> Option<&TransitionOutcome> {
        self.last_outcome.as_ref()
    }

    pub fn plane(&self, plane: MediaPlane) -> &PlaneTimeline {
        match plane {
            MediaPlane::Video => &self.video,
            MediaPlane::Audio => &self.audio,
        }
    }

    fn plane_mut(&mut self, plane: MediaPlane) -> &mut PlaneTimeline {
        match plane {
            MediaPlane::Video => &mut self.video,
            MediaPlane::Audio => &mut self.audio,
        }
    }

    /// 当前待安装声明（SwitchRequested/SwitchExecuted 阶段的 plan 只读）。
    pub fn pending_plan(&self) -> Option<&ProgramTimelinePlan> {
        self.plan.as_ref()
    }

    /// ② 声明 transition（唯一 Plan 生产路径; Stable-only, fail-closed）。
    /// video/audio 锚对独立（§9: 不共享数值序列）, 共享 epoch/段世代 id。
    pub fn declare_transition(
        &mut self,
        target: Uuid,
        switch_epoch: u64,
        video: AnchorPair,
        audio: AnchorPair,
    ) -> Result<ProgramTimelinePlan, TransitionFailure> {
        let TimelinePhase::Stable { source } = self.phase else {
            return Err(TransitionFailure::InvalidPhase {
                operation: "declare_transition",
            });
        };
        if target == source {
            return Err(TransitionFailure::TargetEqualsActive(target));
        }
        let segment_id = SegmentId(self.segment_counter + 1);
        let plan = ProgramTimelinePlan {
            target,
            switch_epoch,
            video: SourceSegment::declare(target, self.epoch, segment_id, video)?,
            audio: SourceSegment::declare(target, self.epoch, segment_id, audio)?,
        };
        self.segment_counter += 1;
        self.video.transition = Some(PlaneTransitionState::AwaitSegmentEvent);
        self.audio.transition = Some(PlaneTransitionState::AwaitSegmentEvent);
        self.phase = TimelinePhase::SwitchRequested {
            from: source,
            to: target,
        };
        self.plan = Some(plan);
        Ok(self.plan.expect("plan 已置"))
    }

    /// 执行失败回滚（adapter `switch` 返回 Err 时）: 声明作废, 回 Stable
    /// 旧源——epoch/世代不变, 时间线零变化（失败路径 ⑨ 处置）。
    pub fn abort_transition(&mut self) -> Result<(), TransitionFailure> {
        let restore = match self.phase.clone() {
            TimelinePhase::SwitchRequested { from, .. }
            | TimelinePhase::SwitchExecuted { from, .. } => Some(from),
            _ => None,
        };
        match restore {
            Some(source) => {
                self.phase = TimelinePhase::Stable { source };
                self.plan = None;
                for p in [&mut self.video, &mut self.audio] {
                    p.transition = None;
                    p.boundary = None;
                }
                Ok(())
            }
            None => Err(TransitionFailure::InvalidPhase {
                operation: "abort_transition",
            }),
        }
    }

    fn plan_of(&self) -> Result<&ProgramTimelinePlan, TransitionFailure> {
        self.plan.as_ref().ok_or(TransitionFailure::MappingMissing)
    }

    /// ⑤⑥⑦ 证据只认 **SwitchExecuted 之后**（微观序: active-pad 翻转 →
    /// Segment event → 下一枚 B buffer; IMP-5 冻结序）。
    fn exec_ctx(&self) -> Result<Uuid, TransitionFailure> {
        match self.phase {
            TimelinePhase::SwitchExecuted { to, .. } => Ok(to),
            _ => Err(TransitionFailure::InvalidPhase {
                operation: "transition_evidence",
            }),
        }
    }

    /// ④ adapter 已执行切换（active-pad 翻转完成——SwitchExecuted 事件）。
    /// 校验 switch_epoch 与声明的联动（身份: 声明↔执行同一计划）。
    pub fn on_switch_executed(&mut self, switch_epoch: u64) -> Result<(), TransitionFailure> {
        let ctx = match self.phase.clone() {
            TimelinePhase::SwitchRequested { from, to } => Some((from, to)),
            _ => None,
        };
        match ctx {
            Some((from, to)) => {
                let expected = self.plan_of()?.switch_epoch;
                if switch_epoch != expected {
                    return Err(TransitionFailure::EpochInconsistent {
                        got: switch_epoch,
                        expected,
                    });
                }
                self.phase = TimelinePhase::SwitchExecuted { from, to };
                Ok(())
            }
            None => Err(TransitionFailure::InvalidPhase {
                operation: "on_switch_executed",
            }),
        }
    }

    /// ⑤ Segment(target) 事件被 downstream 事件流证实（每 plane 独立;
    /// 身份三件闭合之一——声明+Event+Buffer, 禁瞬时 readback）。
    pub fn on_segment_event(
        &mut self,
        plane: MediaPlane,
        observed_source: Uuid,
    ) -> Result<(), TransitionFailure> {
        let target = self.exec_ctx()?;
        if observed_source != target {
            return Err(TransitionFailure::SegmentMismatch {
                plane,
                observed_source,
                declared_target: target,
            });
        }
        // 顺序违反=拒收当前证据（transition 仍在途, 不进终态）。
        let state = self.plane(plane).transition;
        if !matches!(state, Some(PlaneTransitionState::AwaitSegmentEvent)) {
            return Err(TransitionFailure::EvidenceOutOfOrder { plane });
        }
        self.plane_mut(plane).transition = Some(PlaneTransitionState::AwaitFirstMappedBuffer);
        Ok(())
    }

    /// ⑥⑦ 首枚 B 实际缓冲到达 selector 输出侧且映射已施加——Authority 校验
    /// 证据与声明一致（mapped == f(source_pts); IMP-4 不自动接受）并记录
    /// 连续性。双平面齐备时闭合 ⑧（Preserve/NewEpoch）。
    pub fn on_mapped_buffer(
        &mut self,
        plane: MediaPlane,
        source_id: Uuid,
        source_pts: u64,
        mapped_pts: u64,
    ) -> Result<(), TransitionFailure> {
        let target = self.exec_ctx()?;
        if source_id != target {
            return Err(TransitionFailure::SegmentMismatch {
                plane,
                observed_source: source_id,
                declared_target: target,
            });
        }
        let plan = *self.plan_of()?;
        let seg = plan.plane_segment(plane);
        let expected = seg.map_pts(source_pts);
        if expected != Some(mapped_pts) {
            return self.fail_closed(TransitionFailure::MappingMismatch {
                plane,
                observed: mapped_pts,
                expected,
            });
        }
        // 顺序违反=拒收当前证据（⑥ 必须在 ⑤ 之后; transition 仍在途）。
        let state = self.plane(plane).transition;
        if !matches!(state, Some(PlaneTransitionState::AwaitFirstMappedBuffer)) {
            return Err(TransitionFailure::EvidenceOutOfOrder { plane });
        }
        let p = self.plane_mut(plane);
        p.transition = Some(PlaneTransitionState::Mapped);
        // 四态纪律（§6）: 已声明边界被观测证实 → DiscontinuityDeclared
        // （合法边界事实; 非 NonMonotonic, 亦不洗成普通 ValidMonotonic）。
        p.pts_state = PtsMonotonicity::DiscontinuityDeclared;
        p.last_source_pts = Some(source_pts);
        // 连续性: mapped ≥ 观测到的上一 Program 位置（None=无前值, 视为连续）。
        let continuity = match p.last_program_pts {
            Some(last) if mapped_pts < last => PlaneContinuity::Unproven,
            _ => PlaneContinuity::Continuous,
        };
        p.continuity = continuity;
        p.boundary = Some((source_pts, mapped_pts));
        p.last_program_pts = Some(mapped_pts);
        // 双平面齐备 → ⑧ 闭合。
        if matches!(self.video.transition, Some(PlaneTransitionState::Mapped))
            && matches!(self.audio.transition, Some(PlaneTransitionState::Mapped))
        {
            self.close_transition();
        }
        Ok(())
    }

    /// ⑧ 闭合: 组装证据并裁三结局（Preserve/NewEpoch; 矛盾已在证据入口
    /// FailClosed）。
    fn close_transition(&mut self) {
        let plan = *self.plan_of().expect("闭合时 plan 必在");
        let target = plan.target;
        let (v_boundary, a_boundary) = (self.video.boundary, self.audio.boundary);
        let (v_boundary, a_boundary) = (
            v_boundary.expect("Mapped 平面必有边界帧"),
            a_boundary.expect("Mapped 平面必有边界帧"),
        );
        let evidence = TimelineTransitionEvidence {
            declared_segment: plan.video.segment_id,
            observed_segment: plan.video.segment_id,
            program_epoch: self.epoch,
            source_id: target,
            source_pts: v_boundary.0,
            mapped_program_pts: v_boundary.1,
            mapping_offset: plan.video.offset,
            video_continuity: self.video.continuity,
            audio_continuity: self.audio.continuity,
            discontinuity_state: self.video.pts_state,
            undeclared_backward_jump: None,
        };
        let preserved = self.video.continuity == PlaneContinuity::Continuous
            && self.audio.continuity == PlaneContinuity::Continuous;
        let outcome = if preserved {
            // Preserve（§十一: epoch 不变——同世代延续）。
            let mapped = TimelineMapped {
                program_epoch: self.epoch,
                source_id: target,
                segment_id: plan.video.segment_id,
                mapping: plan.video,
                evidence,
            };
            for (p, seg) in [(&mut self.video, plan.video), (&mut self.audio, plan.audio)] {
                p.current_source = target;
                p.current_segment = seg;
            }
            TransitionOutcome::Preserved {
                epoch: self.epoch,
                mapped,
            }
        } else {
            // NewEpoch（§十一: 连续性不可证 → epoch+1; 段按观测实况 re-base
            // ——offset=已观测映射值, 不改旧 PTS——R2）。
            let new_epoch = ProgramEpoch(self.epoch.0 + 1);
            let segment_id = SegmentId(self.segment_counter + 1);
            let rebased = |boundary: (u64, u64), seg: &SourceSegment| SourceSegment {
                source_id: target,
                program_epoch: new_epoch,
                segment_id,
                source_start_pts: boundary.0,
                program_start_pts: boundary.1,
                offset: seg.offset,
            };
            let video = rebased(v_boundary, &plan.video);
            let audio = rebased(a_boundary, &plan.audio);
            for (p, seg, boundary) in [
                (&mut self.video, video, v_boundary),
                (&mut self.audio, audio, a_boundary),
            ] {
                p.current_source = target;
                p.current_segment = seg;
                p.last_program_pts = Some(boundary.1);
                p.pts_state = PtsMonotonicity::DiscontinuityDeclared;
                p.continuity = if p.continuity == PlaneContinuity::Continuous {
                    PlaneContinuity::Continuous
                } else {
                    PlaneContinuity::DeclaredDiscontinuity
                };
            }
            self.epoch = new_epoch;
            self.segment_counter += 1;
            TransitionOutcome::NewEpoch {
                epoch: new_epoch,
                video,
                audio,
            }
        };
        for p in [&mut self.video, &mut self.audio] {
            p.transition = None;
            p.boundary = None;
        }
        // 段历史只增不改（第三十二轮风险 3）: 提交段（Preserve=声明段 /
        // NewEpoch=rebase 段）append, 既有条目永不变异。
        self.video_history.push(self.video.current_segment);
        self.audio_history.push(self.audio.current_segment);
        self.last_outcome = Some(outcome.clone());
        self.phase = TimelinePhase::TimelineTransition {
            to: target,
            outcome,
        };
    }
    /// ⑨ settle 完成（稳定证据由 Runtime 策略判定后上报——阈值不入 Domain）
    /// → ⑩ Stable(target)。返回本次 transition 结局。
    pub fn confirm_settled(&mut self) -> Result<TransitionOutcome, TransitionFailure> {
        match self.phase.clone() {
            TimelinePhase::TimelineTransition { to, outcome } => {
                self.phase = TimelinePhase::Stable { source: to };
                self.plan = None;
                Ok(outcome)
            }
            _ => Err(TransitionFailure::InvalidPhase {
                operation: "confirm_settled",
            }),
        }
    }

    /// 稳态/settle 期间的 Program PTS 观测推进（四态机, 不洗状态）:
    /// 回退且无声明覆盖 → NonMonotonic sticky + FailClosed（终裁 §十/§十一）。
    pub fn on_program_pts(
        &mut self,
        plane: MediaPlane,
        mapped_pts: u64,
    ) -> Result<(), TransitionFailure> {
        let p = self.plane_mut(plane);
        if let Some(last) = p.last_program_pts {
            if mapped_pts < last {
                p.pts_state = PtsMonotonicity::NonMonotonic;
                p.continuity = PlaneContinuity::Violated;
                p.last_program_pts = Some(mapped_pts);
                let reason = TransitionFailure::UndeclaredBackwardJump {
                    plane,
                    observed: mapped_pts,
                    last,
                };
                return self.fail_closed(reason);
            }
        }
        if p.pts_state == PtsMonotonicity::Unknown {
            p.pts_state = PtsMonotonicity::ValidMonotonic;
        }
        // DiscontinuityDeclared/ValidMonotonic/NonMonotonic 均保持（sticky）。
        p.last_program_pts = Some(mapped_pts);
        Ok(())
    }

    /// 外部检出的 FailClosed 入口（如 mapping missing: 缓冲已流而无可言明
    /// 安装——Batch 2 Runtime/Adapter 检出上报）。
    pub fn fail_closed(&mut self, reason: TransitionFailure) -> Result<(), TransitionFailure> {
        self.last_outcome = Some(TransitionOutcome::Failed {
            reason: reason.clone(),
        });
        if !matches!(self.phase, TimelinePhase::TransitionFailed { .. }) {
            self.phase = TimelinePhase::TransitionFailed {
                reason: reason.clone(),
            };
        }
        Err(reason)
    }

    /// §8 证据行投影（observed_at=观察层 wall clock——绝不用于计算 PTS, R1）。
    /// 单行字段以 video 平面为规范载体（结构文档约定）。
    pub fn snapshot(&self, observed_at_ms: u64) -> TimelineObservation {
        TimelineObservation {
            program_epoch: self.epoch,
            source_id: Some(self.video.current_source),
            segment_id: Some(self.video.current_segment.segment_id),
            input_pts: self.video.last_source_pts,
            mapped_program_pts: self.video.last_program_pts,
            mapping_offset: Some(self.video.current_segment.offset),
            discontinuity_state: self.video.pts_state,
            video_continuity: self.video.continuity,
            audio_continuity: self.audio.continuity,
            observed_at_ms,
        }
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;

    fn anchors(program: u64, source: u64) -> AnchorPair {
        AnchorPair {
            program_anchor: program,
            source_anchor: source,
        }
    }

    /// 规范微观序驱动（①-⑩ 全链, anchors 由参数注入）。
    fn run_canonical(
        authority: &mut TimelineAuthority,
        target: Uuid,
        switch_epoch: u64,
        video: AnchorPair,
        audio: AnchorPair,
    ) -> ProgramTimelinePlan {
        let plan = authority
            .declare_transition(target, switch_epoch, video, audio)
            .expect("声明");
        authority
            .on_switch_executed(switch_epoch)
            .expect("执行确认");
        authority
            .on_segment_event(MediaPlane::Video, target)
            .expect("video segment");
        authority
            .on_segment_event(MediaPlane::Audio, target)
            .expect("audio segment");
        authority
            .on_mapped_buffer(
                MediaPlane::Video,
                target,
                video.source_anchor,
                video.program_anchor,
            )
            .expect("video 首枚映射缓冲");
        authority
            .on_mapped_buffer(
                MediaPlane::Audio,
                target,
                audio.source_anchor,
                audio.program_anchor,
            )
            .expect("audio 首枚映射缓冲");
        plan
    }

    #[test]
    fn timeline_rt_01_initial_identity_segment_and_epoch_zero() {
        // 初始权威: epoch 0 + 恒等段（program=source, offset 0）; phase Stable;
        // 快照诚实（无 PTS 观测=None/Unknown/Unproven——absence≠evidence）。
        let a = Uuid::new_v4();
        let mut authority = TimelineAuthority::new(a);
        assert_eq!(authority.epoch(), ProgramEpoch(0));
        assert_eq!(authority.phase(), &TimelinePhase::Stable { source: a });
        assert_eq!(authority.plane(MediaPlane::Video).current_segment.offset, 0);
        let snap = authority.snapshot(1_000);
        assert_eq!(snap.source_id, Some(a));
        assert_eq!(snap.segment_id, Some(SegmentId(0)));
        assert_eq!(snap.mapping_offset, Some(0));
        assert_eq!(snap.input_pts, None);
        assert_eq!(snap.mapped_program_pts, None);
        assert_eq!(snap.discontinuity_state, PtsMonotonicity::Unknown);
        assert_eq!(snap.video_continuity, PlaneContinuity::Unproven);
        // 声明目标=当前源 → 拒收。
        assert_eq!(
            authority.declare_transition(a, 1, anchors(1, 1), anchors(1, 1)),
            Err(TransitionFailure::TargetEqualsActive(a))
        );
    }

    #[test]
    fn timeline_rt_01_declare_freezes_mapping_semantics_per_plane() {
        // 声明=保存 Segment 结构（offset=program_anchor−source_anchor, Freeze
        // §4）; V/A 锚独立（§9 不共享数值序列）但共享 epoch/段世代 id;
        // map_pts 段内映射可逆推。**禁 max(last+dur) 假闭合**（R2: 声明不含
        // 任何 last_program_pts 单值逻辑）。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut authority = TimelineAuthority::new(a);
        let plan = authority
            .declare_transition(b, 1, anchors(10_000, 3_000), anchors(9_000, 5_000))
            .expect("声明");
        assert_eq!(plan.video.offset, 7_000);
        assert_eq!(plan.audio.offset, 4_000, "V/A 偏移独立");
        assert_eq!(plan.video.segment_id, plan.audio.segment_id);
        assert_eq!(plan.video.program_epoch, plan.audio.program_epoch);
        assert_eq!(plan.target, b);
        assert_eq!(plan.switch_epoch, 1);
        assert_eq!(plan.video.map_pts(3_000), Some(10_000));
        assert_eq!(plan.video.map_pts(3_040), Some(10_040), "段内映射连续外推");
        // 非 Stable 再声明 → 拒收。
        assert!(matches!(
            authority.declare_transition(a, 2, anchors(1, 1), anchors(1, 1)),
            Err(TransitionFailure::InvalidPhase { .. })
        ));
    }

    #[test]
    fn timeline_rt_01_canonical_microorder_preserves_epoch_and_maps() {
        // ①-⑩ 全链（Preserve 路径）: 生效边界=Segment 事件+下一枚 B 实际
        // buffer; 双平面齐备闭合 TimelineTransition; **epoch 不变**（§十一
        // Preserve）; 证据字段=IMP-7 结构; settle→Stable(B)。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut authority = TimelineAuthority::new(a);
        // 预置 program 观测位置（连续性基准）。
        authority
            .on_program_pts(MediaPlane::Video, 4_000)
            .expect("预置 video program pts");
        authority
            .on_program_pts(MediaPlane::Audio, 8_000)
            .expect("预置 audio program pts");
        let plan = run_canonical(
            &mut authority,
            b,
            1,
            anchors(4_040, 3_000),
            anchors(8_040, 6_000),
        );
        assert!(matches!(
            authority.phase(),
            TimelinePhase::TimelineTransition { .. }
        ));
        let outcome = authority.confirm_settled().expect("settle");
        assert_eq!(authority.phase(), &TimelinePhase::Stable { source: b });
        let TransitionOutcome::Preserved { epoch, mapped } = outcome else {
            panic!("连续性成立应 Preserve, 得 {outcome:?}");
        };
        assert_eq!(epoch, ProgramEpoch(0), "Preserve=同世代延续（epoch 不变）");
        assert_eq!(mapped.source_id, b);
        assert_eq!(mapped.segment_id, plan.video.segment_id);
        assert_eq!(mapped.evidence.source_pts, 3_000);
        assert_eq!(mapped.evidence.mapped_program_pts, 4_040);
        assert_eq!(mapped.evidence.mapping_offset, 1_040);
        assert_eq!(
            mapped.evidence.video_continuity,
            PlaneContinuity::Continuous
        );
        assert_eq!(
            mapped.evidence.audio_continuity,
            PlaneContinuity::Continuous
        );
        assert_eq!(
            mapped.evidence.discontinuity_state,
            PtsMonotonicity::DiscontinuityDeclared
        );
        assert_eq!(mapped.evidence.undeclared_backward_jump, None);
        // 段生效: 当前段=B 声明段; 后续 program 观测沿映射轴单调保持
        // DiscontinuityDeclared（不洗回 ValidMonotonic——§6 边界事实保留）。
        assert_eq!(
            authority.plane(MediaPlane::Video).current_segment.offset,
            1_040
        );
        authority
            .on_program_pts(MediaPlane::Video, 4_080)
            .expect("映射后单调推进");
        assert_eq!(
            authority.plane(MediaPlane::Video).pts_state,
            PtsMonotonicity::DiscontinuityDeclared,
            "边界事实保留, 不洗状态"
        );
    }

    #[test]
    fn timeline_rt_01_evidence_authority_not_auto_accepted() {
        // IMP-4: mapped ≠ f(source_pts) → FailClosed(MappingMismatch)——
        // "GStreamer 说 PTS=xxx" 不被自动接受。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut authority = TimelineAuthority::new(a);
        authority
            .declare_transition(b, 1, anchors(10_000, 3_000), anchors(20_000, 6_000))
            .expect("声明");
        authority.on_switch_executed(1).expect("执行");
        authority
            .on_segment_event(MediaPlane::Video, b)
            .expect("段");
        let err = authority
            .on_mapped_buffer(MediaPlane::Video, b, 3_000, 99_999)
            .expect_err("伪映射必须拒收");
        assert!(matches!(
            err,
            TransitionFailure::MappingMismatch {
                observed: 99_999,
                expected: Some(10_000),
                ..
            }
        ));
        assert!(matches!(
            authority.phase(),
            TimelinePhase::TransitionFailed { .. }
        ));
    }

    #[test]
    fn timeline_rt_01_identity_closure_by_declaration_event_buffer() {
        // 终裁 §八: 身份=声明+Event+Buffer 三件闭合——任一环 source≠target
        // 即 FailClosed(SegmentMismatch)。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut authority = TimelineAuthority::new(a);
        authority
            .declare_transition(b, 1, anchors(10_000, 3_000), anchors(20_000, 6_000))
            .expect("声明");
        authority.on_switch_executed(1).expect("执行");
        let err = authority
            .on_segment_event(MediaPlane::Video, a)
            .expect_err("旧源段事件不得通过");
        assert!(matches!(err, TransitionFailure::SegmentMismatch { .. }));
        // buffer 环同样拒收（重建权威走 buffer 环）。
        let mut authority2 = TimelineAuthority::new(a);
        authority2
            .declare_transition(b, 1, anchors(10_000, 3_000), anchors(20_000, 6_000))
            .expect("声明");
        authority2.on_switch_executed(1).expect("执行");
        authority2
            .on_segment_event(MediaPlane::Video, b)
            .expect("段");
        assert!(matches!(
            authority2
                .on_mapped_buffer(MediaPlane::Video, a, 3_000, 10_000)
                .unwrap_err(),
            TransitionFailure::SegmentMismatch { .. }
        ));
    }

    #[test]
    fn timeline_rt_01_epoch_link_and_evidence_order_fail_closed() {
        // switch_epoch 联动不一致 → FailClosed; buffer 先于 Segment 事件
        // → EvidenceOutOfOrder（微观序 ⑤ 先于 ⑥ 是冻结序）。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut authority = TimelineAuthority::new(a);
        authority
            .declare_transition(b, 1, anchors(10_000, 3_000), anchors(20_000, 6_000))
            .expect("声明");
        assert!(matches!(
            authority.on_switch_executed(2).unwrap_err(),
            TransitionFailure::EpochInconsistent {
                got: 2,
                expected: 1
            }
        ));
        let mut authority2 = TimelineAuthority::new(a);
        authority2
            .declare_transition(b, 1, anchors(10_000, 3_000), anchors(20_000, 6_000))
            .expect("声明");
        authority2.on_switch_executed(1).expect("执行");
        assert!(matches!(
            authority2
                .on_mapped_buffer(MediaPlane::Video, b, 3_000, 10_000)
                .unwrap_err(),
            TransitionFailure::EvidenceOutOfOrder { .. }
        ));
        // 顺序违反=拒收当前证据, transition 仍在途（非终态）。
        assert!(matches!(
            authority2.phase(),
            TimelinePhase::SwitchExecuted { .. }
        ));
    }

    #[test]
    fn timeline_rt_01_continuity_unprovable_lands_new_epoch_no_pts_rewrite() {
        // NewEpoch（§十一）: 声明锚低于已观测 program 位置 → 连续性不可证
        // → epoch+1 + 段按观测实况 re-base（offset=已观测值, 不改旧 PTS——
        // R2 绝对禁区: 禁 max/硬接）。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut authority = TimelineAuthority::new(a);
        authority
            .on_program_pts(MediaPlane::Video, 50_000)
            .expect("预置");
        authority
            .on_program_pts(MediaPlane::Audio, 60_000)
            .expect("预置");
        // 声明映射到 40_000（< 50_000 已观测位置）——映射后边界帧将低于旧轴。
        run_canonical(
            &mut authority,
            b,
            1,
            anchors(40_000, 30_000),
            anchors(70_000, 65_000),
        );
        let outcome = authority.confirm_settled().expect("settle");
        let TransitionOutcome::NewEpoch {
            epoch,
            video,
            audio,
        } = outcome
        else {
            panic!("连续性不可证应 NewEpoch, 得 {outcome:?}");
        };
        assert_eq!(epoch, ProgramEpoch(1), "世代推进");
        assert_eq!(video.source_start_pts, 30_000, "re-base 采观测实况");
        assert_eq!(video.program_start_pts, 40_000, "不改旧 PTS（观测值原样）");
        assert_eq!(video.offset, 10_000, "offset=已观测映射值");
        assert_eq!(audio.offset, 5_000);
        // audio 锚（70_000 ≥ 60_000）本身连续——平面连续性保留; video 平面
        // = DeclaredDiscontinuity（NewEpoch 合法世代边界, 非 Violated）。
        assert_eq!(
            authority.plane(MediaPlane::Video).continuity,
            PlaneContinuity::DeclaredDiscontinuity
        );
        assert_eq!(
            authority.plane(MediaPlane::Audio).continuity,
            PlaneContinuity::Continuous
        );
        assert_eq!(authority.phase(), &TimelinePhase::Stable { source: b });
    }

    #[test]
    fn timeline_rt_01_new_epoch_rebase_offset_invariant() {
        // 第三十九轮: P1-B 撤销后的不变量锁——NewEpoch rebase 虽沿用
        // plan offset, 但 on_mapped_buffer 的映射校验（:597-606）已先证
        // mapped−source==offset, 故 boundary 差值必然与之相等。直接断言
        // 双平面 offset==program_start_pts−source_start_pts, 防未来 rebase
        // 改动破坏该不变量; 同时锁 NewEpoch 平面 pts_state=
        // DiscontinuityDeclared（合法世代边界, 不洗回 ValidMonotonic）。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut authority = TimelineAuthority::new(a);
        authority
            .on_program_pts(MediaPlane::Video, 50_000)
            .expect("预置");
        authority
            .on_program_pts(MediaPlane::Audio, 60_000)
            .expect("预置");
        run_canonical(
            &mut authority,
            b,
            1,
            anchors(40_000, 30_000),
            anchors(70_000, 65_000),
        );
        let outcome = authority.confirm_settled().expect("settle");
        let TransitionOutcome::NewEpoch {
            epoch,
            video,
            audio,
        } = outcome
        else {
            panic!("连续性不可证应 NewEpoch, 得 {outcome:?}");
        };
        assert_eq!(epoch, ProgramEpoch(1), "世代推进");
        for (name, seg) in [("video", video), ("audio", audio)] {
            let delta =
                i64::try_from(seg.program_start_pts - seg.source_start_pts).expect("差值超 i64");
            assert_eq!(
                seg.offset, delta,
                "{name} 平面 NewEpoch rebase 不变量 offset==program_start−source_start"
            );
        }
        assert_eq!(
            authority.plane(MediaPlane::Video).pts_state,
            PtsMonotonicity::DiscontinuityDeclared
        );
        assert_eq!(
            authority.plane(MediaPlane::Audio).pts_state,
            PtsMonotonicity::DiscontinuityDeclared
        );
    }

    #[test]
    fn timeline_rt_01_undeclared_backward_jump_sticky_fail_closed() {
        // 稳态回退（无声明覆盖）→ NonMonotonic sticky + FailClosed; 之后即使
        // forward 观测也不洗回（终裁 §十: 禁把 NonMonotonic 改回 ValidMonotonic）。
        let a = Uuid::new_v4();
        let mut authority = TimelineAuthority::new(a);
        authority
            .on_program_pts(MediaPlane::Video, 1_000)
            .expect("首观测");
        let err = authority
            .on_program_pts(MediaPlane::Video, 900)
            .expect_err("未声明回退必须 FailClosed");
        assert!(matches!(
            err,
            TransitionFailure::UndeclaredBackwardJump {
                observed: 900,
                last: 1_000,
                ..
            }
        ));
        assert!(matches!(
            authority.phase(),
            TimelinePhase::TransitionFailed { .. }
        ));
        authority
            .on_program_pts(MediaPlane::Video, 2_000)
            .expect("失败后观测仍记录（证据连续）");
        assert_eq!(
            authority.plane(MediaPlane::Video).pts_state,
            PtsMonotonicity::NonMonotonic,
            "sticky——不洗"
        );
    }

    #[test]
    fn timeline_rt_01_abort_returns_to_stable_without_timeline_change() {
        // adapter switch 失败回滚: 声明作废回 Stable 旧源, epoch/段世代
        // 不变, 时间线零变化; 可重新声明。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut authority = TimelineAuthority::new(a);
        authority
            .declare_transition(b, 1, anchors(10_000, 3_000), anchors(20_000, 6_000))
            .expect("声明");
        authority.abort_transition().expect("中止");
        assert_eq!(authority.phase(), &TimelinePhase::Stable { source: a });
        assert_eq!(authority.epoch(), ProgramEpoch(0));
        assert_eq!(authority.plane(MediaPlane::Video).transition, None);
        authority
            .declare_transition(b, 1, anchors(11_000, 3_000), anchors(21_000, 6_000))
            .expect("中止后可重新声明（epoch 联动重取）");
    }

    #[test]
    fn timeline_rt_01_external_fail_closed_mapping_missing() {
        // 外部检出 FailClosed 入口: mapping missing（缓冲已流而无可言明
        // 安装——Batch 2 Runtime 检出上报）→ 终态非 Stable, 不得当正常。
        let a = Uuid::new_v4();
        let mut authority = TimelineAuthority::new(a);
        authority
            .fail_closed(TransitionFailure::MappingMissing)
            .unwrap_err();
        assert!(matches!(
            authority.phase(),
            TimelinePhase::TransitionFailed {
                reason: TransitionFailure::MappingMissing
            }
        ));
        assert!(matches!(
            authority.last_outcome(),
            Some(TransitionOutcome::Failed {
                reason: TransitionFailure::MappingMissing
            })
        ));
    }

    #[test]
    fn timeline_rt_01_observation_keyset_locked_to_freeze_shape() {
        // 键集恰十键 wire 锁（Freeze §8 结构照录——字段蔓延防线）。
        let json = serde_json::to_value(TimelineObservation::no_evidence(ProgramEpoch(0), 123))
            .expect("序列化");
        let mut keys: Vec<&str> = json
            .as_object()
            .expect("对象")
            .keys()
            .map(|k| k.as_str())
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "audio_continuity",
                "discontinuity_state",
                "input_pts",
                "mapped_program_pts",
                "mapping_offset",
                "observed_at_ms",
                "program_epoch",
                "segment_id",
                "source_id",
                "video_continuity",
            ],
            "键集恰十键（§8 冻结形状）"
        );
        // no_evidence = 诚实缺席（absence≠evidence, 零伪造）。
        assert_eq!(json["program_epoch"], 0);
        assert_eq!(json["source_id"], serde_json::Value::Null);
        assert_eq!(json["discontinuity_state"], "Unknown");
        assert_eq!(json["video_continuity"], "unproven");
    }

    #[test]
    fn timeline_rt_01_no_evidence_carries_current_epoch_not_fake_zero() {
        // 第三十二轮前置②: 实际 epoch=7 时缺席行禁报 0——"0"会变成看起来
        // 真实的值（absence≠evidence）; epoch 如实携带 + 其余字段缺席。
        let row = TimelineObservation::no_evidence(ProgramEpoch(7), 42);
        assert_eq!(row.program_epoch, ProgramEpoch(7));
        assert_eq!(row.source_id, None);
        assert_eq!(row.segment_id, None);
        assert_eq!(row.mapped_program_pts, None);
        assert_eq!(row.discontinuity_state, PtsMonotonicity::Unknown);
        assert_eq!(row.video_continuity, PlaneContinuity::Unproven);
        assert_eq!(row.observed_at_ms, 42);
    }

    #[test]
    fn timeline_rt_01_segment_history_accumulates_never_overwrites() {
        // 第三十二轮风险 3 锁: Segment=immutable 历史事实（只增不改）;
        // TimelineExecutionState/PlaneTimeline.current_segment 才是当前
        // 运行映射。A→B→A 双 Preserve: 历史三段（恒等+两切换段）, 既有
        // 条目跨后续切换零变异。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut authority = TimelineAuthority::new(a);
        authority
            .on_program_pts(MediaPlane::Video, 4_000)
            .expect("基准 v");
        authority
            .on_program_pts(MediaPlane::Audio, 8_000)
            .expect("基准 a");
        run_canonical(
            &mut authority,
            b,
            1,
            anchors(4_040, 3_000),
            anchors(8_040, 6_000),
        );
        authority.confirm_settled().expect("settle 1");
        let history_after_first = authority.segment_history(MediaPlane::Video).to_vec();
        assert_eq!(history_after_first.len(), 2, "恒等段+首切换段");
        // A→B→A 回切（锚沿 B 世代连续推进——Preserve 同世代）。
        let v_now = authority.plane(MediaPlane::Video).last_program_pts.unwrap();
        let a_now = authority.plane(MediaPlane::Audio).last_program_pts.unwrap();
        run_canonical(
            &mut authority,
            a,
            2,
            anchors(v_now + 40, 3_080),
            anchors(a_now + 20, 6_020),
        );
        authority.confirm_settled().expect("settle 2");
        let history = authority.segment_history(MediaPlane::Video);
        assert_eq!(history.len(), 3, "段历史累积不覆盖");
        assert_eq!(history[0].segment_id, SegmentId(0));
        assert_eq!(history[1], history_after_first[1], "既有段条目零变异");
        assert_eq!(history[2].segment_id, SegmentId(2));
        // 声明中段未提交不入历史（abort 无痕）。
        let mut authority2 = TimelineAuthority::new(a);
        authority2
            .declare_transition(b, 1, anchors(10_000, 3_000), anchors(20_000, 6_000))
            .expect("声明");
        authority2.abort_transition().expect("中止");
        assert_eq!(authority2.segment_history(MediaPlane::Video).len(), 1);
    }

    #[test]
    fn timeline_rt_01_snapshot_reflects_authority_state_not_health() {
        // TimelineMapped ≠ TimelineHealthy（R8）: snapshot 只投影权威状态
        // （段/映射/连续性证据）, 不产出任何"健康"结论。
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut authority = TimelineAuthority::new(a);
        run_canonical(
            &mut authority,
            b,
            1,
            anchors(10_000, 3_000),
            anchors(20_000, 6_000),
        );
        authority.confirm_settled().expect("settle");
        let snap = authority.snapshot(9_999);
        assert_eq!(snap.source_id, Some(b));
        assert_eq!(snap.input_pts, Some(3_000));
        assert_eq!(snap.mapped_program_pts, Some(10_000));
        assert_eq!(snap.mapping_offset, Some(7_000));
        assert_eq!(
            snap.discontinuity_state,
            PtsMonotonicity::DiscontinuityDeclared
        );
        assert_eq!(snap.video_continuity, PlaneContinuity::Continuous);
        assert_eq!(snap.audio_continuity, PlaneContinuity::Continuous);
        assert_eq!(snap.observed_at_ms, 9_999, "wall clock 只作观察层元数据");
    }
}
