//! A2-8-01: Switch Execution Adapter SPI —— 与 `MediaBackend` **平行**的执行面契约。
//!
//! **冻结 #2（probe §7）**: `MediaBackend` 五方法是单管线生命周期语义
//! （instantiate/start/stop/recover/observe）, switch 不塞入。本 trait 是
//! Program-level 切换执行面: 消费 ExecutionGroup 的双输入 → 物化 Program
//! graph → 显式切换 → 观测。
//!
//! **冻结 #5（topology=实现细节）**: 本契约面**零 GStreamer 词**——
//! inter 系/单图 selector/任何拓扑选择是 Adapter 实现内部细节（Mock/
//! GStreamer 实现可替换, 不经 Domain/契约变更）。
//!
//! **状态三分离**: `switch()` 改变 Execution 实态（selector 端）;
//! `observe()` 返回 **Observed 平面**（实际 active + PTS 实测——命令回显
//! 之外的独立读数）。Desired 推进只认 Observed 确认
//! （`ExecutionGroup::complete_switch`）。

use crate::pipeline::{PipelineHandle, PtsMonotonicity};
use crate::program_timeline::{AnchorPair, ProgramEpoch, ProgramTimelinePlan, TimelineObservation};
use crate::switch_execution::{ExecutionGroup, SwitchError, SwitchExecutionPlan};
use uuid::Uuid;

/// 帧边界证据（T4: element property ≠ PASS——边界是切换执行的必带证据,
/// 非 Option; 缺边界证据 = 未发生合法切换）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameBoundary {
    /// 切换对齐到完整视频帧边界（FRAME_SWITCH 平面; video+audio 同 epoch 成对）。
    FrameAligned,
}

/// 已执行切换的证据——Desired 推进的唯一合法输入。
///
/// `av_epoch`: 本次切换的 video+audio **成对**纪元号（单计数器承载两平面
/// ——方案 A 语义: 成对切换由类型承载, 单面切构造不出）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SwitchExecuted {
    pub boundary: FrameBoundary,
    pub av_epoch: u64,
}

/// R58 步骤5.1: 单平面 cutover drain 确认证据（确认式 Release 的返回面
/// ——INV-F1 queue in-flight drain 以真实下游 Segment 事件证实）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaneDrainEvidence {
    /// 下游新世代 Segment 已确认（queue 保序——该 Segment 到达=其入队前
    /// 排队的全部 Arm 前旧世代缓冲已被消费门处置=排空事实锚）。
    pub segment_confirmed: bool,
    /// 该平面 cutover 丢弃帧计数（selector 探针层+appsink 消费层合计;
    /// arm 清零, 每次切换独立计数）。
    pub discarded: u64,
}

/// R58 步骤5.1: 双平面 cutover drain 确认证据（Both-confirmed 原子
/// Release 的返回面; generation=fence arm 计数——观测/证据面对账用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CutoverDrainEvidence {
    pub generation: u64,
    pub video: PlaneDrainEvidence,
    pub audio: PlaneDrainEvidence,
}

/// 单输入双平面 PTS 观测（六路 PTS 观测面中的输入四路; 复用
/// `PtsMonotonicity` 三态——absence≠evidence）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct InputPts {
    pub device_id: Uuid,
    pub video_pts: Option<u64>,
    pub audio_pts: Option<u64>,
    pub video_pts_state: PtsMonotonicity,
    pub audio_pts_state: PtsMonotonicity,
    /// 注入/观测到的停滞（帧计数冻结）——Observation 事实, 非故障结论
    /// （归因属 Custody/fold, 此处只报证据）。
    pub stalled: bool,
}

/// Observed 平面快照（一次 `observe()` 的完整读数）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProgramObservation {
    /// 实际活跃源（video+audio 联合判定; None = 未运行/无证据）。
    pub observed_active: Option<Uuid>,
    /// video 平面实际 active（T5 成对校验: 应与 audio 平面一致）。
    pub video_active: Option<Uuid>,
    /// audio 平面实际 active。
    pub audio_active: Option<Uuid>,
    /// adapter 侧已执行切换计数（与 `SwitchExecuted.av_epoch` 同源）。
    pub switch_epoch: u64,
    /// 组内各输入的 PTS 观测（每设备一行, 组序）。
    pub input_pts: Vec<InputPts>,
    pub program_video_pts: Option<u64>,
    pub program_audio_pts: Option<u64>,
    pub program_video_pts_state: PtsMonotonicity,
    pub program_audio_pts_state: PtsMonotonicity,
    pub program_video_frames: u64,
    pub program_audio_frames: u64,
}

