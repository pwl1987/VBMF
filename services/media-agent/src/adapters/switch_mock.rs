//! A2-8-01: Mock Switch Execution Adapter——确定性 PTS 流仿真（mock feature）。
//!
//! 与 `MockBackend` 同律: 纯 Rust 零 GStreamer, 解锁 switch 执行面的
//! mock 层验证（T1/T2/T3/T5/T6）。语义模型:
//! - **成对切换**: video+audio 共享单一 active 与 av_epoch——单面切构造不出;
//! - **PTS 连续性（双模式）**: 未安装 timeline 时 program 出口是**独立再生成
//!   流**（FRAME_SWITCH = RAW→RAW 重新编码平面, A2-8-01 既有语义保持——
//!   跨切换单调不回退）; 安装 C-TIMELINE-01 timeline 声明后出口=**映射后
//!   源流**（`program = f(source)`——F5 selector 后 probe+声明映射语义同构）,
//!   且翻转后首个 observe tick 只出现 Segment(B) 等价事件、再下一 tick 才有
//!   首枚 B 映射缓冲（生效边界=下一缓冲——F2/F6 微观序同构）;
//! - **停滞注入**: `stall()` 冻结指定设备帧计数（Observation 事实位,
//!   故障结论归 fold/Custody）。

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use crate::contracts::switch::{
    CutoverDrainEvidence, FrameBoundary, InputPts, PlaneDrainEvidence, PlaneExecutionFacts,
    ProgramExecutionObservation, ProgramObservation, SwitchAnchors, SwitchExecuted,
    SwitchExecutionAdapter, TimelineExecutionFacts,
};
use crate::pipeline::{PipelineHandle, PtsMonotonicity, NEXT_PIPELINE_ID};
use crate::program::SwitchPolicy;
use crate::program_timeline::{
    AnchorPair, PlaneContinuity, ProgramEpoch, ProgramTimelinePlan, TimelineObservation,
};
use crate::switch_execution::{ExecutionGroup, SwitchDesired, SwitchError, SwitchExecutionPlan};
use uuid::Uuid;

/// 每观测 tick 的 video PTS 推进量（25fps 帧间隔, ms 时基）。
const VIDEO_PTS_STEP: u64 = 40;
/// 每观测 tick 的 audio PTS 推进量。
const AUDIO_PTS_STEP: u64 = 20;

/// C-TIMELINE-01 Batch 1: adapter 侧 timeline 执行态仿真（确定性构造级——
/// 非隐藏 mapping: 声明→翻转→Segment(B)→首枚映射缓冲, 与 SIM-01 F2/F5/F6
/// 行为同构; Authority 在 Domain, Mock 只仿真 adapter 执行/证据角色）。
struct MockTimelineState {
    plan: ProgramTimelinePlan,
    /// ⑤ 翻转后首个 observe tick: Segment(B) 等价事件已出现（尚无 B 缓冲）。
    segment_seen: bool,
    /// ⑥⑦ 再下一个 observe tick: 首枚 B 缓冲已按声明映射施加。
    first_mapped: bool,
    /// ⑥⑦ 首枚映射缓冲事实（[video, audio]: source→mapped）。
    first: [(u64, u64); 2],
    /// 最近观测（[video, audio]: source→mapped）——持续证据。
    last: [(u64, u64); 2],
}

struct MockGraph {
    /// 组内设备源（组序; 恰双）。
    devices: [Uuid; 2],
    /// 构建期 Desired 提取的初始 active。
    initial_active: Uuid,
    started: bool,
    /// video+audio 共享 active（成对切换的结构性承载）。
    active: Option<Uuid>,
    av_epoch: u64,
    tick: u64,
    stalled: HashSet<Uuid>,
    /// device → (video_pts, audio_pts) 最新值。
    pts: HashMap<Uuid, (u64, u64)>,
    program_pts: (u64, u64),
    program_frames: (u64, u64),
    /// R58 步骤6: 程序出口单调性真状态机（替换硬编码 ValidMonotonic——
    /// R57-terminal 点名缺口: 段内回退→NonMonotonic sticky; 干净已声明
    /// 边界→DiscontinuityDeclared; 语义=真实健康弧 observe_plain/
    /// note_declared_boundary 的 mock 镜像最小集）。
    program_v_state: PtsMonotonicity,
    program_a_state: PtsMonotonicity,
    /// C-TIMELINE-01: 已安装 timeline 声明（None=legacy 独立再生成流模式）。
    timeline: Option<MockTimelineState>,
    /// R58 步骤5/6: cutover fence 状态 + 交错模型簿记（armed=消费门生效
    /// ——tick 交付与竞态窗窜帧一律 cutover-discard; 丢弃计数 per-plane
    /// 诚实累计, arm 清零; generation 递增; 七事件交错日志 arm 清空）。
    cutover_fence_armed: bool,
    cutover_discarded: (u64, u64),
    cutover_generation: u64,
    interleave: Vec<CutoverInterleaveEvent>,
    /// R58 步骤6: 竞态窗窜帧预置（staged 注入——下一次锚采样读毕即投递,
    /// 模型=[锚采样→install] µs 窗内旧世代映射帧穿越消费门）。
    pending_straggler: Option<(u64, u64)>,
}

/// R58 步骤6: cutover 交错事件词汇（终裁锁死七事件——控制/数据线程
/// 交错的可断言证据面）。生产序=AnchorSampled→OldStraggler→
/// [OldBufferDropped: 门处置]→InstallNew→SwitchNew→FenceConfirmed
/// （排空确认+Release）→FirstNewMapped; 无 Fence 模式无 OldBufferDropped/
/// FenceConfirmed（窜帧直写观测——基线被推进）。
/// R58 步骤6 HOLD-1: 消费门先于一切 timeline 证据状态推进——tick 路径
/// 的 Armed 丢弃（①a 基准观测/post-switch Armed 窗口在途缓冲）同记
/// OldBufferDropped（门处置如实入账; 词汇仍七, 事件数如实）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutoverInterleaveEvent {
    AnchorSampled { program_video: u64 },
    OldStraggler { mapped_video: u64 },
    OldBufferDropped,
    FenceConfirmed { generation: u64, discarded: u64 },
    InstallNew { segment: u64 },
    SwitchNew { epoch: u64 },
    FirstNewMapped { mapped_video: u64 },
}

impl MockGraph {
    /// 确定性初值: 首源 video=1000/audio=800, 次源 ×2——两源流天然可区分。
    fn device_base(device: &Uuid, devices: &[Uuid; 2]) -> (u64, u64) {
        let idx = if devices[0] == *device { 0 } else { 1 };
        (1000 * (idx as u64 + 1), 800 * (idx as u64 + 1))
    }

    /// R58 步骤6: 程序出口 plain 写点状态推进（真实 observe_video_pts 镜像:
    /// 首观测→ValidMonotonic; 回退→NonMonotonic sticky; 否则保持）。
    fn observe_program_plain(&mut self, v: u64, a: u64) {
        let (lv, la) = self.program_pts;
        self.program_v_state = plain_step(self.program_v_state, v, lv);
        self.program_a_state = plain_step(self.program_a_state, a, la);
    }

    /// R58 步骤6: 已声明边界写点状态推进（note_declared_boundary+observe_
    /// declared 净语义镜像: 违例→NonMonotonic sticky; 干净→Discontinuity
    /// Declared——上一段内 NonMonotonic 就此解除, R53 段作用域生命周期）。
    fn observe_program_declared(&mut self, v: u64, a: u64) {
        let (lv, la) = self.program_pts;
        self.program_v_state = declared_step(self.program_v_state, v, lv);
        self.program_a_state = declared_step(self.program_a_state, a, la);
    }

    /// R58 步骤6: 竞态窗窜帧投递（消费门同裁决——armed→丢弃+计数+日志;
    /// open→plain 写点推进基线, 正是 M1 无 Fence 穿透路径）。
    fn deliver_straggler(&mut self, mapped: (u64, u64)) {
        self.interleave.push(CutoverInterleaveEvent::OldStraggler {
            mapped_video: mapped.0,
        });
        if self.cutover_fence_armed {
            self.cutover_discarded = (self.cutover_discarded.0 + 1, self.cutover_discarded.1 + 1);
            self.interleave
                .push(CutoverInterleaveEvent::OldBufferDropped);
            return;
        }
        self.observe_program_plain(mapped.0, mapped.1);
        self.program_pts = mapped;
        self.program_frames = (self.program_frames.0 + 1, self.program_frames.1 + 1);
    }

