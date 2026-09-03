//! A2-8-01: Mock Switch Execution Adapter——确定性 PTS 流仿真（mock feature）。
//!
//! 与 `MockBackend` 同律: 纯 Rust 零 GStreamer, 解锁 switch 执行面的
//! mock 层验证（T1/T2/T3/T5/T6）。语义模型:
//! - **成对切换**: video+audio 共享单一 active 与 av_epoch——单面切构造不出;
//! - **PTS 连续性**: program 出口 PTS 是**独立再生成流**（FRAME_SWITCH =
//!   RAW→RAW 重新编码平面）, 跨切换单调不回退; 输入 PTS 按设备独立推进;
//! - **停滞注入**: `stall()` 冻结指定设备帧计数（Observation 事实位,
//!   故障结论归 fold/Custody）。

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use crate::contracts::switch::{
    FrameBoundary, InputPts, ProgramObservation, SwitchExecuted, SwitchExecutionAdapter,
};
use crate::pipeline::{PipelineHandle, PtsMonotonicity, NEXT_PIPELINE_ID};
use crate::program::SwitchPolicy;
use crate::switch_execution::{ExecutionGroup, SwitchDesired, SwitchError, SwitchExecutionPlan};
use uuid::Uuid;

/// 每观测 tick 的 video PTS 推进量（25fps 帧间隔, ms 时基）。
const VIDEO_PTS_STEP: u64 = 40;
/// 每观测 tick 的 audio PTS 推进量。
const AUDIO_PTS_STEP: u64 = 20;

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
        // Program 出口: 独立再生成流——active 存活且未停滞才推进。
        let alive = graph
            .active
            .map(|d| graph.started && !graph.stalled.contains(&d))
            .unwrap_or(false);
        if alive {
            graph.program_pts = (
                graph.program_pts.0 + VIDEO_PTS_STEP,
                graph.program_pts.1 + AUDIO_PTS_STEP,
            );
            graph.program_frames = (graph.program_frames.0 + 1, graph.program_frames.1 + 1);
        }
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
        // 成对切换: video+audio 共享 active/av_epoch（单字段承载——单面切
        // 在本模型中不可构造, 方案 A 语义由结构保证）。
        g.active = Some(plan.target);
        g.av_epoch = plan.epoch;
        Ok(SwitchExecuted {
            boundary: FrameBoundary::FrameAligned,
            av_epoch: g.av_epoch,
        })
    }

    fn observe(&self, graph: &PipelineHandle) -> ProgramObservation {
        let mut graphs = self.graphs.lock().unwrap();
        let Some(g) = graphs.get_mut(graph) else {
            return ProgramObservation {
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
            };
        };
        Self::tick_once(g);
        let running = g.started;
        ProgramObservation {
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
        }
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
        let obs = adapter.observe(&graph);
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
        let obs = adapter.observe(&graph);
        assert!(obs.program_video_pts.is_some() && obs.program_audio_pts.is_some());
        let devices: Vec<Uuid> = obs.input_pts.iter().map(|p| p.device_id).collect();
        assert!(devices.contains(&a) && devices.contains(&b));
    }

    #[test]
    fn switch_rt_01_explicit_switch_flips_observed_active() {
        // T3: A→B→A 真实执行切换——adapter 内部 active 实态翻转
        // （Observed 独立读数）, 非 Rust 状态字段回显。
        let (a, b, mut group, graph, adapter) = running_group_and_graph();
        assert_eq!(adapter.observe(&graph).observed_active, Some(a));

        let e1 = do_switch(&mut group, &adapter, &graph, b);
        assert_eq!(adapter.observe(&graph).observed_active, Some(b));
        assert_eq!(e1.av_epoch, 1);
        assert!(group.complete_switch(b), "Observed=B 应落定 Desired");

        let e2 = do_switch(&mut group, &adapter, &graph, a);
        assert_eq!(adapter.observe(&graph).observed_active, Some(a));
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
        let obs = adapter.observe(&graph);
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
        let obs = adapter.observe(&graph);
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
        // 平面——出口独立流）。
        let (_a, b, mut group, graph, adapter) = running_group_and_graph();
        let v0 = adapter.observe(&graph).program_video_pts.unwrap();
        let a0 = adapter.observe(&graph).program_audio_pts.unwrap();
        do_switch(&mut group, &adapter, &graph, b);
        let after = adapter.observe(&graph);
        assert!(after.program_video_pts.unwrap() > v0, "video PTS 不得回退");
        assert!(after.program_audio_pts.unwrap() > a0, "audio PTS 不得回退");
        assert_eq!(
            after.program_video_pts_state,
            PtsMonotonicity::ValidMonotonic
        );
        let again = adapter.observe(&graph);
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
        let before = adapter.observe(&graph);
        let after = adapter.observe(&graph);
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
        let obs = adapter.observe(&graph);
        assert_eq!(obs.observed_active, Some(a), "observe 零副作用不切换");
        let plan = group
            .plan_switch(&crate::switch_execution::SwitchIntent {
                target: b,
                policy: SwitchPolicy::FrameSwitch,
            })
            .expect("显式 Intent 产出 Plan");
        group.begin_switch(&plan).expect("begin");
        adapter.switch(&graph, &plan).expect("执行");
        let obs2 = adapter.observe(&graph);
        assert_eq!(obs2.observed_active, Some(b), "仅显式链生效");
        assert!(!group.complete_switch(a), "旧源回显不落定");
        assert!(group.complete_switch(b), "Observed=B 落定");
    }
}