/// C-TIMELINE-01（IMP-4, 第三十一轮终裁 §六）: Program observation **唯一
/// 面**——既有 `ProgramObservation` 语义零污染, timeline 证据**并列同行**
/// （不塞字段进 ProgramObservation, 亦非第二 observation SPI）。
///
/// 消费方经 `.program` 读既有列、经 `.timeline` 读时间线证据行; L4-TIMELINE
/// 直接消费 timeline evidence（第二批升级判据时接线）。`TimelineObservation`
/// 是 **evidence 不是 authority**（IMP-4）——观测行不裁时间线, 裁决权在
/// `TimelineAuthority`（Domain）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProgramExecutionObservation {
    pub program: ProgramObservation,
    pub timeline: TimelineObservation,
}

/// C-TIMELINE-01 ①（第三十二轮 §七硬条件）: 切换锚观测——Adapter **只观测
/// 不声明**: program 连续性锚（当前出口位置+步长）与 target 源连续性锚
/// （target 分支位置+步长）。**offset 归 `TimelineAuthority` 声明**——
/// GStreamer probe 禁重算 offset 覆盖 Domain 声明。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchAnchors {
    pub video: AnchorPair,
    pub audio: AnchorPair,
}

/// C-TIMELINE-01 ⑤⑥⑦: 单平面 adapter 执行事实（Runtime 据此驱动 Authority
/// ——证据输入, 非 wire 行; 身份=声明+Event+Buffer 三件闭合, 无 readback）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaneExecutionFacts {
    /// ⑤ Segment(target) 事件已在该平面 selector 出口观测（声明驱动身份:
    /// 翻转后该平面首个 Segment 即 target 段——F2）。
    pub segment_observed: bool,
    /// ⑥⑦ 首枚 target 缓冲（source_pts 原值, mapped=声明映射施加后）。
    pub first_mapped: Option<(u64, u64)>,
    /// 最近观测（source, mapped）——持续证据。
    pub last_observed: Option<(u64, u64)>,
}

/// C-TIMELINE-01: adapter 侧 timeline 执行事实（含声明的 program_epoch——
/// Adapter 当前已知 epoch, 供证据行 no_evidence 携带[第三十二轮前置②]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineExecutionFacts {
    pub program_epoch: ProgramEpoch,
    pub video: PlaneExecutionFacts,
    pub audio: PlaneExecutionFacts,
}

/// Switch Execution Adapter——Program graph 的物化/切换/观测 owner。
///
/// SPI 方法在无调用点的 feature 组合下可能未消费; 与 `MediaBackend`
/// 一致在 trait 级允许 dead_code。
#[allow(dead_code)]
pub trait SwitchExecutionAdapter: Send + Sync {
    /// 从 ExecutionGroup（恰双输入 + Desired=Active(initial)）物化 Program
    /// graph, 返回其句柄（复用 `PipelineHandle`——它是真实管线实例身份,
    /// 非 second registry）。
    fn build_program_graph(&self, group: &ExecutionGroup) -> Result<PipelineHandle, SwitchError>;

    /// 启动 Program graph（此后 observe 才有 Observed 证据）。
    fn start_program(&self, graph: &PipelineHandle) -> Result<(), SwitchError>;

    /// 执行显式切换（video+audio 成对, 帧边界对齐）。重校验 plan
    /// （fail-closed 纵深: 目标∈组 / FRAME first / epoch 单调）。
    fn switch(
        &self,
        graph: &PipelineHandle,
        plan: &SwitchExecutionPlan,
    ) -> Result<SwitchExecuted, SwitchError>;

    /// C-TIMELINE-01（IMP-5 ③, Freeze §11 签名=implementation 首刀落地）:
    /// 安装 timeline transition 声明（`TimelineAuthority` 产出 → adapter 侧
    /// 执行态: expected target + per-plane segments/mapping——终裁 §七
    /// `TimelineExecutionState` 的安装入口）。**必须在 `switch` 之前安装**
    /// （pre-flip canonical 序; F6 实证无竞态）。**install 只做安装——
    /// 真正执行由 EVENT/BUFFER probe 完成, 禁"install 完=TimelineMapped"
    /// 的 Intent/Fact 混淆**（第三十二轮 §六硬条件）。
    ///
    /// 默认=**未实装 fail-closed**（诚实错误非静默）: GStreamer 实装=
    /// C-TIMELINE-01 第二批（selector 后 per-plane EVENT+BUFFER probe）;
    /// Mock 已实装（确定性仿真: 安装→翻转→Segment(B)→首枚映射缓冲）。
    fn install_timeline_transition(
        &self,
        graph: &PipelineHandle,
        plan: &ProgramTimelinePlan,
    ) -> Result<(), SwitchError> {
        let _ = (graph, plan);
        Err(SwitchError::Backend(
            "timeline execution layer not installed (C-TIMELINE-01 batch 2)".into(),
        ))
    }

