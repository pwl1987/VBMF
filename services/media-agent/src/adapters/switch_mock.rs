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
    FrameBoundary, InputPts, ProgramExecutionObservation, ProgramObservation, SwitchExecuted,
    SwitchExecutionAdapter,
};
use crate::pipeline::{PipelineHandle, PtsMonotonicity, NEXT_PIPELINE_ID};
use crate::program::SwitchPolicy;
use crate::program_timeline::{PlaneContinuity, ProgramTimelinePlan, TimelineObservation};
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
    /// C-TIMELINE-01: 已安装 timeline 声明（None=legacy 独立再生成流模式）。
    timeline: Option<MockTimelineState>,
}

impl MockGraph {
    /// 确定性初值: 首源 video=1000/audio=800, 次源 ×2——两源流天然可区分。
    fn device_base(device: &Uuid, devices: &[Uuid; 2]) -> (u64, u64) {
        let idx = if devices[0] == *device { 0 } else { 1 };
        (1000 * (idx as u64 + 1), 800 * (idx as u64 + 1))
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
            {
                let t = graph.timeline.as_mut().expect("timeline_active");
                if !t.segment_seen {
                    // ⑤ 边界 tick: Segment(B) 事件先于首枚 B 缓冲——本 tick
                    // 出口不交付缓冲（生效边界=下一缓冲, F6 微观序）。
                    t.segment_seen = true;
                    return;
                }
                t.first_mapped = true;
            }
            let active = graph.active.expect("alive ⇒ active");
            let (v, a) = graph.pts[&active];
            let (video_seg, audio_seg) = {
                let t = graph.timeline.as_ref().expect("timeline_active");
                (t.plan.video, t.plan.audio)
            };
            graph.program_pts = (
                video_seg
                    .map_pts(v)
                    .expect("mock 映射偏移在可表示范围（测试锚构造级）"),
                audio_seg
                    .map_pts(a)
                    .expect("mock 映射偏移在可表示范围（测试锚构造级）"),
            );
            graph.program_frames = (graph.program_frames.0 + 1, graph.program_frames.1 + 1);
            return;
        }
        // legacy: 独立再生成流——跨切换单调不回退。
        graph.program_pts = (
            graph.program_pts.0 + VIDEO_PTS_STEP,
            graph.program_pts.1 + AUDIO_PTS_STEP,
        );
        graph.program_frames = (graph.program_frames.0 + 1, graph.program_frames.1 + 1);
    }

    /// 停滞注入（测试钩子: 冻结该设备 PTS/帧计数; program 出口在其为
    /// active 时随之冻结——starvation 仿真）。
    pub fn stall(&self, graph: &PipelineHandle, device_id: Uuid) {
        if let Some(g) = self.graphs.lock().unwrap().get_mut(graph) {
            g.stalled.insert(device_id);
        }
    }
}

impl SwitchExecutionAdapter for MockSwitchExecutionAdapter {
    fn build_program_graph(&self, group: &ExecutionGroup) -> Result<PipelineHandle, SwitchError> {
        let initial_active = match group.desired {
            SwitchDesired::ActiveInput(active) => active,
            switching @ SwitchDesired::Switching { .. } => {
                return Err(SwitchError::NotActiveSource(switching))
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
            timeline: None,
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
        if plan.switch_epoch != g.av_epoch + 1 {
            return Err(SwitchError::StalePlanEpoch {
                got: plan.switch_epoch,
                expected: g.av_epoch + 1,
            });
        }
        g.timeline = Some(MockTimelineState {
            plan: *plan,
            segment_seen: false,
            first_mapped: false,
        });
        Ok(())
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
        if plan.epoch != g.av_epoch + 1 {
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
                timeline: TimelineObservation::no_evidence(MockGraph::now_ms()),
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
            program_video_pts_state: if running {
                PtsMonotonicity::ValidMonotonic
            } else {
                PtsMonotonicity::Unknown
            },
            program_audio_pts_state: if running {
                PtsMonotonicity::ValidMonotonic
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
            TimelineObservation::no_evidence(MockGraph::now_ms())
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
    use crate::program_timeline::{AnchorPair, MediaPlane, TimelineAuthority, TransitionOutcome};
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
        // pre-flip 安装纪律（IMP-5 ③）: 声明 epoch 必须=下一次执行; 目标
        // 已 active 拒收; 未运行 graph 拒收。
        let (_a, b, mut group, graph, adapter) = running_group_and_graph();
        let plan = crate::program_timeline::ProgramTimelinePlan {
            target: b,
            switch_epoch: 9, // ≠ av_epoch+1=1
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
                got: 9,
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