    fn input_pts_row(&self, device: Uuid) -> InputPts {
        let (v, a) = self.pts[&device];
        InputPts {
            device_id: device,
            video_pts: Some(v),
            audio_pts: Some(a),
            video_pts_state: PtsMonotonicity::ValidMonotonic,
            audio_pts_state: PtsMonotonicity::ValidMonotonic,
            stalled: self.stalled.contains(&device),
        }
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// Mock Switch Execution Adapter（`Default` 构造; 内部 Mutex 图表）。
#[derive(Default)]
pub struct MockSwitchExecutionAdapter {
    graphs: Mutex<HashMap<PipelineHandle, MockGraph>>,
}

impl MockSwitchExecutionAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    /// 观测一个 tick（驱动 PTS/帧计数推进——观测即推进的仿真时钟）。
    /// R58 步骤6: 程序出口交付受 cutover fence 消费门裁决——Armed 期
    /// 到达缓冲一律 cutover-discard（计数+日志+不写 pts/状态/帧数）。
    /// R58 步骤6 HOLD-1 修复: 门裁决先于一切 timeline 证据状态推进——
    /// 被丢弃缓冲不占用 first_mapped 槽位（终裁处方: Fence Drop 必须
    /// 先于任何 first_mapped/timeline evidence 状态推进）。Segment 事件
    /// 观测照常推进——EVENT 不被门拦截, 与真实探针类型分离同构。
    fn tick_once(graph: &mut MockGraph) {
        graph.tick += 1;
        for device in graph.devices {
            if !graph.stalled.contains(&device) {
                let (v, a) = graph.pts[&device];
                graph
                    .pts
                    .insert(device, (v + VIDEO_PTS_STEP, a + AUDIO_PTS_STEP));
            }
        }
        // Program 出口: active 存活且未停滞才交付。
        let alive = graph
            .active
            .map(|d| graph.started && !graph.stalled.contains(&d))
            .unwrap_or(false);
        if !alive {
            return;
        }
        // C-TIMELINE-01 双模式: 已安装声明且对应切换已执行 → 映射后源流;
        // 否则 legacy 独立再生成流（A2-8-01 语义保持——零行为回退）。
        let timeline_active = graph
            .timeline
            .as_ref()
            .is_some_and(|t| graph.av_epoch >= t.plan.switch_epoch);
        if timeline_active {
            if !graph
                .timeline
                .as_ref()
                .expect("timeline_active")
                .segment_seen
            {
                // ⑤ 边界 tick: Segment(B) 事件先于首枚 B 缓冲——本 tick
                // 出口不交付缓冲（生效边界=下一缓冲, F6 微观序）。Segment
                // =EVENT 不受消费门拦截（与真实 EVENT/BUFFER 探针类型分
                // 离同构——drain 确认锚即挂在该事件上）。
                graph
                    .timeline
                    .as_mut()
                    .expect("timeline_active")
                    .segment_seen = true;
                return;
            }
            // R58 步骤6 HOLD-1 修复: 消费门先于一切 timeline 证据状态推进
            // ——Armed 期到达的缓冲在写 first_mapped/facts/程序 PTS/帧数
            // 之前丢弃（被丢弃缓冲不得占用"首枚已映射"槽位; 门不分辨世代
            // 只认 Armed——post-switch Armed 窗口/confirm→release 控制隙
            // 内到达的新世代缓冲同受此处置, 真适配器消费门同语义）。门
            // 处置如实入交错日志（HOLD-1: 修复前 tick 路径丢弃不可见）。
            if graph.cutover_fence_armed {
                graph.cutover_discarded =
                    (graph.cutover_discarded.0 + 1, graph.cutover_discarded.1 + 1);
                graph
                    .interleave
                    .push(CutoverInterleaveEvent::OldBufferDropped);
                return;
            }
            let is_first_mapped_tick = {
                let t = graph.timeline.as_mut().expect("timeline_active");
                let first = !t.first_mapped;
                t.first_mapped = true;
                first
            };
            let active = graph.active.expect("alive ⇒ active");
            let (v, a) = graph.pts[&active];
            let (video_seg, audio_seg) = {
                let t = graph.timeline.as_ref().expect("timeline_active");
                (t.plan.video, t.plan.audio)
            };
            let mapped = (
                video_seg
                    .map_pts(v)
                    .expect("mock 映射偏移在可表示范围（测试锚构造级）"),
                audio_seg
                    .map_pts(a)
                    .expect("mock 映射偏移在可表示范围（测试锚构造级）"),
            );
            {
                let t = graph.timeline.as_mut().expect("timeline_active");
                let facts = [(v, mapped.0), (a, mapped.1)];
                if is_first_mapped_tick {
                    t.first = facts;
                }
                t.last = facts;
            }
            if is_first_mapped_tick {
                graph
                    .interleave
                    .push(CutoverInterleaveEvent::FirstNewMapped {
                        mapped_video: mapped.0,
                    });
                graph.observe_program_declared(mapped.0, mapped.1);
            } else {
                graph.observe_program_plain(mapped.0, mapped.1);
            }
            graph.program_pts = mapped;
            graph.program_frames = (graph.program_frames.0 + 1, graph.program_frames.1 + 1);
            return;
        }
        // legacy: 独立再生成流——跨切换单调不回退（Armed 期同受消费门）。
        if graph.cutover_fence_armed {
            graph.cutover_discarded =
                (graph.cutover_discarded.0 + 1, graph.cutover_discarded.1 + 1);
            return;
        }
        let next = (
            graph.program_pts.0 + VIDEO_PTS_STEP,
            graph.program_pts.1 + AUDIO_PTS_STEP,
        );
        graph.observe_program_plain(next.0, next.1);
        graph.program_pts = next;
        graph.program_frames = (graph.program_frames.0 + 1, graph.program_frames.1 + 1);
    }

    /// 停滞注入（测试钩子: 冻结该设备 PTS/帧计数; program 出口在其为
    /// active 时随之冻结——starvation 仿真）。
    pub fn stall(&self, graph: &PipelineHandle, device_id: Uuid) {
        if let Some(g) = self.graphs.lock().unwrap().get_mut(graph) {
            g.stalled.insert(device_id);
        }
    }

    /// R58 步骤6: 竞态窗窜帧直接投递（协议级显式链测试驱动——数据平面
    /// 事件; 消费门同裁决: armed→丢弃, open→plain 推进基线=M1 穿透路径）。
    pub fn deliver_window_straggler(
        &self,
        graph: &PipelineHandle,
        mapped_video: u64,
        mapped_audio: u64,
    ) {
        if let Some(g) = self.graphs.lock().unwrap().get_mut(graph) {
            g.deliver_straggler((mapped_video, mapped_audio));
        }
    }

    /// R58 步骤6: 竞态窗窜帧预置注入（Runtime 级——下一次锚采样读毕即
    /// 投递, 模型=[锚采样→install] µs 窗内旧世代映射帧穿越消费门;
    /// 编排同步调用内不可插针, 由 mock 自身在 ①c 落点后触发）。
    /// R58 步骤6 HOLD-1 命名口径: 本注入=**Runtime 调用链 cut-point
    /// 注入**（确定性——同步落点触发）, 非 OS 意义真实控制/数据线程
    /// 并发竞态; 真实并发归 Step 7 真机验证（终裁 §3）。
    pub fn stage_window_straggler(
        &self,
        graph: &PipelineHandle,
        mapped_video: u64,
        mapped_audio: u64,
    ) {
        if let Some(g) = self.graphs.lock().unwrap().get_mut(graph) {
            g.pending_straggler = Some((mapped_video, mapped_audio));
        }
    }

    /// R58 步骤6: 交错日志读取（证据面——七事件生产序快照）。
    pub fn cutover_interleave_log(&self, graph: &PipelineHandle) -> Vec<CutoverInterleaveEvent> {
        self.graphs
            .lock()
            .unwrap()
            .get(graph)
            .map(|g| g.interleave.clone())
            .unwrap_or_default()
    }
}

/// R58 步骤6: plain 写点单平面状态步进（Unknown→VM; 回退→NM sticky;
/// 否则保持——真实健康弧 observe_*_pts 镜像）。
fn plain_step(state: PtsMonotonicity, pts: u64, last: u64) -> PtsMonotonicity {
    match state {
        PtsMonotonicity::Unknown => PtsMonotonicity::ValidMonotonic,
        s => {
            if pts < last {
                PtsMonotonicity::NonMonotonic
            } else {
                s
            }
        }
    }
}

/// R58 步骤6: 已声明边界单平面状态步进（违例→NM sticky; 干净→DD——
/// 上一段 NM 就此解除, R53 生命周期镜像）。
fn declared_step(state: PtsMonotonicity, pts: u64, last: u64) -> PtsMonotonicity {
    if state != PtsMonotonicity::Unknown && pts < last {
        PtsMonotonicity::NonMonotonic
    } else {
        PtsMonotonicity::DiscontinuityDeclared
    }
}

impl SwitchExecutionAdapter for MockSwitchExecutionAdapter {
    fn build_program_graph(&self, group: &ExecutionGroup) -> Result<PipelineHandle, SwitchError> {
        let initial_active = match group.desired {
            SwitchDesired::ActiveInput(active) => active,
            switching @ SwitchDesired::Switching { .. } => {
                return Err(SwitchError::NotActiveSource(switching))
            }
            // R63-A 强制调用点: 降级终态组不可物化 graph。
            recovery @ SwitchDesired::RecoveryRequired { .. } => {
                return Err(SwitchError::RecoveryRequired(recovery))
            }
        };
        let devices = [group.inputs[0].device_id, group.inputs[1].device_id];
        let pts: HashMap<Uuid, (u64, u64)> = devices
            .iter()
            .map(|d| (*d, MockGraph::device_base(d, &devices)))
            .collect();
        let handle =
            PipelineHandle(NEXT_PIPELINE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
        let graph = MockGraph {
            devices,
            initial_active,
            started: false,
            active: None,
            av_epoch: 0,
            tick: 0,
            stalled: HashSet::new(),
            pts,
            program_pts: MockGraph::device_base(&initial_active, &devices),
            program_frames: (0, 0),
            program_v_state: PtsMonotonicity::Unknown,
            program_a_state: PtsMonotonicity::Unknown,
            timeline: None,
            cutover_fence_armed: false,
            cutover_discarded: (0, 0),
            cutover_generation: 0,
            interleave: Vec::new(),
            pending_straggler: None,
        };
        self.graphs.lock().unwrap().insert(handle, graph);
        Ok(handle)
    }

    fn start_program(&self, graph: &PipelineHandle) -> Result<(), SwitchError> {
        let mut graphs = self.graphs.lock().unwrap();
        let g = graphs
            .get_mut(graph)
            .ok_or(SwitchError::GraphNotRunning(*graph))?;
        g.started = true;
        g.active = Some(g.initial_active);
        Ok(())
    }

    /// R58 步骤5/6（执行契约）: V+A fence Armed（mock 单字段承载双面
    /// ——结构上无单面窗口; barrier 非权威 INV-F3）。交错簿记: 世代递增/
    /// 丢弃计数清零/日志清空（每世代独立证据面）。
    fn arm_cutover_fence(&self, graph: &PipelineHandle) -> Result<(), SwitchError> {
        let mut graphs = self.graphs.lock().unwrap();
        let g = graphs
            .get_mut(graph)
            .ok_or(SwitchError::GraphNotRunning(*graph))?;
        debug_assert!(
            !g.cutover_fence_armed,
            "fence 双重 arm——编排序破坏（前序未 Release）"
        );
        g.cutover_fence_armed = true;
        g.cutover_discarded = (0, 0);
        g.cutover_generation += 1;
        g.interleave.clear();
        Ok(())
    }

    /// R58 步骤5.1/6（执行契约）: 确认式 Release。Mock 无真实 queue/流面
    /// ——**排空确认按"立即可用"建模**（auto-confirm; 协议强制
    /// [Both-confirmed 前置/世代序号匹配/超时 fail-closed] 由真适配器
    /// switch_graph 确定性测试 T-F1/F2/F3 证明——Mock 交错模型表达的是
    /// 控制序/数据序/门处置, 非 queue 保序本身）。丢弃计数**如实**交回
    /// （Step 6 起消费门真实计数——arm 清零, per-plane 累计）。
    fn release_cutover_fence(
        &self,
        graph: &PipelineHandle,
        _timeout: std::time::Duration,
    ) -> Result<CutoverDrainEvidence, SwitchError> {
        let mut graphs = self.graphs.lock().unwrap();
        let g = graphs
            .get_mut(graph)
            .ok_or(SwitchError::GraphNotRunning(*graph))?;
        debug_assert!(g.cutover_fence_armed, "fence release 未 arm——编排序破坏");
        g.cutover_fence_armed = false;
        let ev = CutoverDrainEvidence {
            generation: g.cutover_generation,
            video: PlaneDrainEvidence {
                segment_confirmed: true,
                discarded: g.cutover_discarded.0,
            },
            audio: PlaneDrainEvidence {
                segment_confirmed: true,
                discarded: g.cutover_discarded.1,
            },
        };
        g.interleave.push(CutoverInterleaveEvent::FenceConfirmed {
            generation: ev.generation,
            discarded: g.cutover_discarded.0 + g.cutover_discarded.1,
        });
        Ok(ev)
    }

    /// R58 步骤5.1（执行契约）: 兜底强释（无确认前提; 幂等; **不产
    /// FenceConfirmed 事件**——强释=失败处置非确认, 与真适配器类型面
    /// 分离同构）。丢弃计数如实交回。
    fn force_release_cutover_fence(&self, graph: &PipelineHandle) -> Result<u64, SwitchError> {
        let mut graphs = self.graphs.lock().unwrap();
        let g = graphs
            .get_mut(graph)
            .ok_or(SwitchError::GraphNotRunning(*graph))?;
        g.cutover_fence_armed = false;
        Ok(g.cutover_discarded.0 + g.cutover_discarded.1)
    }

    fn install_timeline_transition(
        &self,
        graph: &PipelineHandle,
        plan: &ProgramTimelinePlan,
    ) -> Result<(), SwitchError> {
        // pre-flip 安装（IMP-5 ③）: 仅运行中 graph / 目标∈组 / 目标非 active /
        // 声明 epoch=下一次执行（身份联动——fail-closed）。
        let mut graphs = self.graphs.lock().unwrap();
        let g = graphs
            .get_mut(graph)
            .ok_or(SwitchError::GraphNotRunning(*graph))?;
        if !g.started {
            return Err(SwitchError::GraphNotRunning(*graph));
        }
        if !g.devices.contains(&plan.target) {
            return Err(SwitchError::TargetNotInGroup(plan.target));
        }
        if g.active == Some(plan.target) {
            return Err(SwitchError::TargetAlreadyActive(plan.target));
        }
        // R63-A 新鲜度谓词: 重放已执行世代（<= av_epoch）拒收; 未来 epoch
        // 的精确锁步不再要求——begin 后失败会留下合法 epoch 间隙（组平面
        // 已消费, adapter 未执行）, 恢复后重试须放行。
        if plan.switch_epoch <= g.av_epoch {
            return Err(SwitchError::StalePlanEpoch {
                got: plan.switch_epoch,
                expected: g.av_epoch + 1,
            });
        }
        g.timeline = Some(MockTimelineState {
            plan: *plan,
            segment_seen: false,
            first_mapped: false,
            first: [(0, 0); 2],
            last: [(0, 0); 2],
        });
        g.interleave.push(CutoverInterleaveEvent::InstallNew {
            segment: plan.video.segment_id.0,
        });
        Ok(())
    }

    fn sample_switch_anchors(
        &self,
        graph: &PipelineHandle,
        target: Uuid,
    ) -> Result<SwitchAnchors, SwitchError> {
        // C-TIMELINE-01 ①: 锚=纯观测（offset 归 Authority 声明——本方法不产
        // offset）。program 连续性锚=当前出口+步长; target 源连续性锚=target
        // 分支+步长。停滞/缺席=无证据 fail-closed。
        // R58 步骤6: 锚采样落点记 AnchorSampled; staged 竞态窗窜帧在此
        // 读毕即投递（[锚采样→install] µs 窗模型——控制/数据交错）。
        let mut graphs = self.graphs.lock().unwrap();
        let g = graphs
            .get_mut(graph)
            .ok_or(SwitchError::GraphNotRunning(*graph))?;
        if !g.started {
            return Err(SwitchError::GraphNotRunning(*graph));
        }
        if !g.devices.contains(&target) {
            return Err(SwitchError::TargetNotInGroup(target));
        }
        if g.active == Some(target) {
            return Err(SwitchError::TargetAlreadyActive(target));
        }
        if g.stalled.contains(&target) {
            return Err(SwitchError::Backend(
                "target 停滞——锚证据不足（absence≠evidence）fail-closed".into(),
            ));
        }
        let (pv, pa) = g.program_pts;
        let (bv, ba) = *g
            .pts
            .get(&target)
            .ok_or_else(|| SwitchError::Backend("target 分支 PTS 缺席——锚证据不足".into()))?;
        g.interleave
            .push(CutoverInterleaveEvent::AnchorSampled { program_video: pv });
        if let Some(straggler) = g.pending_straggler.take() {
            g.deliver_straggler(straggler);
        }
        Ok(SwitchAnchors {
            video: AnchorPair {
                program_anchor: pv + VIDEO_PTS_STEP,
                source_anchor: bv + VIDEO_PTS_STEP,
            },
            audio: AnchorPair {
                program_anchor: pa + AUDIO_PTS_STEP,
                source_anchor: ba + AUDIO_PTS_STEP,
            },
        })
    }

    fn timeline_execution_facts(&self, graph: &PipelineHandle) -> Option<TimelineExecutionFacts> {
        let graphs = self.graphs.lock().unwrap();
        let g = graphs.get(graph)?;
        let t = g.timeline.as_ref()?;
        if g.av_epoch < t.plan.switch_epoch {
            return None; // 声明在途未执行——尚无执行事实（诚实缺席）。
        }
        let plane_facts = |i: usize, segment_observed: bool, mapped: bool| PlaneExecutionFacts {
            segment_observed,
            first_mapped: mapped.then_some(t.first[i]),
            last_observed: mapped.then_some(t.last[i]),
        };
        Some(TimelineExecutionFacts {
            program_epoch: t.plan.video.program_epoch,
            video: plane_facts(0, t.segment_seen, t.first_mapped),
            audio: plane_facts(1, t.segment_seen, t.first_mapped),
        })
    }

    fn switch(
        &self,
        graph: &PipelineHandle,
        plan: &SwitchExecutionPlan,
    ) -> Result<SwitchExecuted, SwitchError> {
        let mut graphs = self.graphs.lock().unwrap();
        let g = graphs
            .get_mut(graph)
            .ok_or(SwitchError::GraphNotRunning(*graph))?;
        if !g.started {
            return Err(SwitchError::GraphNotRunning(*graph));
        }
        // 纵深重校验（group 已校验——adapter 不信任调用方, fail-closed 一致）。
        if !g.devices.contains(&plan.target) {
            return Err(SwitchError::TargetNotInGroup(plan.target));
        }
        if plan.policy != SwitchPolicy::FrameSwitch {
            return Err(SwitchError::UnsupportedPolicy(plan.policy));
        }
        // R63-A 新鲜度谓词（同 install 位点）: 重放已执行世代拒收, 合法
        // epoch 间隙（失败重试）放行。
        if plan.epoch <= g.av_epoch {
            return Err(SwitchError::StalePlanEpoch {
                got: plan.epoch,
                expected: g.av_epoch + 1,
            });
        }
        if g.active == Some(plan.target) {
            return Err(SwitchError::TargetAlreadyActive(plan.target));
        }
        // C-TIMELINE-01: 已安装 timeline 声明时, 执行计划必须与声明一致
        // （target+switch_epoch 双联动——身份闭合"声明"环; 不一致=翻转前拒收）。
        if let Some(t) = &g.timeline {
            if t.plan.target != plan.target || t.plan.switch_epoch != plan.epoch {
                return Err(SwitchError::Backend(format!(
                    "timeline 声明与执行计划不一致 (declared target={} epoch={}, plan target={} epoch={})——fail-closed",
                    t.plan.target, t.plan.switch_epoch, plan.target, plan.epoch
                )));
            }
        }
        // 成对切换: video+audio 共享 active/av_epoch（单字段承载——单面切
        // 在本模型中不可构造, 方案 A 语义由结构保证）。
        g.active = Some(plan.target);
        g.av_epoch = plan.epoch;
        g.interleave
            .push(CutoverInterleaveEvent::SwitchNew { epoch: plan.epoch });
        Ok(SwitchExecuted {
            boundary: FrameBoundary::FrameAligned,
            av_epoch: g.av_epoch,
        })
    }

    fn observe(&self, graph: &PipelineHandle) -> ProgramExecutionObservation {
        let mut graphs = self.graphs.lock().unwrap();
        let Some(g) = graphs.get_mut(graph) else {
            return ProgramExecutionObservation {
                program: ProgramObservation {
                    observed_active: None,
                    video_active: None,
                    audio_active: None,
                    switch_epoch: 0,
                    input_pts: Vec::new(),
                    program_video_pts: None,
                    program_audio_pts: None,
                    program_video_pts_state: PtsMonotonicity::Unknown,
                    program_audio_pts_state: PtsMonotonicity::Unknown,
                    program_video_frames: 0,
                    program_audio_frames: 0,
                },
                timeline: TimelineObservation::no_evidence(ProgramEpoch(0), MockGraph::now_ms()),
            };
        };
        Self::tick_once(g);
        let running = g.started;
        let program = ProgramObservation {
            observed_active: running.then_some(g.active).flatten(),
            video_active: running.then_some(g.active).flatten(),
            audio_active: running.then_some(g.active).flatten(),
            switch_epoch: g.av_epoch,
            input_pts: g.devices.iter().map(|d| g.input_pts_row(*d)).collect(),
            program_video_pts: running.then_some(g.program_pts.0),
            program_audio_pts: running.then_some(g.program_pts.1),
            // R58 步骤6: 程序出口单调性=真实状态机读数（替换硬编码
            // ValidMonotonic——R57-terminal 点名缺口闭合）。
            program_video_pts_state: if running {
                g.program_v_state
            } else {
                PtsMonotonicity::Unknown
            },
            program_audio_pts_state: if running {
                g.program_a_state
            } else {
                PtsMonotonicity::Unknown
            },
            program_video_frames: if running { g.program_frames.0 } else { 0 },
            program_audio_frames: if running { g.program_frames.1 } else { 0 },
        };
        // Timeline 证据行: 仅"已安装声明 + 对应切换已执行 + 首枚映射缓冲已
        // 出现"才有事实（此前=no_evidence 诚实缺席——absence≠evidence）。
        let timeline = if running
            && g.timeline
                .as_ref()
                .is_some_and(|t| t.first_mapped && g.av_epoch >= t.plan.switch_epoch)
        {
            let t = g.timeline.as_ref().expect("checked");
            TimelineObservation {
                program_epoch: t.plan.video.program_epoch,
                source_id: g.active,
                segment_id: Some(t.plan.video.segment_id),
                input_pts: g.active.and_then(|d| g.pts.get(&d).map(|(v, _)| *v)),
                mapped_program_pts: Some(g.program_pts.0),
                mapping_offset: Some(t.plan.video.offset),
                discontinuity_state: PtsMonotonicity::DiscontinuityDeclared,
                video_continuity: PlaneContinuity::Continuous,
                audio_continuity: PlaneContinuity::Continuous,
                observed_at_ms: MockGraph::now_ms(),
            }
        } else {
            // 缺席行携带 Adapter 当前已知 epoch（第三十二轮前置②——无声明
            // 状态下已知 epoch=0, 与初始 ProgramEpoch 一致）。
            TimelineObservation::no_evidence(ProgramEpoch(0), MockGraph::now_ms())
        };
        ProgramExecutionObservation { program, timeline }
    }

    fn stop_program(&self, graph: &PipelineHandle) -> Result<(), SwitchError> {
        let mut graphs = self.graphs.lock().unwrap();
        let g = graphs
            .get_mut(graph)
            .ok_or(SwitchError::GraphNotRunning(*graph))?;
        g.started = false;
        g.active = None;
        Ok(())
    }
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::adapters::mock::MockBackend;
    use crate::contracts::backend::MediaBackend;
    use crate::program_timeline::{
        AnchorPair, MediaPlane, SegmentId, SourceSegment, TimelineAuthority, TransitionOutcome,
    };
    use crate::session::{SessionId, SessionInput};

    fn input(device_id: Uuid, handle: u64) -> SessionInput {
        SessionInput {
            device_id,
            handle: PipelineHandle(handle),
        }
    }

    /// T1 语义: 两路真实（mock）输入同时运行——经 MockBackend 实例化的
    /// 双 handle 组成 ExecutionGroup。
    fn running_group_and_graph() -> (
        Uuid,
        Uuid,
        ExecutionGroup,
        PipelineHandle,
        MockSwitchExecutionAdapter,
    ) {
        let backend = MockBackend;
        let h1 = backend
            .instantiate(&crate::pipeline::PipelinePlan::self_test())
            .expect("mock instantiate A");
        let h2 = backend
            .instantiate(&crate::pipeline::PipelinePlan::self_test())
            .expect("mock instantiate B");
        assert_ne!(h1, h2, "双输入句柄必须互异");
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let group = ExecutionGroup::new(
            SessionId(Uuid::new_v4()),
            vec![input(a, h1.0), input(b, h2.0)],
            a,
        )
        .expect("合法双输入组");
        let adapter = MockSwitchExecutionAdapter::new();
        let graph = adapter
            .build_program_graph(&group)
            .expect("物化 program graph");
        adapter.start_program(&graph).expect("启动 program graph");
        (a, b, group, graph, adapter)
    }

    /// plan→begin→adapter.switch 标准序列（返回执行证据）。
    fn do_switch(
        group: &mut ExecutionGroup,
        adapter: &MockSwitchExecutionAdapter,
        graph: &PipelineHandle,
        target: Uuid,
    ) -> SwitchExecuted {
        let plan = group
            .plan_switch(&crate::switch_execution::SwitchIntent {
                target,
                policy: SwitchPolicy::FrameSwitch,
            })
            .expect("合法计划");
        group.begin_switch(&plan).expect("begin 推进");
        adapter.switch(graph, &plan).expect("adapter 执行切换")
    }

    #[test]
    fn switch_rt_01_group_two_inputs_running() {
        // T1: A/B 双输入同时运行（双 handle + program graph 同时观测两路）。
        let (a, b, _group, graph, adapter) = running_group_and_graph();
        let obs = adapter.observe(&graph).program;
        assert_eq!(obs.observed_active, Some(a));
        assert_eq!(obs.input_pts.len(), 2, "两路输入均在观测面");
        assert!(obs
            .input_pts
            .iter()
            .all(|p| { p.device_id == a || p.device_id == b }));
    }

    #[test]
    fn switch_rt_01_program_graph_consumes_group() {
        // T2: 汇入同一 Program Execution——graph 句柄独立于两输入句柄,
        // 观测面同时携带两输入 PTS + program 出口 PTS。
        let (a, b, group, graph, adapter) = running_group_and_graph();
        let input_handles: Vec<u64> = group.inputs.iter().map(|i| i.handle.0).collect();
        assert!(
            !input_handles.contains(&graph.0),
            "program graph 句柄独立于输入管线句柄"
        );
        let obs = adapter.observe(&graph).program;
        assert!(obs.program_video_pts.is_some() && obs.program_audio_pts.is_some());
        let devices: Vec<Uuid> = obs.input_pts.iter().map(|p| p.device_id).collect();
        assert!(devices.contains(&a) && devices.contains(&b));
    }

    #[test]
    fn switch_rt_01_explicit_switch_flips_observed_active() {
        // T3: A→B→A 真实执行切换——adapter 内部 active 实态翻转
        // （Observed 独立读数）, 非 Rust 状态字段回显。
        let (a, b, mut group, graph, adapter) = running_group_and_graph();
        assert_eq!(adapter.observe(&graph).program.observed_active, Some(a));

        let e1 = do_switch(&mut group, &adapter, &graph, b);
        assert_eq!(adapter.observe(&graph).program.observed_active, Some(b));
        assert_eq!(e1.av_epoch, 1);
        assert!(group.complete_switch(b), "Observed=B 应落定 Desired");

        let e2 = do_switch(&mut group, &adapter, &graph, a);
        assert_eq!(adapter.observe(&graph).program.observed_active, Some(a));
        assert_eq!(e2.av_epoch, 2);
        assert!(group.complete_switch(a));
        assert_eq!(group.desired, SwitchDesired::ActiveInput(a));
    }

    #[test]
    fn switch_rt_01_switch_executes_at_frame_boundary() {
        // T4: 帧边界是必带证据（非 Option; 无边界=未发生合法切换）。
        let (_a, b, mut group, graph, adapter) = running_group_and_graph();
        let executed = do_switch(&mut group, &adapter, &graph, b);
        assert_eq!(executed.boundary, FrameBoundary::FrameAligned);
    }

    #[test]
    fn switch_rt_01_paired_av_switch_same_epoch() {
        // T5: Video/Audio 成对切换——双平面同 epoch 同目标（单面切构造不出）。
        let (a, b, mut group, graph, adapter) = running_group_and_graph();
        let executed = do_switch(&mut group, &adapter, &graph, b);
        let obs = adapter.observe(&graph).program;
        assert_eq!(obs.video_active, Some(b), "video 平面切到 B");
        assert_eq!(obs.audio_active, Some(b), "audio 平面同切 B");
        assert_eq!(obs.video_active, obs.audio_active, "成对语义");
        assert_eq!(obs.switch_epoch, executed.av_epoch);
        assert_ne!(obs.video_active, Some(a));
    }

    #[test]
    fn switch_rt_01_six_pts_surfaces_trackable() {
        // T6: 六路 PTS 可追踪——输入 A/B 各 {video,audio} + program {video,audio}。
        let (_a, _b, _group, graph, adapter) = running_group_and_graph();
        let obs = adapter.observe(&graph).program;
        assert_eq!(obs.input_pts.len(), 2);
        for p in &obs.input_pts {
            assert!(p.video_pts.is_some(), "{:?} video pts 应在", p.device_id);
            assert!(p.audio_pts.is_some(), "{:?} audio pts 应在", p.device_id);
        }
        assert!(obs.program_video_pts.is_some());
        assert!(obs.program_audio_pts.is_some());
        // 两源 PTS 流天然互异（确定性基值）——六个表面可区分。
        assert_ne!(obs.input_pts[0].video_pts, obs.input_pts[1].video_pts);
    }

    #[test]
    fn switch_rt_01_program_pts_monotonic_across_switch() {
        // T6b: 切换前后 program PTS 不倒退、持续产出（FRAME_SWITCH 再编码
        // 平面——出口独立流; 未安装 timeline=legacy 模式保持不变）。
        let (_a, b, mut group, graph, adapter) = running_group_and_graph();
        let v0 = adapter.observe(&graph).program.program_video_pts.unwrap();
        let a0 = adapter.observe(&graph).program.program_audio_pts.unwrap();
        do_switch(&mut group, &adapter, &graph, b);
        let after = adapter.observe(&graph).program;
        assert!(after.program_video_pts.unwrap() > v0, "video PTS 不得回退");
        assert!(after.program_audio_pts.unwrap() > a0, "audio PTS 不得回退");
        assert_eq!(
            after.program_video_pts_state,
            PtsMonotonicity::ValidMonotonic
        );
        let again = adapter.observe(&graph).program;
        assert!(again.program_video_pts.unwrap() > after.program_video_pts.unwrap());
        assert!(
            again.program_video_frames > after.program_video_frames,
            "出口持续产出"
        );
    }

    #[test]
    fn switch_rt_01_switch_requires_running_graph() {
        // fail-closed: 未 start / 未知句柄 → GraphNotRunning。
        let backend = MockBackend;
        let h1 = backend
            .instantiate(&crate::pipeline::PipelinePlan::self_test())
            .unwrap();
        let h2 = backend
            .instantiate(&crate::pipeline::PipelinePlan::self_test())
            .unwrap();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let group = ExecutionGroup::new(
            SessionId(Uuid::new_v4()),
            vec![input(a, h1.0), input(b, h2.0)],
            a,
        )
        .unwrap();
        let adapter = MockSwitchExecutionAdapter::new();
        let graph = adapter.build_program_graph(&group).unwrap();
        let plan = group
            .plan_switch(&crate::switch_execution::SwitchIntent {
                target: b,
                policy: SwitchPolicy::FrameSwitch,
            })
            .unwrap();
        assert_eq!(
            adapter.switch(&graph, &plan),
            Err(SwitchError::GraphNotRunning(graph)),
            "未 start 的 graph 拒绝切换"
        );
        let unknown = PipelineHandle(999_999);
        assert_eq!(
            adapter.switch(&unknown, &plan),
            Err(SwitchError::GraphNotRunning(unknown))
        );
    }

    #[test]
    fn switch_rt_01_stall_freezes_own_pts_only() {
        // 停滞注入: B 冻结而 A/program 照常推进（T7 fold 的证据源预演;
        // program 在 active=A 存活）。
        let (a, b, _group, graph, adapter) = running_group_and_graph();
        adapter.stall(&graph, b);
        let before = adapter.observe(&graph).program;
        let after = adapter.observe(&graph).program;
        let b_row_before = before.input_pts.iter().find(|p| p.device_id == b).unwrap();
        let b_row_after = after.input_pts.iter().find(|p| p.device_id == b).unwrap();
        assert_eq!(b_row_before.video_pts, b_row_after.video_pts, "B 冻结");
        assert!(b_row_after.stalled, "B 停滞事实位");
        let a_row_before = before.input_pts.iter().find(|p| p.device_id == a).unwrap();
        let a_row_after = after.input_pts.iter().find(|p| p.device_id == a).unwrap();
        assert!(a_row_after.video_pts > a_row_before.video_pts, "A 照常推进");
        assert!(
            after.program_video_pts.unwrap() > before.program_video_pts.unwrap(),
            "active=A 存活时 program 持续"
        );
    }

    #[test]
    fn switch_rt_01_session_input_keyset_locked() {
        // T9: SessionInput 键集恰 {device_id, handle}——active/is_active 等
        // switch state 字段蔓延在此锁死（switch state 与 Session lifecycle
        // 状态空间绝对分离, 终裁 §7.4; wire 面键集锁 = 字段蔓延防线）。
        let json = serde_json::to_value(input(Uuid::new_v4(), 7)).expect("序列化");
        let map = json.as_object().expect("SessionInput 序列化为对象");
        let mut keys: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
        keys.sort();
        assert_eq!(keys, vec!["device_id", "handle"], "键集锁死（恰两键）");
    }

    #[test]
    fn switch_rt_01_no_auto_failover_path() {
        // T12 类型级反证 #1: GroupAction 封闭词表穷尽 destructure——未来
        // 新增任何切换/输入倒换变体将在此**编译失败**（词表膨胀 tripwire;
        // 自动 failover 在观测折叠面不可构造）。
        let sample = crate::watchdog::GroupAction::ReportInputFailure {
            device_id: Uuid::new_v4(),
            reason: crate::watchdog::InputFailureReason::CountersFrozen,
        };
        match sample {
            crate::watchdog::GroupAction::ReportInputFailure { .. } => {}
        }
        // 反证 #2: 切换唯一入口 = 显式 Intent→plan→begin→adapter.switch 链。
        // observe 零副作用（Observed 只读）; 无 Intent 即无 Plan（plan_switch
        // 必收 &SwitchIntent——无 trigger/auto/recover 入口存在）。
        let (a, b, mut group, graph, adapter) = running_group_and_graph();
        let obs = adapter.observe(&graph).program;
        assert_eq!(obs.observed_active, Some(a), "observe 零副作用不切换");
        let plan = group
            .plan_switch(&crate::switch_execution::SwitchIntent {
                target: b,
                policy: SwitchPolicy::FrameSwitch,
            })
            .expect("显式 Intent 产出 Plan");
        group.begin_switch(&plan).expect("begin");
        adapter.switch(&graph, &plan).expect("执行");
        let obs2 = adapter.observe(&graph).program;
        assert_eq!(obs2.observed_active, Some(b), "仅显式链生效");
        assert!(!group.complete_switch(a), "旧源回显不落定");
        assert!(group.complete_switch(b), "Observed=B 落定");
    }

    // ── C-TIMELINE-01 Batch 1: timeline 安装/执行/证据（Mock 闭环）────────

    #[test]
    fn switch_rt_02_install_pre_flip_fail_closed() {
        // pre-flip 安装纪律（IMP-5 ③ + R63-A 新鲜度谓词）: 重放已执行世代
        // 拒收（未来 epoch 的新鲜度归组平面——失败重试的合法间隙放行）;
        // 目标已 active 拒收; 未运行 graph 拒收。
        let (_a, b, mut group, graph, adapter) = running_group_and_graph();
        let plan = crate::program_timeline::ProgramTimelinePlan {
            target: b,
            switch_epoch: 0, // <= av_epoch(0)——重放拒收
            video: crate::program_timeline::SourceSegment::identity(
                b,
                crate::program_timeline::ProgramEpoch(0),
                crate::program_timeline::SegmentId(1),
            ),
            audio: crate::program_timeline::SourceSegment::identity(
                b,
                crate::program_timeline::ProgramEpoch(0),
                crate::program_timeline::SegmentId(1),
            ),
        };
        assert_eq!(
            adapter.install_timeline_transition(&graph, &plan),
            Err(SwitchError::StalePlanEpoch {
                got: 0,
                expected: 1
            })
        );
        // 合法安装（epoch=1）→ 执行后 B active → 再安装目标=B 拒收。
        let ok_plan = crate::program_timeline::ProgramTimelinePlan {
            switch_epoch: 1,
            ..plan
        };
        adapter
            .install_timeline_transition(&graph, &ok_plan)
            .expect("pre-flip 合法安装");
        do_switch(&mut group, &adapter, &graph, b);
        let next = crate::program_timeline::ProgramTimelinePlan {
            switch_epoch: 2,
            ..ok_plan
        };
        assert_eq!(
            adapter.install_timeline_transition(&graph, &next),
            Err(SwitchError::TargetAlreadyActive(b)),
            "目标已 active——安装拒收"
        );
    }

    #[test]
    fn switch_rt_02_canonical_order_and_mapped_outlet_close_loop() {
        // C-TIMELINE-01 Mock 闭环（IMP-5 ①-⑩ + Authority 闭合纪律）:
        // 取锚→Authority 声明→pre-flip 安装→翻转→[Segment(B) tick→首枚
        // 映射缓冲 tick]→出口=映射后源流→Authority 校验证据闭合 Preserve。
        let (a, b, mut group, graph, adapter) = running_group_and_graph();
        let mut authority = TimelineAuthority::new(a);

        // ① 采样锚（mock 观测两 tick 建立基线; 锚=下一帧位）。
        let _o1 = adapter.observe(&graph).program;
        let o2 = adapter.observe(&graph).program;
        let prog_v = o2.program_video_pts.expect("program v 在");
        let prog_a = o2.program_audio_pts.expect("program a 在");
        let b_row = o2.input_pts.iter().find(|p| p.device_id == b).unwrap();
        let video_anchors = AnchorPair {
            program_anchor: prog_v + VIDEO_PTS_STEP,
            source_anchor: b_row.video_pts.expect("B v 在") + VIDEO_PTS_STEP,
        };
        let audio_anchors = AnchorPair {
            program_anchor: prog_a + AUDIO_PTS_STEP,
            source_anchor: b_row.audio_pts.expect("B a 在") + AUDIO_PTS_STEP,
        };
        // ② Authority 声明（连续性基准=已观测 program 位置）。
        authority
            .on_program_pts(MediaPlane::Video, prog_v)
            .expect("基准 video");
        authority
            .on_program_pts(MediaPlane::Audio, prog_a)
            .expect("基准 audio");
        let plan = authority
            .declare_transition(b, 1, video_anchors, audio_anchors)
            .expect("声明");
        // ③ pre-flip 安装。
        adapter
            .install_timeline_transition(&graph, &plan)
            .expect("安装");
        // ④ 翻转（显式 Intent 链——timeline 不改切换入口）。
        do_switch(&mut group, &adapter, &graph, b);

        // ⑤ 翻转后首个 observe tick: Segment(B) 出现但**无缓冲交付**——
        // 生效边界=下一缓冲（F6; timeline 行仍诚实缺席）。
        let boundary = adapter.observe(&graph);
        assert_eq!(boundary.program.observed_active, Some(b));
        assert_eq!(
            boundary.program.program_video_frames, o2.program_video_frames,
            "边界 tick 无缓冲交付"
        );
        assert_eq!(boundary.timeline.mapped_program_pts, None);
        assert_eq!(
            boundary.timeline.video_continuity,
            PlaneContinuity::Unproven
        );

        // ⑥⑦ 下一 tick: 首枚 B 缓冲按声明映射施加——出口=f(source)。
        let mapped = adapter.observe(&graph);
        let b_now = mapped
            .program
            .input_pts
            .iter()
            .find(|p| p.device_id == b)
            .unwrap();
        let expect_v = plan
            .video
            .map_pts(b_now.video_pts.expect("B v 在"))
            .expect("映射可表示");
        assert_eq!(
            mapped.program.program_video_pts,
            Some(expect_v),
            "出口=映射后源流（F5 语义）"
        );
        assert!(expect_v > prog_v, "映射后连续（首帧落锚≥旧位置）");
        // timeline 证据行成事实。
        assert_eq!(mapped.timeline.program_epoch, plan.video.program_epoch);
        assert_eq!(mapped.timeline.source_id, Some(b));
        assert_eq!(mapped.timeline.segment_id, Some(plan.video.segment_id));
        assert_eq!(mapped.timeline.input_pts, b_now.video_pts);
        assert_eq!(mapped.timeline.mapped_program_pts, Some(expect_v));
        assert_eq!(mapped.timeline.mapping_offset, Some(plan.video.offset));
        assert_eq!(
            mapped.timeline.discontinuity_state,
            PtsMonotonicity::DiscontinuityDeclared
        );
        assert_eq!(
            mapped.timeline.video_continuity,
            PlaneContinuity::Continuous
        );
        assert_eq!(
            mapped.timeline.audio_continuity,
            PlaneContinuity::Continuous
        );

        // ⑧ Authority 闭合（证据=adapter 观测实况——非命令回显）。
        authority.on_switch_executed(1).expect("④ 确认");
        authority
            .on_segment_event(MediaPlane::Video, b)
            .expect("⑤ video");
        authority
            .on_segment_event(MediaPlane::Audio, b)
            .expect("⑤ audio");
        authority
            .on_mapped_buffer(
                MediaPlane::Video,
                b,
                b_now.video_pts.expect("B v 在"),
                expect_v,
            )
            .expect("⑥⑦ video");
        let b_audio_now = b_now.audio_pts.expect("B a 在");
        let expect_a = plan.audio.map_pts(b_audio_now).expect("映射可表示");
        authority
            .on_mapped_buffer(MediaPlane::Audio, b, b_audio_now, expect_a)
            .expect("⑥⑦ audio");
        // ⑨⑩ settle → Stable(B), Preserve（epoch 不变——终裁 §十一）。
        let outcome = authority.confirm_settled().expect("settle");
        assert!(matches!(outcome, TransitionOutcome::Preserved { .. }));
        assert_eq!(authority.epoch(), crate::program_timeline::ProgramEpoch(0));
        assert_eq!(
            authority.phase(),
            &crate::program_timeline::TimelinePhase::Stable { source: b }
        );
    }

    /// R58 步骤6 M1 装置: 段 #1（旧段）已生效的映射流在流（B active,
    /// epoch 1; 声明段经 SourceSegment 直构——Authority 不参与, 本组测试
    /// 聚焦 adapter 交错面）。
    fn m1_old_segment_rig() -> (
        Uuid,
        Uuid,
        ExecutionGroup,
        PipelineHandle,
        MockSwitchExecutionAdapter,
    ) {
        let (a, b, mut group, graph, adapter) = running_group_and_graph();
        let _ = adapter.observe(&graph).program;
        let o2 = adapter.observe(&graph).program;
        let b_row = o2
            .input_pts
            .iter()
            .find(|p| p.device_id == b)
            .expect("B 行");
        let seg1_v = SourceSegment::declare(
            b,
            ProgramEpoch(0),
            SegmentId(1),
            AnchorPair {
                program_anchor: o2.program_video_pts.expect("v") + VIDEO_PTS_STEP,
                source_anchor: b_row.video_pts.expect("bv") + VIDEO_PTS_STEP,
            },
        )
        .expect("段#1 video");
        let seg1_a = SourceSegment::declare(
            b,
            ProgramEpoch(0),
            SegmentId(1),
            AnchorPair {
                program_anchor: o2.program_audio_pts.expect("a") + AUDIO_PTS_STEP,
                source_anchor: b_row.audio_pts.expect("ba") + AUDIO_PTS_STEP,
            },
        )
        .expect("段#1 audio");
        adapter
            .install_timeline_transition(
                &graph,
                &ProgramTimelinePlan {
                    target: b,
                    switch_epoch: 1,
                    video: seg1_v,
                    audio: seg1_a,
                },
            )
            .expect("安装#1");
        do_switch(&mut group, &adapter, &graph, b);
        let _seg_tick = adapter.observe(&graph);
        let mapped1 = adapter.observe(&graph).program;
        assert_eq!(
            mapped1.program_video_pts_state,
            PtsMonotonicity::DiscontinuityDeclared,
            "#1 首枚映射=干净声明边界（装置前提）"
        );
        assert!(group.complete_switch(b), "#1 Observed=B 落定 Desired");
        (a, b, group, graph, adapter)
    }

    #[test]
    fn switch_rt_03_m1_no_fence_straggler_reproduces_nonmonotonic() {
        // R58 步骤6 T-M1-FAIL（协议级·无 Fence = pre-R58 编排序, 不 arm
        // 不 release）: [锚采样→install] 竞态窗窜帧 plain 写弧推进程序基线
        // → 新段首枚映射边界帧落回退位 → NonMonotonic sticky——M1 在
        // mock 的确定性复现（真实侧=switch_graph m1 红测在案）。
        // 窜帧关系同构: mock tick 模型边界帧带 2-tick 前导（segment tick+
        // first-mapped tick）→ 边界=P+2 步长; 真实 M1 窜帧恒领先边界一帧
        // （P+40ms vs P 零间隙）——窜帧=边界+1 帧。
        let (a, _b, mut group, graph, adapter) = m1_old_segment_rig();
        // ①a 基准观测（无 arm——基线随本 tick 续流推进后读取）。
        let pre = adapter.observe(&graph).program;
        let p_v = pre.program_video_pts.expect("P_v");
        let p_a = pre.program_audio_pts.expect("P_a");
        let boundary_v = p_v + 2 * VIDEO_PTS_STEP;
        let boundary_a = p_a + 2 * AUDIO_PTS_STEP;
        // ①c 锚采样（AnchorSampled 落点——控制线程）。
        let anchors = adapter.sample_switch_anchors(&graph, a).expect("锚");
        // 竞态窗窜帧（数据线程事件直投——open 门 plain 写弧=基线被推进）。
        adapter.deliver_window_straggler(
            &graph,
            boundary_v + VIDEO_PTS_STEP,
            boundary_a + AUDIO_PTS_STEP,
        );
        // ②③ 声明段 #2 + install（InstallNew）。
        let seg2_v =
            SourceSegment::declare(a, ProgramEpoch(0), SegmentId(2), anchors.video).expect("段#2v");
        let seg2_a =
            SourceSegment::declare(a, ProgramEpoch(0), SegmentId(2), anchors.audio).expect("段#2a");
        adapter
            .install_timeline_transition(
                &graph,
                &ProgramTimelinePlan {
                    target: a,
                    switch_epoch: 2,
                    video: seg2_v,
                    audio: seg2_a,
                },
            )
            .expect("安装#2");
        // ④ 翻转（SwitchNew）。
        do_switch(&mut group, &adapter, &graph, a);
        // ⑤ Segment tick → ⑥ 首枚映射（边界帧）。
        let _ = adapter.observe(&graph);
        let mapped2 = adapter.observe(&graph).program;
        assert_eq!(
            mapped2.program_video_pts,
            Some(boundary_v),
            "边界帧=P+2 步长（mock 模型前导）"
        );
        assert_eq!(
            mapped2.program_video_pts_state,
            PtsMonotonicity::NonMonotonic,
            "无 Fence→M1 FAIL: 窜帧推进的基线(边界+1帧) 越过边界帧 → 违例边界 NM"
        );
        assert_eq!(
            mapped2.program_audio_pts_state,
            PtsMonotonicity::NonMonotonic,
            "audio 面同判（V+A 成对语义）"
        );
        // sticky: 段内后续单调帧不恢复（R53 生命周期镜像）。
        let next = adapter.observe(&graph).program;
        assert_eq!(
            next.program_video_pts_state,
            PtsMonotonicity::NonMonotonic,
            "NM sticky——普通单调帧不自动恢复"
        );
        // 交错日志: #2 窗口生产序五事件; 无 Fence 全程无 OldBufferDropped/
        // FenceConfirmed（窜帧直写观测——穿透路径在案）。
        let log = adapter.cutover_interleave_log(&graph);
        assert!(!log.contains(&CutoverInterleaveEvent::OldBufferDropped));
        assert!(
            !log.iter()
                .any(|e| matches!(e, CutoverInterleaveEvent::FenceConfirmed { .. })),
            "无 Fence→无确认事件"
        );
        let tail_start = log
            .iter()
            .rposition(|e| matches!(e, CutoverInterleaveEvent::AnchorSampled { .. }))
            .expect("#2 AnchorSampled 在");
        let tail = &log[tail_start..];
        assert_eq!(
            tail.len(),
            5,
            "AnchorSampled→OldStraggler→InstallNew→SwitchNew→FirstNewMapped"
        );
        assert!(matches!(
            tail[0],
            CutoverInterleaveEvent::AnchorSampled { .. }
        ));
        assert!(matches!(
            tail[1],
            CutoverInterleaveEvent::OldStraggler { .. }
        ));
        assert!(matches!(tail[2], CutoverInterleaveEvent::InstallNew { .. }));
        assert!(matches!(tail[3], CutoverInterleaveEvent::SwitchNew { .. }));
        assert!(matches!(
            tail[4],
            CutoverInterleaveEvent::FirstNewMapped { .. }
        ));
    }

    #[test]
    fn switch_rt_03_m1_fence_closes_race_boundary_declared() {
        // R58 步骤6 T-M1-PASS（协议级·有 Fence = R58 编排序）: ⓪arm→①a
        // （Armed 期 tick 交付被消费门处置——基线冻结+OldBufferDropped
        // 如实入日志）→①c 锚采样→竞态窗窜帧（armed 门→丢弃+计数, 不写
        // 弧）→install→switch→确认式 Release（mock auto-confirm——协议
        // 强制由真适配器 T-F1/F2/F3 证明）→Segment tick→首枚映射 ≥ 冻结
        // 基线 → 干净声明边界 Discontinuity Declared; 八事件生产序完整
        // 在案（七词汇——arm 清空日志, 恰为该序）。
        let (a, _b, mut group, graph, adapter) = m1_old_segment_rig();
        // ⓪ arm（生产序——①a 之前; 世代 1/计数清零/日志清空）。
        adapter.arm_cutover_fence(&graph).expect("arm");
        // ①a 基准观测（Armed——本 tick 交付被门处置, 基线冻结于 #1 末值）。
        let pre = adapter.observe(&graph).program;
        let p_v = pre.program_video_pts.expect("P_v");
        let p_a = pre.program_audio_pts.expect("P_a");
        let boundary_v = p_v + 2 * VIDEO_PTS_STEP;
        let boundary_a = p_a + 2 * AUDIO_PTS_STEP;
        // ①c 锚采样（AnchorSampled）。
        let anchors = adapter.sample_switch_anchors(&graph, a).expect("锚");
        // 竞态窗窜帧（armed 门→OldStraggler+OldBufferDropped——不写弧）。
        adapter.deliver_window_straggler(
            &graph,
            boundary_v + VIDEO_PTS_STEP,
            boundary_a + AUDIO_PTS_STEP,
        );
        // ②③④ 声明/install/翻转。
        let seg2_v =
            SourceSegment::declare(a, ProgramEpoch(0), SegmentId(2), anchors.video).expect("段#2v");
        let seg2_a =
            SourceSegment::declare(a, ProgramEpoch(0), SegmentId(2), anchors.audio).expect("段#2a");
        adapter
            .install_timeline_transition(
                &graph,
                &ProgramTimelinePlan {
                    target: a,
                    switch_epoch: 2,
                    video: seg2_v,
                    audio: seg2_a,
                },
            )
            .expect("安装#2");
        do_switch(&mut group, &adapter, &graph, a);
        // 确认式 Release（executed 之后——mock auto-confirm; 计数=①a tick
        // v+a + 窜帧 v+a = 4 如实）。
        let ev = adapter
            .release_cutover_fence(&graph, std::time::Duration::from_secs(1))
            .expect("确认式 Release");
        assert_eq!(ev.generation, 1);
        assert_eq!(ev.video.discarded, 2, "video 面=①a tick+窜帧");
        assert_eq!(ev.audio.discarded, 2, "audio 面对称");
        // ⑤ Segment tick → ⑥ 首枚映射（门已开——干净声明边界）。
        let _ = adapter.observe(&graph);
        let mapped2 = adapter.observe(&graph).program;
        assert_eq!(mapped2.program_video_pts, Some(boundary_v));
        assert_eq!(
            mapped2.program_video_pts_state,
            PtsMonotonicity::DiscontinuityDeclared,
            "有 Fence→M1 PASS: 窜帧被处置基线未推进 → 干净声明边界（对照无 Fence=NM）"
        );
        assert_eq!(
            mapped2.program_audio_pts_state,
            PtsMonotonicity::DiscontinuityDeclared
        );
        // 八事件生产序（七词汇——HOLD-1 修复后 ①a Armed tick 门处置
        // 如实入日志为首个事件; arm 清空前无残余）。
        let log = adapter.cutover_interleave_log(&graph);
        assert_eq!(log.len(), 8, "八事件生产序（含 ①a Armed tick 门处置）");
        assert!(matches!(log[0], CutoverInterleaveEvent::OldBufferDropped));
        assert!(matches!(
            log[1],
            CutoverInterleaveEvent::AnchorSampled { .. }
        ));
        assert!(matches!(
            log[2],
            CutoverInterleaveEvent::OldStraggler { .. }
        ));
        assert!(matches!(log[3], CutoverInterleaveEvent::OldBufferDropped));
        assert!(matches!(log[4], CutoverInterleaveEvent::InstallNew { .. }));
        assert!(matches!(log[5], CutoverInterleaveEvent::SwitchNew { .. }));
        assert!(matches!(
            log[6],
            CutoverInterleaveEvent::FenceConfirmed {
                generation: 1,
                discarded: 4
            }
        ));
        assert!(matches!(
            log[7],
            CutoverInterleaveEvent::FirstNewMapped { .. }
        ));
    }

    #[test]
    fn switch_rt_03_m1_armed_gate_precedes_first_mapped_evidence_state() {
        // R58 步骤6 HOLD-1 回归（终裁处方测试）: post-switch 仍 Armed 窗口
        // 内到达的 timeline 缓冲被消费门丢弃时——first_mapped/timeline
        // 证据状态不得推进（被丢弃缓冲不占用"首枚已映射"槽位）; Release
        // 后首枚真正放行的缓冲才成为 FirstNewMapped。序列=install→switch
        // →Armed→缓冲到达→Drop→first_mapped 仍 false→release→下一枚
        // 放行缓冲=FirstNewMapped。修复前该窗口 first_mapped 被丢弃 tick
        // 预占（丢弃不可见+首放行帧走 plain 弧——FirstNewMapped 永不出现
        // 且 timeline 行提前出事实, 本测双断言锁定）。
        let (a, _b, mut group, graph, adapter) = m1_old_segment_rig();
        // ①a 基准观测（未 arm——正常交付, 读基线）。
        let pre = adapter.observe(&graph).program;
        let p_v = pre.program_video_pts.expect("P_v");
        // ①c 锚采样 → ②③ 声明段 #2 + install（InstallNew）。
        let anchors = adapter.sample_switch_anchors(&graph, a).expect("锚");
        let seg2_v =
            SourceSegment::declare(a, ProgramEpoch(0), SegmentId(2), anchors.video).expect("段#2v");
        let seg2_a =
            SourceSegment::declare(a, ProgramEpoch(0), SegmentId(2), anchors.audio).expect("段#2a");
        adapter
            .install_timeline_transition(
                &graph,
                &ProgramTimelinePlan {
                    target: a,
                    switch_epoch: 2,
                    video: seg2_v,
                    audio: seg2_a,
                },
            )
            .expect("安装#2");
        // ⓪ arm（install 后——专测 post-switch Armed 窗口; 生产编排 arm
        // 在 ①a 之前, Armed 横跨 ⓪→release, 窗口语义相同。arm 清空日志
        // ——本测日志恰为窗口四事件）。
        adapter.arm_cutover_fence(&graph).expect("arm");
        // ④ 翻转（SwitchNew; timeline#2 生效）。
        do_switch(&mut group, &adapter, &graph, a);
        // ⑤ Segment tick（EVENT 不受门拦截——segment_seen 照常推进, 无
        // 缓冲交付）。
        let seg_tick = adapter.observe(&graph);
        assert_eq!(
            seg_tick.timeline.mapped_program_pts, None,
            "Segment tick 无缓冲交付"
        );
        // post-switch Armed 窗口: 首枚 timeline#2 缓冲候选到达 → 消费门 Drop。
        let dropped = adapter.observe(&graph);
        assert_eq!(
            dropped.timeline.mapped_program_pts, None,
            "被丢弃缓冲不得成为首映射——first_mapped 未推进（timeline 行仍诚实缺席）"
        );
        assert_eq!(
            dropped.program.program_video_pts,
            Some(p_v),
            "被丢弃缓冲不写程序出口（基线冻结）"
        );
        assert_eq!(
            dropped.program.program_video_frames, pre.program_video_frames,
            "被丢弃缓冲不进帧数"
        );
        // 确认式 Release（mock auto-confirm——本窗口恰一次门丢弃 v+a=2）。
        let ev = adapter
            .release_cutover_fence(&graph, std::time::Duration::from_secs(1))
            .expect("确认式 Release");
        assert_eq!(ev.generation, 1);
        assert_eq!(ev.video.discarded, 1, "窗口内恰一次 video 门丢弃");
        assert_eq!(ev.audio.discarded, 1, "audio 面对称");
        // Release 后首枚真正放行缓冲 = FirstNewMapped + 干净声明边界 DD。
        // 锚采样后 tick 序: Segment tick + dropped tick（设备 PTS 无条件
        // 推进）→ 首放行帧源位=锚源+3 步长 → 映射=P+3 步长（比常规边界
        // 多一个 dropped tick——模型前导如实）。
        let mapped2 = adapter.observe(&graph).program;
        assert_eq!(mapped2.program_video_pts, Some(p_v + 3 * VIDEO_PTS_STEP));
        assert_eq!(
            mapped2.program_video_pts_state,
            PtsMonotonicity::DiscontinuityDeclared,
            "首放行帧=首映射（被丢弃候选未占槽）→ declared 弧 DD（非 plain）"
        );
        assert!(group.complete_switch(a), "#2 Observed=A 落定 Desired");
        // 窗口四事件: SwitchNew→OldBufferDropped（门处置）→FenceConfirmed
        // →FirstNewMapped（修复前: 槽位被丢弃 tick 预占——FirstNewMapped
        // 永不出现且日志无该门处置事件）。
        let log = adapter.cutover_interleave_log(&graph);
        assert_eq!(log.len(), 4, "窗口四事件: {:?}", log);
        assert!(matches!(
            log[0],
            CutoverInterleaveEvent::SwitchNew { epoch: 2 }
        ));
        assert!(matches!(log[1], CutoverInterleaveEvent::OldBufferDropped));
        assert!(matches!(
            log[2],
            CutoverInterleaveEvent::FenceConfirmed {
                generation: 1,
                discarded: 2
            }
        ));
        assert!(matches!(
            log[3],
            CutoverInterleaveEvent::FirstNewMapped { mapped_video } if mapped_video == p_v + 3 * VIDEO_PTS_STEP
        ));
    }

    #[test]
    fn switch_rt_02_legacy_observation_surface_unchanged_without_install() {
        // 未安装 timeline: 既有 observation 逐字段不变 + timeline 行=
        // no_evidence（诚实缺席——不伪造时间线事实）。
        let (_a, b, mut group, graph, adapter) = running_group_and_graph();
        do_switch(&mut group, &adapter, &graph, b);
        let obs = adapter.observe(&graph);
        assert_eq!(obs.program.observed_active, Some(b));
        assert!(obs.program.program_video_pts.is_some());
        assert_eq!(obs.timeline.source_id, None);
        assert_eq!(obs.timeline.mapped_program_pts, None);
        assert_eq!(obs.timeline.discontinuity_state, PtsMonotonicity::Unknown);
        assert_eq!(obs.timeline.video_continuity, PlaneContinuity::Unproven);
        // 键集锁: timeline 行恰十键（Freeze §8 形状防蔓延）。
        let json = serde_json::to_value(&obs.timeline).expect("序列化");
        assert_eq!(json.as_object().expect("对象").len(), 10);
    }
}