    /// C-TIMELINE-01 ①: 采样下一次切换的 V/A 锚对（纯观测; 零证据=fail-closed
    /// Err——absence≠evidence）。默认=未实装 fail-closed。
    fn sample_switch_anchors(
        &self,
        graph: &PipelineHandle,
        target: Uuid,
    ) -> Result<SwitchAnchors, SwitchError> {
        let _ = (graph, target);
        Err(SwitchError::Backend(
            "timeline anchor sampling not installed (C-TIMELINE-01 batch 2)".into(),
        ))
    }

    /// C-TIMELINE-01 ⑤⑥⑦: adapter 侧 per-plane 执行事实（声明段下 Segment
    /// 事件观测 + 首枚映射缓冲——Runtime 的 Authority 证据输入）。默认=None
    /// （未实装——诚实缺席）。
    fn timeline_execution_facts(&self, graph: &PipelineHandle) -> Option<TimelineExecutionFacts> {
        let _ = graph;
        None
    }

    /// R58 步骤5.1（INV-F1/F2/F3 编码进 Adapter 执行契约——执行层端口扩展,
    /// Domain 语义零变更; program_timeline/Gate 不触碰）: V+A 双面 cutover
    /// fence Armed。**原子双面**: 单临界区 V+A 同装（终裁 §5/§12——两把
    /// 独立锁顺序加锁的 Video=Armed/Audio=Open 微观窗口被结构性消除）。
    /// **barrier=执行屏障, 非执行权威**（INV-F3: 永不自宣布切换完成——
    /// Release 仅由编排于 `switch` 落点后驱动; 失败路径经
    /// `force_release_cutover_fence` 兜底恢复流面, 切换错误如实上抛）。
    /// BUFFER-only（EVENT 微观序保持——Segment 观测与世代身份捕获不受影响）。
    fn arm_cutover_fence(&self, graph: &PipelineHandle) -> Result<(), SwitchError>;

    /// R58 步骤5.1（INV-F1 确认闭环——终裁 §2/§4/§9/§10）: **确认式
    /// Release**。阻塞等待双平面下游新世代 Segment 确认（Armed 期 selector
    /// 侧捕获本次切换 Segment 事件序号; appsink 前下游探针按序号匹配——
    /// queue 保序 ⇒ Segment 到达=Arm 前入队旧世代缓冲已被消费门全部处置
    /// =排空事实锚; **真实媒体事件证据, 非时间猜测**——timeout 仅异常界）,
    /// Both-confirmed 后与确认**同一临界区**原子 Open 双面。超时 → Err 且
    /// fence 保持 Armed（fail-closed; 编排走 `force_release_cutover_fence`
    /// 兜底——强释属失败处置, 非确认放行; T-F3: 任一面未确认 Both Release
    /// 不得发生）。返回确认证据（世代/双面 confirmed+丢弃计数）。
    fn release_cutover_fence(
        &self,
        graph: &PipelineHandle,
        timeout: std::time::Duration,
    ) -> Result<CutoverDrainEvidence, SwitchError>;

    /// R58 步骤5.1: 兜底强释（**仅失败路径**——编排守卫 Drop bottom-line;
    /// 无确认前提, 恢复流面防输出饿死; 返回丢弃计数）。确认式 Release 的
    /// Both-confirmed 前置语义不受影响（成功路径从不经此——T-F3 在案）。
    fn force_release_cutover_fence(&self, graph: &PipelineHandle) -> Result<u64, SwitchError>;

    /// Observed 平面读数（实际 active + 六路 PTS + 帧计数 + timeline 证据
    /// 行——C-TIMELINE-01 起经 `ProgramExecutionObservation` 单一组合面）。
    fn observe(&self, graph: &PipelineHandle) -> ProgramExecutionObservation;

    /// 停止 Program graph（Observed 归零; 输入管线生命周期不受影响——
    /// 归 SessionManager）。
    fn stop_program(&self, graph: &PipelineHandle) -> Result<(), SwitchError>;
}
